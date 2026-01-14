pub trait SerialWriter {
    fn write_byte(&self, b: u8);
}

static mut SERIAL: Option<&'static dyn SerialWriter> = None;

pub unsafe fn register_serial_writer(w: &'static dyn SerialWriter) {
    unsafe {
        SERIAL = Some(w);
    }
}

#[inline(always)]
pub fn write_byte(b: u8) {
    unsafe {
        if let Some(w) = SERIAL {
            w.write_byte(b);
        }
    }
}
