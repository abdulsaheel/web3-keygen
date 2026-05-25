//! Configuration loading from `config.toml`.
//!
//! Panics on parse errors so misconfiguration is loud at startup.

use serde::Deserialize;

// ── top-level ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub scanner: ScannerConfig,
    pub server: ServerConfig,
    pub output: OutputConfig,
    pub chains: Vec<ChainConfig>,
}

// ── chain list ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ChainConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub chain_type: ChainType,
    /// JSON-RPC endpoint (EVM chains).
    pub rpc_url: Option<String>,
    /// Electrs-compatible REST API base (BTC chains).
    pub api_url: Option<String>,
    /// Minimum native-token balance that counts as a hit (in BTC / ETH / etc.).
    pub balance_threshold: f64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChainType {
    Evm,
    Btc,
}

// ── scanner ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ScannerConfig {
    /// Rayon keygen threads. 0 = rayon default (num logical CPUs).
    #[serde(default)]
    pub threads: usize,

    /// Keypairs generated per rayon batch.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Max concurrent balance-check calls in flight.
    #[serde(default = "default_rpc_concurrency")]
    pub rpc_concurrency: usize,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            threads: 0,
            batch_size: default_batch_size(),
            rpc_concurrency: default_rpc_concurrency(),
        }
    }
}

// ── server ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    /// SHA-256 hex digest of the dashboard password.
    #[serde(default = "default_password_sha256")]
    pub password_sha256: String,
    /// Secret used to compute HMAC session tokens.
    #[serde(default = "default_session_secret")]
    pub session_secret: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            password_sha256: default_password_sha256(),
            session_secret: default_session_secret(),
        }
    }
}

// ── output ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_hits_log")]
    pub hits_log: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self { hits_log: default_hits_log() }
    }
}

// ── loader ────────────────────────────────────────────────────────────────────

/// Read `config.toml` from the current working directory. Panics on any error.
pub fn load() -> Config {
    let raw = std::fs::read_to_string("config.toml")
        .expect("config.toml not found — place it in the working directory");
    toml::from_str(&raw).expect("config.toml parse error")
}

// ── defaults ──────────────────────────────────────────────────────────────────

fn default_batch_size() -> usize { 256 }
fn default_rpc_concurrency() -> usize { 50 }
fn default_hits_log() -> String { "hits.log".to_owned() }
fn default_port() -> u16 { 8080 }
fn default_password_sha256() -> String {
    "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8".to_owned()
}
fn default_session_secret() -> String { "change-me-random-secret".to_owned() }
