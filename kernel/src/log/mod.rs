use core::fmt;

static mut LOGGER: Option<&'static dyn LogSink> = None;

pub trait LogSink {
    fn write_str(&self, s: &str);
}

pub fn set_logger(l: &'static dyn LogSink) {
    unsafe { LOGGER = Some(l) }
}

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    struct Adapter;
    impl Write for Adapter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            unsafe {
                if let Some(l) = LOGGER {
                    l.write_str(s);
                }
            }
            Ok(())
        }
    }
    let _ = Adapter.write_fmt(args);
}

#[macro_export]
macro_rules! klog {
    ($($arg:tt)*) => {
        $crate::log::_print(core::format_args!($($arg)*))
    }
}

#[macro_export]
macro_rules! klogln {
    () => {
        $crate::klog!("\n")
    };
    ($($arg:tt)*) => {
        $crate::klog!("{}\n", core::format_args!($($arg)*))
    };
}
