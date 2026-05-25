use sha2::{Digest, Sha256};
use sha3::Keccak256;

use crate::types::{Chain, KeyPair};
use crate::utils::crypto::secp256k1_keypair;

pub fn generate() -> KeyPair {
    let (sk, pk) = secp256k1_keypair();
    let uncompressed = pk.serialize_uncompressed();
    let hash = Keccak256::digest(&uncompressed[1..]);
    // Tron address: 0x41 || keccak256(pub)[12..]
    let mut raw = [0u8; 21];
    raw[0] = 0x41;
    raw[1..].copy_from_slice(&hash[12..]);
    let checksum = {
        let a = Sha256::digest(raw);
        let b = Sha256::digest(a);
        [b[0], b[1], b[2], b[3]]
    };
    let mut full = [0u8; 25];
    full[..21].copy_from_slice(&raw);
    full[21..].copy_from_slice(&checksum);
    KeyPair {
        chain: Chain::Tron,
        private_key: hex::encode(sk.secret_bytes()),
        public_key: hex::encode(uncompressed),
        address: bs58::encode(full).into_string(),
    }
}
