use bech32::{Bech32, Hrp};
use sha2::{Digest, Sha256};

use crate::types::{Chain, KeyPair};
use crate::utils::crypto::secp256k1_keypair;

pub fn generate() -> KeyPair {
    let (sk, pk) = secp256k1_keypair();
    let compressed = pk.serialize();
    let sha = Sha256::digest(compressed);
    let hash160: [u8; 20] = ripemd::Ripemd160::digest(sha).into();

    let hrp = Hrp::parse("cosmos").unwrap();
    let address = bech32::encode::<Bech32>(hrp, &hash160).expect("bech32 encode");

    KeyPair {
        chain: Chain::Cosmos,
        private_key: hex::encode(sk.secret_bytes()),
        public_key: hex::encode(compressed),
        address,
    }
}
