//! Watcher: continuous keygen + balance-checking with shared AppState.
//!
//! Architecture:
//!   rayon threadpool  ──(unbounded_channel)──►  tokio balance-checker
//!                                                    │
//!                                             Arc<Mutex<AppState>>
//!                                                    │
//!                                            axum web server reads

use std::{
    fs::OpenOptions,
    io::Write as IoWrite,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use futures::future::join_all;
use rayon::prelude::*;
use tokio::sync::mpsc as async_mpsc;

use crate::{
    balances::checker::{check_btc_balance, check_evm_balance},
    config::{ChainConfig, ChainType, Config},
    generator,
    types::Chain,
};

// ── public types ──────────────────────────────────────────────────────────────

/// A keypair that passed the balance threshold on a specific chain.
#[derive(Clone, Debug, serde::Serialize)]
pub struct HitRecord {
    /// Chain name from config (e.g. "ethereum", "bitcoin", "base").
    pub chain: String,
    pub address: String,
    pub public_key: String,
    pub private_key: String,
    /// Balance in native token units (ETH, BTC, …).
    pub balance: f64,
    /// Unix timestamp (seconds) when this hit was found.
    pub found_at_secs: u64,
}

/// Shared state visible to the watcher and the HTTP server.
pub struct AppState {
    pub generated: u64,
    pub checked: u64,
    pub hits: Vec<HitRecord>,
    pub start: Instant,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            generated: 0,
            checked: 0,
            hits: Vec::new(),
            start: Instant::now(),
        }
    }
}

// ── channel message ───────────────────────────────────────────────────────────

/// What the rayon producer sends to the tokio checker.
enum KeygenMsg {
    /// An Ethereum-compatible keypair to check across all EVM chains.
    Evm { address: String, public_key: String, private_key: String },
    /// A Bitcoin keypair to check against the BTC API.
    Btc { address: String, public_key: String, private_key: String },
}

// ── watcher entry point ───────────────────────────────────────────────────────

/// Spawn the rayon keygen loop + tokio balance-checker.
/// Neither thread ever stops; they run until the process exits.
pub fn run_watcher(config: Config, state: Arc<Mutex<AppState>>) {
    // Bounded at 2 × rpc_concurrency — backpressures the rayon producer so the
    // channel never grows unbounded and eats all RAM.
    // Channel capped at batch_size — producer blocks until checker drains it.
    let cap = config.scanner.batch_size.max(8);
    let (tx, rx) = async_mpsc::channel::<KeygenMsg>(cap);

    // ── rayon producer ────────────────────────────────────────────────────────
    let state_gen = Arc::clone(&state);
    let cfg = config.clone();
    let tx_clone = tx;
    std::thread::spawn(move || {
        // Use at most 1/8 of logical CPUs, minimum 1.
        let max_threads = {
            let cpus = num_cpus::get();
            ((cpus / 8).max(1)).min(cpus)
        };
        let threads = if cfg.scanner.threads > 0 {
            cfg.scanner.threads.min(max_threads)
        } else {
            max_threads
        };
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .ok();

        let has_evm = cfg.chains.iter().any(|c| c.chain_type == ChainType::Evm);
        let has_btc = cfg.chains.iter().any(|c| c.chain_type == ChainType::Btc);

        loop {
            let batch_size = cfg.scanner.batch_size;

            // Generate EVM keypairs.
            let evm_batch: Vec<KeygenMsg> = if has_evm {
                (0..batch_size)
                    .into_par_iter()
                    .map(|_| {
                        let kp = generator::generate(Chain::Ethereum);
                        KeygenMsg::Evm {
                            address: kp.address,
                            public_key: kp.public_key,
                            private_key: kp.private_key,
                        }
                    })
                    .collect()
            } else {
                vec![]
            };

            // Generate BTC keypairs.
            let btc_batch: Vec<KeygenMsg> = if has_btc {
                (0..batch_size)
                    .into_par_iter()
                    .map(|_| {
                        let kp = generator::generate(Chain::Bitcoin);
                        KeygenMsg::Btc {
                            address: kp.address,
                            public_key: kp.public_key,
                            private_key: kp.private_key,
                        }
                    })
                    .collect()
            } else {
                vec![]
            };

            let count = (evm_batch.len() + btc_batch.len()) as u64;
            {
                let mut s = state_gen.lock().unwrap();
                s.generated += count;
            }

            for msg in evm_batch.into_iter().chain(btc_batch) {
                if tx_clone.blocking_send(msg).is_err() {
                    return; // receiver dropped — server shut down
                }
            }
        }
    });

    // ── tokio balance-checker ─────────────────────────────────────────────────
    let state_check = Arc::clone(&state);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime for checker");
        rt.block_on(checker_loop(config, state_check, rx));
    });
}

// ── async checker loop ────────────────────────────────────────────────────────

