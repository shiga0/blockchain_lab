//! Blockchain structure and operations
//!
//! The blockchain is a linked list of blocks, where each block
//! references its predecessor via the previous block's hash.
//!
//! ## Data Structure:
//! ```text
//! Genesis Block ← Block 1 ← Block 2 ← ... ← Tip Block
//!     ↓              ↓          ↓                ↓
//!   height=0     height=1   height=2         best_height
//! ```
//!
//! ## Comparison:
//! - **Bitcoin**: Linear chain, longest chain rule
//! - **Ethereum**: Linear chain with uncle blocks
//! - **Kaspa**: DAG (blocks can have multiple parents)
//! - **This implementation**: Linear chain

use crate::consensus::{Consensus, ProofOfWork, DEFAULT_TARGET_BITS};
use crate::execution::{Transaction, TXOutput};
use crate::primitives::block::{current_timestamp, Block};
use crate::crypto::sha256;
use data_encoding::HEXLOWER;
use sled::{Db, Tree};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

const TIP_BLOCK_HASH_KEY: &str = "tip_block_hash";
const BLOCKS_TREE: &str = "blocks";

/// The blockchain structure
#[derive(Clone)]
pub struct Blockchain {
    /// Hash of the tip (most recent) block
    tip_hash: Arc<RwLock<String>>,
    /// Database for persistent storage
    db: Db,
}

impl Blockchain {
    /// Create a new blockchain with genesis block
    pub fn create(genesis_address: &str, db: Db) -> Self {
        let blocks_tree = db.open_tree(BLOCKS_TREE).unwrap();

        let tip_hash = if let Some(data) = blocks_tree.get(TIP_BLOCK_HASH_KEY).unwrap() {
            // Blockchain already exists
            String::from_utf8(data.to_vec()).unwrap()
        } else {
            // Create genesis block
            let coinbase_tx = Transaction::new_coinbase_tx(genesis_address);
            let block = Self::create_genesis_block(&coinbase_tx);
            Self::store_block(&blocks_tree, &block);
            block.get_hash().to_string()
        };

        Blockchain {
            tip_hash: Arc::new(RwLock::new(tip_hash)),
            db,
        }
    }

    /// Open an existing blockchain
    pub fn open(db: Db) -> Result<Self, &'static str> {
        let blocks_tree = db.open_tree(BLOCKS_TREE).unwrap();

        let tip_bytes = blocks_tree
            .get(TIP_BLOCK_HASH_KEY)
            .unwrap()
            .ok_or("No existing blockchain found")?;

        let tip_hash = String::from_utf8(tip_bytes.to_vec()).unwrap();

