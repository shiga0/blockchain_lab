//! Command-line interface
//!
//! This module implements the CLI commands for interacting with the blockchain.
//!
//! ## Available commands:
//! - `createblockchain` - Create a new blockchain
//! - `createwallet` - Create a new wallet
//! - `getbalance` - Get wallet balance
//! - `listaddresses` - List all wallet addresses
//! - `send` - Send coins
//! - `printchain` - Print the blockchain
//! - `reindexutxo` - Rebuild UTXO index
//! - `startnode` - Start a P2P node

use clap::{Parser, Subcommand};

/// Blockchain CLI application
#[derive(Debug, Parser)]
#[command(name = "blockchain_base")]
#[command(about = "A reference blockchain implementation for learning")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Available CLI commands
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new blockchain with genesis block
    #[command(name = "createblockchain")]
    CreateBlockchain {
        /// Address to receive genesis block reward
        address: String,
    },

    /// Create a new wallet
    #[command(name = "createwallet")]
    CreateWallet,

    /// Get the balance of an address
    #[command(name = "getbalance")]
    GetBalance {
        /// Wallet address to check
        address: String,
    },

    /// List all wallet addresses
    #[command(name = "listaddresses")]
    ListAddresses,

    /// Send coins from one address to another
    #[command(name = "send")]
    Send {
        /// Sender's address
        from: String,
        /// Recipient's address
        to: String,
        /// Amount to send
        amount: i32,
        /// Mine immediately (1 = yes, 0 = no)
        #[arg(default_value = "0")]
        mine: usize,
    },

    /// Print all blocks in the blockchain
    #[command(name = "printchain")]
    PrintChain,

    /// Rebuild the UTXO index
    #[command(name = "reindexutxo")]
    ReindexUtxo,

    /// Start a P2P node
    #[command(name = "startnode")]
    StartNode {
        /// Mining address (enables mining mode)
        miner: Option<String>,
    },
}

impl Cli {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
