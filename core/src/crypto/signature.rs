//! Digital signature operations
//!
//! This module provides ECDSA signature generation and verification.
//!
//! ## Comparison with other blockchains:
//! - **Bitcoin**: ECDSA with secp256k1 curve, also Schnorr (Taproot)
//! - **Ethereum**: ECDSA with secp256k1 curve
//! - **Solana**: Ed25519 (EdDSA)
//! - **This implementation**: ECDSA with P256 curve

use ring::rand::SystemRandom;
use ring::signature::{
    EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_FIXED, ECDSA_P256_SHA256_FIXED_SIGNING,
};

/// Generate a new ECDSA key pair
///
/// Returns the private key in PKCS#8 format
pub fn generate_keypair() -> Vec<u8> {
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
    pkcs8.as_ref().to_vec()
}

/// Extract public key from PKCS#8 private key
pub fn public_key_from_pkcs8(pkcs8: &[u8]) -> Vec<u8> {
    let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8).unwrap();
    key_pair.public_key().as_ref().to_vec()
}

/// Sign a message using ECDSA
///
/// # Arguments
/// * `pkcs8` - Private key in PKCS#8 format
/// * `message` - Message to sign
///
/// # Returns
/// Digital signature bytes
pub fn sign(pkcs8: &[u8], message: &[u8]) -> Vec<u8> {
    let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8).unwrap();
    let rng = SystemRandom::new();
    key_pair.sign(&rng, message).unwrap().as_ref().to_vec()
}

/// Verify an ECDSA signature
///
/// # Arguments
/// * `public_key` - Public key bytes
/// * `signature` - Signature to verify
/// * `message` - Original message
///
/// # Returns
/// `true` if signature is valid, `false` otherwise
pub fn verify(public_key: &[u8], signature: &[u8], message: &[u8]) -> bool {
    let peer_public_key = ring::signature::UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, public_key);
    peer_public_key.verify(message, signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let pkcs8 = generate_keypair();
        assert!(!pkcs8.is_empty());

        // Each call should generate different key
        let pkcs8_2 = generate_keypair();
        assert_ne!(pkcs8, pkcs8_2);
    }

    #[test]
    fn test_public_key_extraction() {
        let pkcs8 = generate_keypair();
        let public_key = public_key_from_pkcs8(&pkcs8);
        assert!(!public_key.is_empty());
    }

    #[test]
    fn test_sign_and_verify() {
        let pkcs8 = generate_keypair();
        let public_key = public_key_from_pkcs8(&pkcs8);

        let message = b"test message";
        let signature = sign(&pkcs8, message);

        // Valid signature should verify
        assert!(verify(&public_key, &signature, message));

        // Wrong message should fail
        assert!(!verify(&public_key, &signature, b"wrong message"));
    }

    #[test]
    fn test_different_keys_different_signatures() {
        let pkcs8_1 = generate_keypair();
        let pkcs8_2 = generate_keypair();

        let message = b"same message";
        let sig1 = sign(&pkcs8_1, message);
        let sig2 = sign(&pkcs8_2, message);

        // Different keys produce different signatures
        assert_ne!(sig1, sig2);
    }
}
