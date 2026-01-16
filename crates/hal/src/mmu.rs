use core::num::NonZeroU64;
use core::ptr::NonNull;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub u64);

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub u64);

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PageSize(pub NonZeroU64);

impl PageSize {
    pub const SIZE_4K: PageSize = unsafe { PageSize(NonZeroU64::new_unchecked(4096)) };

    // Add 2M/1G later if desired; Keep MVP at 4K to start.
}

bitflags::bitflags! {
    /// Architecture-neutral mapping intent.
    ///
    /// These are *semantic* flags. Arch decides exact PTE bits/attributes
    pub struct MapFlags: u32 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXEC = 1 << 2;

        /// User-accessible mapping (user mode can access).
        const USER = 1 << 3;

        /// Global mapping (not flushed on address space switch if supported)
        const GLOBAL = 1 << 4;

        /// Uncached / device-ish mapping intent.
        /// Arch chooses exact memory attributes.
        const UNCACHED = 1 << 5;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapError {
    Unaligned,
    AlreadyMapped,
    NotMapped,
    OutOfMemory, // for page-table allocation (arch is allowed to request frames)
    InvalidArgs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranslateError {
    NotMapped,
    InvalidAddress,
}

/// An opaque handle to an address space root.
///
/// Opaque to kernel: backed by an arch-owned object;
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct AddressSpace(NonNull<()>);

unsafe impl Sync for AddressSpace {}
unsafe impl Send for AddressSpace {}

impl AddressSpace {
    pub const unsafe fn from_ptr(ptr: NonNull<()>) -> Self {
        Self(ptr)
    }
    pub const fn as_ptr(&self) -> NonNull<()> {
        self.0
    }
}

/// A minimal frame allocator interface used only for page-table memory.
///
/// This is *not* the kernel PMM API; it's a tiny callback shape.
/// Kernel passes a provider; arch uses it only to allocate internal tables
pub trait PageTableFrameAlloc {
    /// Allocate one 4K physical frame, zeroed.
    fn alloc_frame_4k(&mut self) -> Option<PhysAddr>;

    /// Free one 4k physical frame previously allocated for page tables.
    fn free_frame_4k(&mut self, paddr: PhysAddr);
}

/// MMU backend contract implemented by each arch.
pub trait Mmu {
    /// Initialize MMU state from the currently active address space.
    ///
    /// This is typically called once during boot so future address spaces can
    /// inherit required kernel mappings.
    unsafe fn init_kernel(&self) -> Result<&'static mut AddressSpace, MapError>;

    /// Create a new address space that inherits required kernel mappings
    /// (e.g., higher-half kernel, HHDM if you keep it global).
    ///
    /// Arch may allocate page-table frames via `pt_alloc`.
    /// `init_kernel` must have been called before this.
    unsafe fn address_space_new(
        &self,
        pt_alloc: &mut dyn PageTableFrameAlloc,
    ) -> Result<&'static mut AddressSpace, MapError>;

    /// Destroy an address space and free any page-table memory.
    unsafe fn address_space_destroy(
        &self,
        pt_alloc: &mut dyn PageTableFrameAlloc,
        aspace: &'static mut AddressSpace,
    );

    /// Map a single 4K page.
    ///
    /// - No policy.
    /// - Arch handles intermediate table allocation via pt_alloc.
    unsafe fn map_4k(
        &self,
        pt_alloc: &mut dyn PageTableFrameAlloc,
        aspace: &mut AddressSpace,
        vaddr: VirtAddr,
        paddr: PhysAddr,
        flags: MapFlags,
    ) -> Result<(), MapError>;

    /// Unmap a single 4K page.
    unsafe fn unmap_4k(&self, aspace: &mut AddressSpace, vaddr: VirtAddr) -> Result<(), MapError>;

    /// Update flags on an existing mapping (no remap).
    unsafe fn protect_4k(
        &self,
        aspace: &mut AddressSpace,
        vaddr: VirtAddr,
        flags: MapFlags,
    ) -> Result<(), MapError>;

    unsafe fn translate(
        &self,
        aspace: &AddressSpace,
        vaddr: VirtAddr,
    ) -> Result<PhysAddr, TranslateError>;

    /// Make `aspace` the active address space on the current CPU.
    ///
    /// x86_64: load CR3
    /// aarch64: write TTBRx_EL1 + TLB maintenance
    unsafe fn activate(&self, aspace: &AddressSpace);

    /// TLB maintenance for a single page on current CPU.
    fn flush_tlb_page(&self, vaddr: VirtAddr);

    /// Full TLB flush on current CPU.
    fn flush_tlb_all(&self);

    /// Request a TLB shootdown for a single page on all CPUs.
    ///
    /// Default: local flush only.
    fn shootdown_tlb_page(&self, vaddr: VirtAddr) {
        self.flush_tlb_page(vaddr);
    }

    /// Request a full TLB shootdown on all CPUs.
    ///
    /// Default: local flush only.
    fn shootdown_tlb_all(&self) {
        self.flush_tlb_all();
    }

    /// Returns the currently active address space for this CPU.
    ///
    /// Arch may track this in CPU-local storage (still within arch).
    fn current(&self) -> AddressSpace;
}
