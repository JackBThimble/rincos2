pub trait TimerOps {
    fn now_ticks(&self) -> u64;
    fn frequency_hz(&self) -> u64;
    fn arm_one_shot(&self, deadline_ticks: u64);
}

static mut TIMER: Option<&'static dyn TimerOps> = None;

pub unsafe fn register_timer(t: &'static dyn TimerOps) {
    unsafe {
        TIMER = Some(t);
    }
}

#[inline(always)]
pub fn now_ticks() -> u64 {
    unsafe { TIMER.map(|t| t.now_ticks()).unwrap_or(0) }
}

#[inline(always)]
pub fn frequency_hz() -> u64 {
    unsafe { TIMER.map(|t| t.frequency_hz()).unwrap_or(0) }
}

#[inline(always)]
pub fn arm_one_shot(deadline_ticks: u64) {
    unsafe {
        if let Some(t) = TIMER {
            t.arm_one_shot(deadline_ticks);
        }
    }
}
