#![no_std]

use bootabi::*;
use core::ffi::c_char;
use core::mem::MaybeUninit;
use limine::BaseRevision;
use limine::firmware_type::FirmwareType;
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
#[unsafe(link_section = ".limine_reqs")]
static FIRMWARE_TYPE: FirmwareTypeRequest = FirmwareTypeRequest::new();
#[used]
#[unsafe(link_section = ".limine_reqs")]
static STACK_SIZE: StackSizeRequest = StackSizeRequest::new().with_size(128 * 1024);
#[used]
#[unsafe(link_section = ".limine_reqs_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

// Fixed-capacity array for memmap conversion
const MAX_MEMMAP: usize = 1024;

#[used]
#[unsafe(link_section = ".bss.boot")]
static mut MEMBUF: MaybeUninit<[MemMapEntry; MAX_MEMMAP]> = MaybeUninit::uninit();

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("bootloader_limine: unsupported target_arch");

fn current_arch() -> Arch {
    #[cfg(target_arch = "x86_64")]
    {
        Arch::X86_64
    }
    #[cfg(target_arch = "aarch64")]
    {
        Arch::Aarch64
    }
}

const STRBUF_SIZE: usize = 4096;

#[used]
#[unsafe(link_section = ".bss.boot")]
static mut STRBUF: MaybeUninit<[u8; STRBUF_SIZE]> = MaybeUninit::uninit();

#[used]
#[unsafe(link_section = ".bss.boot")]
static mut STRBUF_USED: usize = 0;

#[inline(always)]
unsafe fn membuf_mut() -> &'static mut [MemMapEntry; MAX_MEMMAP] {
    unsafe { &mut *core::ptr::addr_of_mut!(MEMBUF).cast::<[MemMapEntry; MAX_MEMMAP]>() }
}

#[inline(always)]
unsafe fn strbuf_mut() -> &'static mut [u8; STRBUF_SIZE] {
    unsafe { &mut *core::ptr::addr_of_mut!(STRBUF).cast::<[u8; STRBUF_SIZE]>() }
}

#[repr(C)]
struct RawBootloaderInfoResponse {
    _revision: u64,
    name: *const c_char,
    version: *const c_char,
}

#[repr(C)]
struct RawExecutableCmdlineResponse {
    _revision: u64,
    cmdline: *const c_char,
}

const MAX_STR_BYTES: u64 = 1024;

#[derive(Clone, Copy)]
struct StrView {
    ptr: *const u8,
    len: usize,
}

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
        let buf = strbuf_mut();
        buf[off..off + need].copy_from_slice(bytes);
        STRBUF_USED = off + need;

        ByteSpan {
            ptr: (&buf[off] as *const u8) as u64,
            len: need as u32,
            _pad: 0,
        }
    }
}

fn cstr_view(ptr: *const c_char, hhdm_offset: u64, mem: &MemMapView) -> Option<StrView> {
    let ptr = ptr as u64;
    if ptr == 0
        || mem.entries_ptr == 0
        || mem.entry_count == 0
        || mem.entry_size as usize != core::mem::size_of::<MemMapEntry>()
    {
        return None;
    }

    let entries = unsafe {
        core::slice::from_raw_parts(
            mem.entries_ptr as *const MemMapEntry,
            mem.entry_count as usize,
        )
    };
    let (virt, max_len) = resolve_limine_ptr(ptr, hhdm_offset, entries)?;
    let max_len = core::cmp::min(max_len, MAX_STR_BYTES) as usize;
    if max_len == 0 {
        return None;
    }

    let bytes = unsafe { core::slice::from_raw_parts(virt as *const u8, max_len) };
    let len = bytes.iter().position(|&b| b == 0)?;
    if len == 0 {
        return None;
    }

    Some(StrView {
        ptr: virt as *const u8,
        len,
    })
}

fn resolve_limine_ptr(ptr: u64, hhdm_offset: u64, entries: &[MemMapEntry]) -> Option<(u64, u64)> {
    if hhdm_offset != 0 && ptr < hhdm_offset {
        let max = phys_available(ptr, entries)?;
        return Some((ptr.wrapping_add(hhdm_offset), max));
    }

    let phys = if hhdm_offset != 0 {
        ptr.wrapping_sub(hhdm_offset)
    } else {
        ptr
    };
    let max = phys_available(phys, entries)?;
    Some((ptr, max))
}

fn phys_available(phys: u64, entries: &[MemMapEntry]) -> Option<u64> {
    for entry in entries {
        let base = entry.base.0;
        let end = base.checked_add(entry.len)?;
        if phys >= base && phys < end {
            return Some(end - phys);
        }
    }
    None
}

