//! # CryptoNote Protocol Implementation
//!
//! Core structures for Monero transactions and blocks.
//!
//! ## Key Components
//!
//! - **Key Image**: Unique identifier for spent outputs (prevents double-spending)
//! - **Transaction Input**: References ring of possible outputs
//! - **Transaction Output**: One-time public key for receiver

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// 32-byte public key
pub type PublicKey = [u8; 32];

/// 32-byte secret key
pub type SecretKey = [u8; 32];

/// 32-byte hash
pub type Hash = [u8; 32];

/// 64-byte signature (c, r components) - using Vec for serde compatibility
pub type Signature = Vec<u8>;

/// Key image - unique identifier for spent output
/// I = x * Hp(P) where x is private key, P is public key
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyImage(pub [u8; 32]);

impl KeyImage {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Generate key image from private key and public key
    /// I = x * Hp(P)
    pub fn generate(private_key: &SecretKey, public_key: &PublicKey) -> Self {
        // Simplified: hash(private_key || public_key)
        let mut hasher = Sha256::new();
        hasher.update(b"key_image");
        hasher.update(private_key);
        hasher.update(public_key);
        Self(hasher.finalize().into())
    }
}

/// Account public address (view key + spend key)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAddress {
    /// Public view key (A = a*G)
    pub view_public_key: PublicKey,
    /// Public spend key (B = b*G)
    pub spend_public_key: PublicKey,
}

impl AccountAddress {
    pub fn new(view_public_key: PublicKey, spend_public_key: PublicKey) -> Self {
        Self {
            view_public_key,
            spend_public_key,
        }
    }

    /// Generate address hash for display
    pub fn to_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.view_public_key);
        hasher.update(&self.spend_public_key);
        hasher.finalize().into()
    }
}

/// Account keys (private + public)
#[derive(Clone, Debug)]
pub struct AccountKeys {
    /// Public address
    pub address: AccountAddress,
    /// Secret view key (a)
    pub view_secret_key: SecretKey,
    /// Secret spend key (b)
    pub spend_secret_key: SecretKey,
}

impl AccountKeys {
    pub fn new(
        view_secret_key: SecretKey,
        spend_secret_key: SecretKey,
        view_public_key: PublicKey,
        spend_public_key: PublicKey,
    ) -> Self {
        Self {
            address: AccountAddress::new(view_public_key, spend_public_key),
            view_secret_key,
            spend_secret_key,
        }
    }

    /// Generate new random account keys
    pub fn generate() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let view_secret: SecretKey = rng.gen();
        let spend_secret: SecretKey = rng.gen();

        // Simplified: public = hash(secret) (real: public = secret * G)
        let view_public = Self::derive_public_key(&view_secret);
        let spend_public = Self::derive_public_key(&spend_secret);

        Self::new(view_secret, spend_secret, view_public, spend_public)
    }

    fn derive_public_key(secret: &SecretKey) -> PublicKey {
        let mut hasher = Sha256::new();
        hasher.update(b"public_key");
        hasher.update(secret);
        hasher.finalize().into()
    }
}

/// Transaction output type
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxOutTarget {
    /// Standard output with one-time public key
    ToKey {
        /// One-time public key
        key: PublicKey,
    },
    /// Output with view tag optimization
    ToTaggedKey {
        /// One-time public key
        key: PublicKey,
        /// View tag for fast scanning (1 byte)
        view_tag: u8,
    },
}

/// Transaction output
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxOut {
    /// Amount (0 for RingCT transactions - hidden)
    pub amount: u64,
    /// Output target
    pub target: TxOutTarget,
}

impl TxOut {
    pub fn new(amount: u64, key: PublicKey) -> Self {
        Self {
            amount,
            target: TxOutTarget::ToKey { key },
        }
    }

    pub fn new_with_view_tag(amount: u64, key: PublicKey, view_tag: u8) -> Self {
        Self {
            amount,
            target: TxOutTarget::ToTaggedKey { key, view_tag },
        }
    }

