use crate::hal;

pub fn write_str(s: &str) {
    for b in s.bytes() {
        if b == b'\n' {
            hal::serial_write_byte(b'\r');
        }
        hal::serial_write_byte(b);
    }
}
