//! Extended UTXO Model
//!
//! ## eUTXO vs UTXO Comparison
//!
//! | Feature | Bitcoin UTXO | Cardano eUTXO |
//! |---------|-------------|---------------|
//! | Value | Single asset (BTC) | Multi-asset (ADA + tokens) |
//! | Lock condition | Script hash | Address (payment + staking) |
//! | State | None | Datum (arbitrary data) |
//! | Unlock | Signature only | Redeemer + Script context |
//! | Script access | None | Full transaction view |
//!
//! ## Transaction Flow
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Transaction                              │
//! ├─────────────────────────────────────────────────────────────┤
//! │ Inputs:                                                     │
//! │   ┌─────────────────────────────────────────┐              │
//! │   │ TxIn { txid: abc..., index: 0 }        │              │
//! │   │ + Redeemer { data: ..., ex_units: ... }│              │
//! │   └─────────────────────────────────────────┘              │
//! │                                                             │
//! │ Reference Inputs: (read-only, no spending)                 │
//! │   ┌─────────────────────────────────────────┐              │
//! │   │ TxIn { txid: def..., index: 2 }        │              │
//! │   └─────────────────────────────────────────┘              │
//! │                                                             │
//! │ Outputs:                                                    │
//! │   ┌─────────────────────────────────────────┐              │
//! │   │ TxOut {                                 │              │
//! │   │   address: addr1...,                   │              │
//! │   │   value: 5 ADA + 100 TokenA,           │              │
//! │   │   datum: Some(state_data),             │              │
//! │   │ }                                       │              │
//! │   └─────────────────────────────────────────┘              │
//! │                                                             │
//! │ Mint: { PolicyID_X: { "NewToken": 1000 } }                 │
//! │                                                             │
//! │ Fee: 0.2 ADA                                               │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Transaction hash (32 bytes)
pub type TxHash = [u8; 32];

/// Policy ID for native assets (28 bytes - script hash)
pub type PolicyId = [u8; 28];

/// Asset name (max 32 bytes)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetName(Vec<u8>);

impl AssetName {
    pub fn new(name: &[u8]) -> Result<Self, &'static str> {
        if name.len() > 32 {
            return Err("Asset name too long (max 32 bytes)");
        }
        Ok(Self(name.to_vec()))
    }

    pub fn from_string(s: &str) -> Result<Self, &'static str> {
        Self::new(s.as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Lovelace (1 ADA = 1,000,000 lovelace)
pub type Lovelace = u64;

/// Multi-asset value
#[derive(Debug, Clone, Default)]
pub struct Value {
    /// ADA amount in lovelace
    pub coin: Lovelace,
    /// Native assets: PolicyID -> (AssetName -> Quantity)
    pub multi_asset: HashMap<PolicyId, HashMap<AssetName, i64>>,
}

impl Value {
    /// Create value with only ADA
    pub fn from_coin(coin: Lovelace) -> Self {
        Self {
            coin,
            multi_asset: HashMap::new(),
        }
    }

    /// Create value with ADA and assets
    pub fn new(coin: Lovelace, multi_asset: HashMap<PolicyId, HashMap<AssetName, i64>>) -> Self {
        Self { coin, multi_asset }
    }

    /// Check if this value contains only ADA
    pub fn is_ada_only(&self) -> bool {
        self.multi_asset.is_empty()
            || self
                .multi_asset
                .values()
                .all(|assets| assets.values().all(|&q| q == 0))
    }

    /// Add two values
    pub fn add(&self, other: &Value) -> Self {
        let mut result = self.clone();
        result.coin = result.coin.saturating_add(other.coin);

        for (policy, assets) in &other.multi_asset {
            let entry = result.multi_asset.entry(*policy).or_default();
            for (name, quantity) in assets {
                *entry.entry(name.clone()).or_insert(0) += quantity;
            }
        }

        result
    }

    /// Subtract other from self (for fee calculation)
    pub fn sub(&self, other: &Value) -> Option<Self> {
        if self.coin < other.coin {
            return None;
        }

        let mut result = self.clone();
        result.coin -= other.coin;

        for (policy, assets) in &other.multi_asset {
            if let Some(our_assets) = result.multi_asset.get_mut(policy) {
                for (name, quantity) in assets {
                    let entry = our_assets.entry(name.clone()).or_insert(0);
                    *entry -= quantity;
                    if *entry < 0 {
                        return None; // Insufficient assets
                    }
                }
            } else if assets.values().any(|&q| q > 0) {
                return None; // Missing policy
            }
        }

        Some(result)
    }

    /// Check if value is non-negative
    pub fn is_non_negative(&self) -> bool {
        self.multi_asset
            .values()
            .all(|assets| assets.values().all(|&q| q >= 0))
    }

    /// Add native asset
    pub fn add_asset(&mut self, policy: PolicyId, name: AssetName, quantity: i64) {
        self.multi_asset
            .entry(policy)
            .or_default()
            .entry(name)
            .and_modify(|q| *q += quantity)
            .or_insert(quantity);
    }
}

// =============================================================================
// Transaction Input/Output
// =============================================================================

/// Transaction input reference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TxIn {
    /// Transaction hash
    pub tx_hash: TxHash,
    /// Output index
    pub index: u32,
}

