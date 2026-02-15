//! # Stealth Address Implementation
//!
//! Stealth addresses provide receiver privacy by generating one-time
//! public keys for each transaction output.
//!
//! ## How It Works
//!
//! 1. Sender generates random `r` (transaction private key)
//! 2. Sender computes `R = r*G` (transaction public key)
//! 3. Sender computes one-time key: `P = Hs(r*A)*G + B`
//! 4. Receiver scans: `P' = Hs(a*R)*G + B`, if P' == P, output belongs to them
//!
//! Where:
//! - `(a, A)` = receiver's view key pair
//! - `(b, B)` = receiver's spend key pair
//! - `Hs` = hash to scalar

use crate::cryptonote::{AccountAddress, AccountKeys, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Transaction key pair (for sender)
#[derive(Clone, Debug)]
pub struct TxKeyPair {
    /// Transaction private key (r)
    pub secret_key: SecretKey,
    /// Transaction public key (R = r*G)
    pub public_key: PublicKey,
}

impl TxKeyPair {
    /// Generate new random transaction key pair
    pub fn generate() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let secret_key: SecretKey = rng.gen();
        let public_key = derive_public_key(&secret_key);

        Self {
            secret_key,
            public_key,
        }
    }

    /// Create from existing secret key
    pub fn from_secret(secret_key: SecretKey) -> Self {
        let public_key = derive_public_key(&secret_key);
        Self {
            secret_key,
            public_key,
        }
    }
}

/// Stealth address output
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StealthOutput {
    /// One-time public key (P)
    pub one_time_key: PublicKey,
    /// Transaction public key (R)
    pub tx_public_key: PublicKey,
    /// Output index
    pub output_index: usize,
    /// View tag (optional, for fast scanning)
    pub view_tag: Option<u8>,
}

impl StealthOutput {
    pub fn new(
        one_time_key: PublicKey,
        tx_public_key: PublicKey,
        output_index: usize,
    ) -> Self {
        Self {
            one_time_key,
            tx_public_key,
            output_index,
            view_tag: None,
        }
    }

    pub fn with_view_tag(mut self, view_tag: u8) -> Self {
        self.view_tag = Some(view_tag);
        self
    }
}

/// Key derivation result
#[derive(Clone, Debug)]
pub struct KeyDerivation {
    /// Shared secret (r*A or a*R)
    pub shared_secret: [u8; 32],
}

impl KeyDerivation {
    /// Create key derivation from shared secret computation
    /// Sender: r*A (tx_private_key * receiver_view_public_key)
    /// Receiver: a*R (view_secret_key * tx_public_key)
    ///
    /// In real EC: r*A = r*(a*G) = a*(r*G) = a*R (commutative property)
    /// We simulate this by computing: H(sorted(H(private) , public))
    /// This ensures both parties derive the same shared secret.
    pub fn generate(private_key: &SecretKey, public_key: &PublicKey) -> Self {
        // Compute "our point" from private key (same as derive_public_key)
        let our_point = derive_public_key(private_key);

        // Sort the two points to get a canonical order (simulates commutativity)
        let (first, second) = if our_point <= *public_key {
            (our_point, *public_key)
        } else {
            (*public_key, our_point)
        };

        // Hash the sorted points to get shared secret
        let mut hasher = Sha256::new();
        hasher.update(b"key_derivation");
        hasher.update(&first);
        hasher.update(&second);

        Self {
            shared_secret: hasher.finalize().into(),
        }
    }

    /// Derive scalar from derivation and output index
    /// s = Hs(derivation || output_index)
    pub fn derivation_to_scalar(&self, output_index: usize) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"derivation_to_scalar");
        hasher.update(&self.shared_secret);
        hasher.update((output_index as u64).to_le_bytes());
        hasher.finalize().into()
    }
}

/// Derive public key for specific output
/// P = Hs(derivation || output_index)*G + spend_public_key
pub fn derive_public_key(secret: &SecretKey) -> PublicKey {
    let mut hasher = Sha256::new();
    hasher.update(b"public_key");
    hasher.update(secret);
    hasher.finalize().into()
}

