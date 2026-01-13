use crate::apic;
use crate::idt;
use crate::tsc;
use hal::interrupt::{IrqFrame, dispatch};

#[repr(C)]
pub struct ExceptionContext {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,

    pub vector: u64,
    pub error_code: u64,

    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[unsafe(no_mangle)]
pub extern "C" fn exception_dispatch(ctx: *mut ExceptionContext) {
    let ctx = unsafe { &mut *ctx };

    dispatch(IrqFrame {
        vector: ctx.vector as u8,
        error_code: ctx.error_code,
    });

    if ctx.vector == 14 {
        let _cr2: u64;
        unsafe { core::arch::asm!("mov {}, cr2", out(reg) _cr2) };
    }

    loop {
        unsafe { core::arch::asm!("cli; hlt") }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_dispatch(ctx: *mut ExceptionContext) {
    let ctx = unsafe { &mut *ctx };
    let vec = ctx.vector as u8;

    dispatch(IrqFrame {
        vector: ctx.vector as u8,
        error_code: ctx.error_code,
    });

    if vec == idt::TIMER_VEC {
        if let Some(ticks) = tsc::ticks_from_ns(10_000_000) {
            tsc::set_deadline_after_ticks(ticks);
        }
    }

    unsafe {
        apic::eoi();
    }
}