fn strbuf_alloc_join(a: &[u8], b: &[u8]) -> ByteSpan {
    if a.is_empty() {
        return strbuf_alloc_copy(b);
    }
    if b.is_empty() {
        return strbuf_alloc_copy(a);
    }

    unsafe {
        let off = (STRBUF_USED + 7) & !7;
        let sep = b" ";
        let need = a.len() + sep.len() + b.len();
        if off + need > STRBUF_SIZE {
            return ByteSpan::empty();
        }

        let buf = strbuf_mut();
        let mut cur = off;
        buf[cur..cur + a.len()].copy_from_slice(a);
        cur += a.len();
        buf[cur..cur + sep.len()].copy_from_slice(sep);
        cur += sep.len();
        buf[cur..cur + b.len()].copy_from_slice(b);
        STRBUF_USED = cur + b.len();

        ByteSpan {
            ptr: (&buf[off] as *const u8) as u64,
            len: need as u32,
            _pad: 0,
        }
    }
}

pub fn gather_bootinfo(cpu_count: u32) -> BootInfo {
    let arch = current_arch();
    unsafe {
        STRBUF_USED = 0;
    }

    if !BASE_REVISION.is_valid() || !BASE_REVISION.is_supported() {
        return minimal_bootinfo(arch, cpu_count);
    }

    let boot_mode = match FIRMWARE_TYPE.get_response().map(|r| r.firmware_type()) {
        Some(FirmwareType::X86_BIOS) => BootMode::Bios,
        Some(FirmwareType::UEFI_64) => BootMode::Uefi,
        Some(FirmwareType::UEFI_32) => BootMode::Uefi,
        _ => BootMode::Unknown,
    };

    let hhdm_offset = HHDM.get_response().map(|r| r.offset()).unwrap_or(0);

    let acpi_rsdp = RSDP
        .get_response()
        .map(|r| PhysAddr(r.address() as u64))
        .unwrap_or(PhysAddr(0));

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
    let (memmap_view, memmap_flags) = MEMMAP
        .get_response()
        .map(|resp| convert_memmap_entries(resp.entries()))
        .unwrap_or_else(empty_memmap_view);

    let cmdline = CMDLINE
        .get_response()
        .and_then(|r| {
            let raw = unsafe { &*(r as *const _ as *const RawExecutableCmdlineResponse) };
            cstr_view(raw.cmdline, hhdm_offset, &memmap_view)
        })
        .map(|view| unsafe { core::slice::from_raw_parts(view.ptr, view.len) })
        .map(strbuf_alloc_copy)
        .unwrap_or(ByteSpan::empty());

    let bootloader = BOOTLOADER_INFO
        .get_response()
        .and_then(|r| {
            let raw = unsafe { &*(r as *const _ as *const RawBootloaderInfoResponse) };
            let name = cstr_view(raw.name, hhdm_offset, &memmap_view)
                .map(|view| unsafe { core::slice::from_raw_parts(view.ptr, view.len) });
            let version = cstr_view(raw.version, hhdm_offset, &memmap_view)
                .map(|view| unsafe { core::slice::from_raw_parts(view.ptr, view.len) });
            match (name, version) {
                (Some(a), Some(b)) => Some(strbuf_alloc_join(a, b)),
                (Some(a), None) => Some(strbuf_alloc_copy(a)),
                (None, Some(b)) => Some(strbuf_alloc_copy(b)),
                (None, None) => None,
            }
        })
        .unwrap_or(ByteSpan::empty());

    let (phys_bits, virt_bits) = default_addr_bits(arch);

    BootInfo {
        hdr: BootHeader {
            magic: BOOTABI_MAGIC,
            version: BOOTABI_VERSION,
            header_size: core::mem::size_of::<BootInfo>() as u16,
        },
        arch,
        boot_mode: boot_mode.as_raw(),
        _pad0: [0; 2],
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
        cmdline,
        bootloader,
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
        boot_mode: BootMode::Unknown.as_raw(),
        _pad0: [0; 2],
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

fn empty_memmap_view() -> (MemMapView, u32) {
    (
        MemMapView {
            entries_ptr: 0,
            entry_count: 0,
            entry_size: core::mem::size_of::<MemMapEntry>() as u16,
            flags: 0,
        },
        0u32,
    )
}

fn convert_memmap_entries(entries: &[&limine::memory_map::Entry]) -> (MemMapView, u32) {
    if entries.is_empty() {
        return empty_memmap_view();
    }

    let n_total = entries.len();
    let n = core::cmp::min(n_total, MAX_MEMMAP);
    let entries_ptr = unsafe {
        let membuf = membuf_mut();
        for (i, entry) in entries.iter().take(n).enumerate() {
            let e = **entry;
            membuf[i] = MemMapEntry {
                base: PhysAddr(e.base),
                len: e.length,
                mem_type: map_memtype(e.entry_type),
                _pad: 0,
                flags: 0,
            };
        }
        membuf.as_ptr() as u64
    };
    let truncated = if n_total > n {
        MemMapFlags::TRUNCATED as u32
    } else {
        0
    };

    (
        MemMapView {
            entries_ptr,
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
