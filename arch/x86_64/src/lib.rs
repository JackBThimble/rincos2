#![no_std]
#![feature(x86_amx_intrinsics)]
#![feature(sync_unsafe_cell)]

pub const ARCH_NAME: &str = "x86_64";

pub mod apic;
pub mod cpuid;
pub mod gdt;
pub mod idt;
pub mod interrupts;
pub mod mmu;
pub mod msr;
pub mod serial;
pub mod tsc;
pub mod tss;

use bootabi::BootInfo;
use hal::SerialWriter;

struct Com1Writer;

impl SerialWriter for Com1Writer {
    fn write_byte(&self, b: u8) {
        serial::com1_write(b);
    }
}

static COM1_WRITER: Com1Writer = Com1Writer;
static mut HHDM_OFFSET: u64 = 0;

#[inline(always)]
pub(crate) fn hhdm_offset() -> u64 {
    unsafe { HHDM_OFFSET }
}

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
    unsafe {
        core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
    }
    // Enable FPU/SSE: clear EM/TS, set MP/NE.
    cr0 &= !(1 << 2); // EM
    cr0 &= !(1 << 3); // TS
    cr0 |= 1 << 1; // MP
    cr0 |= 1 << 5; // NE
    unsafe {
        core::arch::asm!("mov cr0, {}", in(reg) cr0, options(nomem, nostack, preserves_flags));
    }
    unsafe {
        core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
    }
    cr4 |= 1 << 9; // OSFXSR
    cr4 |= 1 << 10; // OSXMMEXCPT
    unsafe {
        core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack, preserves_flags));
    }
}

pub unsafe fn init(boot: &BootInfo) {
    unsafe {
        init_mmu(boot);
        init_core();
    }
    let has_time = init_time_source();
    unsafe {
        init_irqs(boot, has_time);
    }
}

pub unsafe fn init_mmu(boot: &BootInfo) {
    unsafe {
        HHDM_OFFSET = boot.hhdm_offset;
        mmu::init_features(false);
    }
}

pub unsafe fn init_core() {
    unsafe {
        let rsp0_top = current_rsp();
        init_gdt_and_segments(rsp0_top);
        idt::init_idt();
        mask_legacy_pic();
        // Build the TSS descriptor after IDT/handlers are live.
        load_tss();
    }
}

pub fn init_time_source() -> bool {
    tsc::init()
}

unsafe fn init_gdt_and_segments(rsp0_top: u64) {
    unsafe {
        gdt::init_tss_only(rsp0_top);
        gdt::build_gdt_entries_only();
        gdt::build_full_table();
        gdt::load_gdt();
        gdt::reload_segments();
    }
}

unsafe fn load_tss() {
    unsafe {
        gdt::build_tss_desc();
        gdt::build_full_table();
        gdt::load_tss();
    }
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

fn mask_legacy_pic() {
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

/// MMU backend
pub fn mmu() -> &'static mmu::X86Mmu {
    &mmu::MMU
}
