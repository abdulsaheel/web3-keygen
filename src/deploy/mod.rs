//! Testnet deployment stub.
//!
//! When a hit is found in dashboard mode and `--deploy-testnet` is set, this
//! module is the intended home for the broadcast logic. Currently a stub.

#[allow(dead_code)]
/// Broadcast a transaction from `private_key` on Sepolia using `rpc_url`.
///
/// TODO: implement `eth_sendRawTransaction` here using the private key and the
/// provided RPC endpoint. The transaction payload (to, value, gas, etc.) should
/// be parameterised by the caller.
pub async fn deploy_to_sepolia(_private_key: &str, _rpc_url: &str) {
    todo!("Testnet deployment: implement eth_sendRawTransaction for Sepolia here")
}