impl TxIn {
    pub fn new(tx_hash: TxHash, index: u32) -> Self {
        Self { tx_hash, index }
    }
}

/// Datum - arbitrary data attached to UTxO
#[derive(Debug, Clone, PartialEq)]
pub enum Datum {
    /// Datum hash only (actual datum in witnesses)
    Hash(DatumHash),
    /// Inline datum (Babbage era, CIP-32)
    Inline(PlutusData),
}

/// Datum hash (32 bytes)
pub type DatumHash = [u8; 32];

/// Plutus data (simplified representation)
#[derive(Debug, Clone, PartialEq)]
pub enum PlutusData {
    /// Integer
    Integer(i128),
    /// Byte string
    Bytes(Vec<u8>),
    /// List
    List(Vec<PlutusData>),
    /// Map
    Map(Vec<(PlutusData, PlutusData)>),
    /// Constructor
    Constr(u64, Vec<PlutusData>),
}

impl PlutusData {
    /// Compute hash of datum
    pub fn hash(&self) -> DatumHash {
        let bytes = self.to_bytes();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hasher.finalize().into()
    }

    /// Serialize to bytes (simplified)
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self {
            PlutusData::Integer(i) => {
                bytes.push(0x00);
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            PlutusData::Bytes(b) => {
                bytes.push(0x01);
                bytes.extend_from_slice(b);
            }
            PlutusData::List(items) => {
                bytes.push(0x02);
                for item in items {
                    bytes.extend(item.to_bytes());
                }
            }
            PlutusData::Map(pairs) => {
                bytes.push(0x03);
                for (k, v) in pairs {
                    bytes.extend(k.to_bytes());
                    bytes.extend(v.to_bytes());
                }
            }
            PlutusData::Constr(tag, fields) => {
                bytes.push(0x04);
                bytes.extend_from_slice(&tag.to_le_bytes());
                for field in fields {
                    bytes.extend(field.to_bytes());
                }
            }
        }
        bytes
    }
}

/// Address type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address {
    /// Payment credential (key hash or script hash)
    pub payment: Credential,
    /// Staking credential (optional)
    pub staking: Option<Credential>,
}

/// Credential - either a key hash or script hash
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Credential {
    /// Public key hash (28 bytes)
    PubKeyHash([u8; 28]),
    /// Script hash (28 bytes)
    ScriptHash([u8; 28]),
}

impl Credential {
    pub fn is_script(&self) -> bool {
        matches!(self, Credential::ScriptHash(_))
    }
}

/// Transaction output
#[derive(Debug, Clone)]
pub struct TxOut {
    /// Destination address
    pub address: Address,
    /// Value (ADA + native assets)
    pub value: Value,
    /// Datum (optional)
    pub datum: Option<Datum>,
    /// Reference script (Babbage era, CIP-33)
    pub reference_script: Option<ScriptHash>,
}

/// Script hash (28 bytes)
pub type ScriptHash = [u8; 28];

impl TxOut {
    /// Create a simple output with just ADA
    pub fn simple(address: Address, coin: Lovelace) -> Self {
        Self {
            address,
            value: Value::from_coin(coin),
            datum: None,
            reference_script: None,
        }
    }

    /// Create output with datum
    pub fn with_datum(address: Address, value: Value, datum: Datum) -> Self {
        Self {
            address,
            value,
            datum: Some(datum),
            reference_script: None,
        }
    }

    /// Check if this output is locked by a script
    pub fn is_script_locked(&self) -> bool {
        self.address.payment.is_script()
    }
}

// =============================================================================
// UTxO Set
// =============================================================================

/// Unspent Transaction Output set
#[derive(Debug, Clone, Default)]
pub struct UTxO {
    /// Map of TxIn -> TxOut
    utxos: HashMap<TxIn, TxOut>,
}

