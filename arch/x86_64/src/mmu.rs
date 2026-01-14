use core::cell::SyncUnsafeCell;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, Ordering};

use hal::mmu::{
    AddressSpace, MapError, MapFlags, Mmu, PageTableFrameAlloc, PhysAddr, TranslateError, VirtAddr,
};

use crate::{cpuid, hhdm_offset, msr};

pub struct X86Mmu;

pub static MMU: X86Mmu = X86Mmu;

/// Internal arch-owned address space object.
/// Kernel never sees this layout.
#[repr(C)]
#[derive(Clone, Copy)]
struct X86AddressSpace {
    pml4_phys: PhysAddr,
}

static KAS: SyncUnsafeCell<Option<X86AddressSpace>> = SyncUnsafeCell::new(None);
static KAS_HANDLE: SyncUnsafeCell<Option<AddressSpace>> = SyncUnsafeCell::new(None);
static CURRENT: SyncUnsafeCell<Option<AddressSpace>> = SyncUnsafeCell::new(None);

const MAX_ADDRESS_SPACES: usize = 64;

const PAGE_SIZE: u64 = 4096;
const ENTRY_COUNT: usize = 512;
const KERNEL_PML4_START: usize = 256;

const ADDR_MASK: u64 = 0x000f_ffff_ffff_f000;

const PTE_P: u64 = 1 << 0;
const PTE_W: u64 = 1 << 1;
const PTE_U: u64 = 1 << 2;
const PTE_PWT: u64 = 1 << 3;
const PTE_PCD: u64 = 1 << 4;
const PTE_PS: u64 = 1 << 7;
const PTE_G: u64 = 1 << 8;
const PTE_NX: u64 = 1 << 63;

const CR0_WP: u64 = 1 << 16;
const CR4_PGE: u64 = 1 << 7;
const EFER_NXE: u64 = 1 << 11;

static NXE_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
struct AddressSpaceSlot {
    used: bool,
    space: X86AddressSpace,
    handle: AddressSpace,
}

const EMPTY_HANDLE: AddressSpace = unsafe { AddressSpace::from_ptr(NonNull::dangling()) };
const EMPTY_SLOT: AddressSpaceSlot = AddressSpaceSlot {
    used: false,
    space: X86AddressSpace {
        pml4_phys: PhysAddr(0),
    },
    handle: EMPTY_HANDLE,
};

static AS_SLOTS: SyncUnsafeCell<[AddressSpaceSlot; MAX_ADDRESS_SPACES]> =
    SyncUnsafeCell::new([EMPTY_SLOT; MAX_ADDRESS_SPACES]);

#[inline(always)]
fn aligned_4k(x: u64) -> bool {
    (x & (PAGE_SIZE - 1)) == 0
}

#[inline(always)]
fn is_canonical(addr: u64) -> bool {
    let sign = (addr >> 47) & 1;
    let upper = addr >> 48;
    if sign == 0 {
        upper == 0
    } else {
        upper == 0xffff
    }
}

#[inline(always)]
fn pml4_index(v: u64) -> usize {
    ((v >> 39) & 0x1ff) as usize
}
#[inline(always)]
fn pdpt_index(v: u64) -> usize {
    ((v >> 30) & 0x1ff) as usize
}
#[inline(always)]
fn pd_index(v: u64) -> usize {
    ((v >> 21) & 0x1ff) as usize
}
#[inline(always)]
fn pt_index(v: u64) -> usize {
    ((v >> 12) & 0x1ff) as usize
}

#[inline(always)]
fn phys_to_virt(p: PhysAddr) -> *mut u64 {
    (p.0 + hhdm_offset()) as *mut u64
}

#[inline(always)]
fn nxe_enabled() -> bool {
    NXE_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
unsafe fn read_cr3() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) v, options(nostack, nomem, preserves_flags));
    }
    v
}

pub unsafe fn init_features(enable_nx: bool) {
    let mut cr0: u64;
    let mut cr4: u64;
    unsafe {
        core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nostack, nomem, preserves_flags));
        core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nostack, nomem, preserves_flags));
    }

    cr0 |= CR0_WP;
    cr4 |= CR4_PGE;

    unsafe {
        core::arch::asm!("mov cr0, {}", in(reg) cr0, options(nostack, nomem, preserves_flags));
        core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nostack, nomem, preserves_flags));
    }

    let mut nxe = false;
    if enable_nx && cpuid::has_nx() {
        let mut efer = unsafe { msr::rdmsr(msr::IA32_EFER) };
        efer |= EFER_NXE;
        unsafe {
            msr::wrmsr(msr::IA32_EFER, efer);
        }
        nxe = true;
    }

    NXE_ENABLED.store(nxe, Ordering::Relaxed);
}

