#![no_std]
#![no_main]

mod arch;
mod debug;
mod drivers;
mod interrupts;
mod kmain;
mod log;
mod panic;
mod time;

use bootabi::{Arch, BOOTABI_MAGIC, BOOTABI_VERSION, BootHeader, BootInfo};

#[unsafe(no_mangle)]
pub extern "C" fn kentry() -> ! {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags))
    }
    debug::early_serial::write_str("ENTER kentry\n");

    #[cfg(target_arch = "x86_64")]
    let arch = Arch::X86_64;
    #[cfg(target_arch = "aarch64")]
    let arch = Arch::Aarch64;

    crate::debug::early_serial::write_str("set arch\n");

    let boot: BootInfo = bootloader_limine::gather_bootinfo(arch, 1);
    crate::debug::early_serial::write_str("AFTER bootinfo\n");

    drivers::console::init(&boot);

    klogln!("logger up");
    klogln!("arch={}", hal::ARCH_NAME);
    klogln!("hhdm={:#X}", boot.hhdm_offset);
    klogln!("mem entries={}", boot.mem.entry_count);

    kmain::kmain(&boot)
}
