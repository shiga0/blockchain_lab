//! Blockchain address generation and validation
//!
//! This module handles address encoding/decoding using Base58Check.
//!
//! ## Address format:
//! ```text
//! [version (1 byte)] + [public_key_hash (20 bytes)] + [checksum (4 bytes)]
//! ```
//!
//! ## Comparison with other blockchains:
//! - **Bitcoin**: Base58Check (legacy), Bech32 (SegWit)
//! - **Ethereum**: Hex with EIP-55 checksum
//! - **Kaspa**: Bech32m
//! - **This implementation**: Base58Check (Bitcoin legacy style)

use crate::crypto::hash::{hash160, double_sha256};

/// Address version byte (mainnet P2PKH)
const VERSION: u8 = 0x00;

/// Checksum length in bytes
pub const CHECKSUM_LEN: usize = 4;

/// Encode bytes to Base58
pub fn base58_encode(data: &[u8]) -> String {
    bs58::encode(data).into_string()
}

/// Decode Base58 string to bytes
pub fn base58_decode(data: &str) -> Result<Vec<u8>, &'static str> {
    bs58::decode(data)
        .into_vec()
        .map_err(|_| "Invalid Base58 string")
}

/// Generate checksum for address data
fn checksum(payload: &[u8]) -> Vec<u8> {
    double_sha256(payload)[..CHECKSUM_LEN].to_vec()
}

/// Generate blockchain address from public key
///
/// Process:
/// 1. Hash160(public_key) -> public_key_hash
/// 2. Prepend version byte
/// 3. Append checksum
/// 4. Base58 encode
pub fn public_key_to_address(public_key: &[u8]) -> String {
    let pub_key_hash = hash160(public_key);
    pub_key_hash_to_address(&pub_key_hash)
}

/// Generate address from public key hash
pub fn pub_key_hash_to_address(pub_key_hash: &[u8]) -> String {
    let mut payload = vec![VERSION];
    payload.extend(pub_key_hash);
    let checksum = checksum(&payload);
    payload.extend(&checksum);
    base58_encode(&payload)
}

/// Extract public key hash from address
pub fn address_to_pub_key_hash(address: &str) -> Result<Vec<u8>, &'static str> {
    let payload = base58_decode(address)?;
    if payload.len() < 1 + CHECKSUM_LEN {
        return Err("Address too short");
    }
    Ok(payload[1..payload.len() - CHECKSUM_LEN].to_vec())
}

/// Validate a blockchain address
///
/// Checks:
/// 1. Valid Base58 encoding
/// 2. Correct length
/// 3. Valid checksum
pub fn validate_address(address: &str) -> bool {
    let payload = match base58_decode(address) {
        Ok(p) => p,
        Err(_) => return false,
    };

    if payload.len() < 1 + CHECKSUM_LEN {
        return false;
    }

    let actual_checksum = &payload[payload.len() - CHECKSUM_LEN..];
    let data = &payload[..payload.len() - CHECKSUM_LEN];
    let expected_checksum = checksum(data);

    actual_checksum == expected_checksum.as_slice()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::signature::{generate_keypair, public_key_from_pkcs8};

    #[test]
    fn test_base58_roundtrip() {
        let original = b"hello world";
        let encoded = base58_encode(original);
        let decoded = base58_decode(&encoded).unwrap();
        assert_eq!(original.to_vec(), decoded);
    }

    #[test]
    fn test_address_generation() {
        let pkcs8 = generate_keypair();
        let public_key = public_key_from_pkcs8(&pkcs8);
        let address = public_key_to_address(&public_key);

        assert!(!address.is_empty());
        assert!(validate_address(&address));
    }

    #[test]
    fn test_unique_addresses() {
        let pk1 = public_key_from_pkcs8(&generate_keypair());
        let pk2 = public_key_from_pkcs8(&generate_keypair());

        let addr1 = public_key_to_address(&pk1);
        let addr2 = public_key_to_address(&pk2);

        assert_ne!(addr1, addr2);
    }

    #[test]
    fn test_invalid_address() {
        // Corrupt a valid address
        let pkcs8 = generate_keypair();
        let public_key = public_key_from_pkcs8(&pkcs8);
        let mut address = public_key_to_address(&public_key);

        // Modify last character
        let last_char = address.pop().unwrap();
        let new_char = if last_char == 'A' { 'B' } else { 'A' };
        address.push(new_char);

        assert!(!validate_address(&address));
    }

    #[test]
    fn test_pub_key_hash_roundtrip() {
        let pkcs8 = generate_keypair();
        let public_key = public_key_from_pkcs8(&pkcs8);
        let original_hash = hash160(&public_key);

        let address = pub_key_hash_to_address(&original_hash);
        let extracted_hash = address_to_pub_key_hash(&address).unwrap();

        assert_eq!(original_hash, extracted_hash);
    }
}
