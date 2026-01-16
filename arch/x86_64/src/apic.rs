use core::ptr::{read_volatile, write_volatile};

use crate::cpuid;
use crate::idt::TIMER_VEC;
use crate::msr::*;

const APIC_BASE_ENABLE: u64 = 1 << 11;
const APIC_BASE_X2APIC: u64 = 1 << 10;
const APIC_BASE_BSP: u64 = 1 << 8;

const LAPIC_ID: u32 = 0x020;
const LAPIC_EOI: u32 = 0x0b0;
const LAPIC_SVR: u32 = 0x0f0;
const LAPIC_LVT_TIMER: u32 = 0x320;
const LAPIC_TIMER_DIV: u32 = 0x3e0;
const LAPIC_ICR_LOW: u32 = 0x300;
const LAPIC_ICR_HIGH: u32 = 0x310;

const SVR_APIC_ENABLE: u32 = 1 << 8;

const LVT_MASKED: u32 = 1 << 16;
const LVT_MODE_TSC_DEADLINE: u32 = 0b10 << 17;
const ICR_DELIVERY_PENDING: u32 = 1 << 12;

const APIC_MODE_NONE: u8 = 0;
const APIC_MODE_XAPIC: u8 = 1;
const APIC_MODE_X2APIC: u8 = 2;

static mut APIC_MODE: u8 = APIC_MODE_NONE;
static mut LAPIC_BASE_VIRT: u64 = 0;

pub unsafe fn init(hhdm_offset: u64) -> bool {
    if cpuid::has_x2apic() {
        let apic_base = unsafe { rdmsr(IA32_APIC_BASE) };
        let new_base = apic_base | APIC_BASE_ENABLE | APIC_BASE_X2APIC;
        unsafe { wrmsr(IA32_APIC_BASE, new_base) };
        unsafe {
            APIC_MODE = APIC_MODE_X2APIC;
            let svr = (SVR_APIC_ENABLE) | 0xff;
            write(LAPIC_SVR, svr);
            write(LAPIC_TIMER_DIV, 0b1011);
            let lvt = (TIMER_VEC as u32) | LVT_MODE_TSC_DEADLINE;
            write(LAPIC_LVT_TIMER, lvt);
            let _id = read(LAPIC_ID);
        }
        return true;
    }

    // read apic base MSR
    let apic_base = unsafe { rdmsr(IA32_APIC_BASE) };
    // physical base is bits 12..35 (4KiB aligned)
    let apic_phys = apic_base & 0xffff_f000;
    // enable xAPIC
    let new_base = apic_base | APIC_BASE_ENABLE;
    unsafe { wrmsr(IA32_APIC_BASE, new_base) };
    // Map via HHDM (Limine gives this)
    unsafe {
        APIC_MODE = APIC_MODE_XAPIC;
        LAPIC_BASE_VIRT = apic_phys.wrapping_add(hhdm_offset);

        let svr = (SVR_APIC_ENABLE) | 0xff;
        write(LAPIC_SVR, svr);

        write(LAPIC_TIMER_DIV, 0b1011);

        let lvt = (TIMER_VEC as u32) | LVT_MODE_TSC_DEADLINE;
        write(LAPIC_LVT_TIMER, lvt);

        let _id = read(LAPIC_ID);
    }
    true
}

pub fn cpu_id() -> u32 {
    unsafe {
        let id = read(LAPIC_ID);
        match APIC_MODE {
            APIC_MODE_XAPIC => id >> 24,
            APIC_MODE_X2APIC => id,
            _ => 0,
        }
    }
}

pub fn send_ipi_all_others(vector: u8) {
    unsafe {
        if APIC_MODE == APIC_MODE_NONE {
            return;
        }
    }

    let icr = (vector as u32) | (0b11 << 18);

    unsafe {
        if APIC_MODE == APIC_MODE_XAPIC {
            write(LAPIC_ICR_HIGH, 0);
            write(LAPIC_ICR_LOW, icr);
            while (read(LAPIC_ICR_LOW) & ICR_DELIVERY_PENDING) != 0 {}
        } else {
            write(LAPIC_ICR_LOW, icr);
        }
    }
}

#[inline(always)]
pub unsafe fn eoi() {
    unsafe {
        if APIC_MODE != APIC_MODE_NONE {
            write(LAPIC_EOI, 0);
        }
    }
}

#[inline(always)]
unsafe fn read(off: u32) -> u32 {
    unsafe {
        match APIC_MODE {
            APIC_MODE_X2APIC => rdmsr(apic_msr(off)) as u32,
            APIC_MODE_XAPIC => {
                let addr = (LAPIC_BASE_VIRT + off as u64) as *const u32;
                read_volatile(addr)
            }
            _ => 0,
        }
    }
}

#[inline(always)]
unsafe fn write(off: u32, val: u32) {
    unsafe {
        match APIC_MODE {
            APIC_MODE_X2APIC => wrmsr(apic_msr(off), val as u64),
            APIC_MODE_XAPIC => {
                let addr = (LAPIC_BASE_VIRT + off as u64) as *mut u32;
                write_volatile(addr, val);
            }
            _ => {}
        }
    }
}

#[inline(always)]
fn apic_msr(off: u32) -> u32 {
    0x800 + (off >> 4)
}
