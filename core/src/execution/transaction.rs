//! Transaction types and operations
//!
//! This module defines the UTXO-based transaction model.
//!
//! ## Transaction structure:
//! ```text
//! Transaction {
//!     id: [u8; 32],          // Transaction hash
//!     vin: Vec<TXInput>,     // Inputs (references to previous outputs)
//!     vout: Vec<TXOutput>,   // Outputs (new UTXOs)
//! }
//! ```
//!
//! ## Comparison with other blockchains:
//! - **Bitcoin**: UTXO model (same as this)
//! - **Ethereum**: Account model (balances stored in state)
//! - **Kaspa**: UTXO model
//! - **Solana**: Account model with program ownership

use crate::crypto::{sha256, hash160, address_to_pub_key_hash, sign, verify};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Mining reward (subsidy) for coinbase transaction
pub const SUBSIDY: i32 = 10;

/// Transaction input - reference to a previous output
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct TXInput {
    /// Transaction ID containing the referenced output
    txid: Vec<u8>,
    /// Index of the output in that transaction
    vout: usize,
    /// Signature proving ownership
    signature: Vec<u8>,
    /// Public key of the spender
    pub_key: Vec<u8>,
}

impl TXInput {
    /// Create a new transaction input
    pub fn new(txid: &[u8], vout: usize) -> Self {
        TXInput {
            txid: txid.to_vec(),
            vout,
            signature: vec![],
            pub_key: vec![],
        }
    }

    pub fn get_txid(&self) -> &[u8] {
        &self.txid
    }

    pub fn get_vout(&self) -> usize {
        self.vout
    }

    pub fn get_pub_key(&self) -> &[u8] {
        &self.pub_key
    }

    pub fn get_signature(&self) -> &[u8] {
        &self.signature
    }

    /// Check if this input was signed by the owner of pub_key_hash
    pub fn uses_key(&self, pub_key_hash: &[u8]) -> bool {
        let locking_hash = hash160(&self.pub_key);
        locking_hash == pub_key_hash
    }

    /// Set the public key
    pub fn set_pub_key(&mut self, pub_key: Vec<u8>) {
        self.pub_key = pub_key;
    }

    /// Set the signature
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = signature;
    }
}

/// Transaction output - spendable coins locked to a public key hash
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TXOutput {
    /// Value in smallest unit (satoshis in Bitcoin)
    value: i32,
    /// Public key hash that can spend this output
    pub_key_hash: Vec<u8>,
}

impl TXOutput {
    /// Create a new output locked to an address
    pub fn new(value: i32, address: &str) -> Self {
        let mut output = TXOutput {
            value,
            pub_key_hash: vec![],
        };
        output.lock(address);
        output
    }

    /// Create output directly from public key hash
    pub fn from_pub_key_hash(value: i32, pub_key_hash: Vec<u8>) -> Self {
        TXOutput { value, pub_key_hash }
    }

    pub fn get_value(&self) -> i32 {
        self.value
    }

    pub fn get_pub_key_hash(&self) -> &[u8] {
        &self.pub_key_hash
    }

    /// Lock this output to an address
    fn lock(&mut self, address: &str) {
        if let Ok(pub_key_hash) = address_to_pub_key_hash(address) {
            self.pub_key_hash = pub_key_hash;
        }
    }

    /// Check if this output can be spent by the owner of pub_key_hash
    pub fn is_locked_with_key(&self, pub_key_hash: &[u8]) -> bool {
        self.pub_key_hash == pub_key_hash
    }
}

/// A blockchain transaction
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Transaction {
    /// Transaction ID (hash of transaction data)
    id: Vec<u8>,
    /// Transaction inputs
    vin: Vec<TXInput>,
    /// Transaction outputs
    vout: Vec<TXOutput>,
}

impl Transaction {
    /// Create a coinbase transaction (mining reward)
    ///
    /// Coinbase transactions have no real inputs and create new coins.
    pub fn new_coinbase_tx(to: &str) -> Self {
        let txout = TXOutput::new(SUBSIDY, to);
        let tx_input = TXInput {
            signature: Uuid::new_v4().as_bytes().to_vec(),
            ..Default::default()
        };

        let mut tx = Transaction {
            id: vec![],
            vin: vec![tx_input],
            vout: vec![txout],
        };

        tx.id = tx.compute_hash();
        tx
    }

    /// Create a new transaction with given inputs and outputs
    pub fn new(vin: Vec<TXInput>, vout: Vec<TXOutput>) -> Self {
        let mut tx = Transaction {
            id: vec![],
            vin,
            vout,
        };
        tx.id = tx.compute_hash();
        tx
    }

