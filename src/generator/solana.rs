use crate::types::{Chain, KeyPair};
use crate::utils::crypto::ed25519_keypair;

pub fn generate() -> KeyPair {
    let (sk, pk) = ed25519_keypair();
    // Solana "keypair" file format: 64 bytes = secret seed (32) || pubkey (32),
    // base58 of the public key is the address.
    let mut full = [0u8; 64];
    full[..32].copy_from_slice(&sk.to_bytes());
    full[32..].copy_from_slice(&pk);
    KeyPair {
        chain: Chain::Solana,
        private_key: bs58::encode(full).into_string(),
        public_key: bs58::encode(pk).into_string(),
        address: bs58::encode(pk).into_string(),
    }
}
