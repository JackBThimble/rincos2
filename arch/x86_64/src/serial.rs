#![allow(dead_code)]

const COM1: u16 = 0x3f8;

pub fn outb(port: u16, val: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
    }
}

pub fn inb(port: u16) -> u8 {
    unsafe {
        let mut v: u8;
        core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nomem, nostack, preserves_flags));
        v
    }
}

pub fn com1_write(b: u8) {
    while (inb(COM1 + 5) & 0x20) == 0 {}
    outb(COM1, b);
}
