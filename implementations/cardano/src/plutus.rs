//! Plutus Script Validation
//!
//! ## Script Validation Model
//!
//! ```text
//! Cardano Plutus:                 Ethereum EVM:
//! ┌────────────────────────┐      ┌────────────────────────┐
//! │ validator(             │      │ function transfer(     │
//! │   datum,               │      │   to,                  │
//! │   redeemer,            │      │   amount               │
//! │   script_context       │      │ ) {                    │
//! │ ) → Bool               │      │   state changes...     │
//! └────────────────────────┘      │ }                      │
//!                                 └────────────────────────┘
//!
//! Plutus: Pure validation (no state changes in script)
//! EVM: Imperative execution (state changes during execution)
//! ```
//!
//! ## Script Context
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     ScriptContext                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │ TxInfo:                                                     │
//! │   - inputs: [(TxOutRef, TxOut)]    // All inputs           │
//! │   - reference_inputs: [...]         // Read-only refs      │
//! │   - outputs: [TxOut]               // All outputs          │
//! │   - fee: Value                     // Transaction fee      │
//! │   - mint: Value                    // Minted/burned        │
//! │   - dcerts: [DCert]                // Certificates         │
//! │   - wdrl: Map<Credential, Int>     // Withdrawals          │
//! │   - valid_range: POSIXTimeRange    // Validity interval    │
//! │   - signatories: [PubKeyHash]      // Required signers     │
//! │   - redeemers: Map<Purpose, Data>  // All redeemers        │
//! │   - data: Map<DatumHash, Datum>    // All datums           │
//! │   - tx_id: TxId                    // Transaction hash     │
//! │                                                             │
//! │ Purpose:                                                    │
//! │   - Spending(TxOutRef)             // Which UTXO           │
//! │   - Minting(CurrencySymbol)        // Which policy         │
//! │   - Certifying(DCert)              // Which cert           │
//! │   - Rewarding(Credential)          // Which reward         │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Validation Flow
//!
//! ```text
//! 1. Collect all script purposes from transaction
//! 2. For each purpose:
//!    a. Find corresponding script
//!    b. Find datum (for spending)
//!    c. Find redeemer
//!    d. Build script context
//!    e. Execute: validator(datum, redeemer, ctx)
//!    f. Check result and accumulate execution units
//! 3. Verify total execution units within budget
//! ```

use crate::eutxo::{
    Address, Credential, DatumHash, ExUnits, Lovelace, PlutusData, PlutusScript, PolicyId,
    Redeemer, ScriptHash, ScriptPurpose, Tx, TxIn, TxOut, UTxO, Value,
};
use std::collections::HashMap;

// =============================================================================
// Script Context
// =============================================================================

/// Time range for validity
#[derive(Debug, Clone, Copy)]
pub struct TimeRange {
    /// Start time (POSIX milliseconds, inclusive)
    pub lower: Option<i64>,
    /// End time (POSIX milliseconds, exclusive)
    pub upper: Option<i64>,
}

impl TimeRange {
    pub fn always() -> Self {
        Self {
            lower: None,
            upper: None,
        }
    }

    pub fn from(start: i64) -> Self {
        Self {
            lower: Some(start),
            upper: None,
        }
    }

    pub fn until(end: i64) -> Self {
        Self {
            lower: None,
            upper: Some(end),
        }
    }

    pub fn between(start: i64, end: i64) -> Self {
        Self {
            lower: Some(start),
            upper: Some(end),
        }
    }

    /// Check if time is within range
    pub fn contains(&self, time: i64) -> bool {
        let after_start = self.lower.map_or(true, |l| time >= l);
        let before_end = self.upper.map_or(true, |u| time < u);
        after_start && before_end
    }
}

/// Transaction info available to scripts
#[derive(Debug, Clone)]
pub struct TxInfo {
    /// All inputs with their resolved outputs
    pub inputs: Vec<(TxIn, TxOut)>,
    /// Reference inputs (read-only)
    pub reference_inputs: Vec<(TxIn, TxOut)>,
    /// All outputs
    pub outputs: Vec<TxOut>,
    /// Transaction fee
    pub fee: Value,
    /// Minted/burned value
    pub mint: Value,
    /// Validity time range
    pub valid_range: TimeRange,
    /// Required signatories
    pub signatories: Vec<[u8; 28]>,
    /// All datums in transaction
    pub datums: HashMap<DatumHash, PlutusData>,
    /// Transaction hash
    pub tx_id: [u8; 32],
}

/// Full script context
#[derive(Debug, Clone)]
pub struct ScriptContext {
    /// Transaction info
    pub tx_info: TxInfo,
    /// Purpose of this script execution
    pub purpose: ScriptPurpose,
}

