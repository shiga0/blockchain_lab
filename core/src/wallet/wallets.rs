//! Wallet collection and persistence
//!
//! This module manages multiple wallets and persists them to disk.

use crate::wallet::Wallet;
use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;

/// Default wallet file name
pub const WALLET_FILE: &str = "wallet.dat";

/// Collection of wallets
pub struct Wallets {
    /// Map of address -> Wallet
    wallets: HashMap<String, Wallet>,
    /// File path for persistence
    file_path: PathBuf,
}

impl Default for Wallets {
    fn default() -> Self {
        Self::new()
    }
}

impl Wallets {
    /// Create/load wallets from the default location
    pub fn new() -> Self {
        let file_path = current_dir().unwrap().join(WALLET_FILE);
        let mut wallets = Wallets {
            wallets: HashMap::new(),
            file_path,
        };
        wallets.load_from_file();
        wallets
    }

    /// Create/load wallets from a specific file
    pub fn from_file(file_path: PathBuf) -> Self {
        let mut wallets = Wallets {
            wallets: HashMap::new(),
            file_path,
        };
        wallets.load_from_file();
        wallets
    }

    /// Create a new wallet and return its address
    pub fn create_wallet(&mut self) -> String {
        let wallet = Wallet::new();
        let address = wallet.get_address();
        self.wallets.insert(address.clone(), wallet);
        self.save_to_file();
        address
    }

    /// Get all wallet addresses
    pub fn get_addresses(&self) -> Vec<String> {
        self.wallets.keys().cloned().collect()
    }

    /// Get a wallet by address
    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        self.wallets.get(address)
    }

    /// Load wallets from file
    fn load_from_file(&mut self) {
        if !self.file_path.exists() {
            return;
        }

        let mut file = match File::open(&self.file_path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let metadata = match file.metadata() {
            Ok(m) => m,
            Err(_) => return,
        };

        let mut buf = vec![0; metadata.len() as usize];
        if file.read(&mut buf).is_err() {
            return;
        }

        if let Ok(wallets) = bincode::deserialize(&buf) {
            self.wallets = wallets;
        }
    }

    /// Save wallets to file
    fn save_to_file(&self) {
        let file = match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file_path)
        {
            Ok(f) => f,
            Err(_) => return,
        };

        let mut writer = BufWriter::new(file);
        if let Ok(bytes) = bincode::serialize(&self.wallets) {
            let _ = writer.write_all(&bytes);
            let _ = writer.flush();
        }
    }

    /// Get number of wallets
    pub fn len(&self) -> usize {
        self.wallets.len()
    }

    /// Check if there are no wallets
    pub fn is_empty(&self) -> bool {
        self.wallets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_wallet() {
        let temp_path = PathBuf::from("/tmp/test_wallets.dat");
        let _ = fs::remove_file(&temp_path);

        let mut wallets = Wallets::from_file(temp_path.clone());
        let addr = wallets.create_wallet();

        assert!(!addr.is_empty());
        assert_eq!(wallets.len(), 1);
        assert!(wallets.get_wallet(&addr).is_some());

        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn test_persistence() {
        let temp_path = PathBuf::from("/tmp/test_wallets_persist.dat");
        let _ = fs::remove_file(&temp_path);

        // Create and save wallet
        let addr = {
            let mut wallets = Wallets::from_file(temp_path.clone());
            wallets.create_wallet()
        };

        // Load and verify
        let wallets = Wallets::from_file(temp_path.clone());
        assert!(wallets.get_wallet(&addr).is_some());

        let _ = fs::remove_file(&temp_path);
    }
}
