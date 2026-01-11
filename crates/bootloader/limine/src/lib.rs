#![no_std]

use bootabi::*;
use limine::BaseRevision;
use limine::request::*;

static BASE_REVISION: BaseRevision = BaseRevision::new();
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();
static HHDM: HhdmRequest = HhdmRequest::new();
static MEMMAP: MemoryMapRequest = MemoryMapRequest::new();
static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();
static RSDP: RsdpRequest = RsdpRequest::new();
static CMDLINE: ExecutableCmdlineRequest = ExecutableCmdlineRequest::new();

// You likely want these stored in a static buffer because you can't easily build
// a slice of converted entries without owning memory. Keep it minimal:
// - allocate a fixed-capacity array for memmap conversion
// - truncate if huge (rare). You can increase later.
const MAX_MEMMAP: usize = 512;
static mut MEMBUF: [MemMapEntry; MAX_MEMMAP] = [MemMapEntry {
    base: PhysAddr(0),
    len: 0,
    typ: MemType::Reserved,
    flags: MemFlags::NONE,
}; MAX_MEMMAP];

pub fn gather_bootinfo(arch: Arch, cpu_count: u32) -> BootInfo<'static> {
    assert!(BASE_REVISION.is_valid());
    assert!(BASE_REVISION.is_supported());

    let hhdm_offset = HHDM.get_response().map(|r| r.offset()).unwrap_or(0);

    let bootloader = BOOTLOADER_INFO
        .get_response()
        .and_then(|r| Some(r.name()))
        .map(|s| unsafe { core::str::from_utf8_unchecked(s.as_bytes()) });

    let cmdline = CMDLINE
        .get_response()
        .and_then(|r| Some(r.cmdline()))
        .map(|s| unsafe { core::str::from_utf8_unchecked(s.to_bytes()) });

    let acpi_rsdp = RSDP
        .get_response()
        .and_then(|r| Some(r.address()))
        .map(|p| PhysAddr(p as u64));

    let fb = FRAMEBUFFER
        .get_response()
        .and_then(|r| r.framebuffers().next())
        .map(|fb| Framebuffer {
            addr: PhysAddr(fb.addr() as u64),
            width: fb.width() as u32,
            height: fb.height() as u32,
            pitch: fb.pitch() as u32,
            bpp: fb.bpp(),
            format: match fb.memory_model() {
                limine::framebuffer::MemoryModel::RGB => FbFormat::Rgb,
                _ => FbFormat::Unknown,
            },
            _pad: 0,
        });

    // Memmap conversion
    let limine_entries = MEMMAP.get_response().map(|r| r.entries()).unwrap();
    let converted = convert_memmap(limine_entries);

    BootInfo {
        hdr: BootHeader {
            magic: BOOTABI_MAGIC,
            version: BOOTABI_VERISON,
            header_size: core::mem::size_of::<BootInfo>() as u16,
        },
        arch,
        cpu_count,
        phys_addr_bits: 0,
        virt_addr_bits: 0,
        _pad0: [0; 2],
        hhdm_offset,
        mem: MemMap { entries: converted },
        acpi_rsdp,
        fb,
        cmdline,
        bootloader,
    }
}

fn convert_memmap(entries: &[&limine::memory_map::Entry]) -> &'static [MemMapEntry] {
    unsafe {
        let n = core::cmp::min(entries.len(), MAX_MEMMAP);
        for i in 0..n {
            let e = &entries[i];
            MEMBUF[i] = MemMapEntry {
                base: PhysAddr(e.base),
                len: e.length,
                typ: map_memtype(e.entry_type),
                flags: MemFlags::NONE,
            };
        }
        &MEMBUF[..n]
    }
}

fn map_memtype(t: limine::memory_map::EntryType) -> MemType {
    use limine::memory_map::EntryType as L;
    match t {
        L::USABLE => MemType::Usable,
        L::RESERVED => MemType::Reserved,
        L::ACPI_RECLAIMABLE => MemType::AcpiReclaimable,
        L::ACPI_NVS => MemType::AcpiNvs,
        L::BAD_MEMORY => MemType::BadMemory,
        L::BOOTLOADER_RECLAIMABLE => MemType::BootloaderReclaimable,
        L::EXECUTABLE_AND_MODULES => MemType::KernelAndModules,
        _ => MemType::Reserved,
    }
}
