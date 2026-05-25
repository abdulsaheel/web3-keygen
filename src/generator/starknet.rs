use crate::types::{Chain, KeyPair};
use crate::utils::crypto::os_random;
use crate::utils::encoding::{felt_if_in_range, FeltHex};

pub fn generate() -> KeyPair {
    // Reject-sample 32-byte OS entropy strictly below the stark curve order
    // (bias-free). Account contract address derivation depends on the wallet
    // implementation, so we expose the raw public key as the identity.
    loop {
        let bytes = os_random::<32>();
        if let Some(sk) = felt_if_in_range(&bytes) {
            let pk = starknet_crypto::get_public_key(&sk);
            return KeyPair {
                chain: Chain::Starknet,
                private_key: format!("0x{:064x}", FeltHex(&sk)),
                public_key: format!("0x{:064x}", FeltHex(&pk)),
                address: format!("0x{:064x}", FeltHex(&pk)),
            };
        }
    }
}
