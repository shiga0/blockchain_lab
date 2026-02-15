//! Memory pool for pending transactions
//!
//! The mempool holds transactions that have been validated but not yet
//! included in a block.
//!
//! ## Comparison with other blockchains:
//! - **Bitcoin**: Priority based on fee rate and age
//! - **Ethereum**: Gas price ordering, nonce ordering per account
//! - **Kaspa**: Similar to Bitcoin
//! - **This implementation**: Simple FIFO ordering

use crate::execution::Transaction;
use data_encoding::HEXLOWER;
use std::collections::HashMap;
use std::sync::RwLock;

/// Memory pool for pending transactions
pub struct MemoryPool {
    /// Map of transaction ID (hex) -> Transaction
    inner: RwLock<HashMap<String, Transaction>>,
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryPool {
    /// Create a new empty memory pool
    pub fn new() -> Self {
        MemoryPool {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Check if a transaction exists in the pool
    pub fn contains(&self, txid_hex: &str) -> bool {
        self.inner.read().unwrap().contains_key(txid_hex)
    }

    /// Add a transaction to the pool
    pub fn add(&self, tx: Transaction) {
        let txid_hex = HEXLOWER.encode(tx.get_id());
        self.inner.write().unwrap().insert(txid_hex, tx);
    }

    /// Get a transaction by ID
    pub fn get(&self, txid_hex: &str) -> Option<Transaction> {
        self.inner.read().unwrap().get(txid_hex).cloned()
    }

    /// Remove a transaction from the pool
    pub fn remove(&self, txid_hex: &str) {
        self.inner.write().unwrap().remove(txid_hex);
    }

    /// Get all transactions in the pool
    pub fn get_all(&self) -> Vec<Transaction> {
        self.inner.read().unwrap().values().cloned().collect()
    }

    /// Get number of transactions in the pool
    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    /// Check if the pool is empty
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }

    /// Clear all transactions from the pool
    pub fn clear(&self) {
        self.inner.write().unwrap().clear();
    }

    /// Remove multiple transactions by their IDs
    pub fn remove_batch(&self, txids: &[Vec<u8>]) {
        let mut inner = self.inner.write().unwrap();
        for txid in txids {
            let txid_hex = HEXLOWER.encode(txid);
            inner.remove(&txid_hex);
        }
    }
}

/// Tracks blocks that are being downloaded
pub struct BlockInTransit {
    inner: RwLock<Vec<Vec<u8>>>,
}

impl Default for BlockInTransit {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockInTransit {
    /// Create a new empty tracker
    pub fn new() -> Self {
        BlockInTransit {
            inner: RwLock::new(vec![]),
        }
    }

    /// Add block hashes to download queue
    pub fn add_blocks(&self, blocks: &[Vec<u8>]) {
        let mut inner = self.inner.write().unwrap();
        for hash in blocks {
            inner.push(hash.to_vec());
        }
    }

    /// Get the first block hash in the queue
    pub fn first(&self) -> Option<Vec<u8>> {
        self.inner.read().unwrap().first().cloned()
    }

    /// Remove a block hash from the queue
    pub fn remove(&self, block_hash: &[u8]) {
        let mut inner = self.inner.write().unwrap();
        if let Some(idx) = inner.iter().position(|x| x == block_hash) {
            inner.remove(idx);
        }
    }

    /// Clear all pending blocks
    pub fn clear(&self) {
        self.inner.write().unwrap().clear();
    }

    /// Get number of blocks in transit
    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    /// Check if there are no blocks in transit
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
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
    fn test_mempool_add_and_get() {
        let pool = MemoryPool::new();
        let tx = create_test_tx();
        let txid_hex = HEXLOWER.encode(tx.get_id());

        pool.add(tx.clone());

        assert!(pool.contains(&txid_hex));
        assert_eq!(pool.len(), 1);

        let retrieved = pool.get(&txid_hex).unwrap();
        assert_eq!(retrieved.get_id(), tx.get_id());
    }

    #[test]
    fn test_mempool_remove() {
        let pool = MemoryPool::new();
        let tx = create_test_tx();
        let txid_hex = HEXLOWER.encode(tx.get_id());

        pool.add(tx);
        assert!(!pool.is_empty());

        pool.remove(&txid_hex);
        assert!(pool.is_empty());
    }

    #[test]
    fn test_mempool_get_all() {
        let pool = MemoryPool::new();
        let tx1 = create_test_tx();
        let tx2 = create_test_tx();

        pool.add(tx1);
        pool.add(tx2);

        let all = pool.get_all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_block_in_transit() {
        let tracker = BlockInTransit::new();
        let hash1 = vec![1, 2, 3];
        let hash2 = vec![4, 5, 6];

        tracker.add_blocks(&[hash1.clone(), hash2.clone()]);
        assert_eq!(tracker.len(), 2);

        let first = tracker.first().unwrap();
        assert_eq!(first, hash1);

        tracker.remove(&hash1);
        assert_eq!(tracker.len(), 1);
    }
}
