use hal::interrupt::{IrqFrame, IrqKind, InterruptHandler};

struct KernelInterrupts;

static HANDLER: KernelInterrupts = KernelInterrupts;

pub fn init() {
    unsafe {
        hal::interrupt::register_handler(&HANDLER);
    }
}

impl InterruptHandler for KernelInterrupts {
    fn on_interrupt(&self, frame: IrqFrame) {
        match frame.kind {
            IrqKind::Timer => crate::time::on_timer_tick(),
            IrqKind::Fault => handle_fault(frame),
            IrqKind::External => {
                // TODO: route to device drivers
                let _ = frame;
            }
            IrqKind::Spurious | IrqKind::Unknown => {}
        }
    }
}

fn handle_fault(frame: IrqFrame) {
    // Policy: fatal faults abort the current execution context.
    panic!(
        "fault {:?} err={:#x} addr={:#x}",
        frame.fault_kind, frame.error_code, frame.fault_addr
    );
}