impl UTxO {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
        }
    }

    /// Add a UTxO
    pub fn insert(&mut self, txin: TxIn, txout: TxOut) {
        self.utxos.insert(txin, txout);
    }

    /// Remove a UTxO (when spent)
    pub fn remove(&mut self, txin: &TxIn) -> Option<TxOut> {
        self.utxos.remove(txin)
    }

    /// Get a UTxO
    pub fn get(&self, txin: &TxIn) -> Option<&TxOut> {
        self.utxos.get(txin)
    }

    /// Check if UTxO exists
    pub fn contains(&self, txin: &TxIn) -> bool {
        self.utxos.contains_key(txin)
    }

    /// Get all UTxOs
    pub fn iter(&self) -> impl Iterator<Item = (&TxIn, &TxOut)> {
        self.utxos.iter()
    }

    /// Total value in UTxO set
    pub fn total_value(&self) -> Value {
        self.utxos
            .values()
            .fold(Value::default(), |acc, txout| acc.add(&txout.value))
    }

    /// Filter UTxOs by address
    pub fn by_address(&self, address: &Address) -> Vec<(&TxIn, &TxOut)> {
        self.utxos
            .iter()
            .filter(|(_, txout)| &txout.address == address)
            .collect()
    }
}

// =============================================================================
// Redeemer
// =============================================================================

/// Execution units for script execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExUnits {
    /// CPU steps
    pub mem: u64,
    /// Memory units
    pub cpu: u64,
}

impl ExUnits {
    pub fn new(mem: u64, cpu: u64) -> Self {
        Self { mem, cpu }
    }

    /// Check if within budget
    pub fn within_budget(&self, max_mem: u64, max_cpu: u64) -> bool {
        self.mem <= max_mem && self.cpu <= max_cpu
    }
}

/// Script purpose - what the script is validating
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScriptPurpose {
    /// Validating spending of a UTxO
    Spending(TxIn),
    /// Validating minting/burning of tokens
    Minting(PolicyId),
    /// Validating stake certificate
    Certifying(u32), // Certificate index
    /// Validating reward withdrawal
    Rewarding([u8; 28]), // Stake credential hash
}

/// Redeemer - data used to unlock a script
#[derive(Debug, Clone)]
pub struct Redeemer {
    /// Script purpose
    pub purpose: ScriptPurpose,
    /// Redeemer data
    pub data: PlutusData,
    /// Execution budget
    pub ex_units: ExUnits,
}

// =============================================================================
// Transaction
// =============================================================================

/// Transaction body
#[derive(Debug, Clone)]
pub struct TxBody {
    /// Inputs (UTxOs being spent)
    pub inputs: Vec<TxIn>,
    /// Reference inputs (read-only, CIP-31)
    pub reference_inputs: Vec<TxIn>,
    /// Outputs
    pub outputs: Vec<TxOut>,
    /// Transaction fee
    pub fee: Lovelace,
    /// Validity interval start (slot)
    pub validity_start: Option<u64>,
    /// Validity interval end (slot)
    pub validity_end: Option<u64>,
    /// Minted/burned assets
    pub mint: HashMap<PolicyId, HashMap<AssetName, i64>>,
    /// Required signers (key hashes)
    pub required_signers: Vec<[u8; 28]>,
    /// Collateral inputs (for script failures)
    pub collateral: Vec<TxIn>,
    /// Collateral return output
    pub collateral_return: Option<TxOut>,
}

/// Transaction witness set
#[derive(Debug, Clone, Default)]
pub struct TxWitnessSet {
    /// Verification key witnesses (signatures)
    pub vkey_witnesses: Vec<VKeyWitness>,
    /// Plutus scripts
    pub plutus_scripts: Vec<PlutusScript>,
    /// Datum values (for datum hashes in outputs)
    pub datums: Vec<PlutusData>,
    /// Redeemers
    pub redeemers: Vec<Redeemer>,
}

/// Verification key witness
#[derive(Debug, Clone)]
pub struct VKeyWitness {
    /// Verification key (32 bytes)
    pub vkey: [u8; 32],
    /// Signature (64 bytes)
    pub signature: [u8; 64],
}

/// Plutus script (simplified)
#[derive(Debug, Clone)]
pub struct PlutusScript {
    /// Script version
    pub version: PlutusVersion,
    /// Compiled script bytes
    pub bytes: Vec<u8>,
}

/// Plutus version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlutusVersion {
    V1,
    V2,
    V3,
}

impl PlutusScript {
    /// Compute script hash
    pub fn hash(&self) -> ScriptHash {
        let mut hasher = Sha256::new();
        hasher.update(&[self.version as u8]);
        hasher.update(&self.bytes);
        let full_hash: [u8; 32] = hasher.finalize().into();
        let mut result = [0u8; 28];
        result.copy_from_slice(&full_hash[..28]);
        result
    }
}

