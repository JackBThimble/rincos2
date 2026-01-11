#![no_std]

pub const BOOTABI_MAGIC: u32 = 0x424f4f54; // 'BOOT'
pub const BOOTABI_VERISON: u16 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BootHeader {
    pub magic: u32,
    pub version: u16,
    pub header_size: u16,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    X86_64 = 1,
    Aarch64 = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BootInfo<'a> {
    pub hdr: BootHeader,

    // Identity
    pub arch: Arch,
    pub cpu_count: u32,

    // Addressing model
    pub phys_addr_bits: u8, // optional, 0 if unknown
    pub virt_addr_bits: u8, // optional, 0 if unknown
    pub _pad0: [u8; 2],

    // Higher-half direct map (HHDM) base (virtual = phys + hhdm_offset)
    pub hhdm_offset: u64,

    // Memory map
    pub mem: MemMap<'a>,

    // ACPI (RSDP physical pointer)
    pub acpi_rsdp: Option<PhysAddr>,

    // Framebuffer (optional; many early boots only need serial)
    pub fb: Option<Framebuffer>,

    // Command line (optional)
    pub cmdline: Option<&'a str>,

    // Bootloader identification (optional)
    pub bootloader: Option<&'a str>,
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PhysAddr(pub u64);

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct VirtAddr(pub u64);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemMap<'a> {
    pub entries: &'a [MemMapEntry],
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemType {
    Usable = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    AcpiNvs = 4,
    Mmio = 5,
    BadMemory = 6,

    BootloaderReclaimable = 7,
    KernelAndModules = 8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemMapEntry {
    pub base: PhysAddr,
    pub len: u64,
    pub typ: MemType,
    pub flags: MemFlags,
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct MemFlags: u32 {
        const NONE = 0;
        // Set if region must be mapped to non-cacheable, etc. (optional)
        const NO_CACHE = 1 << 0;
        // Set if region is already in use by firmware and must be preserved
        const FIRMWARE = 1 << 1;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Framebuffer {
    pub addr: PhysAddr,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u16,
    pub format: FbFormat,
    pub _pad: u8,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FbFormat {
    Rgb = 1,
    Bgr = 2,
    Unknown = 255,
}
