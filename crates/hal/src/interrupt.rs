#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrqKind {
    Fault = 1,
    Timer = 2,
    External = 3,
    Spurious = 4,
    Unknown = 0xff,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultKind {
    None = 0,
    PageFault = 1,
    GeneralProtection = 2,
    InvalidOpcode = 3,
    DivideByZero = 4,
    DoubleFault = 5,
    Unknown = 0xff,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IrqFrame {
    pub kind: IrqKind,
    pub fault_kind: FaultKind,
    pub irq: u16,
    pub error_code: u64,
    pub fault_addr: u64,
}

pub trait InterruptHandler {
    fn on_interrupt(&self, frame: IrqFrame);
}

static mut HANDLER_DATA: usize = 0;
static mut HANDLER_VTABLE: usize = 0;
static mut HANDLER_SET: u8 = 0;

pub unsafe fn register_handler(h: &'static dyn InterruptHandler) {
    unsafe {
        let (data, vtable): (usize, usize) =
            core::mem::transmute::<&'static dyn InterruptHandler, (usize, usize)>(h);
        HANDLER_DATA = data;
        HANDLER_VTABLE = vtable;
        HANDLER_SET = 1;
    }
}

#[inline(always)]
pub fn dispatch(frame: IrqFrame) {
    unsafe {
        if HANDLER_SET == 0 {
            return;
        }
        let ptr: *const dyn InterruptHandler =
            core::mem::transmute::<(usize, usize), *const dyn InterruptHandler>((
                HANDLER_DATA,
                HANDLER_VTABLE,
            ));
        (&*ptr).on_interrupt(frame);
    }
}
