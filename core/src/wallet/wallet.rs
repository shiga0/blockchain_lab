//! Wallet implementation
//!
//! A wallet manages keypairs and generates addresses for receiving funds.
//!
//! ## Wallet types comparison:
//! - **Bitcoin**: HD wallets (BIP32/39/44), hardware wallets
//! - **Ethereum**: EOA (Externally Owned Accounts), smart contract wallets
//! - **This implementation**: Simple keypair-based wallet

use crate::crypto::{
    generate_keypair, public_key_from_pkcs8, public_key_to_address, hash160,
};
use serde::{Deserialize, Serialize};

/// A single wallet (keypair + address)
#[derive(Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// Private key in PKCS#8 format
    pkcs8: Vec<u8>,
    /// Public key
    public_key: Vec<u8>,
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}

impl Wallet {
    /// Create a new wallet with a fresh keypair
    pub fn new() -> Self {
        let pkcs8 = generate_keypair();
        let public_key = public_key_from_pkcs8(&pkcs8);
        Wallet { pkcs8, public_key }
    }

    /// Get the wallet's address
    pub fn get_address(&self) -> String {
        public_key_to_address(&self.public_key)
    }

    /// Get the public key
    pub fn get_public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// Get the private key (PKCS#8 format)
    pub fn get_pkcs8(&self) -> &[u8] {
        &self.pkcs8
    }

    /// Get the public key hash
    pub fn get_public_key_hash(&self) -> Vec<u8> {
        hash160(&self.public_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::validate_address;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new();
        assert!(!wallet.get_public_key().is_empty());
        assert!(!wallet.get_pkcs8().is_empty());
    }

    #[test]
    fn test_wallet_address() {
        let wallet = Wallet::new();
        let address = wallet.get_address();
        assert!(!address.is_empty());
        assert!(validate_address(&address));
    }

    #[test]
    fn test_unique_wallets() {
        let w1 = Wallet::new();
        let w2 = Wallet::new();
        assert_ne!(w1.get_address(), w2.get_address());
    }

    #[test]
    fn test_public_key_hash() {
        let wallet = Wallet::new();
        let hash = wallet.get_public_key_hash();
        assert_eq!(hash.len(), 20); // RIPEMD160 output
    }
}