        Ok(Blockchain {
            tip_hash: Arc::new(RwLock::new(tip_hash)),
            db,
        })
    }

    /// Create the genesis block
    fn create_genesis_block(coinbase_tx: &Transaction) -> Block {
        let transactions = vec![coinbase_tx.clone()];
        let pow = ProofOfWork::new(DEFAULT_TARGET_BITS);

        let tx_hash = sha256(&transactions.iter().flat_map(|tx| tx.get_id().to_vec()).collect::<Vec<u8>>());
        let prev_hash = "None".to_string();
        let timestamp = current_timestamp();

        let (nonce, hash) = pow
            .create_block_proof(&tx_hash, &prev_hash, timestamp)
            .unwrap();

        Block::with_timestamp(timestamp, prev_hash, transactions, 0, nonce, hash)
    }

    /// Store a block in the database
    fn store_block(blocks_tree: &Tree, block: &Block) {
        let hash = block.get_hash();
        blocks_tree.insert(hash, block.serialize()).unwrap();
        blocks_tree.insert(TIP_BLOCK_HASH_KEY, hash).unwrap();
    }

    /// Get the database
    pub fn get_db(&self) -> &Db {
        &self.db
    }

    /// Get the tip hash
    pub fn get_tip_hash(&self) -> String {
        self.tip_hash.read().unwrap().clone()
    }

    /// Set a new tip hash
    pub fn set_tip_hash(&self, new_tip_hash: &str) {
        let mut tip_hash = self.tip_hash.write().unwrap();
        *tip_hash = new_tip_hash.to_string();
    }

    /// Mine a new block with the given transactions
    pub fn mine_block(&self, transactions: &[Transaction]) -> Block {
        let best_height = self.get_best_height();
        let prev_hash = self.get_tip_hash();
        let pow = ProofOfWork::new(DEFAULT_TARGET_BITS);

        let tx_hash = sha256(
            &transactions
                .iter()
                .flat_map(|tx| tx.get_id().to_vec())
                .collect::<Vec<u8>>(),
        );
        let timestamp = current_timestamp();

        let (nonce, hash) = pow
            .create_block_proof(&tx_hash, &prev_hash, timestamp)
            .unwrap();

        let block = Block::with_timestamp(
            timestamp,
            prev_hash,
            transactions.to_vec(),
            best_height + 1,
            nonce,
            hash,
        );

        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        Self::store_block(&blocks_tree, &block);
        self.set_tip_hash(block.get_hash());

        block
    }

    /// Add an existing block to the chain
    pub fn add_block(&self, block: &Block) {
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();

        // Don't add if already exists
        if blocks_tree.get(block.get_hash()).unwrap().is_some() {
            return;
        }

        // Store the block
        blocks_tree.insert(block.get_hash(), block.serialize()).unwrap();

        // Update tip if this block is higher
        let tip_block_bytes = blocks_tree.get(self.get_tip_hash()).unwrap();
        if let Some(bytes) = tip_block_bytes {
            let tip_block = Block::deserialize(&bytes);
            if block.get_height() > tip_block.get_height() {
                blocks_tree.insert(TIP_BLOCK_HASH_KEY, block.get_hash()).unwrap();
                self.set_tip_hash(block.get_hash());
            }
        }
    }

    /// Get the best (tip) block height
    pub fn get_best_height(&self) -> usize {
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        let tip_bytes = blocks_tree.get(self.get_tip_hash()).unwrap();
        if let Some(bytes) = tip_bytes {
            let block = Block::deserialize(&bytes);
            return block.get_height();
        }
        0
    }

    /// Get a block by its hash
    pub fn get_block(&self, block_hash: &[u8]) -> Option<Block> {
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        blocks_tree
            .get(block_hash)
            .unwrap()
            .map(|bytes| Block::deserialize(&bytes))
    }

    /// Get all block hashes
    pub fn get_block_hashes(&self) -> Vec<Vec<u8>> {
        let mut hashes = vec![];
        let mut iterator = self.iterator();
        while let Some(block) = iterator.next() {
            hashes.push(block.get_hash_bytes());
        }
        hashes
    }

    /// Find a transaction by ID
    pub fn find_transaction(&self, txid: &[u8]) -> Option<Transaction> {
        let mut iterator = self.iterator();
        while let Some(block) = iterator.next() {
            for tx in block.get_transactions() {
                if tx.get_id() == txid {
                    return Some(tx.clone());
                }
            }
        }
        None
    }

    /// Find all UTXOs in the blockchain
    pub fn find_utxo(&self) -> HashMap<String, Vec<TXOutput>> {
        let mut utxo: HashMap<String, Vec<TXOutput>> = HashMap::new();
        let mut spent_txos: HashMap<String, Vec<usize>> = HashMap::new();

        let mut iterator = self.iterator();
        while let Some(block) = iterator.next() {
            for tx in block.get_transactions() {
                let txid_hex = HEXLOWER.encode(tx.get_id());

                // Collect unspent outputs
                'outputs: for (idx, out) in tx.get_vout().iter().enumerate() {
                    // Check if this output was spent
                    if let Some(spent_indices) = spent_txos.get(&txid_hex) {
                        if spent_indices.contains(&idx) {
                            continue 'outputs;
                        }
                    }
                    utxo.entry(txid_hex.clone()).or_default().push(out.clone());
                }

                // Record spent outputs (skip coinbase)
                if !tx.is_coinbase() {
                    for vin in tx.get_vin() {
                        let vin_txid_hex = HEXLOWER.encode(vin.get_txid());
                        spent_txos
                            .entry(vin_txid_hex)
                            .or_default()
                            .push(vin.get_vout());
                    }
                }
            }
        }

        utxo
    }

    /// Create an iterator over all blocks
    pub fn iterator(&self) -> BlockchainIterator {
        BlockchainIterator::new(self.get_tip_hash(), self.db.clone())
    }
}

/// Iterator for traversing the blockchain from tip to genesis
pub struct BlockchainIterator {
    db: Db,
    current_hash: String,
}

impl BlockchainIterator {
    fn new(tip_hash: String, db: Db) -> Self {
        BlockchainIterator {
            current_hash: tip_hash,
            db,
        }
    }

    /// Get the next block (moving towards genesis)
    pub fn next(&mut self) -> Option<Block> {
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        let data = blocks_tree.get(&self.current_hash).unwrap()?;
        let block = Block::deserialize(&data);
        self.current_hash = block.get_prev_hash().to_string();
        Some(block)
    }
}

#[cfg(test)]
mod tests {
    // Tests would require database setup
    // Placeholder for now
}
