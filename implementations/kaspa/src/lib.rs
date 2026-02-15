//! # Kaspa-style Implementation (GHOSTDAG/BlockDAG)
//!
//! This module implements Kaspa-specific features, particularly the GHOSTDAG protocol.
//!
//! ## Key Differences from Core
//!
//! | Aspect | Core (blockchain-lab-core) | Kaspa |
//! |--------|---------------------------|-------|
//! | Block Structure | Linear Chain | DAG (Directed Acyclic Graph) |
//! | Consensus | PoW (longest chain) | GHOSTDAG (heaviest DAG) |
//! | Block Time | Configurable | ~1 second |
//! | Hash Function | SHA256 | BLAKE2b |
//! | Parents | 1 (prev_hash) | Multiple (up to K parents) |
//! | Orphan Handling | Discarded | Included in DAG |
//! | Finality | Probabilistic | GHOSTDAG ordering |
//!
//! ## GHOSTDAG Overview
//!
//! GHOSTDAG (Greedy Heaviest-Observed Sub-Tree DAG) allows:
//! - Multiple blocks to be created in parallel
//! - No orphan blocks (all valid blocks are included)
//! - High throughput (~1 block/second)
//! - Total ordering despite parallel creation
//!
//! ## Modules to Implement
//!
//! - [ ] `dag/` - DAG data structure
//! - [ ] `consensus/ghostdag.rs` - GHOSTDAG protocol
//! - [ ] `consensus/blue_score.rs` - Blue score calculation
//! - [ ] `crypto/blake2b.rs` - BLAKE2b hashing

pub mod consensus;
pub mod dag;

/// Kaspa-specific constants
pub mod constants {
    /// Target block time in milliseconds (1 second)
    pub const TARGET_BLOCK_TIME_MS: u64 = 1000;

    /// Maximum number of parents per block
    pub const MAX_BLOCK_PARENTS: usize = 10;

    /// GHOSTDAG K parameter (anticone size limit)
    pub const GHOSTDAG_K: u64 = 18;

    /// Pruning depth
    pub const PRUNING_DEPTH: u64 = 185_798;
}
