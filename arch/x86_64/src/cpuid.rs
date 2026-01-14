use core::arch::x86_64::__cpuid_count;

#[inline(always)]
pub fn has_leaf(leaf: u32) -> bool {
    let max = __cpuid_count(0, 0).eax;
    leaf <= max
}

#[inline(always)]
pub fn cpuid(leaf: u32, sub: u32) -> (u32, u32, u32, u32) {
    let r = __cpuid_count(leaf, sub);
    (r.eax, r.ebx, r.ecx, r.edx)
}

/// Invariant TSC: CPUID.80000007H:EDX[8]
pub fn has_invariant_tsc() -> bool {
    let (max_ext, _, _, _) = cpuid(0x8000_0000, 0);
    if max_ext < 0x8000_0007 {
        return false;
    }
    let (_, _, _, edx) = cpuid(0x8000_0007, 0);
    (edx & (1 << 8)) != 0
}

/// CPUID.1H:ECX[21] x2APIC support
pub fn has_x2apic() -> bool {
    let (_, _, ecx, _) = cpuid(1, 0);
    (ecx & (1 << 21)) != 0
}

/// Returns Some(tsc_hz) if available via CPUID, else None.
/// Best source: CPUID.15H (TSC/crystal ratio + crystal Hz)
/// Fallback: CPUID.16H base MHz (less reliable)
pub fn tsc_hz() -> Option<u64> {
    // Leaf 0x15: eax=denom, ebx=numer, ecx=crystal_hz
    if has_leaf(0x15) {
        let (eax, ebx, ecx, _) = cpuid(0x15, 0);
        let denom = eax as u64;
        let numer = ebx as u64;
        let crystal = ecx as u64;

        if denom != 0 && numer != 0 && crystal != 0 {
            // tsc_hz = crystal * numer / denom
            return Some(crystal.saturating_mul(numer) / denom);
        }
    }

    if has_leaf(0x16) {
        let (eax, _, _, _) = cpuid(0x16, 0);
        let mhz = eax as u64;
        if mhz != 0 {
            return Some(mhz * 1_000_000);
        }
    }
    None
}
