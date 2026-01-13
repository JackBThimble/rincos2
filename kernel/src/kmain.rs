use bootabi::BootInfo;

pub fn kmain(boot: &BootInfo) -> ! {
    crate::debug::early_serial::write_str("ENTER kmain\n");
    crate::klogln!("[init] hal");
    unsafe {
        crate::arch::init(boot);
    }

    crate::klogln!("[ok] idle");
    loop {
        crate::arch::cpu_relax();
    }
}