async fn checker_loop(
    config: Config,
    state: Arc<Mutex<AppState>>,
    mut rx: async_mpsc::Receiver<KeygenMsg>,
) {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("reqwest client");

    // One semaphore permit = one address being checked across all its chains.
    // Keeps total in-flight RPC calls = rpc_concurrency × chains_per_address.
    let concurrency = config.scanner.rpc_concurrency;
    let sem = Arc::new(tokio::sync::Semaphore::new(concurrency));
    // Tokio worker threads also capped at 1/8 CPUs.
    let _concurrency = concurrency;

    // Split chains by type once; clone the sub-lists for use inside tasks.
    let evm_chains: Vec<ChainConfig> = config
        .chains
        .iter()
        .filter(|c| c.chain_type == ChainType::Evm)
        .cloned()
        .collect();
    let btc_chains: Vec<ChainConfig> = config
        .chains
        .iter()
        .filter(|c| c.chain_type == ChainType::Btc)
        .cloned()
        .collect();

    let evm_chains = Arc::new(evm_chains);
    let btc_chains = Arc::new(btc_chains);
    let hits_log = config.output.hits_log.clone();

    loop {
        // Drain up to `concurrency` messages per tick.
        let mut batch = Vec::new();
        for _ in 0..concurrency {
            match rx.try_recv() {
                Ok(msg) => batch.push(msg),
                Err(_) => break,
            }
        }

        if batch.is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            continue;
        }

        let mut handles = Vec::with_capacity(batch.len());

        for msg in batch {
            let http = http.clone();
            let sem = sem.clone();
            let evm_chains = Arc::clone(&evm_chains);
            let btc_chains = Arc::clone(&btc_chains);
            let state = Arc::clone(&state);
            let hits_log = hits_log.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("sem closed");

                match msg {
                    KeygenMsg::Evm { address, public_key, private_key } => {
                        // Fan out to every EVM chain concurrently.
                        let checks: Vec<_> = evm_chains
                            .iter()
                            .map(|chain| {
                                let http = http.clone();
                                let addr = address.clone();
                                let rpc = chain.rpc_url.clone().unwrap_or_default();
                                let threshold = chain.balance_threshold;
                                let chain_name = chain.name.clone();
                                async move {
                                    let bal = check_evm_balance(&http, &rpc, &addr).await;
                                    (chain_name, threshold, bal)
                                }
                            })
                            .collect();

                        let results = join_all(checks).await;
                        let checked_count = results.len() as u64;

                        {
                            let mut s = state.lock().unwrap();
                            s.checked += checked_count;
                        }

                        let now_secs = now_unix();
                        for (chain_name, threshold, maybe_bal) in results {
                            if let Some(bal) = maybe_bal {
                                if bal >= threshold {
                                    let hit = HitRecord {
                                        chain: chain_name,
                                        address: address.clone(),
                                        public_key: public_key.clone(),
                                        private_key: private_key.clone(),
                                        balance: bal,
                                        found_at_secs: now_secs,
                                    };
                                    append_hit_log(&hits_log, &hit);
                                    let mut s = state.lock().unwrap();
                                    s.hits.push(hit);
                                }
                            }
                        }
                    }

                    KeygenMsg::Btc { address, public_key, private_key } => {
                        let checks: Vec<_> = btc_chains
                            .iter()
                            .map(|chain| {
                                let http = http.clone();
                                let addr = address.clone();
                                let api = chain.api_url.clone().unwrap_or_default();
                                let threshold = chain.balance_threshold;
                                let chain_name = chain.name.clone();
                                async move {
                                    let bal = check_btc_balance(&http, &api, &addr).await;
                                    (chain_name, threshold, bal)
                                }
                            })
                            .collect();

                        let results = join_all(checks).await;
                        let checked_count = results.len() as u64;

                        {
                            let mut s = state.lock().unwrap();
                            s.checked += checked_count;
                        }

                        let now_secs = now_unix();
                        for (chain_name, threshold, maybe_bal) in results {
                            if let Some(bal) = maybe_bal {
                                if bal >= threshold {
                                    let hit = HitRecord {
                                        chain: chain_name,
                                        address: address.clone(),
                                        public_key: public_key.clone(),
                                        private_key: private_key.clone(),
                                        balance: bal,
                                        found_at_secs: now_secs,
                                    };
                                    append_hit_log(&hits_log, &hit);
                                    let mut s = state.lock().unwrap();
                                    s.hits.push(hit);
                                }
                            }
                        }
                    }
                }
            }));
        }

        for h in handles {
            let _ = h.await;
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn append_hit_log(path: &str, hit: &HitRecord) {
    let line = format!(
        "{} | [{}] {} | {:.8} | pubkey: {} | privkey: {}\n",
        unix_to_hms(hit.found_at_secs),
        hit.chain,
        hit.address,
        hit.balance,
        hit.public_key,
        hit.private_key,
    );
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}

fn unix_to_hms(secs: u64) -> String {
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}Z", h, m, s)
}
