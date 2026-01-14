use core::cell::SyncUnsafeCell;
use core::ptr::NonNull;

use hal::mmu::{
    AddressSpace, MapError, MapFlags, Mmu, PageTableFrameAlloc, PhysAddr, TranslateError, VirtAddr,
};

use crate::hhdm_offset;

pub struct X86Mmu;

pub static MMU: X86Mmu = X86Mmu;

/// Internal arch-owned address space object.
/// Kernel never sees this layout.
#[repr(C)]
struct X86AddressSpace {
    pml4_phys: PhysAddr,
}

static KAS: SyncUnsafeCell<Option<X86AddressSpace>> = SyncUnsafeCell::new(None);
static KAS_HANDLE: SyncUnsafeCell<Option<AddressSpace>> = SyncUnsafeCell::new(None);
static CURRENT: SyncUnsafeCell<Option<AddressSpace>> = SyncUnsafeCell::new(None);

const PAGE_SIZE: u64 = 4096;
const ENTRY_COUNT: usize = 512;

const PTE_P: u64 = 1 << 0;
const PTE_W: u64 = 1 << 1;
const PTE_U: u64 = 1 << 2;
const PTE_PWT: u64 = 1 << 3;
const PTE_PCD: u64 = 1 << 4;
const PTE_G: u64 = 1 << 8;
const PTE_NX: u64 = 1 << 63;

#[inline(always)]
fn aligned_4k(x: u64) -> bool {
    (x & (PAGE_SIZE - 1)) == 0
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

    if !flags.contains(MapFlags::EXEC) {
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
        return Ok(PhysAddr(entry & 0x000F_FFFF_FFFF_F000));
    }

    let frame = pt_alloc.alloc_frame_4k().ok_or(MapError::OutOfMemory)?;
    unsafe {
        zero_frame(frame);
    }

    let mut flags = PTE_P | PTE_W;
    if user {
        flags |= PTE_U;
    }

    parent[idx] = (frame.0 & 0x000F_FFFF_FFFF_F000) | flags;

    Ok(frame)
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
    unsafe fn address_space_new(
        &self,
        pt_alloc: &mut dyn PageTableFrameAlloc,
    ) -> Result<&'static mut AddressSpace, MapError> {
        unsafe {
            if let Some(h) = (*KAS_HANDLE.get()).as_mut() {
                return Ok(h);
            }

            let pml4 = pt_alloc.alloc_frame_4k().ok_or(MapError::OutOfMemory)?;
            zero_frame(pml4);

            *KAS.get() = Some(X86AddressSpace { pml4_phys: pml4 });

            let kas_ref: &'static mut X86AddressSpace = (*KAS.get()).as_mut().unwrap();

            let handle_ptr = NonNull::new(kas_ref as *mut _ as *mut ()).unwrap();
            *KAS_HANDLE.get() = Some(AddressSpace::from_ptr(handle_ptr));
            let h: &'static mut AddressSpace = (*KAS_HANDLE.get()).as_mut().unwrap();

            *CURRENT.get() = Some(*h);

            Ok(h)
        }
    }

    unsafe fn address_space_destroy(
        &self,
        _pt_alloc: &mut dyn PageTableFrameAlloc,
        _aspace: &'static mut AddressSpace,
    ) {
        // TODO: walk tables and free page-table framse via pt_alloc.
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

            pt[idx] = (paddr.0 & 0x000f_ffff_ffff_f000) | flags_to_pte(flags);
        }

        self.flush_tlb_page(vaddr);

        Ok(())
    }

    unsafe fn unmap_4k(&self, aspace: &mut AddressSpace, vaddr: VirtAddr) -> Result<(), MapError> {
        if !aligned_4k(vaddr.0) {
            return Err(MapError::Unaligned);
        }

        let root = as_x86_mut(aspace).pml4_phys;

        unsafe {
            let pml4 = table_mut(root);

            let e1 = pml4[pml4_index(vaddr.0)];
            if (e1 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pdpt = table_mut(PhysAddr(e1 & 0x000F_FFFF_FFFF_F000));

            let e2 = pdpt[pdpt_index(vaddr.0)];
            if (e2 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pd = table_mut(PhysAddr(e2 & 0x000F_FFFF_FFFF_F000));

            let e3 = pd[pd_index(vaddr.0)];
            if (e3 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pt = table_mut(PhysAddr(e3 & 0x000F_FFFF_FFFF_F000));

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

        let root = as_x86_mut(aspace).pml4_phys;
        unsafe {
            let pml4 = table_mut(root);

            let e1 = pml4[pml4_index(vaddr.0)];
            if (e1 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pdpt = table_mut(PhysAddr(e1 & 0x000F_FFFF_FFFF_F000));

            let e2 = pdpt[pdpt_index(vaddr.0)];
            if (e2 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pd = table_mut(PhysAddr(e2 & 0x000f_ffff_ffff_f000));

            let e3 = pd[pd_index(vaddr.0)];
            if (e3 & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }
            let pt = table_mut(PhysAddr(e3 & 0x000f_ffff_ffff_f000));

            let idx = pt_index(vaddr.0);
            if (pt[idx] & PTE_P) == 0 {
                return Err(MapError::NotMapped);
            }

            let paddr = pt[idx] & 0x000f_ffff_ffff_f000;
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
        let root = as_x86(aspace).pml4_phys;
        unsafe {
            let pml4 = table_mut(root);

            let e1 = pml4[pml4_index(vaddr.0)];
            if (e1 & PTE_P) == 0 {
                return Err(TranslateError::NotMapped);
            }
            let pdpt = table_mut(PhysAddr(e1 & 0x000f_ffff_ffff_f000));

            let e2 = pdpt[pdpt_index(vaddr.0)];
            if (e2 & PTE_P) == 0 {
                return Err(TranslateError::NotMapped);
            }
            let pd = table_mut(PhysAddr(e2 & 0x000f_ffff_ffff_f000));

            let e3 = pd[pd_index(vaddr.0)];
            if (e3 & PTE_P) == 0 {
                return Err(TranslateError::NotMapped);
            }
            let pt = table_mut(PhysAddr(e3 & 0x000f_ffff_ffff_f000));

            let e4 = pt[pt_index(vaddr.0)];
            if (e4 & PTE_P) == 0 {
                return Err(TranslateError::NotMapped);
            }

            let base = e4 & 0x000f_ffff_ffff_f000;
            let off = vaddr.0 & 0xfff;
            Ok(PhysAddr(base | off))
        }
    }

    unsafe fn activate(&self, aspace: &AddressSpace) {
        let root = as_x86(aspace).pml4_phys.0;
        let cr3 = root & 0x000f_ffff_ffff_f000;
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
