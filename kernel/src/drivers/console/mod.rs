mod serial;

use crate::log;
use bootabi::BootInfo;

static SERIAL: serial::Serial = serial::Serial::new();
// static FB: fb::FbConsole = fb::FbConsole::new();

pub fn init(_boot: &BootInfo) {
    SERIAL.init();

    log::set_logger(&SERIAL);

    SERIAL.write_raw(b'O');
    SERIAL.write_raw(b'K');
    SERIAL.write_raw(b'\n');
}
