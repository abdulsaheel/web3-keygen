//! Low-level cryptographic primitives.
//!
//! All entropy is sourced exclusively from `getrandom::getrandom`, which delegates
//! to the OS CSPRNG (`getrandom(2)` on Linux, `getentropy(2)` on macOS/BSD,
//! `BCryptGenRandom` on Windows). No userspace RNG seeding, no PRNG expansion.

/// Pull `N` bytes straight from the OS CSPRNG. Panics on failure (the OS
/// being unable to provide entropy is unrecoverable; refusing to fabricate
/// fallback randomness is the whole point).
#[inline(always)]
pub fn os_random<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::getrandom(&mut buf).expect("OS entropy source unavailable");
    buf
}

/// Reject-sample a valid secp256k1 scalar straight from OS entropy.
pub fn secp256k1_keypair() -> (secp256k1::SecretKey, secp256k1::PublicKey) {
    let secp = secp256k1::SECP256K1;
    loop {
        let bytes = os_random::<32>();
        if let Ok(sk) = secp256k1::SecretKey::from_slice(&bytes) {
            let pk = secp256k1::PublicKey::from_secret_key(secp, &sk);
            return (sk, pk);
        }
    }
}

/// Generate an ed25519 keypair from OS entropy. Returns (signing key, pubkey bytes).
pub fn ed25519_keypair() -> (ed25519_dalek::SigningKey, [u8; 32]) {
    let seed = os_random::<32>();
    let sk = ed25519_dalek::SigningKey::from_bytes(&seed);
    let pk = sk.verifying_key().to_bytes();
    (sk, pk)
}

/// Blake2b-256 hash of `input`.
pub fn blake2b_256(input: &[u8]) -> [u8; 32] {
    use blake2::digest::consts::U32;
    use blake2::{Blake2b, Digest};
    let mut h = Blake2b::<U32>::new();
    h.update(input);
    h.finalize().into()
}
