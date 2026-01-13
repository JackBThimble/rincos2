#![no_std]
#![feature(x86_amx_intrinsics)]

pub const ARCH_NAME: &str = "x86_64";

pub mod apic;
pub mod cpuid;
pub mod gdt;
pub mod idt;
pub mod interrupts;
pub mod msr;
pub mod serial;
pub mod tsc;
pub mod tss;
mod util;

use bootabi::BootInfo;

pub unsafe fn init(boot: &BootInfo) {
    let rsp0_top = current_rsp();

    unsafe {
        gdt::init_gdt_and_tss(rsp0_top);
        idt::init_idt();
    }

    pic_mask_all();

    tsc::init();
    unsafe {
        apic::init(boot.hhdm_offset);
    }

    if let Some(ticks) = tsc::ticks_from_ns(10_000_000) {
        tsc::set_deadline_after_ticks(ticks);
    }
}

#[inline(always)]
fn current_rsp() -> u64 {
    unsafe {
        let v: u64;
        core::arch::asm!("mov {}, rsp", out(reg) v, options(nomem, nostack, preserves_flags));
        v
    }
}

fn pic_mask_all() {
    const PIC1_DATA: u16 = 0x21;
    const PIC2_DATA: u16 = 0xa1;

    crate::serial::outb(PIC1_DATA, 0xff);
    crate::serial::outb(PIC2_DATA, 0xff);
}

pub fn cpu_relax() {
    unsafe { core::arch::asm!("pause", options(nomem, nostack, preserves_flags)) }
}
