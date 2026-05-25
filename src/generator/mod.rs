pub mod aptos;
pub mod bitcoin;
pub mod cosmos;
pub mod ethereum;
pub mod solana;
pub mod starknet;
pub mod sui;
pub mod tron;

use crate::types::{Chain, KeyPair};

/// Generate one keypair for the given chain. This is the single dispatch point
/// used by both normal mode and dashboard mode.
pub fn generate(chain: Chain) -> KeyPair {
    match chain {
        Chain::Ethereum => ethereum::generate(),
        Chain::Bitcoin => bitcoin::generate(),
        Chain::Solana => solana::generate(),
        Chain::Tron => tron::generate(),
        Chain::Cosmos => cosmos::generate(),
        Chain::Aptos => aptos::generate(),
        Chain::Sui => sui::generate(),
        Chain::Starknet => starknet::generate(),
    }
}
