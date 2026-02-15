//! # Avalanche-style Implementation (Snowball Consensus + Subnets)
//!
//! This module implements Avalanche-specific features.
//!
//! ## Key Differences from Core
//!
//! | Aspect | Core (blockchain-lab-core) | Avalanche |
//! |--------|---------------------------|-----------|
//! | Consensus | PoW | Snowball (probabilistic sampling) |
//! | Finality | Probabilistic | Probabilistic (~1-2 sec) |
//! | Leader | Miner | None (random sampling) |
//! | Quorum | 51% hashrate | k-sample with α threshold |
//! | Architecture | Single chain | Multi-chain (Subnets) |
//!
//! ## Avalanche Consensus Family
//!
//! Avalanche uses a novel consensus approach based on **repeated random sampling**.
//!
//! ```text
//! Traditional BFT:                   Avalanche Snowball:
//!
//! All validators must agree          Sample k random validators
//!        ↓                                  ↓
//! O(n²) message complexity           O(k) messages per round
//!        ↓                                  ↓
//! Deterministic finality             Probabilistic finality
//!        ↓                                  ↓
//! Slow (needs 2/3+ responses)        Fast (small sample size)
//! ```
//!
//! ## Protocol Hierarchy
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        Snowball                             │
//! │  Long-term preference tracking via preferenceStrength      │
//! │  (tracks cumulative votes for each option)                 │
//! ├─────────────────────────────────────────────────────────────┤
//! │                       Snowflake                             │
//! │  Confidence counter with threshold β                        │
//! │  (tracks consecutive successful polls)                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │                         Slush                               │
//! │  Simple preference based on last successful poll           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Snowball Consensus Flow
//!
//! ```text
//! Node A wants to finalize block B:
//!
//! ┌────────────────────────────────────────────────────────────┐
//! │ Round 1: Sample k=20 random validators                     │
//! │          Ask: "Do you prefer block B?"                     │
//! │          Responses: 16 prefer B (≥ α=15)                   │
//! │          → Update preference to B, confidence++            │
//! ├────────────────────────────────────────────────────────────┤
//! │ Round 2: Sample another k=20 validators                    │
//! │          Responses: 17 prefer B (≥ α=15)                   │
//! │          → confidence++ (now 2)                            │
//! ├────────────────────────────────────────────────────────────┤
//! │ ...repeat...                                               │
//! ├────────────────────────────────────────────────────────────┤
//! │ Round 20: confidence reaches β=20                          │
//! │           → Block B is FINALIZED                           │
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Parameters
//!
//! - **k (sample size)**: Number of validators to query each round (default: 20)
//! - **α (alpha/quorum)**: Minimum responses to update preference (default: 15)
//! - **β (beta/threshold)**: Consecutive successful polls for finality (default: 20)
//!
//! ## Subnet Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Primary Network                         │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐          │
//! │  │  P-Chain    │ │  X-Chain    │ │  C-Chain    │          │
//! │  │ (Platform)  │ │  (Assets)   │ │   (EVM)     │          │
//! │  └─────────────┘ └─────────────┘ └─────────────┘          │
//! └─────────────────────────────────────────────────────────────┘
//!                            ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Custom Subnets                           │
//! │  ┌─────────────────────┐ ┌─────────────────────┐           │
//! │  │     Subnet A        │ │     Subnet B        │           │
//! │  │ (own validators)    │ │ (own validators)    │           │
//! │  │ ┌─────┐ ┌─────┐    │ │ ┌─────────────┐     │           │
//! │  │ │VM 1 │ │VM 2 │    │ │ │  Custom VM  │     │           │
//! │  │ └─────┘ └─────┘    │ │ └─────────────┘     │           │
//! │  └─────────────────────┘ └─────────────────────┘           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`snowball`] - Snowball/Snowflake/Slush consensus
//! - [`validator`] - Validator set and stake-weighted sampling
//! - [`subnet`] - Subnet and chain management

pub mod snowball;
pub mod validator;
pub mod subnet;

/// Avalanche-specific constants
/// Reference: avalanchego/snow/consensus/snowball/parameters.go
pub mod constants {
    /// Sample size (k) - number of validators to query each round
    pub const K: usize = 20;

    /// Alpha preference - minimum votes to change preference
    pub const ALPHA_PREFERENCE: usize = 15;

    /// Alpha confidence - minimum votes to increase confidence
    pub const ALPHA_CONFIDENCE: usize = 15;

    /// Beta - consecutive successful polls required for finality
    pub const BETA: usize = 20;

    /// Maximum concurrent polls
    pub const CONCURRENT_REPOLLS: usize = 4;

    /// Target number of blocks being processed
    pub const OPTIMAL_PROCESSING: usize = 10;

    /// Maximum blocks in processing state
    pub const MAX_OUTSTANDING_ITEMS: usize = 256;

    /// Maximum time any block can be in processing (seconds)
    pub const MAX_ITEM_PROCESSING_TIME_SECS: u64 = 30;

    /// Primary network finality time (~1-2 seconds)
    pub const EXPECTED_FINALITY_SECS: f64 = 1.5;
}
