//! # Cosmos-style Implementation (Tendermint BFT + ABCI)
//!
//! This module implements Cosmos/CometBFT-specific features.
//!
//! ## Key Differences from Core
//!
//! | Aspect | Core (blockchain-lab-core) | Cosmos |
//! |--------|---------------------------|--------|
//! | Consensus | PoW | Tendermint BFT |
//! | Finality | Probabilistic | Instant (2/3+ precommits) |
//! | Block Time | Variable | ~1-7 seconds |
//! | Validator Selection | Mining | Stake-based |
//! | Application | Monolithic | ABCI (modular) |
//! | Chain Communication | None | IBC |
//!
//! ## Tendermint BFT Overview
//!
//! Tendermint is a Byzantine Fault Tolerant consensus protocol with instant finality.
//! It can tolerate up to 1/3 Byzantine (malicious) validators.
//!
//! ```text
//! Consensus Round Flow:
//!
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         Round N                                 │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  ┌──────────┐    ┌──────────┐    ┌───────────┐    ┌──────────┐│
//! │  │ Propose  │───→│ Prevote  │───→│ Precommit │───→│  Commit  ││
//! │  │          │    │          │    │           │    │          ││
//! │  │ Proposer │    │   All    │    │    All    │    │  2/3+    ││
//! │  │ creates  │    │validators│    │ validators│    │ commits  ││
//! │  │  block   │    │   vote   │    │   vote    │    │  block   ││
//! │  └──────────┘    └──────────┘    └───────────┘    └──────────┘│
//! │       │              │                │                │      │
//! │       │         Need 2/3+        Need 2/3+        Instant    │
//! │       │         prevotes        precommits       Finality    │
//! │       │              │                │                │      │
//! │       └──────────────┴────────────────┴────────────────┘      │
//! │                                                                 │
//! │  Timeout → New Round (increment round, possibly new proposer)  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Round Steps
//!
//! ```text
//! NewHeight → NewRound → Propose → Prevote → PrevoteWait
//!                                       ↓
//!                           Precommit → PrecommitWait → Commit
//!                                                         ↓
//!                                                    NewHeight
//! ```
//!
//! ## ABCI (Application Blockchain Interface)
//!
//! ABCI separates consensus from application logic:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    CometBFT (Consensus)                     │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │  P2P Network  │  Mempool  │  Consensus State Machine │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └────────────────────────┬────────────────────────────────────┘
//!                          │ ABCI Interface
//!                          ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Application (Cosmos SDK)                 │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │   x/bank   │   x/staking   │   x/gov   │   x/ibc   │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## ABCI Methods (Consensus Connection)
//!
//! 1. **InitChain**: Initialize at genesis
//! 2. **PrepareProposal**: Proposer builds block
//! 3. **ProcessProposal**: Validators validate block
//! 4. **FinalizeBlock**: Execute transactions
//! 5. **Commit**: Persist state, return app hash
//!
//! ## Modules
//!
//! - [`consensus`] - Tendermint BFT consensus
//! - [`types`] - Block, Vote, Validator, Commit structures
//! - [`abci`] - Application Blockchain Interface

pub mod consensus;
pub mod types;
pub mod abci;

/// Cosmos/CometBFT-specific constants
/// Reference: cometbft/config/config.go
pub mod constants {
    /// Default timeout for proposal phase (ms)
    pub const TIMEOUT_PROPOSE_MS: u64 = 3000;

    /// Increase in proposal timeout per round (ms)
    pub const TIMEOUT_PROPOSE_DELTA_MS: u64 = 500;

    /// Default timeout for prevote phase (ms)
    pub const TIMEOUT_PREVOTE_MS: u64 = 1000;

    /// Increase in prevote timeout per round (ms)
    pub const TIMEOUT_PREVOTE_DELTA_MS: u64 = 500;

    /// Default timeout for precommit phase (ms)
    pub const TIMEOUT_PRECOMMIT_MS: u64 = 1000;

    /// Increase in precommit timeout per round (ms)
    pub const TIMEOUT_PRECOMMIT_DELTA_MS: u64 = 500;

    /// Time between blocks (ms)
    pub const TIMEOUT_COMMIT_MS: u64 = 1000;

    /// Maximum block size in bytes (21MB)
    pub const MAX_BLOCK_SIZE_BYTES: u64 = 22_020_096;

    /// Maximum number of validators
    pub const MAX_VALIDATORS: usize = 10_000;

    /// Byzantine fault tolerance threshold (1/3)
    /// Must have > 2/3 votes for consensus
    pub const BFT_THRESHOLD: f64 = 2.0 / 3.0;

    /// Maximum evidence age in blocks
    pub const MAX_EVIDENCE_AGE_BLOCKS: u64 = 100_000;

    /// Maximum evidence age in duration (48 hours in seconds)
    pub const MAX_EVIDENCE_AGE_SECS: u64 = 48 * 60 * 60;
}
