#![no_std]
pub const ARCH_NAME: &str = "aarch64";

use bootabi::BootInfo;

pub fn early_init() {
    disable_interrupts();
}

pub fn init(_boot: &BootInfo) {
    // Later: set exception vectors, enable MMU, set timer, UART.
}

pub unsafe fn init_core() {
    // Later: set exception vectors and CPU state.
}

pub unsafe fn init_gdt_tss() {
    // Not applicable on aarch64.
}

pub unsafe fn init_idt() {
    // Not applicable on aarch64.
}

pub fn init_pic() {
    // Not applicable on aarch64.
}

pub fn init_time_source() -> bool {
    false
}

pub unsafe fn init_irqs(_boot: &BootInfo, _has_time: bool) -> bool {
    // Later: configure interrupt controller and timers.
    false
}

pub fn cpu_relax() {
    unsafe { core::arch::asm!("yield", options(nomem, nostack, preserves_flags)) }
}

pub fn disable_interrupts() {
    unsafe { core::arch::asm!("msr daifset, #2", options(nomem, nostack, preserves_flags)) }
}

pub fn enable_interrupts() {
    unsafe { core::arch::asm!("msr daifclr, #2", options(nomem, nostack, preserves_flags)) }
}
