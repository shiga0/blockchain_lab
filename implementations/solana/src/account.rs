//! Solana Account Module
//!
//! ## Account Model Comparison
//!
//! | Aspect | Bitcoin (UTXO) | Ethereum (Account) | Solana (Account+Owner) |
//! |--------|---------------|-------------------|------------------------|
//! | State | Implicit (UTXO set) | balance + nonce | lamports + data + owner |
//! | Ownership | Script-based | Private key | Program-based |
//! | Data Storage | None | Contract storage | Account data field |
//! | Parallelism | High (no shared state) | Low (sequential) | High (declared access) |
//!
//! ## Solana Account Model
//!
//! Every piece of state in Solana is an Account:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                    Account                          │
//! ├─────────────────────────────────────────────────────┤
//! │ pubkey: Pubkey         (32 bytes, address)         │
//! │ lamports: u64          (balance in lamports)       │
//! │ data: Vec<u8>          (arbitrary program data)    │
//! │ owner: Pubkey          (program that owns this)    │
//! │ executable: bool       (is this account a program?)│
//! │ rent_epoch: u64        (epoch for rent collection) │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Ownership Model
//!
//! - Only the owner program can modify account data
//! - Anyone can credit lamports, only owner can debit
//! - System Program owns all "wallet" accounts
//! - Program accounts are owned by BPF Loader
//!
//! ```text
//! System Program (11111111111111111111111111111111)
//!        │
//!        ├── owns → User Wallet A
//!        ├── owns → User Wallet B
//!        └── owns → User Wallet C
//!
//! BPF Loader (BPFLoaderUpgradeab1e11111111111111111)
//!        │
//!        ├── owns → Token Program (executable)
//!        └── owns → My DeFi App (executable)
//!
//! Token Program (TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA)
//!        │
//!        ├── owns → Token Mint Account
//!        ├── owns → User A Token Account
//!        └── owns → User B Token Account
//! ```

use crate::constants::LAMPORTS_PER_SOL;

/// 32-byte public key (Ed25519)
pub type Pubkey = [u8; 32];

/// System Program address (all 1s in base58 → all 0s in bytes for simplicity here)
pub const SYSTEM_PROGRAM_ID: Pubkey = [0u8; 32];

/// Solana Account
///
/// Reference: solana/sdk/src/account.rs
#[derive(Debug, Clone)]
pub struct Account {
    /// Balance in lamports (1 SOL = 1e9 lamports)
    pub lamports: u64,
    /// Account data (program state)
    pub data: Vec<u8>,
    /// Program that owns this account
    pub owner: Pubkey,
    /// Is this account executable (a program)?
    pub executable: bool,
    /// Epoch at which this account will owe rent
    pub rent_epoch: u64,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            lamports: 0,
            data: Vec::new(),
            owner: SYSTEM_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        }
    }
}

impl Account {
    /// Create a new account with given lamports, owned by System Program
    pub fn new(lamports: u64) -> Self {
        Self {
            lamports,
            ..Default::default()
        }
    }

    /// Create a new account with data, owned by specified program
    pub fn new_data(lamports: u64, data: Vec<u8>, owner: Pubkey) -> Self {
        Self {
            lamports,
            data,
            owner,
            executable: false,
            rent_epoch: 0,
        }
    }

    /// Create an executable account (program)
    pub fn new_executable(lamports: u64, data: Vec<u8>, owner: Pubkey) -> Self {
        Self {
            lamports,
            data,
            owner,
            executable: true,
            rent_epoch: 0,
        }
    }

    /// Get balance in SOL
    pub fn balance_sol(&self) -> f64 {
        self.lamports as f64 / LAMPORTS_PER_SOL as f64
    }

    /// Check if this account is owned by the given program
    pub fn is_owned_by(&self, program_id: &Pubkey) -> bool {
        &self.owner == program_id
    }

    /// Check if this account is a signer account (wallet)
    pub fn is_signer_account(&self) -> bool {
        self.is_owned_by(&SYSTEM_PROGRAM_ID) && !self.executable
    }

