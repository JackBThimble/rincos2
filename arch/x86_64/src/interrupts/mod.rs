use crate::apic;
use crate::idt;
use crate::mmu;
use hal::interrupt::{dispatch, FaultKind, IrqFrame, IrqKind};

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
    let vec = ctx.vector as u8;
    let fault_kind = decode_fault_kind(vec);
    let fault_addr = if fault_kind == FaultKind::PageFault {
        read_cr2()
    } else {
        0
    };

    dispatch(IrqFrame {
        kind: IrqKind::Fault,
        fault_kind,
        irq: 0,
        error_code: ctx.error_code,
        fault_addr,
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_dispatch(ctx: *mut ExceptionContext) {
    let ctx = unsafe { &mut *ctx };
    let vec = ctx.vector as u8;
    if vec == idt::TLB_SHOOTDOWN_VEC {
        mmu::handle_tlb_shootdown();
        unsafe {
            apic::eoi();
        }
        return;
    }
    let kind = if vec == idt::TIMER_VEC {
        IrqKind::Timer
    } else {
        IrqKind::External
    };

    dispatch(IrqFrame {
        kind,
        fault_kind: FaultKind::None,
        irq: irq_line(vec),
        error_code: ctx.error_code,
        fault_addr: 0,
    });

    unsafe {
        apic::eoi();
    }
}

#[inline(always)]
fn read_cr2() -> u64 {
    unsafe {
        let v: u64;
        core::arch::asm!("mov {}, cr2", out(reg) v, options(nomem, nostack, preserves_flags));
        v
    }
}

fn decode_fault_kind(vec: u8) -> FaultKind {
    match vec {
        0 => FaultKind::DivideByZero,
        6 => FaultKind::InvalidOpcode,
        8 => FaultKind::DoubleFault,
        13 => FaultKind::GeneralProtection,
        14 => FaultKind::PageFault,
        _ => FaultKind::Unknown,
    }
}

fn irq_line(vec: u8) -> u16 {
    if (32..48).contains(&vec) {
        (vec - 32) as u16
    } else {
        0
    }
}
