//! Primitive data types
//!
//! This module contains the fundamental data structures of the blockchain.
//!
//! ## Core Primitives:
//! - `Block` - Container for transactions
//! - `Blockchain` - The chain of blocks
//!
//! ## Comparison:
//!
//! | Primitive | Bitcoin | Ethereum | Kaspa | This |
//! |-----------|---------|----------|-------|------|
//! | Block structure | Linear | Linear+uncles | DAG | Linear |
//! | Block time | 10 min | 12 sec | 1 sec | Configurable |
//! | Max block size | 1 MB | Gas limit | - | - |

pub mod block;
pub mod blockchain;

pub use block::{Block, current_timestamp};
pub use blockchain::{Blockchain, BlockchainIterator};
