use crate::{hal, log::LogSink};

pub struct Serial;

impl Serial {
    pub const fn new() -> Self {
        Self
    }

    pub fn init(&self) {
        // COM1 already usable in QEMU, full init later
    }

    #[inline(always)]
    pub fn write_raw(&self, b: u8) {
        hal::serial_write_byte(b);
    }
}

impl LogSink for Serial {
    fn write_str(&self, s: &str) {
        for b in s.bytes() {
            if b == b'\n' {
                self.write_raw(b'\r');
            }
            self.write_raw(b);
        }
    }
}
