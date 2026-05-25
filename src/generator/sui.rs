use crate::types::{Chain, KeyPair};
use crate::utils::crypto::{blake2b_256, ed25519_keypair};

pub fn generate() -> KeyPair {
    let (sk, pk) = ed25519_keypair();
    // Sui address: blake2b-256(flag=0x00 || pubkey), hex-prefixed.
    let mut buf = [0u8; 33];
    buf[0] = 0x00; // ed25519 flag
    buf[1..].copy_from_slice(&pk);
    let addr = blake2b_256(&buf);
    KeyPair {
        chain: Chain::Sui,
        private_key: format!("0x{}", hex::encode(sk.to_bytes())),
        public_key: format!("0x{}", hex::encode(pk)),
        address: format!("0x{}", hex::encode(addr)),
    }
}
