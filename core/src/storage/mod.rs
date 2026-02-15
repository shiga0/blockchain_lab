//! Storage layer
//!
//! This module handles persistent storage of blockchain data.
//!
//! ## Storage backends:
//! - **sled** - Embedded key-value store (this implementation)
//! - **LevelDB** - Used by Bitcoin Core, go-ethereum
//! - **RocksDB** - Used by Kaspa, higher performance
//!
//! ## Data organization:
//! ```text
//! ┌─────────────────────────────────────────┐
//! │              Database                   │
//! ├─────────────────────────────────────────┤
//! │  blocks/                                │
//! │    <block_hash> -> Block                │
//! │    tip_block_hash -> <hash>             │
//! ├─────────────────────────────────────────┤
//! │  chainstate/ (UTXO)                     │
//! │    <txid> -> Vec<TXOutput>              │
//! └─────────────────────────────────────────┘
//! ```

use sled::Db;
use std::env::current_dir;
use std::path::PathBuf;

/// Default data directory name
const DEFAULT_DATA_DIR: &str = "data";

/// Database wrapper for blockchain storage
pub struct Storage {
    db: Db,
    path: PathBuf,
}

impl Storage {
    /// Open storage at the default location
    pub fn open_default() -> Self {
        let path = current_dir().unwrap().join(DEFAULT_DATA_DIR);
        Self::open(&path)
    }

    /// Open storage at a specific path
    pub fn open(path: &PathBuf) -> Self {
        let db = sled::open(path).expect("Failed to open database");
        Storage {
            db,
            path: path.clone(),
        }
    }

    /// Get the underlying database
    pub fn get_db(&self) -> Db {
        self.db.clone()
    }

    /// Get the storage path
    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }

    /// Clear all data (use with caution!)
    pub fn clear(&self) {
        self.db.clear().expect("Failed to clear database");
    }

    /// Check if storage is empty
    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }

    /// Flush to disk
    pub fn flush(&self) {
        self.db.flush().expect("Failed to flush database");
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        let _ = self.db.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_storage_creation() {
        let temp_path = PathBuf::from("/tmp/blockchain_test_storage");
        let _ = fs::remove_dir_all(&temp_path);

        let storage = Storage::open(&temp_path);
        assert!(storage.is_empty());

        let _ = fs::remove_dir_all(&temp_path);
    }
}
