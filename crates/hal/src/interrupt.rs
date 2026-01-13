#[repr(C)]
#[derive(Clone, Copy)]
pub struct IrqFrame {
    pub vector: u8,
    pub error_code: u64,
}

pub trait InterruptHandler {
    fn on_interrupt(&self, frame: IrqFrame);
}

static mut HANDLER: Option<&'static dyn InterruptHandler> = None;

pub unsafe fn register_handler(h: &'static dyn InterruptHandler) {
    unsafe {
        HANDLER = Some(h);
    }
}

#[inline(always)]
pub fn dispatch(frame: IrqFrame) {
    unsafe {
        if let Some(h) = HANDLER {
            h.on_interrupt(frame);
        }
    }
}
