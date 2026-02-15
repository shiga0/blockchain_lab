//! Consensus mechanism traits
//!
//! This module defines the interface for consensus algorithms, allowing
//! different mechanisms to be plugged in for comparison.
//!
//! ## Comparison with other blockchains:
//! - **Bitcoin**: Proof of Work (Nakamoto consensus)
//! - **Ethereum**: Proof of Stake (post-merge)
//! - **Kaspa**: GHOSTDAG (DAG-based PoW)
//! - **Solana**: Proof of History + Tower BFT
//! - **This implementation**: Pluggable (default: PoW)

use crate::primitives::Block;

/// Result type for consensus operations
pub type ConsensusResult<T> = Result<T, ConsensusError>;

/// Errors that can occur during consensus operations
#[derive(Debug, Clone)]
pub enum ConsensusError {
    /// Block validation failed
    InvalidBlock(String),
    /// Proof verification failed
    InvalidProof(String),
    /// Mining/block creation failed
    MiningFailed(String),
}

impl std::fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusError::InvalidBlock(msg) => write!(f, "Invalid block: {}", msg),
            ConsensusError::InvalidProof(msg) => write!(f, "Invalid proof: {}", msg),
            ConsensusError::MiningFailed(msg) => write!(f, "Mining failed: {}", msg),
        }
    }
}

impl std::error::Error for ConsensusError {}

/// Consensus mechanism trait
///
/// This trait defines the interface that any consensus algorithm must implement.
/// By abstracting consensus, we can easily compare different mechanisms:
///
/// ```rust,ignore
/// // Example: Switching between consensus mechanisms
/// let pow = ProofOfWork::new(difficulty);
/// let block = pow.create_block(txs, prev_hash, height)?;
///
/// // Or with a different consensus:
/// let pos = ProofOfStake::new(validators);
/// let block = pos.create_block(txs, prev_hash, height)?;
/// ```
pub trait Consensus: Send + Sync {
    /// Get the name of this consensus mechanism
    fn name(&self) -> &'static str;

    /// Validate a block according to this consensus rules
    ///
    /// # Arguments
    /// * `block` - The block to validate
    /// * `prev_block` - The previous block (None for genesis)
    fn validate_block(&self, block: &Block, prev_block: Option<&Block>) -> ConsensusResult<()>;

    /// Create a new block (mining for PoW, proposing for PoS, etc.)
    ///
    /// # Arguments
    /// * `transactions_hash` - Hash of all transactions
    /// * `prev_hash` - Hash of the previous block
    /// * `timestamp` - Block timestamp
    ///
    /// # Returns
    /// Tuple of (nonce, block_hash)
    fn create_block_proof(
        &self,
        transactions_hash: &[u8],
        prev_hash: &str,
        timestamp: i64,
    ) -> ConsensusResult<(i64, String)>;

    /// Get the current difficulty/stake requirement
    fn get_difficulty(&self) -> u32;
}

/// Chain selection trait for fork resolution
///
/// Different blockchains use different rules:
/// - **Bitcoin**: Longest chain (most cumulative work)
/// - **Ethereum**: Heaviest chain (GHOST-derived)
/// - **Kaspa**: GHOSTDAG ordering
pub trait ChainSelector: Send + Sync {
    /// Select the canonical chain from multiple candidates
    ///
    /// # Arguments
    /// * `chains` - List of chain tips to choose from
    ///
    /// # Returns
    /// Index of the selected chain
    fn select_chain(&self, chain_heights: &[(String, usize)]) -> usize;
}

/// Simple longest chain selector (Bitcoin-style)
pub struct LongestChainSelector;

impl ChainSelector for LongestChainSelector {
    fn select_chain(&self, chain_heights: &[(String, usize)]) -> usize {
        chain_heights
            .iter()
            .enumerate()
            .max_by_key(|(_, (_, height))| height)
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_longest_chain_selector() {
        let selector = LongestChainSelector;

        let chains = vec![
            ("hash1".to_string(), 10),
            ("hash2".to_string(), 15),
            ("hash3".to_string(), 12),
        ];

        let selected = selector.select_chain(&chains);
        assert_eq!(selected, 1); // chain with height 15
    }
}
