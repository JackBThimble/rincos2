use bootabi::{BootInfo, MemMapEntry, MemType};
use hal::mmu::{
    AddressSpace, MapError, MapFlags, Mmu, PageTableFrameAlloc, PhysAddr, TranslateError, VirtAddr,
};
use spin::Mutex;

const PAGE_SIZE: u64 = 4096;
const MAX_FREE_FRAMES: usize = 128;

const EMPTY_PADDR: PhysAddr = PhysAddr(0);

struct BootPtAlloc {
    entries_ptr: u64,
    entry_count: usize,
    cur_entry: usize,
    cur_addr: u64,
    hhdm_offset: u64,
    free_list: [PhysAddr; MAX_FREE_FRAMES],
    free_count: usize,
}

impl BootPtAlloc {
    fn new(boot: &BootInfo) -> Option<Self> {
        if boot.hhdm_offset == 0 {
            return None;
        }
        if boot.mem.entries_ptr == 0 || boot.mem.entry_count == 0 {
            return None;
        }
        if boot.mem.entry_size as usize != core::mem::size_of::<MemMapEntry>() {
            return None;
        }

        Some(Self {
            entries_ptr: boot.mem.entries_ptr,
            entry_count: boot.mem.entry_count as usize,
            cur_entry: 0,
            cur_addr: 0,
            hhdm_offset: boot.hhdm_offset,
            free_list: [EMPTY_PADDR; MAX_FREE_FRAMES],
            free_count: 0,
        })
    }

    fn entry_at(&self, idx: usize) -> Option<MemMapEntry> {
        if idx >= self.entry_count {
            return None;
        }

        let ptr = self.entries_ptr as *const MemMapEntry;
        let entry = unsafe { *ptr.add(idx) };
        Some(entry)
    }

    fn next_from_map(&mut self) -> Option<PhysAddr> {
        while self.cur_entry < self.entry_count {
            let entry = self.entry_at(self.cur_entry)?;
            if !is_usable(entry.mem_type) {
                self.cur_entry += 1;
                self.cur_addr = 0;
                continue;
            }

            let base = entry.base.0;
            let end = base.checked_add(entry.len)?;
            let mut addr = if self.cur_addr < base {
                base
            } else {
                self.cur_addr
            };

            addr = align_up(addr, PAGE_SIZE);
            if addr.checked_add(PAGE_SIZE)? > end {
                self.cur_entry += 1;
                self.cur_addr = 0;
                continue;
            }

            self.cur_addr = addr + PAGE_SIZE;
            return Some(PhysAddr(addr));
        }

        None
    }

    fn zero_frame(&self, paddr: PhysAddr) {
        let virt = paddr.0.wrapping_add(self.hhdm_offset) as *mut u8;
        unsafe {
            core::ptr::write_bytes(virt, 0, PAGE_SIZE as usize);
        }
    }
}

impl PageTableFrameAlloc for BootPtAlloc {
    fn alloc_frame_4k(&mut self) -> Option<PhysAddr> {
        let paddr = if self.free_count > 0 {
            self.free_count -= 1;
            self.free_list[self.free_count]
        } else {
            self.next_from_map()?
        };

        self.zero_frame(paddr);
        Some(paddr)
    }

    fn free_frame_4k(&mut self, paddr: PhysAddr) {
        if self.free_count < MAX_FREE_FRAMES {
            self.free_list[self.free_count] = paddr;
            self.free_count += 1;
        }
    }
}

static PT_ALLOC: Mutex<Option<BootPtAlloc>> = Mutex::new(None);

pub fn init(boot: &BootInfo) {
    let alloc = BootPtAlloc::new(boot).expect("vm: missing HHDM or memmap");
    *PT_ALLOC.lock() = Some(alloc);

    unsafe {
        let _ = crate::arch::mmu()
            .init_kernel()
            .expect("vm: mmu kernel init failed");
    }
}

pub fn new_address_space() -> Result<&'static mut AddressSpace, MapError> {
    let mut guard = PT_ALLOC.lock();
    let alloc = guard.as_mut().expect("vm: not initialized");
    unsafe { crate::arch::mmu().address_space_new(alloc) }
}

pub fn destroy_address_space(aspace: &'static mut AddressSpace) {
    let mut guard = PT_ALLOC.lock();
    let alloc = guard.as_mut().expect("vm: not initialized");
    unsafe {
        crate::arch::mmu().address_space_destroy(alloc, aspace);
    }
}

pub fn map_4k(
    aspace: &mut AddressSpace,
    vaddr: VirtAddr,
    paddr: PhysAddr,
    flags: MapFlags,
) -> Result<(), MapError> {
    let mut guard = PT_ALLOC.lock();
    let alloc = guard.as_mut().expect("vm: not initialized");
    unsafe { crate::arch::mmu().map_4k(alloc, aspace, vaddr, paddr, flags) }
}

pub fn unmap_4k(aspace: &mut AddressSpace, vaddr: VirtAddr) -> Result<(), MapError> {
    unsafe { crate::arch::mmu().unmap_4k(aspace, vaddr) }
}

pub fn protect_4k(
    aspace: &mut AddressSpace,
    vaddr: VirtAddr,
    flags: MapFlags,
) -> Result<(), MapError> {
    unsafe { crate::arch::mmu().protect_4k(aspace, vaddr, flags) }
}

pub fn translate(aspace: &AddressSpace, vaddr: VirtAddr) -> Result<PhysAddr, TranslateError> {
    unsafe { crate::arch::mmu().translate(aspace, vaddr) }
}

fn is_usable(mem_type: MemType) -> bool {
    matches!(mem_type, MemType::Usable | MemType::BootloaderReclaimable)
}

fn align_up(value: u64, align: u64) -> u64 {
    debug_assert!(align.is_power_of_two());
    (value + align - 1) & !(align - 1)
}
