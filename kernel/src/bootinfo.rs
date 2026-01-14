use bootabi::{BootInfo, ByteSpan, MemMapEntry, MemMapFlags, MemType};

pub fn print_boot_info(boot: &BootInfo) {
    crate::klogln!(
        "[boot] hdr magic={:#x} version={} size={}",
        boot.hdr.magic,
        boot.hdr.version,
        boot.hdr.header_size
    );
    crate::klogln!(
        "[boot] mode={}",
        match bootabi::BootMode::from_raw(boot.boot_mode) {
            bootabi::BootMode::Bios => "BIOS",
            bootabi::BootMode::Uefi => "UEFI",
            bootabi::BootMode::Unknown => "Unknown",
        }
    );
    crate::klogln!(
        "[boot] arch={:?} cpu_count={} phys_bits={} virt_bits={}",
        boot.arch,
        boot.cpu_count,
        boot.phys_addr_bits,
        boot.virt_addr_bits
    );
    crate::klogln!(
        "[boot] hhdm={:#x} acpi_rsdp={:#x}",
        boot.hhdm_offset,
        boot.acpi_rsdp.0
    );
    crate::klogln!("[boot] boot_flags={:#x}", boot.boot_flags);

    print_bytespan("cmdline", boot.cmdline);
    print_bytespan("bootloader", boot.bootloader);

    if boot.fb.addr.0 != 0 {
        crate::klogln!(
            "[boot] fb addr={:#x} {}x{} pitch={} bpp={} format={:?}",
            boot.fb.addr.0,
            boot.fb.width,
            boot.fb.height,
            boot.fb.pitch,
            boot.fb.bpp,
            boot.fb.format
        );
    } else {
        crate::klogln!("[boot] fb=<none>");
    }

    crate::klogln!(
        "[mem] entries={} entry_size={} flags={:#x}{}",
        boot.mem.entry_count,
        boot.mem.entry_size,
        boot.mem.flags,
        mem_flags_suffix(boot.mem.flags)
    );

    if boot.mem.entry_count == 0 || boot.mem.entries_ptr == 0 {
        crate::klogln!("[mem] <none>");
        return;
    }

    if boot.mem.entry_size as usize != core::mem::size_of::<MemMapEntry>() {
        crate::klogln!(
            "[mem] unsupported entry_size={} (expected {})",
            boot.mem.entry_size,
            core::mem::size_of::<MemMapEntry>()
        );
        return;
    }

    let entries = unsafe {
        core::slice::from_raw_parts(
            boot.mem.entries_ptr as *const MemMapEntry,
            boot.mem.entry_count as usize,
        )
    };
    for (i, entry) in entries.iter().enumerate() {
        crate::klogln!(
            "[mem] {:02} base={:#x} len={:#x} type={}",
            i,
            entry.base.0,
            entry.len,
            mem_type_name(entry.mem_type)
        );
    }
}

fn print_bytespan(label: &str, span: ByteSpan) {
    if span.is_empty() {
        crate::klogln!("[boot] {}=<none>", label);
        return;
    }

    let bytes = unsafe { core::slice::from_raw_parts(span.ptr as *const u8, span.len as usize) };
    if let Ok(s) = core::str::from_utf8(bytes) {
        crate::klogln!("[boot] {}={}", label, s);
        return;
    }

    crate::klog!("[boot] {}=0x", label);
    for b in bytes {
        crate::klog!("{:02x}", b);
    }
    crate::klogln!();
}

fn mem_flags_suffix(flags: u16) -> &'static str {
    if flags & (MemMapFlags::TRUNCATED as u16) != 0 {
        " (TRUNCATED)"
    } else {
        ""
    }
}

fn mem_type_name(t: MemType) -> &'static str {
    match t {
        MemType::Reserved => "reserved",
        MemType::Usable => "usable",
        MemType::AcpiReclaimable => "acpi_reclaimable",
        MemType::AcpiNvs => "acpi_nvs",
        MemType::Mmio => "mmio",
        MemType::BadMemory => "bad_memory",
        MemType::BootloaderReclaimable => "bootloader_reclaimable",
        MemType::KernelAndModules => "kernel_and_modules",
    }
}