    /// Check if this is a coinbase transaction
    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].pub_key.is_empty() && self.vin[0].txid.is_empty()
    }

    /// Compute transaction hash
    fn compute_hash(&self) -> Vec<u8> {
        let tx_copy = Transaction {
            id: vec![],
            vin: self.vin.clone(),
            vout: self.vout.clone(),
        };
        sha256(&tx_copy.serialize())
    }

    /// Recalculate and set the transaction ID
    pub fn set_id(&mut self) {
        self.id = self.compute_hash();
    }

    pub fn get_id(&self) -> &[u8] {
        &self.id
    }

    pub fn get_id_bytes(&self) -> Vec<u8> {
        self.id.clone()
    }

    pub fn get_vin(&self) -> &[TXInput] {
        &self.vin
    }

    pub fn get_vin_mut(&mut self) -> &mut Vec<TXInput> {
        &mut self.vin
    }

    pub fn get_vout(&self) -> &[TXOutput] {
        &self.vout
    }

    /// Serialize transaction to bytes
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    /// Deserialize transaction from bytes
    pub fn deserialize(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }

    /// Create a trimmed copy for signing (without signatures)
    pub fn trimmed_copy(&self) -> Self {
        let inputs: Vec<TXInput> = self
            .vin
            .iter()
            .map(|vin| TXInput::new(vin.get_txid(), vin.get_vout()))
            .collect();

        Transaction {
            id: self.id.clone(),
            vin: inputs,
            vout: self.vout.clone(),
        }
    }

    /// Sign the transaction
    ///
    /// # Arguments
    /// * `pkcs8` - Private key in PKCS#8 format
    /// * `prev_txs` - Map of previous transaction ID -> Transaction
    pub fn sign<F>(&mut self, pkcs8: &[u8], get_prev_tx: F)
    where
        F: Fn(&[u8]) -> Option<Transaction>,
    {
        if self.is_coinbase() {
            return;
        }

        let mut tx_copy = self.trimmed_copy();

        for (idx, vin) in self.vin.iter_mut().enumerate() {
            if let Some(prev_tx) = get_prev_tx(vin.get_txid()) {
                tx_copy.vin[idx].signature = vec![];
                tx_copy.vin[idx].pub_key = prev_tx.vout[vin.vout].pub_key_hash.clone();
                tx_copy.set_id();
                tx_copy.vin[idx].pub_key = vec![];

                let signature = sign(pkcs8, tx_copy.get_id());
                vin.set_signature(signature);
            }
        }
    }

    /// Verify all signatures in the transaction
    pub fn verify<F>(&self, get_prev_tx: F) -> bool
    where
        F: Fn(&[u8]) -> Option<Transaction>,
    {
        if self.is_coinbase() {
            return true;
        }

        let mut tx_copy = self.trimmed_copy();

        for (idx, vin) in self.vin.iter().enumerate() {
            let prev_tx = match get_prev_tx(vin.get_txid()) {
                Some(tx) => tx,
                None => return false,
            };

            tx_copy.vin[idx].signature = vec![];
            tx_copy.vin[idx].pub_key = prev_tx.vout[vin.vout].pub_key_hash.clone();
            tx_copy.set_id();
            tx_copy.vin[idx].pub_key = vec![];

            if !verify(vin.get_pub_key(), vin.get_signature(), tx_copy.get_id()) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair, public_key_from_pkcs8, public_key_to_address};

    fn create_test_address() -> (Vec<u8>, String) {
        let pkcs8 = generate_keypair();
        let public_key = public_key_from_pkcs8(&pkcs8);
        let address = public_key_to_address(&public_key);
        (pkcs8, address)
    }

    #[test]
    fn test_coinbase_transaction() {
        let (_, address) = create_test_address();
        let tx = Transaction::new_coinbase_tx(&address);

        assert!(tx.is_coinbase());
        assert!(!tx.get_id().is_empty());
        assert_eq!(tx.get_vin().len(), 1);
        assert_eq!(tx.get_vout().len(), 1);
        assert_eq!(tx.get_vout()[0].get_value(), SUBSIDY);
    }

    #[test]
    fn test_transaction_serialization() {
        let (_, address) = create_test_address();
        let tx = Transaction::new_coinbase_tx(&address);

        let serialized = tx.serialize();
        let deserialized = Transaction::deserialize(&serialized);

        assert_eq!(tx.get_id(), deserialized.get_id());
        assert_eq!(tx.get_vin().len(), deserialized.get_vin().len());
        assert_eq!(tx.get_vout().len(), deserialized.get_vout().len());
    }

    #[test]
    fn test_txoutput_locking() {
        let (_, address) = create_test_address();
        let output = TXOutput::new(100, &address);
        let pub_key_hash = output.get_pub_key_hash();

        assert!(output.is_locked_with_key(pub_key_hash));
        assert!(!output.is_locked_with_key(&[0u8; 20]));
    }

    #[test]
    fn test_unique_coinbase_ids() {
        let (_, address) = create_test_address();
        let tx1 = Transaction::new_coinbase_tx(&address);
        let tx2 = Transaction::new_coinbase_tx(&address);

        assert_ne!(tx1.get_id(), tx2.get_id());
    }
}
