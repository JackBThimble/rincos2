use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::debug::early_serial::write_str("\n[PANIC] ");

    use core::fmt::Write;
    struct W;
    impl Write for W {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            crate::debug::early_serial::write_str(s);
            Ok(())
        }
    }

    let _ = write!(&mut W, "{}\n", info);
    loop {}
}
