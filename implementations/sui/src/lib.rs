//! # Sui-style Implementation (Mysticeti + Object-centric + PTB)
//!
//! This module implements Sui-specific features.
//!
//! ## Key Differences from Other Chains
//!
//! | Aspect | Ethereum | Solana | Sui |
//! |--------|----------|--------|-----|
//! | Data Model | Account | Account+Owner | Object-centric |
//! | Consensus | Casper FFG | Tower BFT | Mysticeti (DAG) |
//! | Execution | Sequential | Parallel | Fastpath/Consensus |
//! | Transactions | EVM calls | Instructions | PTB (composable) |
//!
//! ## Object-Centric Model
//!
//! ```text
//! Sui objects have ownership types that determine execution path:
//!
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      Object Ownership                           │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  Owned Object (AddressOwner)          Shared Object             │
//! │  ┌───────────────────────┐            ┌───────────────────────┐ │
//! │  │ owner: 0xAlice        │            │ initial_shared_ver: 5 │ │
//! │  │ version: 42           │            │ version: 100          │ │
//! │  │ digest: 0xabc...      │            │ digest: 0xdef...      │ │
//! │  └───────────────────────┘            └───────────────────────┘ │
//! │           │                                    │                │
//! │           ▼                                    ▼                │
//! │    ┌─────────────┐                    ┌─────────────────┐      │
//! │    │  Fastpath   │                    │    Consensus    │      │
//! │    │ (no ordering│                    │   (Mysticeti    │      │
//! │    │   needed)   │                    │    ordering)    │      │
//! │    └─────────────┘                    └─────────────────┘      │
//! │                                                                 │
//! │  Immutable Object                                               │
//! │  ┌───────────────────────┐                                     │
//! │  │ owner: Immutable      │  ← No ordering, anyone can read     │
//! │  │ version: 1 (forever)  │                                     │
//! │  └───────────────────────┘                                     │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Mysticeti Consensus (DAG-based BFT)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    DAG Structure                                │
//! │                                                                 │
//! │  Round 3:    [B3_0]───────[B3_1]───────[B3_2]                  │
//! │                │  ╲         │  ╲         │                     │
//! │  Round 2:    [B2_0]───────[B2_1]───────[B2_2]                  │
//! │                │  ╲         │  ╲         │                     │
//! │  Round 1:    [B1_0]───────[B1_1]───────[B1_2]                  │
//! │                                                                 │
//! │  Each block references multiple ancestors (DAG, not chain)     │
//! └─────────────────────────────────────────────────────────────────┘
//!
//! Wave-based Commitment (3 rounds per wave):
//! ┌────────────────────────────────────────────────────────────────┐
//! │ Round N   (Leader):     Leader proposes block                  │
//! │ Round N+1 (Voting):     Validators vote on leader              │
//! │ Round N+2 (Decision):   Commit if 2f+1 support                 │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Programmable Transaction Blocks (PTB)
//!
//! ```text
//! PTB allows composing multiple operations in one transaction:
//!
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ ProgrammableTransaction {                                       │
//! │   inputs: [                                                     │
//! │     Pure(100),           // Amount                              │
//! │     Object(coin_ref),    // Source coin                         │
//! │   ],                                                            │
//! │   commands: [                                                   │
//! │     SplitCoins(Input(1), [Input(0)]),  // Split 100 from coin  │
//! │     TransferObjects([Result(0)], recipient),  // Transfer      │
//! │   ],                                                            │
//! │ }                                                               │
//! └─────────────────────────────────────────────────────────────────┘
//!
//! Result passing between commands:
//!   Command 0 output → Result(0) → Command 1 input
//! ```
//!
//! ## Transaction Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Owned Objects (Fastpath):                                       │
//! │   Client → Validator → Execute immediately → Effects           │
//! │            (no consensus needed)                                │
//! │                                                                 │
//! │ Shared Objects (Consensus):                                     │
//! │   Client → Validator → Certify (2f+1) → Consensus (Mysticeti)  │
//! │          → Sequence → Execute → Effects                        │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`object`] - Object model and ownership
//! - [`mysticeti`] - Mysticeti DAG consensus
//! - [`ptb`] - Programmable Transaction Blocks

pub mod mysticeti;
pub mod object;
pub mod ptb;

/// Sui-specific constants
pub mod constants {
    /// Epoch duration (approximately 24 hours)
    pub const EPOCH_DURATION_MS: u64 = 24 * 60 * 60 * 1000;

    /// Target checkpoint interval
    pub const CHECKPOINT_INTERVAL_MS: u64 = 200;

    /// Consensus round time target
    pub const ROUND_TIMEOUT_MS: u64 = 500;

    /// Byzantine fault tolerance threshold
    /// Total validators: 3f + 1, tolerates f faults
    pub const BFT_THRESHOLD_DENOMINATOR: u64 = 3;

    /// Minimum validators for quorum: 2f + 1
    pub const QUORUM_THRESHOLD: f64 = 2.0 / 3.0;

    /// Maximum PTB commands
    pub const MAX_PTB_COMMANDS: usize = 1024;

    /// Maximum transaction size (128 KB)
    pub const MAX_TX_SIZE: usize = 128 * 1024;

    /// Maximum gas budget
    pub const MAX_GAS_BUDGET: u64 = 50_000_000_000;

    /// Object ID length (32 bytes)
    pub const OBJECT_ID_LENGTH: usize = 32;

    /// Digest length (32 bytes)
    pub const DIGEST_LENGTH: usize = 32;
}
