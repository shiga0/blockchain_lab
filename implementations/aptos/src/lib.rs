//! # Aptos-style Blockchain Implementation
//!
//! Aptos は Move 言語ベースの高性能 Layer 1 ブロックチェーン。
//! 主な特徴:
//!
//! 1. **AptosBFT**: DAG ベースの BFT コンセンサス (DiemBFT/Jolteon の進化形)
//! 2. **Block-STM**: 楽観的並列実行エンジン
//! 3. **Account/Resource Model**: Move のリソース型によるアカウントモデル
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         Aptos Architecture                               │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                 │
//! │  │   Client    │───▶│   Mempool   │───▶│  Consensus  │                 │
//! │  └─────────────┘    └─────────────┘    │  (AptosBFT) │                 │
//! │                                        └──────┬──────┘                 │
//! │                                               │                         │
//! │                                               ▼                         │
//! │                                    ┌─────────────────┐                  │
//! │                                    │   Block-STM     │                  │
//! │                                    │  (Parallel Exec)│                  │
//! │                                    └────────┬────────┘                  │
//! │                                             │                           │
//! │                           ┌─────────────────┼─────────────────┐         │
//! │                           ▼                 ▼                 ▼         │
//! │                    ┌───────────┐     ┌───────────┐     ┌───────────┐   │
//! │                    │  Move VM  │     │ MVHashMap │     │  Storage  │   │
//! │                    └───────────┘     └───────────┘     └───────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## AptosBFT Consensus (DAG-based)
//!
//! ```text
//! DAG Structure (similar to Sui Mysticeti):
//!
//! Round 3:    [N3_0]───────[N3_1]───────[N3_2]───────[N3_3]
//!               │  ╲         │  ╲         │  ╲         │
//! Round 2:    [N2_0]───────[N2_1]───────[N2_2]───────[N2_3]
//!               │  ╲         │  ╲         │  ╲         │
//! Round 1:    [N1_0]───────[N1_1]───────[N1_2]───────[N1_3]
//!               │           │           │           │
//! Genesis:    [G_0]       [G_1]       [G_2]       [G_3]
//!
//! Node = (epoch, round, author) + payload + parents
//! NodeCertificate = NodeMetadata + AggregateSignature (2f+1)
//! ```
//!
//! ## Block-STM Parallel Execution
//!
//! ```text
//! Optimistic Concurrency Control:
//!
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         Block-STM Execution                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  Transactions: [tx_1, tx_2, tx_3, tx_4, tx_5, ...]                     │
//! │                    │      │      │      │      │                        │
//! │                    ▼      ▼      ▼      ▼      ▼                        │
//! │               ┌──────────────────────────────────┐                      │
//! │               │     Parallel Execution Phase     │                      │
//! │               │  (Execute optimistically)        │                      │
//! │               └──────────────────────────────────┘                      │
//! │                    │      │      │      │      │                        │
//! │                    ▼      ▼      ▼      ▼      ▼                        │
//! │               ┌──────────────────────────────────┐                      │
//! │               │      MVHashMap (Multi-Version)   │                      │
//! │               │  key -> [(txn_idx, incarnation, value), ...]           │
//! │               └──────────────────────────────────┘                      │
//! │                    │      │      │      │      │                        │
//! │                    ▼      ▼      ▼      ▼      ▼                        │
//! │               ┌──────────────────────────────────┐                      │
//! │               │      Validation Phase            │                      │
//! │               │  (Check read-set versions)       │                      │
//! │               └──────────────────────────────────┘                      │
//! │                         │                                               │
//! │                         ▼                                               │
//! │               ┌─────────────────┐                                       │
//! │               │ Validation OK?  │                                       │
//! │               └────────┬────────┘                                       │
//! │                   Yes / │ ╲ No                                          │
//! │                      │  │   │                                           │
//! │               Commit ▼  │   ▼ Re-execute (incarnation++)               │
//! │                         │                                               │
//! └─────────────────────────────────────────────────────────────────────────┘
//!
//! Incarnation: Each transaction may execute multiple times
//!   - incarnation 0: Initial execution
//!   - incarnation 1: Re-execution after conflict
//!   - ...
//! ```
//!
//! ## Account/Resource Model
//!
//! ```text
//! Account Structure:
//!
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  Account (0x1234...abcd)                                                │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  Resources:                                                             │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │ 0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>                │   │
//! │  │   └── coin: Coin { value: 1000000 }                             │   │
//! │  │   └── frozen: false                                             │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │ 0x1::account::Account                                           │   │
//! │  │   └── authentication_key: 0x1234...                             │   │
//! │  │   └── sequence_number: 42                                       │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! │  Modules (if any):                                                      │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │ my_module (bytecode)                                            │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//!
//! Key differences from Sui:
//! - Account-centric (not object-centric)
//! - Resources live under accounts
//! - Global storage keyed by (address, type)
//! - Sequence numbers for replay protection
//! ```

pub mod account;
pub mod aptos_bft;
pub mod block_stm;

// Re-exports
pub use account::*;
pub use aptos_bft::*;
pub use block_stm::*;

/// Aptos epoch duration in milliseconds (~2 hours)
pub const EPOCH_DURATION_MS: u64 = 7_200_000;

/// Block time in milliseconds (~1 second)
pub const BLOCK_TIME_MS: u64 = 1_000;

/// Maximum transactions per block
pub const MAX_TRANSACTIONS_PER_BLOCK: usize = 10_000;

/// Maximum gas per block
pub const MAX_GAS_PER_BLOCK: u64 = 100_000_000;

/// Account address length in bytes
pub const ADDRESS_LENGTH: usize = 32;

/// BFT threshold (2f+1 out of 3f+1 validators)
pub const BFT_THRESHOLD_NUMERATOR: u64 = 2;
pub const BFT_THRESHOLD_DENOMINATOR: u64 = 3;