    /// Calculate rent-exempt minimum balance
    ///
    /// Accounts must maintain a minimum balance to avoid rent collection.
    /// Formula: (data_len + 128) * rent_per_byte_year * 2 years
    pub fn rent_exempt_minimum(data_len: usize) -> u64 {
        // Simplified: ~0.00089088 SOL per byte for 2 years
        // Real formula uses epochs and rates
        let bytes = data_len as u64 + 128; // 128 bytes for account metadata
        bytes * 6960 // ~6960 lamports per byte for rent exemption
    }

    /// Check if account is rent exempt
    pub fn is_rent_exempt(&self) -> bool {
        self.lamports >= Self::rent_exempt_minimum(self.data.len())
    }
}

/// Account metadata for transaction processing
///
/// Used by the runtime to track account access during execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMeta {
    /// Account public key
    pub pubkey: Pubkey,
    /// Is this account a signer?
    pub is_signer: bool,
    /// Is this account writable?
    pub is_writable: bool,
}

impl AccountMeta {
    /// Create a writable, non-signer account meta
    pub fn new(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: true,
        }
    }

    /// Create a read-only account meta
    pub fn new_readonly(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: false,
        }
    }
}

/// Account storage (simplified)
///
/// In real Solana, this is a complex structure with:
/// - Append-only storage
/// - Account index
/// - Bank snapshots
#[derive(Debug, Default)]
pub struct AccountStore {
    accounts: std::collections::HashMap<Pubkey, Account>,
}

impl AccountStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get an account by pubkey
    pub fn get(&self, pubkey: &Pubkey) -> Option<&Account> {
        self.accounts.get(pubkey)
    }

    /// Get a mutable reference to an account
    pub fn get_mut(&mut self, pubkey: &Pubkey) -> Option<&mut Account> {
        self.accounts.get_mut(pubkey)
    }

    /// Store an account
    pub fn store(&mut self, pubkey: Pubkey, account: Account) {
        self.accounts.insert(pubkey, account);
    }

    /// Transfer lamports between accounts
    pub fn transfer(
        &mut self,
        from: &Pubkey,
        to: &Pubkey,
        lamports: u64,
    ) -> Result<(), &'static str> {
        // Check source has enough lamports
        let from_account = self.accounts.get_mut(from).ok_or("source not found")?;
        if from_account.lamports < lamports {
            return Err("insufficient lamports");
        }
        from_account.lamports -= lamports;

        // Credit destination (create if doesn't exist)
        let to_account = self.accounts.entry(*to).or_insert_with(Account::default);
        to_account.lamports += lamports;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_balance() {
        let account = Account::new(1_000_000_000); // 1 SOL
        assert_eq!(account.balance_sol(), 1.0);
    }

    #[test]
    fn test_account_ownership() {
        let mut account = Account::new(100);
        assert!(account.is_owned_by(&SYSTEM_PROGRAM_ID));

        let custom_program: Pubkey = [1u8; 32];
        account.owner = custom_program;
        assert!(!account.is_owned_by(&SYSTEM_PROGRAM_ID));
        assert!(account.is_owned_by(&custom_program));
    }

    #[test]
    fn test_rent_exempt() {
        // Empty account (0 data bytes)
        let minimum = Account::rent_exempt_minimum(0);
        assert!(minimum > 0);

        let account = Account::new(minimum);
        assert!(account.is_rent_exempt());

        let poor_account = Account::new(1);
        assert!(!poor_account.is_rent_exempt());
    }

    #[test]
    fn test_account_transfer() {
        let mut store = AccountStore::new();

        let alice: Pubkey = [1u8; 32];
        let bob: Pubkey = [2u8; 32];

        store.store(alice, Account::new(1000));
        store.store(bob, Account::new(500));

        store.transfer(&alice, &bob, 300).unwrap();

        assert_eq!(store.get(&alice).unwrap().lamports, 700);
        assert_eq!(store.get(&bob).unwrap().lamports, 800);
    }
}
