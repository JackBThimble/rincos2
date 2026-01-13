#![no_std]

use bootabi::*;
use limine::BaseRevision;
use limine::request::*;

#[used]
#[unsafe(link_section = ".limine_reqs_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();
#[used]
#[unsafe(link_section = ".limine_reqs")]
static BASE_REVISION: BaseRevision = BaseRevision::new();
#[used]
#[unsafe(link_section = ".limine_reqs")]
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();
#[used]
#[unsafe(link_section = ".limine_reqs")]
static HHDM: HhdmRequest = HhdmRequest::new();
#[used]
#[unsafe(link_section = ".limine_reqs")]
static MEMMAP: MemoryMapRequest = MemoryMapRequest::new();
#[used]
#[unsafe(link_section = ".limine_reqs")]
static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();
#[used]
#[unsafe(link_section = ".limine_reqs")]
static RSDP: RsdpRequest = RsdpRequest::new();
#[used]
#[unsafe(link_section = ".limine_reqs")]
static CMDLINE: ExecutableCmdlineRequest = ExecutableCmdlineRequest::new();
#[used]
#[unsafe(link_section = ".limine_reqs_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

// Fixed-capacity array for memmap conversion
const MAX_MEMMAP: usize = 1024;

#[used]
#[unsafe(link_section = ".bss.boot")]
static mut MEMBUF: [MemMapEntry; MAX_MEMMAP] = [MemMapEntry {
    base: PhysAddr(0),
    len: 0,
    mem_type: MemType::Reserved,
    _pad: 0,
    flags: 0,
}; MAX_MEMMAP];

#[inline(always)]
fn dbg_serial(byte: u8) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        const COM1: u16 = 0x3f8;
        core::arch::asm!("out dx, al", in("dx") COM1, in("al") byte, options(nomem, nostack));
    }
}

#[inline(always)]
fn span_from_cstr_phys(cstr: &core::ffi::CStr) -> ByteSpan {
    let bytes = cstr.to_bytes();
    ByteSpan {
        ptr: bytes.as_ptr() as usize as u64, // NOTE: this is a *virtual* address in most cases
        len: bytes.len() as u32,
        _pad: 0,
    }
}

const STRBUF_SIZE: usize = 4096;

#[used]
#[unsafe(link_section = ".bss.boot")]
static mut STRBUF: [u8; STRBUF_SIZE] = [0; STRBUF_SIZE];

#[used]
#[unsafe(link_section = ".bss.boot")]
static mut STRBUF_USED: usize = 0;

fn strbuf_alloc_copy(bytes: &[u8]) -> ByteSpan {
    if bytes.is_empty() {
        return ByteSpan::empty();
    }

    unsafe {
        let off = (STRBUF_USED + 7) & !7;
        let need = bytes.len();
        if off + need > STRBUF_SIZE {
            return ByteSpan::empty();
        }
        STRBUF[off..off + need].copy_from_slice(bytes);
        STRBUF_USED = off + need;

        ByteSpan {
            ptr: (&STRBUF[off] as *const u8) as u64,
            len: need as u32,
            _pad: 0,
        }
    }
}

