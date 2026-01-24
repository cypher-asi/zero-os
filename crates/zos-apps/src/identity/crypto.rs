//! Cryptographic helpers for identity service
//!
//! This module provides simplified crypto operations for WASM environments.
//! In production, these would be replaced with proper cryptographic libraries.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use zos_identity::ipc::NeuralShard;
use zos_identity::KeyError;

use crate::syscall;

/// Generate random bytes using the kernel's entropy sources.
///
/// Uses wallclock and PID for entropy source in WASM.
/// In production, this would use a proper getrandom syscall.
pub fn generate_random_bytes(len: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; len];
    let time = syscall::get_wallclock();
    let pid = syscall::get_pid();

    // Simple PRNG seeded with time and PID
    let mut state = time ^ ((pid as u64) << 32);
    for byte in bytes.iter_mut() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *byte = (state >> 56) as u8;
    }
    bytes
}

/// Convert bytes to hex string.
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Format a u128 as a UUID string (8-4-4-4-12 format with dashes).
pub fn format_uuid(value: u128) -> String {
    let hex = format!("{:032x}", value);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Convert hex string to bytes.
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, &'static str> {
    if hex.len() % 2 != 0 {
        return Err("Invalid hex length");
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| "Invalid hex"))
        .collect()
}

/// Decode base64 string to bytes (standard alphabet, with optional padding).
pub fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    const DECODE_TABLE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, // 0-15
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, // 16-31
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1, -1, 63, // 32-47 (+, /)
        52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, // 48-63 (0-9)
        -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, // 64-79 (A-O)
        15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1, -1, -1, // 80-95 (P-Z)
        -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, // 96-111 (a-o)
        41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1, // 112-127 (p-z)
    ];

    // Remove padding and whitespace
    let input = input.trim_end_matches('=');
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let mut output = Vec::with_capacity((input.len() * 3) / 4);
    let mut buffer: u32 = 0;
    let mut bits_collected: u8 = 0;

    for c in input.chars() {
        let c = c as usize;
        if c >= 128 {
            return Err("Invalid base64 character");
        }
        let value = DECODE_TABLE[c];
        if value < 0 {
            return Err("Invalid base64 character");
        }
        buffer = (buffer << 6) | (value as u32);
        bits_collected += 6;
        if bits_collected >= 8 {
            bits_collected -= 8;
            output.push((buffer >> bits_collected) as u8);
            buffer &= (1 << bits_collected) - 1;
        }
    }

    Ok(output)
}

/// Simple Shamir secret sharing (3-of-5) - mock implementation.
///
/// Production would use a proper Shamir library with polynomial interpolation.
pub fn shamir_split(secret: &[u8], threshold: usize, shares: usize) -> Vec<NeuralShard> {
    let _ = threshold; // Would be used in real implementation
    let mut shards = Vec::with_capacity(shares);

    for i in 1..=shares {
        // Generate a shard by XORing secret with deterministic "random" data
        let mut shard_bytes = Vec::with_capacity(secret.len() + 1);
        shard_bytes.push(i as u8); // Shard index

        // Generate deterministic padding based on index
        let mut state = (i as u64).wrapping_mul(0x9E3779B97F4A7C15);
        for &byte in secret.iter() {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(i as u64);
            shard_bytes.push(byte ^ (state >> 56) as u8);
        }

        shards.push(NeuralShard {
            index: i as u8,
            hex: bytes_to_hex(&shard_bytes),
        });
    }

    shards
}

/// Reconstruct secret from shards (mock implementation).
///
/// Production would use polynomial interpolation for proper Shamir reconstruction.
pub fn shamir_reconstruct(shards: &[NeuralShard]) -> Result<Vec<u8>, KeyError> {
    if shards.len() < 3 {
        return Err(KeyError::InsufficientShards);
    }

    // Use first shard to reconstruct (simplified - real Shamir uses polynomial interpolation)
    let shard = &shards[0];
    let shard_bytes =
        hex_to_bytes(&shard.hex).map_err(|e| KeyError::InvalidShard(String::from(e)))?;

    if shard_bytes.is_empty() {
        return Err(KeyError::InvalidShard(String::from("Empty shard")));
    }

    let index = shard_bytes[0] as u64;
    let mut secret = Vec::with_capacity(shard_bytes.len() - 1);

    // Reverse the XOR operation
    let mut state = index.wrapping_mul(0x9E3779B97F4A7C15);
    for &byte in &shard_bytes[1..] {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(index);
        secret.push(byte ^ (state >> 56) as u8);
    }

    Ok(secret)
}

/// Derive a public key from entropy with a salt (mock Ed25519/X25519).
///
/// Production would use proper key derivation functions.
pub fn derive_public_key(entropy: &[u8], salt: &str) -> [u8; 32] {
    let mut combined = Vec::with_capacity(entropy.len() + salt.len());
    combined.extend_from_slice(entropy);
    combined.extend_from_slice(salt.as_bytes());

    // XOR fold to 32 bytes (mock derivation)
    let mut public_key = [0u8; 32];
    for (i, &byte) in combined.iter().enumerate() {
        public_key[i % 32] ^= byte;
    }

    // Add more mixing
    for i in 0..32 {
        let state = (public_key[i] as u64)
            .wrapping_mul(0x9E3779B97F4A7C15)
            .wrapping_add(i as u64);
        public_key[i] = (state >> 56) as u8;
    }

    public_key
}

/// Sign a challenge with machine key (mock Ed25519 signature).
///
/// In production this would be proper Ed25519 signing.
pub fn sign_challenge(challenge: &[u8], signing_key: &[u8; 32]) -> [u8; 64] {
    let mut signature = [0u8; 64];
    for (i, &byte) in challenge.iter().enumerate() {
        let key_byte = signing_key[i % 32];
        signature[i % 64] ^= byte ^ key_byte;
    }
    // Add more mixing
    let mut state = 0x9E3779B97F4A7C15u64;
    for i in 0..64 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(signature[i] as u64);
        signature[i] = (state >> 56) as u8;
    }
    signature
}
