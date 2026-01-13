use bootabi::BootInfo;

#[cfg(target_arch = "x86_64")]
use arch_x86_64 as arch;

#[cfg(target_arch = "aarch64")]
use arch_aarch64 as arch;

pub unsafe fn init(boot: &BootInfo) {
    unsafe {
        arch::init(boot);
    }
}

#[inline(always)]
pub fn cpu_relax() {
    arch::cpu_relax();
}