/// Generate one-time public key for output
/// P = Hs(r*A || i)*G + B
pub fn generate_one_time_public_key(
    tx_private_key: &SecretKey,
    receiver_address: &AccountAddress,
    output_index: usize,
) -> PublicKey {
    // 1. Compute key derivation: r*A
    let derivation = KeyDerivation::generate(tx_private_key, &receiver_address.view_public_key);

    // 2. Derive scalar: s = Hs(derivation || i)
    let scalar = derivation.derivation_to_scalar(output_index);

    // 3. Compute P = s*G + B
    // Simplified: hash(scalar || spend_public_key)
    let mut hasher = Sha256::new();
    hasher.update(b"one_time_key");
    hasher.update(&scalar);
    hasher.update(&receiver_address.spend_public_key);
    hasher.finalize().into()
}

/// Generate view tag for fast scanning
/// view_tag = first_byte(Hs("view_tag" || derivation || i))
pub fn generate_view_tag(
    derivation: &KeyDerivation,
    output_index: usize,
) -> u8 {
    let mut hasher = Sha256::new();
    hasher.update(b"view_tag");
    hasher.update(&derivation.shared_secret);
    hasher.update((output_index as u64).to_le_bytes());
    hasher.finalize()[0]
}

/// Check if output belongs to account (receiver side)
pub fn is_output_to_account(
    output: &StealthOutput,
    account_keys: &AccountKeys,
) -> bool {
    // 1. Compute key derivation: a*R
    let derivation = KeyDerivation::generate(
        &account_keys.view_secret_key,
        &output.tx_public_key,
    );

    // 2. Check view tag if present (optimization)
    if let Some(expected_tag) = output.view_tag {
        let computed_tag = generate_view_tag(&derivation, output.output_index);
        if computed_tag != expected_tag {
            return false;
        }
    }

    // 3. Derive expected one-time key
    let scalar = derivation.derivation_to_scalar(output.output_index);
    let mut hasher = Sha256::new();
    hasher.update(b"one_time_key");
    hasher.update(&scalar);
    hasher.update(&account_keys.address.spend_public_key);
    let expected_key: PublicKey = hasher.finalize().into();

    // 4. Compare with actual output key
    output.one_time_key == expected_key
}

/// Derive secret key for spending output
/// x = Hs(a*R || i) + b
pub fn derive_output_secret_key(
    output: &StealthOutput,
    account_keys: &AccountKeys,
) -> SecretKey {
    // 1. Compute key derivation: a*R
    let derivation = KeyDerivation::generate(
        &account_keys.view_secret_key,
        &output.tx_public_key,
    );

    // 2. Derive scalar
    let scalar = derivation.derivation_to_scalar(output.output_index);

    // 3. Compute secret key: x = scalar + spend_secret_key
    // Simplified: hash(scalar || spend_secret_key)
    let mut hasher = Sha256::new();
    hasher.update(b"output_secret_key");
    hasher.update(&scalar);
    hasher.update(&account_keys.spend_secret_key);
    hasher.finalize().into()
}

/// Subaddress for additional privacy
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subaddress {
    /// Major index
    pub major: u32,
    /// Minor index
    pub minor: u32,
    /// Public spend key for subaddress
    pub spend_public_key: PublicKey,
    /// Public view key for subaddress
    pub view_public_key: PublicKey,
}

impl Subaddress {
    /// Generate subaddress from account keys
    pub fn generate(
        account_keys: &AccountKeys,
        major: u32,
        minor: u32,
    ) -> Self {
        // m = Hs(a || account_index || address_index)
        let mut hasher = Sha256::new();
        hasher.update(b"subaddress");
        hasher.update(&account_keys.view_secret_key);
        hasher.update(major.to_le_bytes());
        hasher.update(minor.to_le_bytes());
        let m: [u8; 32] = hasher.finalize().into();

        // D = B + m*G (subaddress spend public key)
        let mut hasher = Sha256::new();
        hasher.update(b"subaddress_spend");
        hasher.update(&account_keys.address.spend_public_key);
        hasher.update(&m);
        let spend_public_key: PublicKey = hasher.finalize().into();

        // C = a*D (subaddress view public key)
        let mut hasher = Sha256::new();
        hasher.update(b"subaddress_view");
        hasher.update(&account_keys.view_secret_key);
        hasher.update(&spend_public_key);
        let view_public_key: PublicKey = hasher.finalize().into();

        Self {
            major,
            minor,
            spend_public_key,
            view_public_key,
        }
    }

