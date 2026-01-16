use core::arch::asm;

pub const IA32_TSC_DEADLINE: u32 = 0x0000_06e0;
pub const IA32_APIC_BASE: u32 = 0x1b;
pub const IA32_EFER: u32 = 0xc000_0080;
pub const IA32_STAR: u32 = 0xc000_0081;
pub const IA32_LSTAR: u32 = 0xc000_0082;
pub const IA32_FMASK: u32 = 0xc000_0084;
pub const IA32_FS_BASE: u32 = 0xc000_0100;
pub const IA32_GS_BASE: u32 = 0xc000_0101;
pub const IA32_KERNEL_GS_BASE: u32 = 0xc000_0102;

#[inline(always)]
pub unsafe fn wrmsr(msr: u32, val: u64) {
    unsafe {
        let lo = val as u32;
        let hi = (val >> 32) as u32;
        asm!(
            "wrmsr",
                in("ecx") msr,
                in("eax") lo,
                in("edx") hi,
                options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline(always)]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    unsafe {
        let lo: u32;
        let hi: u32;

        asm!(
            "rdmsr",
                in("ecx") msr,
                out("eax") lo,
                out("edx") hi,
                options(nostack, preserves_flags, nomem)
        );
        ((hi as u64) << 32) | (lo as u64)
    }
}