pub fn gather_bootinfo(arch: Arch, cpu_count: u32) -> BootInfo {
    dbg_serial(b'G');

    if !BASE_REVISION.is_valid() || !BASE_REVISION.is_supported() {
        dbg_serial(b'!');
        return minimal_bootinfo(arch, cpu_count);
    }
    dbg_serial(b'1');

    let hhdm_offset = HHDM.get_response().map(|r| r.offset()).unwrap_or(0);
    dbg_serial(b'2');

    let acpi_rsdp = RSDP
        .get_response()
        .map(|r| PhysAddr(r.address() as u64))
        .unwrap_or(PhysAddr(0));
    dbg_serial(b'3');

    let fb = Framebuffer {
        addr: PhysAddr(0),
        width: 0,
        height: 0,
        pitch: 0,
        bpp: 0,
        format: FbFormat::Unknown,
        _pad: 0,
    };
    // let _ = FRAMEBUFFER.get_response();
    // .and_then(|r| r.framebuffers().next())
    // .map(|f| Framebuffer {
    //     addr: PhysAddr(f.addr() as u64),
    //     width: f.width() as u32,
    //     height: f.height() as u32,
    //     pitch: f.pitch() as u32,
    //     bpp: f.bpp(),
    //     format: match f.memory_model() {
    //         limine::framebuffer::MemoryModel::RGB => FbFormat::Rgb,
    //         _ => FbFormat::Unknown,
    //     },
    //     _pad: 0,
    // })
    // .unwrap_or(Framebuffer {
    //     addr: PhysAddr(0),
    //     width: 0,
    //     height: 0,
    //     pitch: 0,
    //     bpp: 0,
    //     format: FbFormat::Unknown,
    //     _pad: 0,
    // });
    dbg_serial(b'4');

    let (memmap_view, memmap_flags) = match MEMMAP.get_response() {
        Some(r) => convert_memmap(r.entries()),
        None => (
            MemMapView {
                entries_ptr: 0,
                entry_count: 0,
                entry_size: core::mem::size_of::<MemMapEntry>() as u16,
                flags: 0,
            },
            0u32,
        ),
    };
    dbg_serial(b'5');

    // let cmdline = CMDLINE
    //     .get_response()
    //     .and_then(|r| r.cmdline().to_str().ok())
    //     .map(|s| strbuf_alloc_copy(s.as_bytes()))
    //     .unwrap_or(ByteSpan::empty());
    // dbg_serial(b'6');

    // let bootloader = BOOTLOADER_INFO
    //     .get_response()
    //     .map(|r| r.name().as_bytes())
    //     .map(|b| strbuf_alloc_copy(b))
    //     .unwrap_or(ByteSpan::empty());
    // dbg_serial(b'7');
    //
    let (phys_bits, virt_bits) = default_addr_bits(arch);

    BootInfo {
        hdr: BootHeader {
            magic: BOOTABI_MAGIC,
            version: BOOTABI_VERSION,
            header_size: core::mem::size_of::<BootInfo>() as u16,
        },
        arch,
        _pad0: [0; 3],
        cpu_count,
        phys_addr_bits: phys_bits,
        virt_addr_bits: virt_bits,
        _pad1: [0; 2],
        hhdm_offset,
        mem: MemMapView {
            flags: memmap_view.flags,
            ..memmap_view
        },
        acpi_rsdp,
        fb,
        cmdline: ByteSpan::empty(),
        bootloader: ByteSpan::empty(),
        boot_flags: BootFlags::NONE as u32 | memmap_flags,
        _pad2: 0,
    }
}

fn minimal_bootinfo(arch: Arch, cpu_count: u32) -> BootInfo {
    let (phys_bits, virt_bits) = default_addr_bits(arch);
    BootInfo {
        hdr: BootHeader {
            magic: BOOTABI_MAGIC,
            version: BOOTABI_VERSION,
            header_size: core::mem::size_of::<BootInfo>() as u16,
        },
        arch,
        _pad0: [0; 3],
        cpu_count,
        phys_addr_bits: phys_bits,
        virt_addr_bits: virt_bits,
        _pad1: [0; 2],
        hhdm_offset: 0,
        mem: MemMapView {
            entries_ptr: 0,
            entry_count: 0,
            entry_size: core::mem::size_of::<MemMapEntry>() as u16,
            flags: 0,
        },
        acpi_rsdp: PhysAddr(0),
        fb: Framebuffer {
            addr: PhysAddr(0),
            width: 0,
            height: 0,
            pitch: 0,
            bpp: 0,
            format: FbFormat::Unknown,
            _pad: 0,
        },
        cmdline: ByteSpan::empty(),
        bootloader: ByteSpan::empty(),
        boot_flags: BootFlags::NONE as u32,
        _pad2: 0,
    }
}

fn default_addr_bits(arch: Arch) -> (u8, u8) {
    match arch {
        Arch::X86_64 => (52, 48),
        Arch::Aarch64 => (48, 48),
    }
}

fn convert_memmap(entries: &[&limine::memory_map::Entry]) -> (MemMapView, u32) {
    unsafe {
        let n_total = entries.len();
        let n = core::cmp::min(n_total, MAX_MEMMAP);
        for i in 0..n {
            let e = entries[i];
            MEMBUF[i] = MemMapEntry {
                base: PhysAddr(e.base),
                len: e.length,
                mem_type: map_memtype(e.entry_type),
                _pad: 0,
                flags: 0,
            };
        }
        let truncated = if n_total > n {
            MemMapFlags::TRUNCATED as u32
        } else {
            0
        };

        (
            MemMapView {
                entries_ptr: (&MEMBUF[0] as *const MemMapEntry) as u64,
                entry_count: n as u32,
                entry_size: core::mem::size_of::<MemMapEntry>() as u16,
                flags: if n_total > n {
                    MemMapFlags::TRUNCATED as u16
                } else {
                    0
                },
            },
            truncated,
        )
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