/// Complete transaction
#[derive(Debug, Clone)]
pub struct Tx {
    /// Transaction body
    pub body: TxBody,
    /// Witnesses
    pub witness_set: TxWitnessSet,
    /// Is valid flag (for failed scripts)
    pub is_valid: bool,
}

impl Tx {
    /// Compute transaction hash
    pub fn hash(&self) -> TxHash {
        let mut hasher = Sha256::new();
        // Simplified: hash inputs and outputs
        for input in &self.body.inputs {
            hasher.update(&input.tx_hash);
            hasher.update(&input.index.to_le_bytes());
        }
        for output in &self.body.outputs {
            hasher.update(&output.value.coin.to_le_bytes());
        }
        hasher.update(&self.body.fee.to_le_bytes());
        hasher.finalize().into()
    }

    /// Check basic validity (not script validation)
    pub fn is_structurally_valid(&self) -> bool {
        // Inputs not empty
        if self.body.inputs.is_empty() {
            return false;
        }

        // Fee is positive
        if self.body.fee == 0 {
            return false;
        }

        // All outputs have positive value
        if self.body.outputs.iter().any(|o| o.value.coin == 0) {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_policy_id(id: u8) -> PolicyId {
        let mut arr = [0u8; 28];
        arr[0] = id;
        arr
    }

    fn make_address(id: u8) -> Address {
        let mut key_hash = [0u8; 28];
        key_hash[0] = id;
        Address {
            payment: Credential::PubKeyHash(key_hash),
            staking: None,
        }
    }

    #[test]
    fn test_value_add() {
        let mut v1 = Value::from_coin(1_000_000);
        v1.add_asset(make_policy_id(1), AssetName::from_string("Token").unwrap(), 100);

        let mut v2 = Value::from_coin(500_000);
        v2.add_asset(make_policy_id(1), AssetName::from_string("Token").unwrap(), 50);
        v2.add_asset(make_policy_id(2), AssetName::from_string("Other").unwrap(), 10);

        let result = v1.add(&v2);

        assert_eq!(result.coin, 1_500_000);
        assert_eq!(
            result.multi_asset[&make_policy_id(1)][&AssetName::from_string("Token").unwrap()],
            150
        );
        assert_eq!(
            result.multi_asset[&make_policy_id(2)][&AssetName::from_string("Other").unwrap()],
            10
        );
    }

    #[test]
    fn test_value_sub() {
        let mut v1 = Value::from_coin(1_000_000);
        v1.add_asset(make_policy_id(1), AssetName::from_string("Token").unwrap(), 100);

        let mut v2 = Value::from_coin(500_000);
        v2.add_asset(make_policy_id(1), AssetName::from_string("Token").unwrap(), 30);

        let result = v1.sub(&v2).unwrap();

        assert_eq!(result.coin, 500_000);
        assert_eq!(
            result.multi_asset[&make_policy_id(1)][&AssetName::from_string("Token").unwrap()],
            70
        );
    }

    #[test]
    fn test_utxo_operations() {
        let mut utxo = UTxO::new();

        let txin = TxIn::new([1u8; 32], 0);
        let txout = TxOut::simple(make_address(1), 5_000_000);

        utxo.insert(txin.clone(), txout);

        assert!(utxo.contains(&txin));
        assert_eq!(utxo.get(&txin).unwrap().value.coin, 5_000_000);

        utxo.remove(&txin);
        assert!(!utxo.contains(&txin));
    }

    #[test]
    fn test_plutus_data_hash() {
        let data1 = PlutusData::Integer(42);
        let data2 = PlutusData::Integer(42);
        let data3 = PlutusData::Integer(43);

        assert_eq!(data1.hash(), data2.hash());
        assert_ne!(data1.hash(), data3.hash());
    }

    #[test]
    fn test_script_locked_output() {
        let mut script_hash = [0u8; 28];
        script_hash[0] = 0xAB;

        let script_addr = Address {
            payment: Credential::ScriptHash(script_hash),
            staking: None,
        };

        let txout = TxOut::simple(script_addr, 1_000_000);
        assert!(txout.is_script_locked());

        let key_addr = make_address(1);
        let key_out = TxOut::simple(key_addr, 1_000_000);
        assert!(!key_out.is_script_locked());
    }

    #[test]
    fn test_ex_units_budget() {
        let units = ExUnits::new(1000, 5000);

        assert!(units.within_budget(2000, 10000));
        assert!(!units.within_budget(500, 10000));
        assert!(!units.within_budget(2000, 1000));
    }
}
