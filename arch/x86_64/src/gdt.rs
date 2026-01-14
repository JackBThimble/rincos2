use core::arch::asm;
use core::mem::size_of;

use crate::tss;
use crate::util::SyncUnsafeCell;

#[repr(C, packed)]
struct Gdtr {
    limit: u16,
    base: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct GdtEntry(u64);

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct TssDesc {
    lo: u64,
    hi: u64,
}

static GDT: SyncUnsafeCell<[GdtEntry; 3]> = SyncUnsafeCell::new([
    GdtEntry(0), // null
    GdtEntry(0), // code
    GdtEntry(0), // data
]);

static mut TSS_DESC: TssDesc = TssDesc { lo: 0, hi: 0 };

#[repr(C, align(16))]
struct FullTable {
    gdt: [GdtEntry; 3],
    tss: TssDesc,
}

static mut FULL: FullTable = FullTable {
    gdt: [GdtEntry(0); 3],
    tss: TssDesc { lo: 0, hi: 0 },
};

pub const KERNEL_CS: u16 = 0x08;
pub const KERNEL_DS: u16 = 0x10;
pub const TSS_SEL: u16 = 0x18; // immediately after GDT (we'll load via pseudo-table below)

fn gdt_code64() -> u64 {
    // 64-bit code: present | ring0 | executable | readable
    // base/limit ignored in long mode, but descriptor must be valid.
    0x00AF9A000000FFFF
}

fn gdt_data() -> u64 {
    // present | ring0 | data | writable (L=0 for data segments)
    0x00CF92000000FFFF
}

unsafe fn make_tss_desc(tss_addr: u64, tss_len: u32) -> TssDesc {
    // System segment descriptor (TSS available) is 16 bytes.
    let limit = (tss_len - 1) as u64;

    let mut lo: u64 = 0;
    lo |= limit & 0xFFFF;
    lo |= (tss_addr & 0xFFFF) << 16;
    lo |= ((tss_addr >> 16) & 0xFF) << 32;
    lo |= 0x89u64 << 40; // type=0x9 (available TSS), present=1
    lo |= (limit & 0xF0000) << 32;
    lo |= ((tss_addr >> 24) & 0xFF) << 56;

    let hi = tss_addr >> 32;

    TssDesc { lo, hi }
}

pub unsafe fn build_gdt_tss(rsp0_top: u64) {
    unsafe {
        init_tss_only(rsp0_top);
        build_gdt_entries();
        build_full_table();
    }
}

pub unsafe fn init_tss_only(rsp0_top: u64) {
    unsafe {
        tss::init_tss(rsp0_top);
    }
}

pub unsafe fn build_gdt_entries_only() {
    unsafe {
        let gdt = &mut *GDT.get();

        gdt[1] = GdtEntry(gdt_code64());
        gdt[2] = GdtEntry(gdt_data());
    }
}

pub unsafe fn build_tss_desc() {
    unsafe {
        let tss_addr = tss::tss_ptr() as u64;
        if !is_canonical(tss_addr) {
            return;
        }
        let desc = make_tss_desc(tss_addr, size_of::<tss::Tss64>() as u32);
        core::ptr::write_unaligned(core::ptr::addr_of_mut!(TSS_DESC.lo), desc.lo);
        core::ptr::write_unaligned(core::ptr::addr_of_mut!(TSS_DESC.hi), desc.hi);
    }
}

pub unsafe fn build_gdt_entries() {
    unsafe {
        build_gdt_entries_only();
        build_tss_desc();
    }
}

pub unsafe fn build_full_table() {
    unsafe {
        let gdt = &*GDT.get();
        core::ptr::write_volatile(&mut FULL.gdt[0], gdt[0]);
        core::ptr::write_volatile(&mut FULL.gdt[1], gdt[1]);
        core::ptr::write_volatile(&mut FULL.gdt[2], gdt[2]);

        let lo = core::ptr::read_unaligned(core::ptr::addr_of!(TSS_DESC.lo));
        let hi = core::ptr::read_unaligned(core::ptr::addr_of!(TSS_DESC.hi));
        core::ptr::write_unaligned(core::ptr::addr_of_mut!(FULL.tss.lo), lo);
        core::ptr::write_unaligned(core::ptr::addr_of_mut!(FULL.tss.hi), hi);
    }
}

#[inline(always)]
fn is_canonical(addr: u64) -> bool {
    let sign = (addr >> 47) & 1;
    let upper = addr >> 48;
    if sign == 0 {
        upper == 0
    } else {
        upper == 0xffff
    }
}

pub unsafe fn load_gdt() {
    const GDT_BYTES: usize = core::mem::size_of::<GdtEntry>() * 3
        + core::mem::size_of::<TssDesc>();
    let gdtr = Gdtr {
        limit: (GDT_BYTES - 1) as u16,
        base: (&raw const FULL as *const _ as u64),
    };

    asm!("lgdt [{}]", in(reg) &gdtr, options(readonly, nostack));
}

pub unsafe fn reload_segments() {
    // Reload segment registers (CS needs far jump)
    asm!(
        "push {cs}",
        "lea rax, [rip + 2f]",
        "push rax",
        "retfq",
        "2:",
        cs = const KERNEL_CS,
        options(preserves_flags)
    );

    asm!(
        "mov ax, {ds}",
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",
        "mov fs, ax",
        "mov gs, ax",
        ds = const KERNEL_DS,
        options(nostack)
    );
}

pub unsafe fn load_tss() {
    // Selector points to the TSS descriptor in FULL after 3 entries => 0x18
    asm!("ltr ax", in("ax") TSS_SEL, options(nostack));
}

pub unsafe fn init_gdt_and_tss(rsp0_top: u64) {
    unsafe {
        build_gdt_tss(rsp0_top);
        load_gdt();
        reload_segments();
        load_tss();
    }
}