    pub fn get_key(&self) -> &PublicKey {
        match &self.target {
            TxOutTarget::ToKey { key } => key,
            TxOutTarget::ToTaggedKey { key, .. } => key,
        }
    }
}

/// Transaction input type
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxIn {
    /// Coinbase input (block reward)
    Gen {
        /// Block height
        height: u64,
    },
    /// Standard input with ring signature
    ToKey {
        /// Amount (0 for RingCT)
        amount: u64,
        /// Offsets to ring member outputs (relative encoding)
        key_offsets: Vec<u64>,
        /// Key image for double-spend prevention
        key_image: KeyImage,
    },
}

impl TxIn {
    pub fn coinbase(height: u64) -> Self {
        Self::Gen { height }
    }

    pub fn to_key(amount: u64, key_offsets: Vec<u64>, key_image: KeyImage) -> Self {
        Self::ToKey {
            amount,
            key_offsets,
            key_image,
        }
    }

    pub fn get_key_image(&self) -> Option<&KeyImage> {
        match self {
            Self::ToKey { key_image, .. } => Some(key_image),
            Self::Gen { .. } => None,
        }
    }
}

/// Transaction prefix (without signatures)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionPrefix {
    /// Transaction version
    pub version: u8,
    /// Unlock time (block height or timestamp)
    pub unlock_time: u64,
    /// Inputs
    pub inputs: Vec<TxIn>,
    /// Outputs
    pub outputs: Vec<TxOut>,
    /// Extra data (tx public key, payment ID, etc.)
    pub extra: Vec<u8>,
}

impl TransactionPrefix {
    pub fn new(version: u8, unlock_time: u64) -> Self {
        Self {
            version,
            unlock_time,
            inputs: Vec::new(),
            outputs: Vec::new(),
            extra: Vec::new(),
        }
    }

    pub fn add_input(&mut self, input: TxIn) {
        self.inputs.push(input);
    }

    pub fn add_output(&mut self, output: TxOut) {
        self.outputs.push(output);
    }

    /// Get transaction public key from extra
    pub fn get_tx_public_key(&self) -> Option<PublicKey> {
        // Extra format: [tag, data...]
        // Tag 0x01 = tx public key (32 bytes)
        if self.extra.len() >= 33 && self.extra[0] == 0x01 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&self.extra[1..33]);
            Some(key)
        } else {
            None
        }
    }

    /// Set transaction public key in extra
    pub fn set_tx_public_key(&mut self, key: &PublicKey) {
        self.extra = vec![0x01];
        self.extra.extend_from_slice(key);
    }

    /// Calculate prefix hash
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&[self.version]);
        hasher.update(self.unlock_time.to_le_bytes());

        for input in &self.inputs {
            match input {
                TxIn::Gen { height } => {
                    hasher.update(&[0x00]);
                    hasher.update(height.to_le_bytes());
                }
                TxIn::ToKey {
                    amount,
                    key_offsets,
                    key_image,
                } => {
                    hasher.update(&[0x02]);
                    hasher.update(amount.to_le_bytes());
                    hasher.update((key_offsets.len() as u32).to_le_bytes());
                    for offset in key_offsets {
                        hasher.update(offset.to_le_bytes());
                    }
                    hasher.update(&key_image.0);
                }
            }
        }

        for output in &self.outputs {
            hasher.update(output.amount.to_le_bytes());
            hasher.update(output.get_key());
        }

        hasher.update(&self.extra);
        hasher.finalize().into()
    }
}

/// Ring signature for transaction input
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RingSignature {
    /// Ring signatures (one per ring member)
    pub signatures: Vec<Signature>,
}

impl RingSignature {
    pub fn new(ring_size: usize) -> Self {
        Self {
            signatures: (0..ring_size).map(|_| vec![0u8; 64]).collect(),
        }
    }

    pub fn ring_size(&self) -> usize {
        self.signatures.len()
    }
}

