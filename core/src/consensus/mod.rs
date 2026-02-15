//! Consensus layer
//!
//! This module contains consensus mechanisms and block validation.
//!
//! ## Architecture:
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           Consensus Trait               │
//! │  (interface for all consensus types)    │
//! └───────────────┬─────────────────────────┘
//!                 │
//!     ┌───────────┼───────────┐
//!     ▼           ▼           ▼
//! ┌───────┐  ┌────────┐  ┌─────────┐
//! │  PoW  │  │  PoS   │  │ GHOSTDAG│
//! │       │  │(future)│  │ (future)│
//! └───────┘  └────────┘  └─────────┘
//! ```
//!
//! ## Comparison with other blockchains:
//!
//! | Blockchain | Consensus | Block Time | Finality |
//! |------------|-----------|------------|----------|
//! | Bitcoin | PoW | 10 min | Probabilistic |
//! | Ethereum | PoS | 12 sec | ~13 blocks |
//! | Kaspa | GHOSTDAG | 1 sec | Instant |
//! | Solana | PoH+BFT | 400ms | 100-150ms |
//! | This | PoW (pluggable) | Configurable | Probabilistic |

pub mod traits;
pub mod pow;
pub mod validator;

// Re-export commonly used types
pub use traits::{Consensus, ConsensusError, ConsensusResult, ChainSelector, LongestChainSelector};
pub use pow::{ProofOfWork, DEFAULT_TARGET_BITS};
pub use validator::{BlockValidator, ValidationError, ValidationResult};
