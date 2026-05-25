//! Balance checkers for EVM (JSON-RPC eth_getBalance) and BTC (Electrs UTXO REST).

/// Check a single ETH address via `eth_getBalance`.
/// Returns `Some(eth_balance)` if the call succeeds and the balance is > 0,
/// `None` on any RPC error or zero balance.  Caller applies the threshold.
pub async fn check_evm_balance(
    client: &reqwest::Client,
    rpc_url: &str,
    address: &str,
) -> Option<f64> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id":      1,
        "method":  "eth_getBalance",
        "params":  [address, "latest"]
    });

    let json: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    let hex_str = json["result"].as_str()?;
    let wei = u128::from_str_radix(hex_str.trim_start_matches("0x"), 16).ok()?;
    if wei == 0 { return None; }
    Some(wei as f64 / 1e18)
}

/// Check a Bitcoin address via Electrs-compatible REST API.
/// Calls `GET {api_url}/address/{address}/utxo` and sums the `value` fields
/// (satoshis as u64). Returns `Some(btc)` if the sum is > 0, `None` otherwise.
pub async fn check_btc_balance(
    client: &reqwest::Client,
    api_url: &str,
    address: &str,
) -> Option<f64> {
    let url = format!("{}/address/{}/utxo", api_url.trim_end_matches('/'), address);

    let utxos: serde_json::Value = client
        .get(&url)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    let arr = utxos.as_array()?;
    let total_sats: u64 = arr
        .iter()
        .filter_map(|u| u["value"].as_u64())
        .sum();

    if total_sats == 0 { return None; }
    Some(total_sats as f64 / 1e8)
}
