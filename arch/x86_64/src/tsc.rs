use crate::{cpuid, msr::*};
use core::arch::x86_64::{__rdtscp, _mm_lfence};
use hal::time::TimerOps;

static mut TSC_HZ: u64 = 0;

pub struct TscTimer;

#[inline(always)]
pub fn init() -> bool {
    if !cpuid::has_invariant_tsc() {
        unsafe {
            TSC_HZ = 0;
        }
        return false;
    }
    unsafe {
        TSC_HZ = cpuid::tsc_hz().unwrap_or(0);
    }
    true
}

#[inline(always)]
pub fn hz() -> Option<u64> {
    let h = unsafe { TSC_HZ };
    if h == 0 { None } else { Some(h) }
}

#[inline(always)]
pub fn now() -> u64 {
    unsafe {
        let mut aux: u32 = 0;
        let tsc = __rdtscp(&mut aux as *mut u32);
        _mm_lfence();
        tsc
    }
}

/// Convert nanoseconds to TSC ticks if TSC Hz is known.
pub fn ticks_from_ns(ns: u64) -> Option<u64> {
    let hz = hz()?;
    Some(ns.saturating_mul(hz) / 1_000_000_000)
}

/// Program one-shot deadline at absolute TSC value.
#[inline(always)]
pub fn set_deadline_tsc(abs_tsc: u64) {
    unsafe {
        wrmsr(IA32_TSC_DEADLINE, abs_tsc);
    }
}

#[inline(always)]
pub fn set_deadline_after_ticks(ticks: u64) {
    let t = now().wrapping_add(ticks);
    set_deadline_tsc(t);
}

impl TimerOps for TscTimer {
    fn now_ticks(&self) -> u64 {
        now()
    }

    fn frequency_hz(&self) -> u64 {
        hz().unwrap_or(0)
    }

    fn arm_one_shot(&self, deadline_ticks: u64) {
        set_deadline_tsc(deadline_ticks);
    }
}

pub fn register_timer() {
    static TIMER: TscTimer = TscTimer;
    unsafe {
        hal::time::register_timer(&TIMER);
    }
}
