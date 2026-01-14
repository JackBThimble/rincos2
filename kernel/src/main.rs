#![no_std]
#![no_main]

mod arch;
mod bootinfo;
mod debug;
mod drivers;
mod hal;
mod interrupts;
mod kmain;
mod log;
mod panic;
mod time;

use bootabi::BootInfo;

#[unsafe(no_mangle)]
pub extern "C" fn kentry() -> ! {
    crate::arch::early_init();
    debug::early_serial::write_str("ENTER kentry\n");
    let boot: BootInfo = bootloader_limine::gather_bootinfo(1);
    crate::debug::early_serial::write_str("AFTER bootinfo\n");

    drivers::console::init(&boot);

    klogln!("logger up");

    kmain::kmain(&boot)
}
