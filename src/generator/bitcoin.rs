use bech32::Hrp;
use sha2::{Digest, Sha256};

use crate::types::{Chain, KeyPair};
use crate::utils::crypto::secp256k1_keypair;

pub fn generate() -> KeyPair {
    let (sk, pk) = secp256k1_keypair();
    let compressed = pk.serialize(); // 33 bytes

    // P2WPKH (native segwit, bc1q...) — the modern default.
    let sha = Sha256::digest(compressed);
    let hash160: [u8; 20] = ripemd::Ripemd160::digest(sha).into();

    let hrp = Hrp::parse("bc").unwrap();
    let address = bech32::segwit::encode_v0(hrp, &hash160).expect("segwit encode");

    // WIF (mainnet, compressed): 0x80 || privkey || 0x01 || checksum
    let mut wif = [0u8; 34];
    wif[0] = 0x80;
    wif[1..33].copy_from_slice(&sk.secret_bytes());
    wif[33] = 0x01;
    let csum = Sha256::digest(Sha256::digest(wif));
    let mut wif_full = [0u8; 38];
    wif_full[..34].copy_from_slice(&wif);
    wif_full[34..].copy_from_slice(&csum[..4]);

    KeyPair {
        chain: Chain::Bitcoin,
        private_key: bs58::encode(wif_full).into_string(),
        public_key: hex::encode(compressed),
        address,
    }
}
