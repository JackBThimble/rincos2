#![no_std]

pub const ARCH_NAME: &str = "x86_64";

pub mod serial;

use bootabi::BootInfo;

pub fn init(_boot: &BootInfo) {
    // Later: GDT/IDT, APIC, paging, timer.
}

pub fn cpu_relax() {
    unsafe { core::arch::asm!("pause", options(nomem, nostack, preserves_flags)) }
}
