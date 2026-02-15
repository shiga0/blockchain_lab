//! # Cardano-style Implementation (Ouroboros PoS + eUTXO + Plutus)
//!
//! This module implements Cardano-specific features.
//!
//! ## Key Differences from Other Chains
//!
//! | Aspect | Bitcoin | Ethereum | Cardano |
//! |--------|---------|----------|---------|
//! | Data Model | UTXO | Account | Extended UTXO (eUTXO) |
//! | Consensus | PoW | PoS (Casper) | PoS (Ouroboros Praos) |
//! | Smart Contracts | Script | EVM | Plutus (validator scripts) |
//! | Native Assets | No | ERC-20 tokens | Ledger-native multi-asset |
//! | Finality | Probabilistic | Economic | Probabilistic (~2160 slots) |
//!
//! ## Extended UTXO Model
//!
//! ```text
//! Bitcoin UTXO:                    Cardano eUTXO:
//! ┌──────────────────┐             ┌──────────────────────────────┐
//! │ TxOut            │             │ TxOut                        │
//! ├──────────────────┤             ├──────────────────────────────┤
//! │ value: Satoshi   │             │ value: Value (multi-asset)   │
//! │ script: P2PKH    │             │ address: Address             │
//! └──────────────────┘             │ datum: Option<Datum>         │
//!                                  │ reference_script: Option<Script>│
//!                                  └──────────────────────────────┘
//!
//! To spend a Cardano eUTXO with a Plutus script:
//! ┌──────────────────────────────────────────────────────────────┐
//! │ Transaction                                                  │
//! ├──────────────────────────────────────────────────────────────┤
//! │ Inputs:                                                      │
//! │   - TxIn: (txid, index)                                     │
//! │   - Redeemer: spending authorization data                   │
//! │                                                              │
//! │ Outputs:                                                     │
//! │   - TxOut: (address, value, datum?)                        │
//! │                                                              │
//! │ Witnesses:                                                   │
//! │   - Scripts: validator scripts                              │
//! │   - Datums: datum values (for datum hashes)                 │
//! │   - Redeemers: (purpose, data, ex_units)                    │
//! └──────────────────────────────────────────────────────────────┘
//!
//! Script validation:
//!   validator(datum, redeemer, script_context) → Bool
//! ```
//!
//! ## Ouroboros Praos Consensus
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                         Epoch                               │
//! │  (432,000 slots = ~5 days)                                  │
//! ├─────────────────────────────────────────────────────────────┤
//! │ Slot 0   Slot 1   Slot 2   ...   Slot 431,999              │
//! │   │        │        │               │                      │
//! │   ▼        ▼        ▼               ▼                      │
//! │ [Block]  [empty]  [Block]  ...   [Block]                   │
//! └─────────────────────────────────────────────────────────────┘
//!
//! Leader Selection (per slot):
//!   1. Compute VRF with slot nonce + stake pool key
//!   2. Compare VRF output to threshold based on stake
//!   3. If VRF < threshold → pool is slot leader
//!   4. Multiple leaders possible (resolved by chain selection)
//! ```
//!
//! ## Native Multi-Asset
//!
//! ```text
//! Value = Coin + MultiAsset
//!
//! ┌────────────────────────────────────────────────────┐
//! │ Value                                              │
//! ├────────────────────────────────────────────────────┤
//! │ coin: 1000000 lovelace (1 ADA)                    │
//! │ multi_asset:                                       │
//! │   PolicyID_A:                                      │
//! │     "TokenA": 100                                  │
//! │     "TokenB": 50                                   │
//! │   PolicyID_B:                                      │
//! │     "NFT001": 1                                    │
//! └────────────────────────────────────────────────────┘
//!
//! PolicyID = Hash of minting policy script
//! - No smart contract needed for transfers
//! - Minting/burning controlled by policy script
//! ```
//!
//! ## Plutus Script Purposes
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ ScriptPurpose                                               │
//! ├─────────────────────────────────────────────────────────────┤
//! │ Spending(TxIn)       - Validate UTXO consumption           │
//! │ Minting(PolicyID)    - Validate token mint/burn            │
//! │ Certifying(DCert)    - Validate stake delegation           │
//! │ Rewarding(StakeAddr) - Validate reward withdrawal          │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`eutxo`] - Extended UTXO model (TxIn, TxOut, Value, Datum)
//! - [`ouroboros`] - Ouroboros Praos consensus
//! - [`plutus`] - Plutus script validation

pub mod eutxo;
pub mod ouroboros;
pub mod plutus;

/// Cardano-specific constants
pub mod constants {
    /// Slot duration in seconds (Shelley onwards)
    pub const SLOT_DURATION_SECS: u64 = 1;

    /// Slots per epoch (Shelley onwards: ~5 days)
    pub const SLOTS_PER_EPOCH: u64 = 432_000;

    /// Security parameter k (max rollback depth)
    pub const SECURITY_PARAMETER: u64 = 2160;

    /// Active slot coefficient (f) - probability of slot having a leader
    /// Mainnet: 0.05 (5% of slots have blocks)
    pub const ACTIVE_SLOT_COEFF: f64 = 0.05;

    /// Lovelace per ADA
    pub const LOVELACE_PER_ADA: u64 = 1_000_000;

    /// Minimum UTXO value (prevents dust)
    pub const MIN_UTXO_VALUE: u64 = 1_000_000; // 1 ADA

    /// Maximum transaction size in bytes
    pub const MAX_TX_SIZE: usize = 16_384;

    /// Maximum block size in bytes
    pub const MAX_BLOCK_SIZE: usize = 90_112;

    /// Maximum execution units per transaction (CPU steps)
    pub const MAX_TX_EX_UNITS_CPU: u64 = 14_000_000_000;

    /// Maximum execution units per transaction (Memory)
    pub const MAX_TX_EX_UNITS_MEM: u64 = 10_000_000;
}