#[inline(always)]
unsafe fn zero_frame(p: PhysAddr) {
    let ptr = phys_to_virt(p);
    unsafe {
        core::ptr::write_bytes(ptr, 0, (PAGE_SIZE / 8) as usize);
    }
}

#[inline(always)]
fn flags_to_pte(flags: MapFlags) -> u64 {
    let mut p = PTE_P;

    if flags.contains(MapFlags::WRITE) {
        p |= PTE_W;
    }
    if flags.contains(MapFlags::USER) {
        p |= PTE_U;
    }
    if flags.contains(MapFlags::GLOBAL) {
        p |= PTE_G;
    }

    if !flags.contains(MapFlags::EXEC) && nxe_enabled() {
        p |= PTE_NX;
    }

    if flags.contains(MapFlags::UNCACHED) {
        p |= PTE_PCD | PTE_PWT;
    }

    p
}

/// Returns mutable reference to a 512-entry table at physical address.
#[inline(always)]
unsafe fn table_mut(p: PhysAddr) -> &'static mut [u64; ENTRY_COUNT] {
    unsafe { &mut *(phys_to_virt(p) as *mut [u64; ENTRY_COUNT]) }
}

/// Allocate next-level table if missing; return physical address of that table
unsafe fn ensure_table(
    pt_alloc: &mut dyn PageTableFrameAlloc,
    parent: &mut [u64; ENTRY_COUNT],
    idx: usize,
    user: bool,
) -> Result<PhysAddr, MapError> {
    let entry = parent[idx];

    if (entry & PTE_P) != 0 {
        return Ok(PhysAddr(entry & ADDR_MASK));
    }

    let frame = pt_alloc.alloc_frame_4k().ok_or(MapError::OutOfMemory)?;
    unsafe {
        zero_frame(frame);
    }

    let mut flags = PTE_P | PTE_W;
    if user {
        flags |= PTE_U;
    }

    parent[idx] = (frame.0 & ADDR_MASK) | flags;

    Ok(frame)
}

unsafe fn alloc_slot(pml4_phys: PhysAddr) -> Result<&'static mut AddressSpace, MapError> {
    let slots = unsafe { &mut *AS_SLOTS.get() };
    for slot in slots.iter_mut() {
        if !slot.used {
            slot.used = true;
            slot.space.pml4_phys = pml4_phys;
            let handle_ptr = NonNull::new(&mut slot.space as *mut _ as *mut ()).unwrap();
            slot.handle = unsafe { AddressSpace::from_ptr(handle_ptr) };
            return Ok(&mut slot.handle);
        }
    }

    Err(MapError::OutOfMemory)
}

unsafe fn free_slot(aspace: &AddressSpace) {
    let slots = unsafe { &mut *AS_SLOTS.get() };
    let needle = aspace.as_ptr().as_ptr();
    for slot in slots.iter_mut() {
        if slot.used && slot.handle.as_ptr().as_ptr() == needle {
            slot.used = false;
            slot.space.pml4_phys = PhysAddr(0);
            return;
        }
    }
}

fn as_x86(aspace: &AddressSpace) -> &'static X86AddressSpace {
    let ptr = aspace.as_ptr().as_ptr() as *const X86AddressSpace;
    unsafe { &*ptr }
}

fn as_x86_mut(aspace: &mut AddressSpace) -> &'static mut X86AddressSpace {
    let ptr = aspace.as_ptr().as_ptr() as *mut X86AddressSpace;
    unsafe { &mut *ptr }
}

