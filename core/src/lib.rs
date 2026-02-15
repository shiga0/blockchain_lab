//! # Blockchain Base
//!
//! A reference blockchain implementation for comparing different blockchain architectures.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    API Layer                            │
//! │  (CLI commands, future: JSON-RPC)                       │
//! └─────────────────────────────────────────────────────────┘
//!                           │
//! ┌─────────────────────────┴───────────────────────────────┐
//! │                 Network Layer                           │
//! │  (P2P server, message protocol, node management)        │
//! └─────────────────────────────────────────────────────────┘
//!                           │
//! ┌─────────────────────────┴───────────────────────────────┐
//! │                Consensus Layer                          │
//! │  (Pluggable: PoW, future: PoS, GHOSTDAG)               │
//! └─────────────────────────────────────────────────────────┘
//!                           │
//! ┌─────────────────────────┴───────────────────────────────┐
//! │               Execution Layer                           │
//! │  (Transactions, Mempool, UTXO Set)                      │
//! └─────────────────────────────────────────────────────────┘
//!                           │
//! ┌─────────────────────────┴───────────────────────────────┐
//! │                Storage Layer                            │
//! │  (Sled database, block storage, state storage)          │
//! └─────────────────────────────────────────────────────────┘
//!                           │
//! ┌─────────────────────────┴───────────────────────────────┐
//! │             Cryptographic Layer                         │
//! │  (Hash, Signatures, Addresses, Merkle Trees)            │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Comparison with Major Blockchains
//!
//! | Aspect | Bitcoin | Ethereum | Kaspa | This |
//! |--------|---------|----------|-------|------|
//! | Consensus | PoW | PoS | GHOSTDAG | PoW (pluggable) |
//! | Data Model | UTXO | Account | UTXO | UTXO |
//! | Block Structure | Linear | Linear | DAG | Linear |
//! | Hash Function | SHA256 | Keccak256 | BLAKE2b | SHA256 |
//! | Signature | ECDSA/Schnorr | ECDSA | ECDSA | ECDSA |
//!
//! ## Module Overview
//!
//! - [`crypto`] - Cryptographic primitives
//! - [`consensus`] - Consensus mechanisms
//! - [`execution`] - Transaction processing
//! - [`primitives`] - Core data structures
//! - [`storage`] - Persistent storage
//! - [`network`] - P2P communication
//! - [`wallet`] - Key management
//! - [`api`] - External interfaces
//! - [`config`] - Configuration

// Core layers
pub mod crypto;
pub mod consensus;
pub mod execution;
pub mod primitives;
pub mod storage;
pub mod network;
pub mod wallet;
pub mod api;
pub mod config;

// Re-export commonly used types
pub use crypto::{
    sha256, double_sha256, hash160,
    generate_keypair, sign, verify,
    validate_address, public_key_to_address,
    compute_merkle_root, MerkleProof,
    CHECKSUM_LEN,
};

pub use consensus::{
    Consensus, ConsensusError, ProofOfWork, DEFAULT_TARGET_BITS,
    BlockValidator, ValidationError,
};

pub use execution::{
    Transaction, TXInput, TXOutput, SUBSIDY,
    MemoryPool, BlockInTransit, UTXOSet,
};

pub use primitives::{
    Block, Blockchain, BlockchainIterator, current_timestamp,
};

pub use storage::Storage;

pub use network::{
    Server, Node, Nodes, Message, OpType,
    send_tx, CENTRAL_NODE, NODE_VERSION,
};

pub use wallet::{Wallet, Wallets, WALLET_FILE};

pub use api::{Cli, Command};

pub use config::{Config, GLOBAL_CONFIG};
