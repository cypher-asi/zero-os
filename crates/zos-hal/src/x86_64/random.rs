//! Random number generation for x86_64 using RDRAND
//!
//! This module provides random byte generation using the hardware RDRAND
//! instruction available on modern x86_64 processors.

/// Check if RDRAND instruction is supported
pub fn is_supported() -> bool {
    // Use CPUID to check for RDRAND support (ECX bit 30 when EAX=1)
    let ecx: u32;
    unsafe {
        core::arch::asm!(
            "push rbx",
            "mov eax, 1",
            "cpuid",
            "pop rbx",
            out("ecx") ecx,
            out("eax") _,
            out("edx") _,
            options(preserves_flags),
        );
    }
    ecx & (1 << 30) != 0
}

/// Read a random 64-bit value using RDRAND instruction
///
/// Returns None if RDRAND fails (hardware retry exhausted).
pub fn rdrand_u64() -> Option<u64> {
    let mut value: u64;
    let success: u8;
    
    unsafe {
        core::arch::asm!(
            "rdrand {0}",
            "setc {1}",
            out(reg) value,
            out(reg_byte) success,
            options(nostack),
        );
    }
    
    if success != 0 {
        Some(value)
    } else {
        None
    }
}

/// Fill a buffer with random bytes using RDRAND
///
/// Returns true if successful, false if RDRAND is not supported or failed.
pub fn fill_random_bytes(buf: &mut [u8]) -> bool {
    if !is_supported() {
        return false;
    }
    
    let mut offset = 0;
    while offset < buf.len() {
        let random = match rdrand_u64() {
            Some(r) => r,
            None => return false,
        };
        
        let bytes = random.to_le_bytes();
        let remaining = buf.len() - offset;
        let to_copy = core::cmp::min(remaining, 8);
        buf[offset..offset + to_copy].copy_from_slice(&bytes[..to_copy]);
        offset += to_copy;
    }
    
    true
}
