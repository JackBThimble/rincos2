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
use hal::SerialWriter;

struct Com1Writer;

impl SerialWriter for Com1Writer {
    fn write_byte(&self, b: u8) {
        serial::com1_write(b);
    }
}

static COM1_WRITER: Com1Writer = Com1Writer;

pub fn early_init() {
    disable_interrupts();
    unsafe {
        core::arch::asm!("cld", options(nomem, nostack));
    }
    unsafe {
        enable_sse();
    }
    unsafe {
        hal::register_serial_writer(&COM1_WRITER);
    }
}

#[inline(always)]
unsafe fn enable_sse() {
    let mut cr0: u64;
    let mut cr4: u64;
    core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
    // Enable FPU/SSE: clear EM/TS, set MP/NE.
    cr0 &= !(1 << 2); // EM
    cr0 &= !(1 << 3); // TS
    cr0 |= 1 << 1; // MP
    cr0 |= 1 << 5; // NE
    core::arch::asm!("mov cr0, {}", in(reg) cr0, options(nomem, nostack, preserves_flags));

    core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
    cr4 |= 1 << 9; // OSFXSR
    cr4 |= 1 << 10; // OSXMMEXCPT
    core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack, preserves_flags));
}

pub unsafe fn init(boot: &BootInfo) {
    unsafe {
        init_core();
    }
    let has_time = init_time_source();
    unsafe {
        init_irqs(boot, has_time);
    }
}

pub unsafe fn init_core() {
    unsafe {
        init_core_stage1a();
        init_core_stage1b1();
        init_core_stage1b2();
        init_core_stage1c();
        init_core_stage2();
        init_core_stage3();
        init_core_stage4();
        init_core_stage5();
        init_core_stage6();
    }
}

pub unsafe fn init_core_stage1a() {
    let rsp0_top = current_rsp();
    unsafe {
        gdt::init_tss_only(rsp0_top);
    }
}

pub unsafe fn init_core_stage1b1() {
    unsafe {
        gdt::build_gdt_entries_only();
    }
}

pub unsafe fn init_core_stage1b2() {
    // no-op: TSS descriptor built in stage6
}

pub unsafe fn init_core_stage1c() {
    unsafe {
        gdt::build_full_table();
    }
}

pub unsafe fn init_core_stage2() {
    unsafe {
        gdt::load_gdt();
    }
}

pub unsafe fn init_core_stage3() {
    unsafe {
        gdt::reload_segments();
    }
}

pub unsafe fn init_core_stage4() {
    unsafe {
        idt::init_idt();
    }
}

pub unsafe fn init_core_stage5() {
    pic_mask_all();
}

pub unsafe fn init_core_stage6() {
    unsafe {
        gdt::build_tss_desc();
        gdt::build_full_table();
        gdt::load_tss();
    }
}

pub fn init_time_source() -> bool {
    tsc::init()
}

pub unsafe fn init_irqs(boot: &BootInfo, has_time: bool) -> bool {
    let apic_ok = unsafe { apic::init(boot.hhdm_offset) };
    if has_time {
        tsc::register_timer();
    }
    apic_ok
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
    crate::serial::outb(0x20, 0x20);
    crate::serial::outb(0xa0, 0x20);
}

pub fn cpu_relax() {
    unsafe { core::arch::asm!("pause", options(nomem, nostack, preserves_flags)) }
}

pub fn disable_interrupts() {
    unsafe { core::arch::asm!("cli", options(nomem, nostack, preserves_flags)) }
}

pub fn enable_interrupts() {
    unsafe { core::arch::asm!("sti", options(nomem, nostack, preserves_flags)) }
}
