# web3-keygen

A high-performance keypair generator and live blockchain scanner written in Rust. Generates cryptographically secure keypairs for 8 blockchain networks and continuously monitors those addresses for non-zero balances in real-time.

All entropy is sourced exclusively from the OS CSPRNG (`getrandom(2)` on Linux, `getentropy(2)` on macOS, `BCryptGenRandom` on Windows) — no userspace RNG, no fallback.

---

## Features

- **8 chains supported** — Ethereum, Bitcoin, Solana, Tron, Cosmos, Aptos, Sui, Starknet
- **OS entropy only** — panics rather than falling back to a weak PRNG
- **Parallel keygen** via Rayon — saturates all CPU cores
- **Async balance checking** via Tokio — concurrent JSON-RPC and REST queries
- **Live web dashboard** — password-protected, auto-refreshing stats UI
- **Configurable thresholds** — per-chain minimum balance before a "hit" is recorded
- **Max-optimized release binary** — LTO, single codegen unit, stripped symbols

---

## Supported Chains

| Chain | Signing Algorithm | Address Format | Scanner |
|-------|------------------|----------------|:-------:|
| Ethereum | secp256k1 | `0x...` (EIP-55 checksum) | ✅ |
| Bitcoin | secp256k1 | `bc1q...` (P2WPKH native segwit) | ✅ |
| Solana | Ed25519 | base58 public key | — |
| Tron | secp256k1 | base58 (`T...`) | — |
| Cosmos | secp256k1 | bech32 (`cosmos1...`) | — |
| Aptos | Ed25519 | `0x...` (SHA3-256 auth key) | — |
| Sui | Ed25519 | `0x...` (blake2b-256) | — |
| Starknet | Stark curve | `0x...` (bare public key) | — |

Scanner mode currently supports Ethereum (EVM-compatible) and Bitcoin. All 8 chains are available in one-shot generation mode.

---

## Installation

**Prerequisites:** Rust 1.75+ (install via [rustup](https://rustup.rs))

```bash
git clone https://github.com/yourorg/web3-keygen
cd web3-keygen
cargo build --release
```

The binary is at `target/release/web3-keygen`.

---

## Usage

### One-shot generation

Generate keypairs and print them, then exit.

```bash
# Generate 1 keypair for each chain (default)
./web3-keygen --generate

# Generate 10 Ethereum and Bitcoin keypairs as JSON
./web3-keygen --generate --count 10 --chains ethereum,bitcoin --format json

# Generate for all supported chains
./web3-keygen --generate --chains ethereum,bitcoin,solana,tron,cosmos,aptos,sui,starknet
```

**Flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--generate` | — | Run in one-shot mode |
| `--chains <list>` | all | Comma-separated chain names |
| `--count <n>` | `1` | Number of keypairs per chain |
| `--format <fmt>` | `text` | Output format: `text` or `json` |

**Example JSON output:**
```json
[
  {
    "chain": "ethereum",
    "private_key": "0xabc123...",
    "address": "0xDe0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"
  }
]
```

### Scanner mode

Run a persistent watcher that generates keypairs and checks balances continuously.

```bash
# Requires config.toml in the current directory
./web3-keygen
```

The scanner:
1. Generates Ethereum and Bitcoin keypairs in parallel across all CPU cores
2. Fans out balance checks to all configured RPC endpoints concurrently
3. Logs any address with balance ≥ threshold to `hits.log` and the in-memory dashboard
4. Serves a live web UI at `http://localhost:8080`

---

## Configuration

Copy and edit `config.toml` before running scanner mode. The file is excluded from `.gitignore` by default — do not commit it if it contains private RPC URLs or secrets.

```toml
[scanner]
# Worker threads for keypair generation. 0 = auto (1/8 of logical CPUs)
threads = 0
# Keypairs generated per batch
batch_size = 8
# Max concurrent RPC calls in flight
rpc_concurrency = 5

[server]
port = 8080
# SHA-256 hash of your dashboard password
# Generate with: echo -n "yourpassword" | sha256sum
password_sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
# Secret key for HMAC session tokens (use a long random string)
session_secret = "change-me-to-a-random-secret"

[output]
hits_log = "hits.log"

# EVM chain example (add as many as you like)
[[chains]]
name = "ethereum"
type = "evm"
rpc_url = "https://mainnet.infura.io/v3/YOUR_KEY"
balance_threshold = 0.0001   # ETH

[[chains]]
name = "arbitrum"
type = "evm"
rpc_url = "https://arb1.arbitrum.io/rpc"
balance_threshold = 0.0001

# Bitcoin example (uses Electrs REST API)
[[chains]]
name = "bitcoin"
type = "btc"
api_url = "https://blockstream.info/api"
balance_threshold = 0.00001  # BTC
```

---

## Web Dashboard

Open `http://localhost:<port>` in your browser. You will be prompted for the password configured in `config.toml`.

The dashboard displays:

- **Keys/sec** — keypair generation throughput
- **RPC calls/sec** — balance check throughput
- **Total generated / checked** — lifetime counters
- **Uptime**
- **Hits table** — every address found with a non-zero balance (chain, address, balance, private key)

Stats refresh automatically every 2 seconds. Sessions use HMAC-SHA256 signed cookies with `HttpOnly` and `SameSite=Strict` flags.

---

## Architecture

```
OS CSPRNG
    │
    ▼
[Rayon] Keygen threads (CPU-bound, parallel)
    │  KeyPair batches
    ▼
[MPSC Channel] (bounded = batch_size, backpressure)
    │
    ▼
[Tokio] Balance checker (async, I/O-bound)
    │  semaphore-gated concurrency
    ▼
[RPC / REST] eth_getBalance · Electrs UTXO
    │
    ▼
Arc<Mutex<AppState>>  ←→  [Axum] /login /dashboard /api/stats
```

The Rayon producer and Tokio consumer run on separate OS threads. The bounded channel provides natural backpressure: if RPC endpoints are slow, keypair generation stalls rather than accumulating unbounded memory.

---

## Security Notes

- **This tool is for research and security education.** The probability of generating a keypair that collides with an existing funded address is astronomically small.
- Private keys are printed to stdout / stored in `hits.log` in plaintext. Treat the log file accordingly.
- Never commit `config.toml` — it contains your session secret and RPC API keys.
- The dashboard password is compared as a SHA-256 hash. Use a strong password.

---

## Performance

Compiled with `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, stripped symbols, and `panic = "abort"`. On modern multi-core hardware the generator can sustain millions of keypairs per second before RPC rate limits become the bottleneck.

To profile keygen throughput without RPC overhead, use one-shot mode:
```bash
time ./web3-keygen --generate --count 1000000 --chains ethereum --format json > /dev/null
```

---

## License

MIT