    /// Check if this is the main address (0, 0)
    pub fn is_main_address(&self) -> bool {
        self.major == 0 && self.minor == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_key_pair() {
        let keypair = TxKeyPair::generate();
        assert_ne!(keypair.secret_key, [0u8; 32]);
        assert_ne!(keypair.public_key, [0u8; 32]);

        // Same secret key should give same public key
        let keypair2 = TxKeyPair::from_secret(keypair.secret_key);
        assert_eq!(keypair.public_key, keypair2.public_key);
    }

    #[test]
    fn test_key_derivation() {
        let private = [1u8; 32];
        let public = [2u8; 32];

        let derivation = KeyDerivation::generate(&private, &public);
        assert_ne!(derivation.shared_secret, [0u8; 32]);

        let scalar0 = derivation.derivation_to_scalar(0);
        let scalar1 = derivation.derivation_to_scalar(1);
        assert_ne!(scalar0, scalar1);
    }

    #[test]
    fn test_one_time_public_key() {
        let receiver = AccountKeys::generate();
        let tx_keypair = TxKeyPair::generate();

        let one_time_key = generate_one_time_public_key(
            &tx_keypair.secret_key,
            &receiver.address,
            0,
        );

        assert_ne!(one_time_key, [0u8; 32]);
    }

    #[test]
    fn test_stealth_output_detection() {
        // Simulate sender creating output
        let receiver = AccountKeys::generate();
        let tx_keypair = TxKeyPair::generate();

        let one_time_key = generate_one_time_public_key(
            &tx_keypair.secret_key,
            &receiver.address,
            0,
        );

        let output = StealthOutput::new(one_time_key, tx_keypair.public_key, 0);

        // Receiver should detect the output
        assert!(is_output_to_account(&output, &receiver));

        // Different account should not detect it
        let other_account = AccountKeys::generate();
        assert!(!is_output_to_account(&output, &other_account));
    }

    #[test]
    fn test_view_tag() {
        let derivation = KeyDerivation::generate(&[1u8; 32], &[2u8; 32]);
        let tag0 = generate_view_tag(&derivation, 0);
        let tag1 = generate_view_tag(&derivation, 1);

        // Different output indices should (usually) give different tags
        // Note: could collide with 1/256 probability
        assert_ne!(tag0, tag1);
    }

    #[test]
    fn test_stealth_output_with_view_tag() {
        let receiver = AccountKeys::generate();
        let tx_keypair = TxKeyPair::generate();

        // Sender computes derivation and one-time key
        let derivation = KeyDerivation::generate(
            &tx_keypair.secret_key,
            &receiver.address.view_public_key,
        );
        let view_tag = generate_view_tag(&derivation, 0);
        let one_time_key = generate_one_time_public_key(
            &tx_keypair.secret_key,
            &receiver.address,
            0,
        );

        let output = StealthOutput::new(one_time_key, tx_keypair.public_key, 0)
            .with_view_tag(view_tag);

        // Should still be detected with view tag
        assert!(is_output_to_account(&output, &receiver));
    }

    #[test]
    fn test_derive_output_secret_key() {
        let receiver = AccountKeys::generate();
        let tx_keypair = TxKeyPair::generate();

        let one_time_key = generate_one_time_public_key(
            &tx_keypair.secret_key,
            &receiver.address,
            0,
        );

        let output = StealthOutput::new(one_time_key, tx_keypair.public_key, 0);
        let secret_key = derive_output_secret_key(&output, &receiver);

        assert_ne!(secret_key, [0u8; 32]);
    }

    #[test]
    fn test_subaddress() {
        let account = AccountKeys::generate();

        let sub_0_0 = Subaddress::generate(&account, 0, 0);
        let sub_0_1 = Subaddress::generate(&account, 0, 1);
        let sub_1_0 = Subaddress::generate(&account, 1, 0);

        assert!(sub_0_0.is_main_address());
        assert!(!sub_0_1.is_main_address());

        // Different indices should give different addresses
        assert_ne!(sub_0_0.spend_public_key, sub_0_1.spend_public_key);
        assert_ne!(sub_0_0.spend_public_key, sub_1_0.spend_public_key);
    }
}
