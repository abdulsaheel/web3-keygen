//! Shared types used across all modules.

use clap::ValueEnum;
use serde::Serialize;

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Ethereum,
    Bitcoin,
    Solana,
    Tron,
    Cosmos,
    Aptos,
    Sui,
    Starknet,
}

#[derive(Serialize, Clone, Debug)]
pub struct KeyPair {
    pub chain: Chain,
    pub private_key: String,
    pub public_key: String,
    pub address: String,
}

pub const ALL_CHAINS: &[Chain] = &[
    Chain::Ethereum,
    Chain::Bitcoin,
    Chain::Solana,
    Chain::Tron,
    Chain::Cosmos,
    Chain::Aptos,
    Chain::Sui,
    Chain::Starknet,
];
