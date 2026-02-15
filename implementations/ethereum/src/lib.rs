//! # Ethereum-style Implementation (Account Model + PoS)
//!
//! This module implements Ethereum-specific features.
//!
//! ## Key Differences from Core
//!
//! | Aspect | Core (blockchain-lab-core) | Ethereum |
//! |--------|---------------------------|----------|
//! | Data Model | UTXO | Account-based |
//! | Consensus | PoW | PoS (Casper FFG) |
//! | Hash Function | SHA256 | Keccak-256 |
//! | Address Format | Base58Check | Hex (0x prefix) + EIP-55 checksum |
//! | State | Implicit (UTXO set) | Explicit (Merkle Patricia Trie) |
//! | Smart Contracts | None | EVM bytecode |
//! | Transaction | Transfer only | Transfer + Contract calls |
//! | Gas | None | Gas for computation |
//!
//! ## Account Model vs UTXO
//!
//! ```text
//! UTXO (Bitcoin/Core):           Account (Ethereum):
//! ┌─────────────┐                ┌─────────────────┐
//! │ TX Output 1 │                │ Account A       │
//! │ 50 coins    │──┐             │ Balance: 100    │
//! └─────────────┘  │             │ Nonce: 5        │
//! ┌─────────────┐  ├─→ Spend     │ Code: 0x...     │
//! │ TX Output 2 │──┘             │ Storage: {...}  │
//! │ 30 coins    │                └─────────────────┘
//! └─────────────┘
//! ```
//!
//! ## Modules to Implement
//!
//! - [ ] `state/account.rs` - Account structure
//! - [ ] `state/trie.rs` - Merkle Patricia Trie
//! - [ ] `execution/evm.rs` - Simple EVM
//! - [ ] `consensus/pos.rs` - Proof of Stake (Casper)
//! - [ ] `crypto/keccak.rs` - Keccak-256 hashing

pub mod consensus;
pub mod state;
pub mod execution;

/// Ethereum-specific constants
pub mod constants {
    /// Target block time in seconds
    pub const TARGET_BLOCK_TIME_SECS: u64 = 12;

    /// Base block reward (in wei) - post-merge: 0
    pub const BLOCK_REWARD_WEI: u64 = 0;

    /// Minimum stake for validator (32 ETH in wei)
    pub const MIN_VALIDATOR_STAKE: u128 = 32_000_000_000_000_000_000;

    /// Gas limit per block
    pub const BLOCK_GAS_LIMIT: u64 = 30_000_000;
}
