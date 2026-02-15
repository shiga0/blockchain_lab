//! Execution layer
//!
//! This module handles transaction processing, mempool management,
//! and UTXO set tracking.
//!
//! ## Architecture:
//! ```text
//! ┌──────────────────────────────────────────┐
//! │           Transaction Pool               │
//! │  (pending transactions awaiting mining)  │
//! └──────────────────┬───────────────────────┘
//!                    │
//!                    ▼
//! ┌──────────────────────────────────────────┐
//! │        Transaction Validation            │
//! │  (signature verification, UTXO checks)   │
//! └──────────────────┬───────────────────────┘
//!                    │
//!                    ▼
//! ┌──────────────────────────────────────────┐
//! │             UTXO Set                     │
//! │  (unspent outputs, balance tracking)     │
//! └──────────────────────────────────────────┘
//! ```
//!
//! ## UTXO Model (This implementation)
//! - Transactions consume existing outputs and create new ones
//! - Each output can only be spent once
//! - Privacy: each transaction can use new addresses
//!
//! ## Account Model (Ethereum style)
//! - Accounts have balances stored in state
//! - Transactions modify account balances directly
//! - Simpler to reason about but less privacy

pub mod transaction;
pub mod mempool;
pub mod utxo;

// Re-export commonly used types
pub use transaction::{Transaction, TXInput, TXOutput, SUBSIDY};
pub use mempool::{MemoryPool, BlockInTransit};
pub use utxo::UTXOSet;
