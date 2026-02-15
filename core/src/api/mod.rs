//! API layer
//!
//! This module provides interfaces for interacting with the blockchain.
//!
//! ## Available interfaces:
//! - **CLI** - Command-line interface (this implementation)
//! - **JSON-RPC** - Remote procedure calls (future)
//! - **REST** - HTTP API (future)
//!
//! ## Comparison:
//! - **Bitcoin**: CLI (bitcoin-cli) + JSON-RPC
//! - **Ethereum**: CLI (geth) + JSON-RPC + GraphQL
//! - **Kaspa**: CLI + gRPC + wRPC

pub mod cli;

pub use cli::{Cli, Command};
