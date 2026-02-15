//! Network layer
//!
//! This module handles P2P communication between nodes.
//!
//! ## Architecture:
//! ```text
//! ┌─────────────────────────────────────────┐
//! │              Server                     │
//! │  (TCP listener, connection handling)    │
//! └──────────────────┬──────────────────────┘
//!                    │
//! ┌──────────────────┴──────────────────────┐
//! │            Message Protocol             │
//! │  (Version, GetBlocks, Inv, GetData...)  │
//! └──────────────────┬──────────────────────┘
//!                    │
//! ┌──────────────────┴──────────────────────┐
//! │             Node Manager                │
//! │  (peer discovery, connection pool)      │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Comparison:
//! - **Bitcoin**: TCP with custom binary protocol
//! - **Ethereum**: devp2p with RLPx encryption
//! - **Kaspa**: gRPC with protobuf
//! - **This implementation**: TCP with JSON serialization

pub mod node;
pub mod message;
pub mod server;

pub use node::{Node, Nodes};
pub use message::{Message, OpType};
pub use server::{Server, send_tx, CENTRAL_NODE, NODE_VERSION, TRANSACTION_THRESHOLD};