impl ScriptContext {
    /// Build script context from transaction and UTxO set
    pub fn build(
        tx: &Tx,
        utxo: &UTxO,
        purpose: ScriptPurpose,
        slot_to_time: impl Fn(u64) -> i64,
    ) -> Result<Self, &'static str> {
        // Resolve inputs
        let mut inputs = Vec::new();
        for txin in &tx.body.inputs {
            let txout = utxo.get(txin).ok_or("Input not found in UTxO")?;
            inputs.push((txin.clone(), txout.clone()));
        }

        // Resolve reference inputs
        let mut reference_inputs = Vec::new();
        for txin in &tx.body.reference_inputs {
            let txout = utxo.get(txin).ok_or("Reference input not found")?;
            reference_inputs.push((txin.clone(), txout.clone()));
        }

        // Build validity range
        let valid_range = TimeRange {
            lower: tx.body.validity_start.map(&slot_to_time),
            upper: tx.body.validity_end.map(&slot_to_time),
        };

        // Collect datums
        let mut datums = HashMap::new();
        for datum in &tx.witness_set.datums {
            datums.insert(datum.hash(), datum.clone());
        }

        // Build mint value
        let mut mint = Value::default();
        for (policy, assets) in &tx.body.mint {
            for (name, quantity) in assets {
                mint.add_asset(*policy, name.clone(), *quantity);
            }
        }

        let tx_info = TxInfo {
            inputs,
            reference_inputs,
            outputs: tx.body.outputs.clone(),
            fee: Value::from_coin(tx.body.fee),
            mint,
            valid_range,
            signatories: tx.body.required_signers.clone(),
            datums,
            tx_id: tx.hash(),
        };

        Ok(ScriptContext { tx_info, purpose })
    }

    /// Get datum for spending script
    pub fn get_spending_datum(&self) -> Option<&PlutusData> {
        if let ScriptPurpose::Spending(txin) = &self.purpose {
            // Find the input and its datum
            for (input, txout) in &self.tx_info.inputs {
                if input == txin {
                    match &txout.datum {
                        Some(crate::eutxo::Datum::Hash(hash)) => {
                            return self.tx_info.datums.get(hash);
                        }
                        Some(crate::eutxo::Datum::Inline(data)) => {
                            return Some(data);
                        }
                        None => return None,
                    }
                }
            }
        }
        None
    }

    /// Find own input (for spending scripts)
    pub fn find_own_input(&self) -> Option<&(TxIn, TxOut)> {
        if let ScriptPurpose::Spending(txin) = &self.purpose {
            self.tx_info
                .inputs
                .iter()
                .find(|(input, _)| input == txin)
        } else {
            None
        }
    }

    /// Check if a pub key hash has signed
    pub fn signed_by(&self, pkh: &[u8; 28]) -> bool {
        self.tx_info.signatories.contains(pkh)
    }
}

// =============================================================================
// Script Validation
// =============================================================================

/// Script validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// Script succeeded
    Success(ExUnits),
    /// Script failed with error
    Failure(String),
}

/// Script validator (simplified - actual implementation would use UPLC interpreter)
pub trait Validator {
    /// Validate script
    fn validate(
        &self,
        datum: Option<&PlutusData>,
        redeemer: &PlutusData,
        ctx: &ScriptContext,
    ) -> ValidationResult;
}

/// Built-in always succeeds validator (for testing)
#[derive(Debug, Clone)]
pub struct AlwaysSucceeds;

impl Validator for AlwaysSucceeds {
    fn validate(
        &self,
        _datum: Option<&PlutusData>,
        _redeemer: &PlutusData,
        _ctx: &ScriptContext,
    ) -> ValidationResult {
        ValidationResult::Success(ExUnits::new(1000, 1000))
    }
}

/// Built-in always fails validator (for testing)
#[derive(Debug, Clone)]
pub struct AlwaysFails;

impl Validator for AlwaysFails {
    fn validate(
        &self,
        _datum: Option<&PlutusData>,
        _redeemer: &PlutusData,
        _ctx: &ScriptContext,
    ) -> ValidationResult {
        ValidationResult::Failure("Always fails".to_string())
    }
}

/// Require signature validator
#[derive(Debug, Clone)]
pub struct RequireSignature {
    pub required_signer: [u8; 28],
}

impl Validator for RequireSignature {
    fn validate(
        &self,
        _datum: Option<&PlutusData>,
        _redeemer: &PlutusData,
        ctx: &ScriptContext,
    ) -> ValidationResult {
        if ctx.signed_by(&self.required_signer) {
            ValidationResult::Success(ExUnits::new(2000, 5000))
        } else {
            ValidationResult::Failure("Missing required signature".to_string())
        }
    }
}

