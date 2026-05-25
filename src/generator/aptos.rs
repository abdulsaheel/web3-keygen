use sha3::Digest;

use crate::types::{Chain, KeyPair};
use crate::utils::crypto::ed25519_keypair;

pub fn generate() -> KeyPair {
    let (sk, pk) = ed25519_keypair();
    // Aptos auth key = sha3-256(pubkey || 0x00); address == auth key initially.
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(pk);
    hasher.update([0x00u8]);
    let auth: [u8; 32] = hasher.finalize().into();
    KeyPair {
        chain: Chain::Aptos,
        private_key: format!("0x{}", hex::encode(sk.to_bytes())),
        public_key: format!("0x{}", hex::encode(pk)),
        address: format!("0x{}", hex::encode(auth)),
    }
}
