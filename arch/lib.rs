#![no_std]
#![feature(sync_unsafe_cell)]

#[cfg(target_arch = "x86_64")]
pub use arch_x86_64::*;

#[cfg(target_arch = "aarch64")]
pub use arch_aarch64::*;

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("arch: unsupported target_arch");
