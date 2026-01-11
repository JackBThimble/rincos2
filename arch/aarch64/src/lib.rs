#![no_std]
pub const ARCH_NAME: &str = "aarch64";

pub fn init<_T>(_boot: &_T) {
    // Later: set exception vectors, enable MMU, set timer, UART.
}

pub fn cpu_relax() {
    unsafe { core::arch::asm!("yield", options(nomem, nostack, preserves_flags)) }
}