/// Time-locked validator
#[derive(Debug, Clone)]
pub struct TimeLock {
    pub unlock_time: i64,
}

impl Validator for TimeLock {
    fn validate(
        &self,
        _datum: Option<&PlutusData>,
        _redeemer: &PlutusData,
        ctx: &ScriptContext,
    ) -> ValidationResult {
        // Check that current time is after unlock time
        let after_unlock = ctx
            .tx_info
            .valid_range
            .lower
            .map_or(false, |t| t >= self.unlock_time);

        if after_unlock {
            ValidationResult::Success(ExUnits::new(3000, 8000))
        } else {
            ValidationResult::Failure(format!(
                "Time lock not expired. Unlock at: {}",
                self.unlock_time
            ))
        }
    }
}

// =============================================================================
// Minting Policy
// =============================================================================

/// Minting policy validator
pub trait MintingPolicy {
    /// Validate minting/burning
    fn validate(&self, redeemer: &PlutusData, ctx: &ScriptContext) -> ValidationResult;
}

/// One-shot minting policy (can only mint once, when specific UTXO is spent)
#[derive(Debug, Clone)]
pub struct OneShotMint {
    /// Required input to consume
    pub required_input: TxIn,
}

impl MintingPolicy for OneShotMint {
    fn validate(&self, _redeemer: &PlutusData, ctx: &ScriptContext) -> ValidationResult {
        // Check that the required input is being spent
        let has_input = ctx
            .tx_info
            .inputs
            .iter()
            .any(|(txin, _)| txin == &self.required_input);

        if has_input {
            ValidationResult::Success(ExUnits::new(5000, 10000))
        } else {
            ValidationResult::Failure("Required UTXO not spent".to_string())
        }
    }
}

/// NFT minting policy (exactly one token)
#[derive(Debug, Clone)]
pub struct NftMint {
    /// Required UTXO (ensures uniqueness)
    pub required_utxo: TxIn,
    /// Expected token name
    pub token_name: crate::eutxo::AssetName,
}

impl MintingPolicy for NftMint {
    fn validate(&self, _redeemer: &PlutusData, ctx: &ScriptContext) -> ValidationResult {
        // Check required UTXO is spent
        let has_utxo = ctx
            .tx_info
            .inputs
            .iter()
            .any(|(txin, _)| txin == &self.required_utxo);

        if !has_utxo {
            return ValidationResult::Failure("Required UTXO not spent".to_string());
        }

        // Check minting exactly 1 of the NFT
        if let ScriptPurpose::Minting(policy_id) = &ctx.purpose {
            if let Some(assets) = ctx.tx_info.mint.multi_asset.get(policy_id) {
                if let Some(&quantity) = assets.get(&self.token_name) {
                    if quantity == 1 {
                        return ValidationResult::Success(ExUnits::new(8000, 15000));
                    }
                }
            }
        }

        ValidationResult::Failure("Must mint exactly 1 NFT".to_string())
    }
}

// =============================================================================
// Transaction Validation
// =============================================================================

/// Script registry for looking up validators
#[derive(Debug, Default)]
pub struct ScriptRegistry {
    scripts: HashMap<ScriptHash, PlutusScript>,
}

impl ScriptRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, script: PlutusScript) {
        let hash = script.hash();
        self.scripts.insert(hash, script);
    }

    pub fn get(&self, hash: &ScriptHash) -> Option<&PlutusScript> {
        self.scripts.get(hash)
    }
}

