//! # Bitcoin-style Implementation
//!
//! This module implements Bitcoin-specific features that differ from the core implementation.
//!
//! ## Key Differences from Core
//!
//! | Aspect | Core (blockchain-lab-core) | Bitcoin |
//! |--------|---------------------------|---------|
//! | Hash Function | SHA256 | Double SHA256 |
//! | Signature Curve | P-256 (NIST) | secp256k1 |
//! | Address Format | Base58Check | Base58Check + Bech32 (SegWit) |
//! | Block Time | Configurable | ~10 minutes |
//! | Difficulty Adjustment | None | Every 2016 blocks |
//! | Script | None | Bitcoin Script (OP_CODES) |
//! | Transaction | Simple UTXO | UTXO with Script |
//!
//! ## Modules to Implement
//!
//! - [ ] `crypto/secp256k1.rs` - secp256k1 curve implementation
//! - [ ] `script/` - Bitcoin Script interpreter
//! - [ ] `consensus/difficulty.rs` - Difficulty adjustment algorithm
//! - [ ] `address/bech32.rs` - SegWit address encoding

pub mod consensus;
pub mod crypto;

/// Bitcoin-specific constants
pub mod constants {
    /// Target block time in seconds (10 minutes)
    pub const TARGET_BLOCK_TIME_SECS: u64 = 600;

    /// Difficulty adjustment interval (every 2016 blocks)
    pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 2016;

    /// Initial block reward (50 BTC in satoshis)
    pub const INITIAL_BLOCK_REWARD: u64 = 50_0000_0000;

    /// Halving interval
    pub const HALVING_INTERVAL: u64 = 210_000;
}
