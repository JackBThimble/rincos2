use bootabi::BootInfo;

pub fn kmain(boot: &BootInfo) -> ! {
    crate::debug::early_serial::write_str("ENTER kmain\n");
    crate::klogln!("[init] arch core");
    unsafe {
        crate::arch::init_core();
    }

    crate::klogln!("[init] interrupts");
    crate::interrupts::init();

    crate::bootinfo::print_boot_info(boot);

    crate::klogln!("[init] arch time");
    let has_time = crate::arch::init_time_source();

    crate::klogln!("[init] arch irqs");
    let apic_ok = unsafe { crate::arch::init_irqs(boot, has_time) };
    if !apic_ok {
        panic!("arch irqs apic unavailable");
    }

    crate::klogln!("[init] time");
    crate::time::init();

    crate::arch::enable_interrupts();

    crate::klogln!("[ok] idle");
    loop {
        crate::arch::cpu_relax();
    }
}
