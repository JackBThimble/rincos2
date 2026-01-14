const TICK_NS: u64 = 10_000_000;

pub fn init() {
    arm_next_tick();
}

pub fn on_timer_tick() {
    arm_next_tick();
}

fn arm_next_tick() {
    let hz = hal::time::frequency_hz();
    if hz == 0 {
        return;
    }
    let ticks = TICK_NS.saturating_mul(hz) / 1_000_000_000;
    if ticks == 0 {
        return;
    }
    let now = hal::time::now_ticks();
    hal::time::arm_one_shot(now.wrapping_add(ticks));
}
