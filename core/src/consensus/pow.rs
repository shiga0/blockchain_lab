//! Proof of Work consensus implementation
//!
//! This module implements the classic PoW consensus used by Bitcoin.
//!
//! ## How it works:
//! 1. Miners repeatedly hash block header with different nonces
//! 2. Goal: find a hash below the target threshold
//! 3. Difficulty adjusts to maintain target block time
//!
//! ## Comparison:
//! - **Bitcoin**: SHA256 double hash, 10 min blocks, 2016 block adjustment
//! - **Ethereum (pre-merge)**: Ethash (memory-hard), 15 sec blocks
//! - **Kaspa**: kHeavyHash, 1 sec blocks (DAG allows this)
//! - **This implementation**: SHA256, configurable difficulty

use crate::consensus::traits::{Consensus, ConsensusError, ConsensusResult};
use crate::crypto::sha256;
use crate::primitives::Block;
use data_encoding::HEXLOWER;
use num_bigint::{BigInt, Sign};
use std::ops::ShlAssign;

/// Default difficulty (number of leading zero bits required)
pub const DEFAULT_TARGET_BITS: u32 = 8;

/// Maximum nonce value
const MAX_NONCE: i64 = i64::MAX;

/// Proof of Work consensus implementation
pub struct ProofOfWork {
    /// Target difficulty (number of leading zero bits)
    target_bits: u32,
    /// Pre-computed target threshold
    target: BigInt,
}

impl ProofOfWork {
    /// Create a new PoW consensus with specified difficulty
    pub fn new(target_bits: u32) -> Self {
        let mut target = BigInt::from(1);
        target.shl_assign(256 - target_bits as usize);
        ProofOfWork { target_bits, target }
    }

    /// Create with default difficulty
    pub fn default_difficulty() -> Self {
        Self::new(DEFAULT_TARGET_BITS)
    }

    /// Prepare data for hashing
    fn prepare_data(
        &self,
        prev_hash: &str,
        transactions_hash: &[u8],
        timestamp: i64,
        nonce: i64,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(prev_hash.as_bytes());
        data.extend(transactions_hash);
        data.extend(timestamp.to_be_bytes());
        data.extend(self.target_bits.to_be_bytes());
        data.extend(nonce.to_be_bytes());
        data
    }

    /// Mine a block (find valid nonce)
    fn mine(
        &self,
        prev_hash: &str,
        transactions_hash: &[u8],
        timestamp: i64,
    ) -> ConsensusResult<(i64, String)> {
        let mut nonce: i64 = 0;

        while nonce < MAX_NONCE {
            let data = self.prepare_data(prev_hash, transactions_hash, timestamp, nonce);
            let hash = sha256(&data);
            let hash_int = BigInt::from_bytes_be(Sign::Plus, &hash);

            if hash_int < self.target {
                return Ok((nonce, HEXLOWER.encode(&hash)));
            }
            nonce += 1;
        }

        Err(ConsensusError::MiningFailed(
            "Exceeded maximum nonce".to_string(),
        ))
    }

    /// Verify that a block's proof of work is valid
    pub fn verify_proof(
        &self,
        prev_hash: &str,
        transactions_hash: &[u8],
        timestamp: i64,
        nonce: i64,
        block_hash: &str,
    ) -> bool {
        let data = self.prepare_data(prev_hash, transactions_hash, timestamp, nonce);
        let hash = sha256(&data);
        let hash_hex = HEXLOWER.encode(&hash);

        if hash_hex != block_hash {
            return false;
        }

        let hash_int = BigInt::from_bytes_be(Sign::Plus, &hash);
        hash_int < self.target
    }
}

impl Consensus for ProofOfWork {
    fn name(&self) -> &'static str {
        "Proof of Work"
    }

    fn validate_block(&self, block: &Block, prev_block: Option<&Block>) -> ConsensusResult<()> {
        // Verify the proof of work
        let transactions_hash = block.hash_transactions();
        let is_valid = self.verify_proof(
            block.get_prev_hash(),
            &transactions_hash,
            block.get_timestamp(),
            block.get_nonce(),
            block.get_hash(),
        );

        if !is_valid {
            return Err(ConsensusError::InvalidProof(
                "Block hash does not meet target".to_string(),
            ));
        }

        // Verify chain linkage
        if let Some(prev) = prev_block {
            if block.get_prev_hash() != prev.get_hash() {
                return Err(ConsensusError::InvalidBlock(
                    "Previous hash mismatch".to_string(),
                ));
            }
            if block.get_height() != prev.get_height() + 1 {
                return Err(ConsensusError::InvalidBlock(
                    "Invalid block height".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn create_block_proof(
        &self,
        transactions_hash: &[u8],
        prev_hash: &str,
        timestamp: i64,
    ) -> ConsensusResult<(i64, String)> {
        self.mine(prev_hash, transactions_hash, timestamp)
    }

    fn get_difficulty(&self) -> u32 {
        self.target_bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pow_mining() {
        let pow = ProofOfWork::new(8); // Low difficulty for testing
        let prev_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let tx_hash = sha256(b"test transactions");
        let timestamp = 1234567890i64;

        let result = pow.create_block_proof(&tx_hash, prev_hash, timestamp);
        assert!(result.is_ok());

        let (nonce, hash) = result.unwrap();

        // Verify the hash starts with appropriate zeros
        assert!(hash.starts_with("00"));

        // Verify the proof
        assert!(pow.verify_proof(prev_hash, &tx_hash, timestamp, nonce, &hash));
    }

    #[test]
    fn test_pow_verification_fails_wrong_hash() {
        let pow = ProofOfWork::new(8);
        let prev_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let tx_hash = sha256(b"test");
        let timestamp = 1234567890i64;

        // Mine a valid block
        let (nonce, _hash) = pow.create_block_proof(&tx_hash, prev_hash, timestamp).unwrap();

        // Verification should fail with wrong hash
        let wrong_hash = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert!(!pow.verify_proof(prev_hash, &tx_hash, timestamp, nonce, wrong_hash));
    }

    #[test]
    fn test_difficulty_affects_hash() {
        // Higher difficulty should produce hash with more leading zeros
        let pow_easy = ProofOfWork::new(4);
        let pow_hard = ProofOfWork::new(12);

        let prev_hash = "0";
        let tx_hash = sha256(b"test");
        let timestamp = 1234567890i64;

        let (_, hash_easy) = pow_easy.create_block_proof(&tx_hash, prev_hash, timestamp).unwrap();
        let (_, hash_hard) = pow_hard.create_block_proof(&tx_hash, prev_hash, timestamp).unwrap();

        // Count leading zeros
        let zeros_easy = hash_easy.chars().take_while(|&c| c == '0').count();
        let zeros_hard = hash_hard.chars().take_while(|&c| c == '0').count();

        assert!(zeros_hard >= zeros_easy);
    }
}
