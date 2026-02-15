//! Block structure and operations
//!
//! A block is the fundamental unit of the blockchain, containing
//! a set of transactions and linking to the previous block.
//!
//! ## Block structure:
//! ```text
//! Block {
//!     timestamp: i64,           // When the block was created
//!     prev_hash: String,        // Hash of previous block
//!     hash: String,             // This block's hash
//!     transactions: Vec<Tx>,    // Transactions in this block
//!     nonce: i64,               // Proof of work nonce
//!     height: usize,            // Block height in chain
//!     merkle_root: Vec<u8>,     // Merkle root of transactions
//! }
//! ```
//!
//! ## Comparison:
//! - **Bitcoin**: Similar structure, includes version, bits (difficulty)
//! - **Ethereum**: Includes state root, receipts root, uncles hash
//! - **Kaspa**: Block references multiple parents (DAG)

use crate::crypto::{sha256, compute_merkle_root};
use crate::execution::Transaction;
use serde::{Deserialize, Serialize};
use sled::IVec;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get current timestamp in milliseconds
pub fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as i64
}

/// A block in the blockchain
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Block {
    /// Timestamp when the block was created
    timestamp: i64,
    /// Hash of the previous block
    prev_hash: String,
    /// Hash of this block
    hash: String,
    /// Transactions included in this block
    transactions: Vec<Transaction>,
    /// Nonce used for proof of work
    nonce: i64,
    /// Height of this block in the chain
    height: usize,
    /// Merkle root of all transactions
    merkle_root: Vec<u8>,
}

impl Block {
    /// Create a new block (without mining - use consensus layer for that)
    pub fn new(
        prev_hash: String,
        transactions: Vec<Transaction>,
        height: usize,
        nonce: i64,
        hash: String,
    ) -> Self {
        let merkle_root = Self::compute_merkle_root(&transactions);
        Block {
            timestamp: current_timestamp(),
            prev_hash,
            hash,
            transactions,
            nonce,
            height,
            merkle_root,
        }
    }

    /// Create a block with explicit timestamp (for testing)
    pub fn with_timestamp(
        timestamp: i64,
        prev_hash: String,
        transactions: Vec<Transaction>,
        height: usize,
        nonce: i64,
        hash: String,
    ) -> Self {
        let merkle_root = Self::compute_merkle_root(&transactions);
        Block {
            timestamp,
            prev_hash,
            hash,
            transactions,
            nonce,
            height,
            merkle_root,
        }
    }

    /// Compute merkle root of transactions
    fn compute_merkle_root(transactions: &[Transaction]) -> Vec<u8> {
        let tx_hashes: Vec<Vec<u8>> = transactions
            .iter()
            .map(|tx| tx.get_id().to_vec())
            .collect();
        compute_merkle_root(&tx_hashes)
    }

    /// Hash all transactions (concatenated hashes)
    pub fn hash_transactions(&self) -> Vec<u8> {
        let mut tx_hashes = vec![];
        for tx in &self.transactions {
            tx_hashes.extend(tx.get_id());
        }
        sha256(&tx_hashes)
    }

    /// Serialize block to bytes
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    /// Deserialize block from bytes
    pub fn deserialize(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }

    // Getters
    pub fn get_timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn get_prev_hash(&self) -> &str {
        &self.prev_hash
    }

    pub fn get_hash(&self) -> &str {
        &self.hash
    }

    pub fn get_hash_bytes(&self) -> Vec<u8> {
        self.hash.as_bytes().to_vec()
    }

    pub fn get_transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    pub fn get_nonce(&self) -> i64 {
        self.nonce
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn get_merkle_root(&self) -> &[u8] {
        &self.merkle_root
    }
}

/// Allow converting Block to sled IVec
impl From<Block> for IVec {
    fn from(block: Block) -> Self {
        let bytes = block.serialize();
        Self::from(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair, public_key_from_pkcs8, public_key_to_address};

    fn create_test_tx() -> Transaction {
        let pkcs8 = generate_keypair();
        let public_key = public_key_from_pkcs8(&pkcs8);
        let address = public_key_to_address(&public_key);
        Transaction::new_coinbase_tx(&address)
    }

    #[test]
    fn test_block_creation() {
        let tx = create_test_tx();
        let block = Block::new(
            "prev_hash".to_string(),
            vec![tx],
            1,
            12345,
            "block_hash".to_string(),
        );

        assert_eq!(block.get_height(), 1);
        assert_eq!(block.get_prev_hash(), "prev_hash");
        assert_eq!(block.get_hash(), "block_hash");
        assert_eq!(block.get_nonce(), 12345);
        assert_eq!(block.get_transactions().len(), 1);
    }

    #[test]
    fn test_block_serialization() {
        let tx = create_test_tx();
        let block = Block::new(
            "prev".to_string(),
            vec![tx],
            0,
            0,
            "hash".to_string(),
        );

        let serialized = block.serialize();
        let deserialized = Block::deserialize(&serialized);

        assert_eq!(block.get_hash(), deserialized.get_hash());
        assert_eq!(block.get_height(), deserialized.get_height());
    }

    #[test]
    fn test_merkle_root_calculation() {
        let tx1 = create_test_tx();
        let tx2 = create_test_tx();

        let block = Block::new(
            "prev".to_string(),
            vec![tx1, tx2],
            0,
            0,
            "hash".to_string(),
        );

        assert!(!block.get_merkle_root().is_empty());
        assert_eq!(block.get_merkle_root().len(), 32);
    }
}
