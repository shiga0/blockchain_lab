//! Wallet layer
//!
//! This module handles key management and wallet operations.
//!
//! ## Wallet types:
//!
//! | Type | Description | Used by |
//! |------|-------------|---------|
//! | Simple | Single keypair | This implementation |
//! | HD (BIP32) | Hierarchical deterministic | Bitcoin, most wallets |
//! | Multi-sig | Multiple signatures required | Bitcoin, Ethereum |
//! | Smart Contract | Code-based wallet | Ethereum |
//!
//! ## Future improvements:
//! - BIP32/39/44 support for HD wallets
//! - Hardware wallet integration
//! - Multi-signature support

pub mod wallet;
pub mod wallets;

pub use wallet::Wallet;
pub use wallets::{Wallets, WALLET_FILE};
