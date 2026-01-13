#![no_std]

pub const BOOTABI_MAGIC: u32 = 0x424f4f54; // 'BOOT'
pub const BOOTABI_VERSION: u16 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BootHeader {
    pub magic: u32,
    pub version: u16,
    pub header_size: u16, // size of BootInfoFixed (not including external buffers)
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    X86_64 = 1,
    Aarch64 = 2,
}

/// Pointer to bytes in memory
/// physical pointer + length
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteSpan {
    pub ptr: u64, // phyical address
    pub len: u32,
    pub _pad: u32,
}

impl ByteSpan {
    pub const fn empty() -> Self {
        Self {
            ptr: 0,
            len: 0,
            _pad: 0,
        }
    }
    pub const fn is_empty(&self) -> bool {
        self.ptr == 0 || self.len == 0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtAddr(pub u64);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysAddr(pub u64);

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootFlags {
    NONE = 0,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemMapFlags {
    NONE = 0,
    TRUNCATED = 1 << 0,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemType {
    Reserved = 0,
    Usable = 1,
    AcpiReclaimable = 2,
    AcpiNvs = 3,
    Mmio = 4,
    BadMemory = 5,
    BootloaderReclaimable = 6,
    KernelAndModules = 7,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemMapEntry {
    pub base: PhysAddr,
    pub len: u64,
    pub mem_type: MemType,
    pub _pad: u8,
    pub flags: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemMapView {
    pub entries_ptr: u64,
    pub entry_count: u32,
    pub entry_size: u16,
    pub flags: u16,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FbFormat {
    Rgb = 1,
    Bgr = 2,
    Unknown = 255,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Framebuffer {
    pub addr: PhysAddr,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u16,
    pub format: FbFormat,
    pub _pad: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BootInfo {
    pub hdr: BootHeader,

    // Identity
    pub arch: Arch,
    pub _pad0: [u8; 3],

    pub cpu_count: u32,

    // Addressing model
    pub phys_addr_bits: u8, // optional, 0 if unknown
    pub virt_addr_bits: u8, // optional, 0 if unknown
    pub _pad1: [u8; 2],

    /// Higher-half direct map (HHDM) base (virtual = phys + hhdm_offset)
    pub hhdm_offset: u64,

    /// Memory map view into a static buffer (entries_ptr is physical)
    pub mem: MemMapView,

    /// ACPI (RSDP physical pointer (0 if none))
    pub acpi_rsdp: PhysAddr,

    /// Framebuffer info (addr = 0 means none)
    pub fb: Framebuffer,

    /// Bootloader-provided command line (physical pointer + length, bytes)
    pub cmdline: ByteSpan,

    /// Bootloader name/version bytes (optional)
    pub bootloader: ByteSpan,

    pub boot_flags: u32,
    pub _pad2: u32,
}
