use core::arch::asm;
use core::cell::SyncUnsafeCell;

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_lo: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_hi: u32,
    zero: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_lo: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_hi: 0,
            zero: 0,
        }
    }
}

static IDT: SyncUnsafeCell<[IdtEntry; 256]> = SyncUnsafeCell::new([IdtEntry::missing(); 256]);

unsafe extern "C" {
    // Provided by stubs.S
    static isr_stub_table: [u64; 32];
    static irq_stub_table: [u64; 16];

    fn irq_224();
    fn irq_225();
}

pub const TIMER_VEC: u8 = 0xe0;
pub const TLB_SHOOTDOWN_VEC: u8 = 0xe1;

pub unsafe fn init_idt() {
    // Exceptions 0..31
    unsafe {
        for vec in 0..32usize {
            let handler = isr_stub_table[vec];
            set_gate(
                vec as u8,
                handler,
                /*ist=*/ if vec == 8 { 1 } else { 0 },
            ); // vec 8 = #DF uses IST1
        }

        // IRQs mapped at 32..47 (we’ll route IOAPIC later)
        for i in 0..16usize {
            let handler = irq_stub_table[i];
            set_gate((32 + i) as u8, handler, 0);
        }

        // Install LAPIC timer vector (TSC-deadline)
        set_gate(TIMER_VEC, irq_224 as *const () as u64, 0);
        set_gate(TLB_SHOOTDOWN_VEC, irq_225 as *const () as u64, 0);

        let idtr = Idtr {
            limit: (core::mem::size_of_val(&IDT) - 1) as u16,
            base: (&IDT as *const _ as u64),
        };
        asm!("lidt [{}]", in(reg) &idtr, options(readonly, nostack));
    }
}

unsafe fn set_gate(vec: u8, handler: u64, ist: u8) {
    unsafe {
        let idt = &mut *IDT.get();
        let entry = &mut idt[vec as usize];
        entry.offset_lo = handler as u16;
        entry.selector = crate::gdt::KERNEL_CS;
        entry.ist = ist & 0x7;
        entry.type_attr = 0x8E; // present=1, DPL=0, type=0xE (interrupt gate)
        entry.offset_mid = (handler >> 16) as u16;
        entry.offset_hi = (handler >> 32) as u32;
        entry.zero = 0;
    }
}
