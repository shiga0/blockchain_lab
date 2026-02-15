//! UTXO (Unspent Transaction Output) Set
//!
//! The UTXO set tracks all unspent outputs in the blockchain.
//! This is crucial for validating transactions and calculating balances.
//!
//! ## UTXO vs Account Model:
//!
//! | Aspect | UTXO (Bitcoin) | Account (Ethereum) |
//! |--------|----------------|-------------------|
//! | State | Set of outputs | Account balances |
//! | Privacy | Better (new addresses) | Worse (reuse) |
//! | Parallelism | Good (independent) | Limited (nonces) |
//! | Complexity | Higher | Lower |
//!
//! ## Comparison:
//! - **Bitcoin**: UTXO model
//! - **Ethereum**: Account model
//! - **Kaspa**: UTXO model
//! - **Solana**: Account model
//! - **This implementation**: UTXO model

use crate::execution::transaction::TXOutput;
use crate::primitives::Block;
use data_encoding::HEXLOWER;
use sled::{Db, Tree};
use std::collections::HashMap;

/// Tree name for UTXO storage
const UTXO_TREE: &str = "chainstate";

/// UTXO Set - tracks all unspent transaction outputs
pub struct UTXOSet {
    db: Db,
}

impl UTXOSet {
    /// Create a new UTXO set with the given database
    pub fn new(db: Db) -> Self {
        UTXOSet { db }
    }

    /// Get the UTXO tree from the database
    fn get_tree(&self) -> Tree {
        self.db.open_tree(UTXO_TREE).unwrap()
    }

    /// Find spendable outputs for a given public key hash
    ///
    /// Returns (accumulated_value, map of txid_hex -> output_indices)
    pub fn find_spendable_outputs(
        &self,
        pub_key_hash: &[u8],
        amount: i32,
    ) -> (i32, HashMap<String, Vec<usize>>) {
        let mut unspent_outputs: HashMap<String, Vec<usize>> = HashMap::new();
        let mut accumulated = 0;
        let utxo_tree = self.get_tree();

        for item in utxo_tree.iter() {
            let (k, v) = item.unwrap();
            let txid_hex = HEXLOWER.encode(&k);
            let outs: Vec<TXOutput> = bincode::deserialize(&v).unwrap();

            for (idx, out) in outs.iter().enumerate() {
                if out.is_locked_with_key(pub_key_hash) && accumulated < amount {
                    accumulated += out.get_value();
                    unspent_outputs
                        .entry(txid_hex.clone())
                        .or_default()
                        .push(idx);
                }
            }
        }

        (accumulated, unspent_outputs)
    }

    /// Find all UTXOs for a public key hash
    pub fn find_utxo(&self, pub_key_hash: &[u8]) -> Vec<TXOutput> {
        let utxo_tree = self.get_tree();
        let mut utxos = vec![];

        for item in utxo_tree.iter() {
            let (_, v) = item.unwrap();
            let outs: Vec<TXOutput> = bincode::deserialize(&v).unwrap();

            for out in outs {
                if out.is_locked_with_key(pub_key_hash) {
                    utxos.push(out);
                }
            }
        }

        utxos
    }

    /// Count total number of UTXO entries (transactions with unspent outputs)
    pub fn count_transactions(&self) -> i32 {
        self.get_tree().iter().count() as i32
    }

    /// Rebuild the UTXO set from the blockchain
    ///
    /// This scans all blocks and rebuilds the UTXO set from scratch.
    pub fn reindex(&self, utxo_map: &HashMap<String, Vec<TXOutput>>) {
        let utxo_tree = self.get_tree();
        utxo_tree.clear().unwrap();

        for (txid_hex, outs) in utxo_map {
            let txid = HEXLOWER.decode(txid_hex.as_bytes()).unwrap();
            let value = bincode::serialize(outs).unwrap();
            utxo_tree.insert(txid, value).unwrap();
        }
    }

    /// Update the UTXO set with a new block
    ///
    /// - Remove spent outputs
    /// - Add new outputs
    pub fn update(&self, block: &Block) {
        let utxo_tree = self.get_tree();

        for tx in block.get_transactions() {
            // Remove spent outputs (skip for coinbase)
            if !tx.is_coinbase() {
                for vin in tx.get_vin() {
                    let outs_bytes = utxo_tree.get(vin.get_txid()).unwrap();
                    if let Some(bytes) = outs_bytes {
                        let outs: Vec<TXOutput> = bincode::deserialize(&bytes).unwrap();
                        let updated_outs: Vec<TXOutput> = outs
                            .into_iter()
                            .enumerate()
                            .filter(|(idx, _)| *idx != vin.get_vout())
                            .map(|(_, out)| out)
                            .collect();

                        if updated_outs.is_empty() {
                            utxo_tree.remove(vin.get_txid()).unwrap();
                        } else {
                            let bytes = bincode::serialize(&updated_outs).unwrap();
                            utxo_tree.insert(vin.get_txid(), bytes).unwrap();
                        }
                    }
                }
            }

            // Add new outputs
            let new_outputs: Vec<TXOutput> = tx.get_vout().to_vec();
            let bytes = bincode::serialize(&new_outputs).unwrap();
            utxo_tree.insert(tx.get_id(), bytes).unwrap();
        }
    }

    /// Get outputs for a specific transaction
    pub fn get_outputs(&self, txid: &[u8]) -> Option<Vec<TXOutput>> {
        let utxo_tree = self.get_tree();
        utxo_tree
            .get(txid)
            .unwrap()
            .map(|bytes| bincode::deserialize(&bytes).unwrap())
    }
}

#[cfg(test)]
mod tests {
    // Tests would require a test database setup
    // Placeholder for now
}
