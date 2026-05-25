//! web3-keygen: high-throughput OS-random keypair generator + live ETH scanner.
//!
//! Run modes:
//!   (default / no flags) — start the watcher + web dashboard on the configured port.
//!   --generate            — print keypairs for the specified chains and exit.
//!
//! All entropy is sourced exclusively from `getrandom::getrandom`, which delegates
//! to the OS CSPRNG (`getrandom(2)` on Linux, `getentropy(2)` on macOS/BSD,
//! `BCryptGenRandom` on Windows). No userspace RNG seeding, no PRNG expansion.

mod balances;
mod config;
mod dashboard;
mod deploy;
mod generator;
mod server;
mod types;
mod utils;

use std::sync::{Arc, Mutex};

use clap::{Parser, ValueEnum};
use rayon::prelude::*;
use types::{Chain, KeyPair, ALL_CHAINS};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(version, about = "OS-random key generator + live ETH scanner")]
struct Cli {
    /// Generate keypairs for the given chains and exit (skips the web server).
    #[arg(long)]
    generate: bool,

    /// How many keys to generate per chain (--generate mode only).
    #[arg(short, long, default_value_t = 1)]
    count: usize,

    /// Chains to generate for (default: all).
    #[arg(short = 'k', long, value_enum, num_args = 1.., value_delimiter = ',')]
    chains: Option<Vec<Chain>>,

    /// Output format (--generate mode only).
    #[arg(short, long, value_enum, default_value_t = Format::Json)]
    format: Format,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Format {
    Json,
    Text,
}

// ── entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let cfg = config::load();

    if cli.generate {
        // One-shot generation mode: generate, print, exit.
        let chains: Vec<Chain> = cli
            .chains
            .clone()
            .unwrap_or_else(|| ALL_CHAINS.to_vec());

        let work: Vec<Chain> = chains
            .iter()
            .flat_map(|c| std::iter::repeat(*c).take(cli.count))
            .collect();

        let results: Vec<KeyPair> = work
            .par_iter()
            .map(|c| generator::generate(*c))
            .collect();

        match cli.format {
            Format::Json => {
                println!("{}", serde_json::to_string_pretty(&results).unwrap());
            }
            Format::Text => {
                for kp in &results {
                    println!("[{:?}]", kp.chain);
                    println!("  private: {}", kp.private_key);
                    println!("  public : {}", kp.public_key);
                    println!("  address: {}", kp.address);
                }
            }
        }
        return;
    }

    // Scanner + web dashboard mode.
    let state = Arc::new(Mutex::new(dashboard::AppState::new()));

    // Spawn rayon keygen + tokio checker in background threads.
    dashboard::run_watcher(cfg.clone(), Arc::clone(&state));

    // Block on the axum web server.
    server::serve(cfg, state).await;
}
