//! # Solana-style Implementation (PoH + Tower BFT + Sealevel)
//!
//! This module implements Solana-specific features.
//!
//! ## Key Differences from Core
//!
//! | Aspect | Core (blockchain-lab-core) | Solana |
//! |--------|---------------------------|--------|
//! | Data Model | UTXO | Account-based (owner model) |
//! | Consensus | PoW | PoH + Tower BFT |
//! | Hash Function | SHA256 | SHA256 (PoH chain) |
//! | Signature | P-256 | Ed25519 |
//! | Block Structure | Linear | Slot → Entry → Shred |
//! | Block Time | Variable | 400ms (slot) |
//! | Execution | Sequential | Parallel (Sealevel) |
//! | Smart Contracts | None | BPF Programs |
//!
//! ## Solana Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Slot (400ms)                          │
//! │  ┌─────────┐ ┌─────────┐ ┌─────────┐     ┌─────────┐      │
//! │  │ Entry 1 │→│ Entry 2 │→│ Entry 3 │→...→│Entry 64 │      │
//! │  │(Tick)   │ │(TXs)    │ │(Tick)   │     │(Tick)   │      │
//! │  └─────────┘ └─────────┘ └─────────┘     └─────────┘      │
//! └─────────────────────────────────────────────────────────────┘
//!                              ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Entry Structure                          │
//! │  ┌──────────────┬──────────────┬─────────────────────────┐ │
//! │  │ num_hashes   │ hash (PoH)   │ transactions[]          │ │
//! │  │ (u64)        │ (SHA256)     │ (parallel executable)   │ │
//! │  └──────────────┴──────────────┴─────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Proof of History (PoH)
//!
//! PoH is a Verifiable Delay Function (VDF) using SHA-256:
//!
//! ```text
//! hash₀ → SHA256(hash₀) → hash₁ → SHA256(hash₁) → hash₂ → ...
//!              ↑                       ↑
//!        num_hashes=1            num_hashes=1
//!
//! Events are mixed into the chain:
//! hash_n → SHA256(hash_n || event_data) → hash_{n+1}
//! ```
//!
//! ## Tower BFT Consensus
//!
//! Vote-based BFT with exponential lockouts:
//!
//! ```text
//! Vote Stack (max 32 votes):
//! ┌────────────────────────────────────┐
//! │ Vote 32: slot=1000, lockout=2^32  │ ← Oldest (highest lockout)
//! │ Vote 31: slot=1001, lockout=2^31  │
//! │ ...                                │
//! │ Vote 2:  slot=1030, lockout=4     │
//! │ Vote 1:  slot=1031, lockout=2     │ ← Newest
//! └────────────────────────────────────┘
//!
//! Switching to a different fork requires waiting for lockouts to expire.
//! Cost of rollback grows exponentially → Economic finality
//! ```
//!
//! ## Account Model (Owner-based)
//!
//! Unlike Ethereum's simple balance model, Solana uses ownership:
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │              Account                      │
//! ├──────────────────────────────────────────┤
//! │ lamports: u64      (balance, 1 SOL=1e9) │
//! │ data: Vec<u8>      (program state)       │
//! │ owner: Pubkey      (program that owns)   │
//! │ executable: bool   (is this a program?)  │
//! │ rent_epoch: u64    (rent tracking)       │
//! └──────────────────────────────────────────┘
//!
//! Only the owner program can modify account data.
//! System Program owns all wallet accounts.
//! ```
//!
//! ## Sealevel Runtime (Parallel Execution)
//!
//! Transactions declare account access upfront:
//!
//! ```text
//! TX1: read(A), write(B)  ─┬─→ Execute in parallel
//! TX2: read(C), write(D)  ─┘   (no conflicts)
//!
//! TX3: write(A)           ─┬─→ Must serialize
//! TX4: read(A)            ─┘   (conflict on A)
//! ```
//!
//! ## Modules
//!
//! - [`consensus`] - Proof of History and Tower BFT
//! - [`account`] - Account model with ownership
//! - [`runtime`] - Sealevel parallel transaction execution
//! - [`program`] - BPF program model

pub mod consensus;
pub mod account;
pub mod runtime;
pub mod program;

/// Solana-specific constants
/// Reference: solana/sdk/program/src/clock.rs
pub mod constants {
    /// Ticks per second (PoH rate)
    pub const TICKS_PER_SECOND: u64 = 160;

    /// Ticks per slot
    pub const TICKS_PER_SLOT: u64 = 64;

    /// Milliseconds per slot (400ms)
    pub const MS_PER_SLOT: u64 = 1000 * TICKS_PER_SLOT / TICKS_PER_SECOND;

    /// Hashes per tick (PoH computation)
    pub const HASHES_PER_TICK: u64 = 12_500;

    /// Slots per epoch (~2 days)
    pub const SLOTS_PER_EPOCH: u64 = 432_000;

    /// Maximum age of a transaction (in slots, ~60 seconds)
    pub const MAX_PROCESSING_AGE: u64 = 150;

    /// Maximum accounts per transaction
    pub const MAX_TX_ACCOUNT_LOCKS: usize = 128;

    /// Lamports per SOL (1 SOL = 1 billion lamports)
    pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

    /// Maximum votes in Tower BFT
    pub const MAX_LOCKOUT_HISTORY: usize = 31;

    /// Initial lockout (2 slots)
    pub const INITIAL_LOCKOUT: u64 = 2;

    /// Threshold depth for vote commitment
    pub const VOTE_THRESHOLD_DEPTH: usize = 8;

    /// Minimum stake for supermajority (2/3)
    pub const SUPERMAJORITY_THRESHOLD: f64 = 2.0 / 3.0;
}
