//! Ethereum State Module (Account Model)
//!
//! ## Account Model vs UTXO
//!
//! ### UTXO (Bitcoin/Core)
//! - State is implicit (set of unspent outputs)
//! - Transactions consume outputs, create new ones
//! - Parallelizable (no shared state)
//!
//! ### Account Model (Ethereum)
//! - State is explicit (account → balance mapping)
//! - Transactions modify account state directly
//! - Simpler for smart contracts
//!
//! ## Account Types
//!
//! 1. **EOA (Externally Owned Account)**: Controlled by private key
//! 2. **Contract Account**: Controlled by code
//!
//! ## State Storage
//!
//! Ethereum uses Merkle Patricia Trie for:
//! - Account state (balance, nonce, code hash, storage root)
//! - Contract storage (key-value pairs)
//!
//! ## TODO
//!
//! - [ ] Account structure
//! - [ ] Merkle Patricia Trie
//! - [ ] State transitions
//! - [ ] Storage slots

use std::collections::HashMap;

/// Ethereum account
#[derive(Debug, Clone, Default)]
pub struct Account {
    /// Account nonce (transaction count for EOA, creation count for contract)
    pub nonce: u64,
    /// Balance in wei
    pub balance: u128,
    /// Contract code hash (empty for EOA)
    pub code_hash: Vec<u8>,
    /// Storage root (Merkle Patricia Trie root)
    pub storage_root: Vec<u8>,
}

/// World state (all accounts)
#[derive(Debug, Default)]
pub struct WorldState {
    accounts: HashMap<Vec<u8>, Account>,
}

impl WorldState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get account by address
    pub fn get_account(&self, address: &[u8]) -> Option<&Account> {
        self.accounts.get(address)
    }

    /// Update account balance
    pub fn transfer(&mut self, from: &[u8], to: &[u8], amount: u128) -> Result<(), &'static str> {
        let from_account = self.accounts.get_mut(from).ok_or("sender not found")?;
        if from_account.balance < amount {
            return Err("insufficient balance");
        }
        from_account.balance -= amount;

        let to_account = self.accounts.entry(to.to_vec()).or_default();
        to_account.balance += amount;

        Ok(())
    }
}
