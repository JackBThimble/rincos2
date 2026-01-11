#![no_std]
#![no_main]

mod debug;
mod drivers;
mod hal;
mod kmain;
mod log;
mod panic;

use bootabi::{Arch, BootInfo};

#[unsafe(no_mangle)]
pub extern "C" fn kentry() -> ! {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags))
    }
    debug::early_serial::write_str("ENTER kentry\n");

    let arch = if cfg!(target_arch = "x86_64") {
        Arch::X86_64
    } else {
        Arch::Aarch64
    };

    let boot: BootInfo<'static> = bootloader_limine::gather_bootinfo(arch, 1);
    crate::debug::early_serial::write_str("AFTER bootinfo\n");

    drivers::console::init(&boot);

    klogln!("logger up");
    klogln!("arch={}", hal::ARCH_NAME);
    klogln!("hhdm={:#X}", boot.hhdm_offset);
    klogln!("mem entries={}", boot.mem.entries.len());

    kmain::kmain(&boot)
}

// use core::sync::atomic::AtomicBool;

// static BOOTED: AtomicBool = AtomicBool::new(false);
//
// #[unsafe(no_mangle)]
// pub extern "C" fn kentry() -> ! {
//     let arch = if cfg!(target_arch = "x86_64") {
//         Arch::X86_64
//     } else {
//         Arch::Aarch64
//     };
//
//     let boot: BootInfo<'static> = bootloader_limine::gather_bootinfo(arch, 1);
//
//     drivers::console::init(&boot);
//     klogln!("bootloader = {:?}", boot.bootloader);
//     klogln!("cmdline    = {:?}", boot.cmdline);
//     klogln!("hhdm       = {:#X}", boot.hhdm_offset);
//
//     kmain::kmain(&boot)
// }