/// Complete transaction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction prefix
    pub prefix: TransactionPrefix,
    /// Ring signatures (one per input)
    pub signatures: Vec<RingSignature>,
    /// RingCT data (for version 2+)
    pub rct_signatures: Option<RctSignatures>,
}

impl Transaction {
    pub fn new(prefix: TransactionPrefix) -> Self {
        Self {
            prefix,
            signatures: Vec::new(),
            rct_signatures: None,
        }
    }

    /// Calculate transaction hash
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.prefix.hash());

        // Include signatures in hash
        for sig in &self.signatures {
            for s in &sig.signatures {
                hasher.update(s);
            }
        }

        hasher.finalize().into()
    }

    /// Check if this is a coinbase transaction
    pub fn is_coinbase(&self) -> bool {
        self.prefix.inputs.len() == 1
            && matches!(self.prefix.inputs[0], TxIn::Gen { .. })
    }

    /// Get all key images
    pub fn get_key_images(&self) -> Vec<&KeyImage> {
        self.prefix
            .inputs
            .iter()
            .filter_map(|input| input.get_key_image())
            .collect()
    }
}

/// RingCT signatures (simplified)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RctSignatures {
    /// RingCT type (1 = RCTTypeSimple, 2 = RCTTypeFull, etc.)
    pub rct_type: u8,
    /// Transaction fee
    pub txn_fee: u64,
    /// Output commitments
    pub out_pk: Vec<PublicKey>,
    /// Encrypted amounts
    pub ecdh_info: Vec<EcdhInfo>,
    /// Range proofs (Bulletproofs)
    pub bulletproofs: Vec<BulletproofData>,
}

impl RctSignatures {
    pub fn new(rct_type: u8, txn_fee: u64) -> Self {
        Self {
            rct_type,
            txn_fee,
            out_pk: Vec::new(),
            ecdh_info: Vec::new(),
            bulletproofs: Vec::new(),
        }
    }
}

/// ECDH info for encrypted amount
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcdhInfo {
    /// Encrypted mask
    pub mask: [u8; 32],
    /// Encrypted amount
    pub amount: [u8; 32],
}

/// Bulletproof range proof data (simplified container)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulletproofData {
    /// Proof data
    pub proof: Vec<u8>,
}

/// Block header
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Major version
    pub major_version: u8,
    /// Minor version
    pub minor_version: u8,
    /// Timestamp
    pub timestamp: u64,
    /// Previous block hash
    pub prev_id: Hash,
    /// Nonce for PoW
    pub nonce: u32,
}

impl BlockHeader {
    pub fn new(major_version: u8, minor_version: u8, timestamp: u64, prev_id: Hash) -> Self {
        Self {
            major_version,
            minor_version,
            timestamp,
            prev_id,
            nonce: 0,
        }
    }

    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&[self.major_version, self.minor_version]);
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(&self.prev_id);
        hasher.update(self.nonce.to_le_bytes());
        hasher.finalize().into()
    }
}

/// Block
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// Miner transaction (coinbase)
    pub miner_tx: Transaction,
    /// Transaction hashes
    pub tx_hashes: Vec<Hash>,
}

impl Block {
    pub fn new(header: BlockHeader, miner_tx: Transaction) -> Self {
        Self {
            header,
            miner_tx,
            tx_hashes: Vec::new(),
        }
    }

    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.header.hash());
        hasher.update(&self.miner_tx.hash());
        for tx_hash in &self.tx_hashes {
            hasher.update(tx_hash);
        }
        hasher.finalize().into()
    }
}

/// Key image tracker for double-spend detection
#[derive(Debug, Default)]
pub struct KeyImageStore {
    spent: HashSet<KeyImage>,
}

