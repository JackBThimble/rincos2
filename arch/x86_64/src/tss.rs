use core::cell::SyncUnsafeCell;

#[repr(C, align(16))]
pub struct Tss64 {
    _rsv0: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    _rsv1: u64,
    pub ist: [u64; 7],
    _rsv2: u64,
    _rsv3: u16,
    pub iopb_offset: u16,
}

impl Tss64 {
    pub const fn new() -> Self {
        Self {
            _rsv0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            _rsv3: 0,
            ist: [0; 7],
            _rsv1: 0,
            _rsv2: 0,
            iopb_offset: core::mem::size_of::<Tss64>() as u16,
        }
    }
}

/// IST stacks
const IST_STACK_SIZE: usize = 16 * 1024;

#[repr(align(16))]
struct Stack([u8; IST_STACK_SIZE]);

#[unsafe(link_section = ".bss.boot")]
static mut DF_IST_STACK: Stack = Stack([0; IST_STACK_SIZE]);

#[inline(always)]
fn stack_top(s: *const Stack) -> u64 {
    (s as u64) + (IST_STACK_SIZE as u64)
}

#[unsafe(link_section = ".bss.boot")]
static TSS: SyncUnsafeCell<Tss64> = SyncUnsafeCell::new(Tss64 {
    _rsv0: 0,
    rsp0: 0,
    rsp1: 0,
    rsp2: 0,
    _rsv3: 0,
    ist: [0; 7],
    _rsv1: 0,
    _rsv2: 0,
    iopb_offset: 0,
});
unsafe impl Sync for Tss64 {}

#[inline(always)]
pub fn tss_ptr() -> *mut Tss64 {
    TSS.get()
}

pub fn init_tss(rsp0_top: u64) {
    unsafe {
        let tss = tss_ptr();
        (*tss).rsp0 = rsp0_top;
        (*tss).ist[0] = stack_top(core::ptr::addr_of_mut!(DF_IST_STACK).cast::<Stack>());
        (*tss).iopb_offset = core::mem::size_of::<Tss64>() as u16;
    }
}