impl Mmu for X86Mmu {
    unsafe fn init_kernel(&self) -> Result<&'static mut AddressSpace, MapError> {
        unsafe {
            if let Some(h) = (*KAS_HANDLE.get()).as_mut() {
                return Ok(h);
            }

            let cr3 = read_cr3() & ADDR_MASK;
            if cr3 == 0 {
                return Err(MapError::InvalidArgs);
            }

            let root = PhysAddr(cr3);
            *KAS.get() = Some(X86AddressSpace { pml4_phys: root });

            let kas_ref: &'static mut X86AddressSpace = (*KAS.get()).as_mut().unwrap();
            let handle_ptr = NonNull::new(kas_ref as *mut _ as *mut ()).unwrap();
            *KAS_HANDLE.get() = Some(AddressSpace::from_ptr(handle_ptr));
            let h: &'static mut AddressSpace = (*KAS_HANDLE.get()).as_mut().unwrap();

            *CURRENT.get() = Some(*h);

            Ok(h)
        }
    }

    unsafe fn address_space_new(
        &self,
        pt_alloc: &mut dyn PageTableFrameAlloc,
    ) -> Result<&'static mut AddressSpace, MapError> {
        unsafe {
            let _ = self.init_kernel()?;
            let kernel_root = (*KAS.get())
                .as_ref()
                .ok_or(MapError::InvalidArgs)?
                .pml4_phys;

            let pml4 = pt_alloc.alloc_frame_4k().ok_or(MapError::OutOfMemory)?;
            zero_frame(pml4);

            let src = phys_to_virt(kernel_root) as *const u64;
            let dst = phys_to_virt(pml4) as *mut u64;
            core::ptr::copy_nonoverlapping(
                src.add(KERNEL_PML4_START),
                dst.add(KERNEL_PML4_START),
                ENTRY_COUNT - KERNEL_PML4_START,
            );

            alloc_slot(pml4)
        }
    }

    unsafe fn address_space_destroy(
        &self,
        pt_alloc: &mut dyn PageTableFrameAlloc,
        aspace: &'static mut AddressSpace,
    ) {
        unsafe {
            if let Some(kas) = (*KAS_HANDLE.get()).as_ref() {
                if kas.as_ptr().as_ptr() == aspace.as_ptr().as_ptr() {
                    return;
                }
            }

            let root = as_x86(aspace).pml4_phys;
            let pml4 = table_mut(root);
            for e1 in pml4[..KERNEL_PML4_START].iter() {
                if (e1 & PTE_P) == 0 {
                    continue;
                }
                if (e1 & PTE_PS) != 0 {
                    continue;
                }
                let pdpt_phys = PhysAddr(e1 & ADDR_MASK);
                let pdpt = table_mut(pdpt_phys);
                for e2 in pdpt.iter() {
                    if (e2 & PTE_P) == 0 {
                        continue;
                    }
                    if (e2 & PTE_PS) != 0 {
                        continue;
                    }
                    let pd_phys = PhysAddr(e2 & ADDR_MASK);
                    let pd = table_mut(pd_phys);
                    for e3 in pd.iter() {
                        if (e3 & PTE_P) == 0 {
                            continue;
                        }
                        if (e3 & PTE_PS) != 0 {
                            continue;
                        }
                        let pt_phys = PhysAddr(e3 & ADDR_MASK);
                        pt_alloc.free_frame_4k(pt_phys);
                    }
                    pt_alloc.free_frame_4k(pd_phys);
                }
                pt_alloc.free_frame_4k(pdpt_phys);
            }

            pt_alloc.free_frame_4k(root);
            free_slot(aspace);
        }
    }

    unsafe fn map_4k(
        &self,
        pt_alloc: &mut dyn PageTableFrameAlloc,
        aspace: &mut AddressSpace,
        vaddr: VirtAddr,
        paddr: PhysAddr,
        flags: MapFlags,
    ) -> Result<(), MapError> {
        if !aligned_4k(vaddr.0) || !aligned_4k(paddr.0) {
            return Err(MapError::Unaligned);
        }
        if !is_canonical(vaddr.0) {
            return Err(MapError::InvalidArgs);
        }

        let user = flags.contains(MapFlags::USER);

        let root = as_x86_mut(aspace).pml4_phys;

        unsafe {
            let pml4 = table_mut(root);

            let pdpt_phys = ensure_table(pt_alloc, pml4, pml4_index(vaddr.0), user)?;
            let pdpt = table_mut(pdpt_phys);

            let pd_phys = ensure_table(pt_alloc, pdpt, pdpt_index(vaddr.0), user)?;
            let pd = table_mut(pd_phys);

            let pt_phys = ensure_table(pt_alloc, pd, pd_index(vaddr.0), user)?;
            let pt = table_mut(pt_phys);

            let idx = pt_index(vaddr.0);

            if (pt[idx] & PTE_P) != 0 {
                return Err(MapError::AlreadyMapped);
            }

            pt[idx] = (paddr.0 & ADDR_MASK) | flags_to_pte(flags);
        }

        self.flush_tlb_page(vaddr);

        Ok(())
    }

    unsafe fn unmap_4k(&self, aspace: &mut AddressSpace, vaddr: VirtAddr) -> Result<(), MapError> {
        if !aligned_4k(vaddr.0) {
            return Err(MapError::Unaligned);
        }
        if !is_canonical(vaddr.0) {
            return Err(MapError::InvalidArgs);
        }

        let root = as_x86_mut(aspace).pml4_phys;

        unsafe {
            let pml4 = table_mut(root);

            let e1 = pml4[pml4_index(vaddr.0)];
            if (e1 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pdpt = table_mut(PhysAddr(e1 & ADDR_MASK));

            let e2 = pdpt[pdpt_index(vaddr.0)];
            if (e2 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pd = table_mut(PhysAddr(e2 & ADDR_MASK));

            let e3 = pd[pd_index(vaddr.0)];
            if (e3 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pt = table_mut(PhysAddr(e3 & ADDR_MASK));

            let idx = pt_index(vaddr.0);
            if (pt[idx] & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }

            pt[idx] = 0;
        }
        self.flush_tlb_page(vaddr);
        Ok(())
    }

    unsafe fn protect_4k(
        &self,
        aspace: &mut AddressSpace,
        vaddr: VirtAddr,
        flags: MapFlags,
    ) -> Result<(), MapError> {
        if !aligned_4k(vaddr.0) {
            return Err(MapError::Unaligned);
        }
        if !is_canonical(vaddr.0) {
            return Err(MapError::InvalidArgs);
        }

        let root = as_x86_mut(aspace).pml4_phys;
        unsafe {
            let pml4 = table_mut(root);

            let e1 = pml4[pml4_index(vaddr.0)];
            if (e1 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pdpt = table_mut(PhysAddr(e1 & ADDR_MASK));

            let e2 = pdpt[pdpt_index(vaddr.0)];
            if (e2 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pd = table_mut(PhysAddr(e2 & ADDR_MASK));

            let e3 = pd[pd_index(vaddr.0)];
            if (e3 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pt = table_mut(PhysAddr(e3 & ADDR_MASK));

            let idx = pt_index(vaddr.0);
            if (pt[idx] & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }

            let paddr = pt[idx] & ADDR_MASK;
            pt[idx] = paddr | flags_to_pte(flags);
        }
        self.flush_tlb_page(vaddr);
        Ok(())
    }

    unsafe fn translate(
        &self,
        aspace: &AddressSpace,
        vaddr: VirtAddr,
    ) -> Result<PhysAddr, hal::mmu::TranslateError> {
        if !is_canonical(vaddr.0) {
            return Err(TranslateError::InvalidAddress);
        }
        let root = as_x86(aspace).pml4_phys;
        unsafe {
            let pml4 = table_mut(root);

            let e1 = pml4[pml4_index(vaddr.0)];
            if (e1 & PTE_P) == 0 {
                return Err(TranslateError::NotMapped);
            }
            let pdpt = table_mut(PhysAddr(e1 & ADDR_MASK));

            let e2 = pdpt[pdpt_index(vaddr.0)];
            if (e2 & PTE_P) == 0 {
                return Err(TranslateError::NotMapped);
            }
            let pd = table_mut(PhysAddr(e2 & ADDR_MASK));

            let e3 = pd[pd_index(vaddr.0)];
            if (e3 & PTE_P) == 0 {
                return Err(TranslateError::NotMapped);
            }
            let pt = table_mut(PhysAddr(e3 & ADDR_MASK));

            let e4 = pt[pt_index(vaddr.0)];
            if (e4 & PTE_P) == 0 {
                return Err(TranslateError::NotMapped);
            }

            let base = e4 & ADDR_MASK;
            let off = vaddr.0 & 0xfff;
            Ok(PhysAddr(base | off))
        }
    }

    unsafe fn activate(&self, aspace: &AddressSpace) {
        let root = as_x86(aspace).pml4_phys.0;
        let cr3 = root & ADDR_MASK;
        unsafe {
            core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack, nomem, preserves_flags));

            *CURRENT.get() = Some(*aspace);
        }
    }

    fn flush_tlb_page(&self, vaddr: VirtAddr) {
        unsafe {
            core::arch::asm!("invlpg [{}]", in(reg) vaddr.0, options(nostack, nomem, preserves_flags));
        }
    }

    fn flush_tlb_all(&self) {
        unsafe {
            let cr3: u64;
            core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, nomem, preserves_flags));
            core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack, nomem, preserves_flags));
        }
    }

    fn current(&self) -> AddressSpace {
        unsafe {
            let cur = (*CURRENT.get()).expect("MMU not initialized");
            cur
        }
    }
}
