//! Hash functions for blockchain operations
//!
//! This module provides cryptographic hash functions used throughout the blockchain.
//!
//! ## Comparison with other blockchains:
//! - **Bitcoin**: Uses SHA256 (double SHA256 for block hashes)
//! - **Ethereum**: Uses Keccak256
//! - **Kaspa**: Uses BLAKE2b
//! - **This implementation**: Uses SHA256 (Bitcoin-style)

use digest::Digest;
use ring::digest::{Context, SHA256};
use ripemd::Ripemd160;

/// Compute SHA256 hash of input data
///
/// Used for:
/// - Transaction IDs
/// - Block hashes
/// - Merkle tree nodes
pub fn sha256(data: &[u8]) -> Vec<u8> {
    let mut context = Context::new(&SHA256);
    context.update(data);
    context.finish().as_ref().to_vec()
}

/// Compute double SHA256 hash (SHA256(SHA256(data)))
///
/// Used in Bitcoin for:
/// - Block header hashing
/// - Address checksum
pub fn double_sha256(data: &[u8]) -> Vec<u8> {
    sha256(&sha256(data))
}

/// Compute RIPEMD160 hash
///
/// Used with SHA256 for address generation:
/// RIPEMD160(SHA256(public_key)) = public key hash
pub fn ripemd160(data: &[u8]) -> Vec<u8> {
    let mut hasher = Ripemd160::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Compute Hash160: RIPEMD160(SHA256(data))
///
/// Standard Bitcoin address hashing function
pub fn hash160(data: &[u8]) -> Vec<u8> {
    ripemd160(&sha256(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let data = b"hello";
        let hash = sha256(data);
        assert_eq!(hash.len(), 32);

        // Same input should produce same hash
        let hash2 = sha256(data);
        assert_eq!(hash, hash2);

        // Different input should produce different hash
        let hash3 = sha256(b"world");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_double_sha256() {
        let data = b"test";
        let hash = double_sha256(data);
        assert_eq!(hash.len(), 32);

        // Should be different from single sha256
        let single = sha256(data);
        assert_ne!(hash, single);
    }

    #[test]
    fn test_ripemd160() {
        let data = b"hello";
        let hash = ripemd160(data);
        assert_eq!(hash.len(), 20);
    }

    #[test]
    fn test_hash160() {
        let data = b"public_key_data";
        let hash = hash160(data);
        assert_eq!(hash.len(), 20);
    }
}
