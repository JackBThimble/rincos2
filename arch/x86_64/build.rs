fn main() {
    cc::Build::new()
        .file("src/interrupts/stubs.S")
        .flag("-m64")
        .no_default_flags(true)
        .compile("interrupt_stubs");

    println!("cargo:rerun-if-changed=src/interrupts/stubs.S");
}
