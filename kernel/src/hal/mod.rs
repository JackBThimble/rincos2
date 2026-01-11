use bootabi::BootInfo;

#[cfg(feature = "x86_64")]
use arch_x86_64 as arch;

#[cfg(feature = "aarch64")]
use arch_aarch64 as arch;

pub const ARCH_NAME: &str = arch::ARCH_NAME;

#[inline(always)]
pub fn init(boot: &BootInfo) {
    arch::init(boot);
}

#[inline(always)]
pub fn cpu_relax() {
    arch::cpu_relax();
}

#[inline(always)]
pub fn serial_write_byte(b: u8) {
    #[cfg(feature = "x86_64")]
    arch::serial::com1_write(b);
}