impl KeyImageStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if key image has been spent
    pub fn is_spent(&self, key_image: &KeyImage) -> bool {
        self.spent.contains(key_image)
    }

    /// Mark key image as spent
    pub fn mark_spent(&mut self, key_image: KeyImage) -> bool {
        self.spent.insert(key_image)
    }

    /// Validate transaction's key images
    pub fn validate_transaction(&self, tx: &Transaction) -> Result<(), &'static str> {
        for key_image in tx.get_key_images() {
            if self.is_spent(key_image) {
                return Err("Double-spend detected: key image already spent");
            }
        }
        Ok(())
    }

    /// Apply transaction (mark all key images as spent)
    pub fn apply_transaction(&mut self, tx: &Transaction) {
        for key_image in tx.get_key_images() {
            self.mark_spent(*key_image);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_image() {
        let private_key = [1u8; 32];
        let public_key = [2u8; 32];
        let key_image = KeyImage::generate(&private_key, &public_key);
        assert_ne!(key_image.0, [0u8; 32]);

        // Same inputs should give same key image
        let key_image2 = KeyImage::generate(&private_key, &public_key);
        assert_eq!(key_image, key_image2);
    }

    #[test]
    fn test_account_address() {
        let view_key = [1u8; 32];
        let spend_key = [2u8; 32];
        let addr = AccountAddress::new(view_key, spend_key);
        assert_eq!(addr.view_public_key, view_key);
        assert_eq!(addr.spend_public_key, spend_key);
    }

    #[test]
    fn test_account_keys() {
        let keys = AccountKeys::generate();
        assert_ne!(keys.view_secret_key, [0u8; 32]);
        assert_ne!(keys.spend_secret_key, [0u8; 32]);
    }

    #[test]
    fn test_tx_out() {
        let key = [1u8; 32];
        let output = TxOut::new(1000, key);
        assert_eq!(output.amount, 1000);
        assert_eq!(*output.get_key(), key);

        let tagged = TxOut::new_with_view_tag(1000, key, 0xAB);
        assert_eq!(*tagged.get_key(), key);
    }

    #[test]
    fn test_tx_in() {
        let coinbase = TxIn::coinbase(100);
        assert!(coinbase.get_key_image().is_none());

        let key_image = KeyImage([3u8; 32]);
        let input = TxIn::to_key(0, vec![1, 2, 3], key_image);
        assert_eq!(input.get_key_image(), Some(&key_image));
    }

    #[test]
    fn test_transaction_prefix() {
        let mut prefix = TransactionPrefix::new(2, 0);
        prefix.add_input(TxIn::coinbase(100));
        prefix.add_output(TxOut::new(1000, [1u8; 32]));

        let hash = prefix.hash();
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_transaction() {
        let mut prefix = TransactionPrefix::new(2, 0);
        prefix.add_input(TxIn::coinbase(100));
        prefix.add_output(TxOut::new(1000, [1u8; 32]));

        let tx = Transaction::new(prefix);
        assert!(tx.is_coinbase());
        assert!(tx.get_key_images().is_empty());

        let hash = tx.hash();
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_key_image_store() {
        let mut store = KeyImageStore::new();
        let key_image = KeyImage([1u8; 32]);

        assert!(!store.is_spent(&key_image));
        assert!(store.mark_spent(key_image));
        assert!(store.is_spent(&key_image));
        assert!(!store.mark_spent(key_image)); // Already spent
    }

    #[test]
    fn test_block_header() {
        let header = BlockHeader::new(1, 0, 1609459200, [0u8; 32]);
        let hash = header.hash();
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_block() {
        let header = BlockHeader::new(1, 0, 1609459200, [0u8; 32]);
        let mut prefix = TransactionPrefix::new(2, 0);
        prefix.add_input(TxIn::coinbase(0));
        prefix.add_output(TxOut::new(1000, [1u8; 32]));
        let miner_tx = Transaction::new(prefix);

        let block = Block::new(header, miner_tx);
        let hash = block.hash();
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_ring_signature() {
        let ring_sig = RingSignature::new(16);
        assert_eq!(ring_sig.ring_size(), 16);
    }
}
