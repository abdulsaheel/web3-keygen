use sha3::{Digest, Keccak256};

use crate::types::{Chain, KeyPair};
use crate::utils::crypto::secp256k1_keypair;
use crate::utils::encoding::eip55_checksum;

pub fn generate() -> KeyPair {
    let (sk, pk) = secp256k1_keypair();
    // Uncompressed pub: 65 bytes (0x04 || X || Y); drop the prefix for keccak.
    let uncompressed = pk.serialize_uncompressed();
    let hash = Keccak256::digest(&uncompressed[1..]);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    KeyPair {
        chain: Chain::Ethereum,
        private_key: format!("0x{}", hex::encode(sk.secret_bytes())),
        public_key: format!("0x{}", hex::encode(&uncompressed[1..])),
        address: eip55_checksum(&addr),
    }
}
