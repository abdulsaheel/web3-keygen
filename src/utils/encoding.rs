//! Encoding helpers: EIP-55 checksum, Starknet felt utilities.

use sha3::{Digest, Keccak256};

/// Produce an EIP-55 mixed-case checksum Ethereum address string ("0x…").
pub fn eip55_checksum(addr: &[u8; 20]) -> String {
    let hex_addr = hex::encode(addr);
    let hash = Keccak256::digest(hex_addr.as_bytes());
    let mut out = String::with_capacity(42);
    out.push_str("0x");
    for (i, c) in hex_addr.chars().enumerate() {
        if c.is_ascii_digit() {
            out.push(c);
        } else {
            let nibble = (hash[i / 2] >> (4 * (1 - (i % 2)))) & 0xf;
            if nibble >= 8 {
                out.push(c.to_ascii_uppercase());
            } else {
                out.push(c);
            }
        }
    }
    out
}

// Stark curve order (EC_ORDER), big-endian.
const STARK_ORDER_BE: [u8; 32] = [
    0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xb7, 0x81, 0x12, 0x6d, 0xca, 0xe7, 0xb2, 0x32, 0x1e, 0x66, 0xa2, 0x41, 0xad, 0xc6, 0x4d, 0x2f,
];

/// Return `Some(felt)` if `bytes` is a valid, non-zero stark field element
/// (i.e. strictly less than the curve order and not zero). Returns `None` for
/// invalid scalars so the caller can reject-sample.
pub fn felt_if_in_range(bytes: &[u8; 32]) -> Option<starknet_crypto::Felt> {
    // Lexicographic BE compare: must be < STARK_ORDER_BE and != 0.
    let mut all_zero = true;
    let mut lt = false;
    for i in 0..32 {
        if bytes[i] != 0 {
            all_zero = false;
        }
        if !lt {
            if bytes[i] < STARK_ORDER_BE[i] {
                lt = true;
            } else if bytes[i] > STARK_ORDER_BE[i] {
                return None;
            }
        }
    }
    if all_zero || !lt {
        return None;
    }
    Some(starknet_crypto::Felt::from_bytes_be(bytes))
}

/// Wrapper for formatting a `starknet_crypto::Felt` as lowercase hex via
/// `std::fmt::LowerHex`.
pub struct FeltHex<'a>(pub &'a starknet_crypto::Felt);

impl<'a> std::fmt::LowerHex for FeltHex<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes = self.0.to_bytes_be();
        for b in &bytes {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}
