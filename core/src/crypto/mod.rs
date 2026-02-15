//! Cryptographic primitives layer
//!
//! This module contains all cryptographic operations used in the blockchain.
//!
//! ## Modules:
//! - `hash` - Hash functions (SHA256, RIPEMD160, Hash160)
//! - `signature` - Digital signatures (ECDSA)
//! - `address` - Address generation and validation (Base58Check)
//! - `merkle` - Merkle tree for transaction verification
//!
//! ## Comparison with other blockchains:
//!
//! | Component | Bitcoin | Ethereum | Kaspa | This |
//! |-----------|---------|----------|-------|------|
//! | Hash | SHA256 | Keccak256 | BLAKE2b | SHA256 |
//! | Signature | ECDSA/Schnorr | ECDSA | ECDSA | ECDSA |
//! | Curve | secp256k1 | secp256k1 | secp256k1 | P256 |
//! | Address | Base58/Bech32 | Hex+EIP55 | Bech32m | Base58 |

pub mod hash;
pub mod signature;
pub mod address;
pub mod merkle;

// Re-export commonly used functions
pub use hash::{sha256, double_sha256, ripemd160, hash160};
pub use signature::{generate_keypair, public_key_from_pkcs8, sign, verify};
pub use address::{
    base58_encode, base58_decode, public_key_to_address, pub_key_hash_to_address,
    address_to_pub_key_hash, validate_address, CHECKSUM_LEN,
};
pub use merkle::{compute_merkle_root, generate_merkle_proof, verify_merkle_proof, MerkleProof};