/// Validate all scripts in a transaction
pub fn validate_transaction(
    tx: &Tx,
    utxo: &UTxO,
    slot_to_time: impl Fn(u64) -> i64 + Copy,
    max_ex_units: ExUnits,
) -> Result<ExUnits, String> {
    let mut total_units = ExUnits::new(0, 0);

    for redeemer in &tx.witness_set.redeemers {
        // Build context for this purpose
        let ctx =
            ScriptContext::build(tx, utxo, redeemer.purpose.clone(), slot_to_time).map_err(|e| {
                format!(
                    "Failed to build context for {:?}: {}",
                    redeemer.purpose, e
                )
            })?;

        // Get datum for spending scripts
        let datum = ctx.get_spending_datum();

        // For this simplified implementation, we just check that:
        // 1. Redeemer exists
        // 2. Ex units are within budget
        if !redeemer.ex_units.within_budget(max_ex_units.mem, max_ex_units.cpu) {
            return Err(format!(
                "Execution units exceed budget for {:?}",
                redeemer.purpose
            ));
        }

        total_units.mem += redeemer.ex_units.mem;
        total_units.cpu += redeemer.ex_units.cpu;
    }

    if !total_units.within_budget(max_ex_units.mem, max_ex_units.cpu) {
        return Err("Total execution units exceed budget".to_string());
    }

    Ok(total_units)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pkh(id: u8) -> [u8; 28] {
        let mut arr = [0u8; 28];
        arr[0] = id;
        arr
    }

    #[test]
    fn test_time_range() {
        let always = TimeRange::always();
        assert!(always.contains(0));
        assert!(always.contains(i64::MAX));

        let from = TimeRange::from(100);
        assert!(!from.contains(99));
        assert!(from.contains(100));
        assert!(from.contains(1000));

        let until = TimeRange::until(100);
        assert!(until.contains(0));
        assert!(until.contains(99));
        assert!(!until.contains(100));

        let between = TimeRange::between(100, 200);
        assert!(!between.contains(99));
        assert!(between.contains(100));
        assert!(between.contains(150));
        assert!(!between.contains(200));
    }

    #[test]
    fn test_always_succeeds() {
        let validator = AlwaysSucceeds;
        let ctx = ScriptContext {
            tx_info: TxInfo {
                inputs: vec![],
                reference_inputs: vec![],
                outputs: vec![],
                fee: Value::from_coin(200_000),
                mint: Value::default(),
                valid_range: TimeRange::always(),
                signatories: vec![],
                datums: HashMap::new(),
                tx_id: [0u8; 32],
            },
            purpose: ScriptPurpose::Spending(TxIn::new([0u8; 32], 0)),
        };

        match validator.validate(None, &PlutusData::Integer(42), &ctx) {
            ValidationResult::Success(_) => {}
            ValidationResult::Failure(e) => panic!("Should succeed: {}", e),
        }
    }

    #[test]
    fn test_require_signature() {
        let required_pkh = make_pkh(1);
        let validator = RequireSignature {
            required_signer: required_pkh,
        };

        // Without signature
        let ctx_no_sig = ScriptContext {
            tx_info: TxInfo {
                inputs: vec![],
                reference_inputs: vec![],
                outputs: vec![],
                fee: Value::from_coin(200_000),
                mint: Value::default(),
                valid_range: TimeRange::always(),
                signatories: vec![],
                datums: HashMap::new(),
                tx_id: [0u8; 32],
            },
            purpose: ScriptPurpose::Spending(TxIn::new([0u8; 32], 0)),
        };

        match validator.validate(None, &PlutusData::Integer(0), &ctx_no_sig) {
            ValidationResult::Failure(_) => {}
            ValidationResult::Success(_) => panic!("Should fail without signature"),
        }

        // With signature
        let ctx_with_sig = ScriptContext {
            tx_info: TxInfo {
                inputs: vec![],
                reference_inputs: vec![],
                outputs: vec![],
                fee: Value::from_coin(200_000),
                mint: Value::default(),
                valid_range: TimeRange::always(),
                signatories: vec![required_pkh],
                datums: HashMap::new(),
                tx_id: [0u8; 32],
            },
            purpose: ScriptPurpose::Spending(TxIn::new([0u8; 32], 0)),
        };

        match validator.validate(None, &PlutusData::Integer(0), &ctx_with_sig) {
            ValidationResult::Success(_) => {}
            ValidationResult::Failure(e) => panic!("Should succeed with signature: {}", e),
        }
    }

    #[test]
    fn test_time_lock() {
        let unlock_time = 1000;
        let validator = TimeLock { unlock_time };

        // Before unlock time
        let ctx_before = ScriptContext {
            tx_info: TxInfo {
                inputs: vec![],
                reference_inputs: vec![],
                outputs: vec![],
                fee: Value::from_coin(200_000),
                mint: Value::default(),
                valid_range: TimeRange::from(500),
                signatories: vec![],
                datums: HashMap::new(),
                tx_id: [0u8; 32],
            },
            purpose: ScriptPurpose::Spending(TxIn::new([0u8; 32], 0)),
        };

        match validator.validate(None, &PlutusData::Integer(0), &ctx_before) {
            ValidationResult::Failure(_) => {}
            ValidationResult::Success(_) => panic!("Should fail before unlock time"),
        }

        // After unlock time
        let ctx_after = ScriptContext {
            tx_info: TxInfo {
                inputs: vec![],
                reference_inputs: vec![],
                outputs: vec![],
                fee: Value::from_coin(200_000),
                mint: Value::default(),
                valid_range: TimeRange::from(1500),
                signatories: vec![],
                datums: HashMap::new(),
                tx_id: [0u8; 32],
            },
            purpose: ScriptPurpose::Spending(TxIn::new([0u8; 32], 0)),
        };

        match validator.validate(None, &PlutusData::Integer(0), &ctx_after) {
            ValidationResult::Success(_) => {}
            ValidationResult::Failure(e) => panic!("Should succeed after unlock time: {}", e),
        }
    }
}
