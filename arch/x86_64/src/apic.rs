use core::arch::asm;
use core::ptr::{read_volatile, write_volatile};

use crate::idt::TIMER_VEC;
use crate::msr::*;

const APIC_BASE_ENABLE: u64 = 1 << 11;
const APIC_BASE_BSP: u64 = 1 << 8;

const LAPIC_ID: u32 = 0x020;
const LAPIC_EOI: u32 = 0x0b0;
const LAPIC_SVR: u32 = 0x0f0;
const LAPIC_LVT_TIMER: u32 = 0x320;
const LAPIC_TIMER_DIV: u32 = 0x3e0;

const SVR_APIC_ENABLE: u32 = 1 << 8;

const LVT_MASKED: u32 = 1 << 16;
const LVT_MODE_TSC_DEADLINE: u32 = 0b10 << 17;

static mut LAPIC_BASE_VIRT: u64 = 0;

pub unsafe fn init(hhdm_offset: u64) {
    // read apic base MSR
    let apic_base = unsafe { rdmsr(IA32_APIC_BASE) };
    // physical base is bits 12..35 (4KiB aligned)
    let apic_phys = apic_base & 0xffff_f000;
    // enable xAPIC
    let new_base = apic_base | APIC_BASE_ENABLE;
    unsafe { wrmsr(IA32_APIC_BASE, new_base) };
    // Map via HHDM (Limine gives this)
    unsafe {
        LAPIC_BASE_VIRT = apic_phys.wrapping_add(hhdm_offset);

        let svr = (SVR_APIC_ENABLE) | 0xff;
        write(LAPIC_SVR, svr);

        write(LAPIC_TIMER_DIV, 0b1011);

        let lvt = (TIMER_VEC as u32) | LVT_MODE_TSC_DEADLINE;
        write(LAPIC_LVT_TIMER, lvt);

        let _id = read(LAPIC_ID);
    }
}

#[inline(always)]
pub unsafe fn eoi() {
    unsafe {
        write(LAPIC_EOI, 0);
    }
}

#[inline(always)]
unsafe fn read(off: u32) -> u32 {
    unsafe {
        let addr = (LAPIC_BASE_VIRT + off as u64) as *const u32;
        read_volatile(addr)
    }
}

#[inline(always)]
unsafe fn write(off: u32, val: u32) {
    unsafe {
        let addr = (LAPIC_BASE_VIRT + off as u64) as *mut u32;
        write_volatile(addr, val);
    }
}
