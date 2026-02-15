//! P2P server for network communication
//!
//! This module implements the TCP server for node communication.
//!
//! ## Architecture:
//! ```text
//! ┌─────────────────────────────────────┐
//! │           TCP Server                │
//! │  (listens for incoming connections) │
//! └──────────────┬──────────────────────┘
//!                │
//!      ┌─────────┴─────────┐
//!      ▼                   ▼
//! ┌─────────────┐   ┌─────────────┐
//! │  Handler 1  │   │  Handler 2  │
//! │  (thread)   │   │  (thread)   │
//! └─────────────┘   └─────────────┘
//! ```

use crate::network::message::{Message, OpType};
use crate::network::node::Nodes;
use crate::primitives::{Block, Blockchain};
use crate::execution::{MemoryPool, BlockInTransit, Transaction, UTXOSet};
use crate::config::Config;
use data_encoding::HEXLOWER;
use log::{error, info};
use once_cell::sync::Lazy;
use serde_json::Deserializer;
use std::error::Error;
use std::io::{BufReader, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

/// Protocol version
pub const NODE_VERSION: usize = 1;

/// Default central node address
pub const CENTRAL_NODE: &str = "127.0.0.1:2001";

/// Minimum transactions before mining
pub const TRANSACTION_THRESHOLD: usize = 2;

/// TCP write timeout in milliseconds
const TCP_WRITE_TIMEOUT: u64 = 1000;

// Global state (thread-safe)
static GLOBAL_NODES: Lazy<Nodes> = Lazy::new(|| {
    let nodes = Nodes::new();
    nodes.add_node(CENTRAL_NODE.to_string());
    nodes
});

static GLOBAL_MEMORY_POOL: Lazy<MemoryPool> = Lazy::new(MemoryPool::new);
static GLOBAL_BLOCKS_IN_TRANSIT: Lazy<BlockInTransit> = Lazy::new(BlockInTransit::new);

/// P2P Server
pub struct Server {
    blockchain: Blockchain,
    config: Config,
}

impl Server {
    /// Create a new server with the given blockchain
    pub fn new(blockchain: Blockchain, config: Config) -> Self {
        Server { blockchain, config }
    }

    /// Start the server
    pub fn run(&self, addr: &str) {
        let listener = TcpListener::bind(addr).expect("Failed to bind to address");
        info!("Server listening on {}", addr);

        // If not the central node, sync with central node
        if addr != CENTRAL_NODE {
            let best_height = self.blockchain.get_best_height();
            send_version(CENTRAL_NODE, best_height, &self.config);
        }

        // Accept connections
        for stream in listener.incoming() {
            let blockchain = self.blockchain.clone();
            let config = self.config.clone();

            thread::spawn(move || {
                match stream {
                    Ok(stream) => {
                        if let Err(e) = handle_connection(blockchain, stream, config) {
                            error!("Error handling connection: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Connection failed: {}", e);
                    }
                }
            });
        }
    }
}

/// Handle an incoming connection
fn handle_connection(
    blockchain: Blockchain,
    stream: TcpStream,
    config: Config,
) -> Result<(), Box<dyn Error>> {
    let peer_addr = stream.peer_addr()?;
    let reader = BufReader::new(&stream);
    let msg_reader = Deserializer::from_reader(reader).into_iter::<Message>();

    for msg in msg_reader {
        let msg = msg?;
        info!("Received from {}: {:?}", peer_addr, msg);

        match msg {
            Message::Version { addr_from, version: _, best_height } => {
                let local_height = blockchain.get_best_height();

                if local_height < best_height {
                    send_get_blocks(&addr_from, &config);
                }
                if local_height > best_height {
                    send_version(&addr_from, local_height, &config);
                }

                if !GLOBAL_NODES.node_is_known(&peer_addr.to_string()) {
                    GLOBAL_NODES.add_node(addr_from);
                }
            }

            Message::GetBlocks { addr_from } => {
                let blocks = blockchain.get_block_hashes();
                send_inv(&addr_from, OpType::Block, &blocks, &config);
            }

            Message::Inv { addr_from, op_type, items } => {
                match op_type {
                    OpType::Block => {
                        GLOBAL_BLOCKS_IN_TRANSIT.add_blocks(&items);
                        if let Some(block_hash) = items.first() {
                            send_get_data(&addr_from, OpType::Block, block_hash, &config);
                            GLOBAL_BLOCKS_IN_TRANSIT.remove(block_hash);
                        }
                    }
                    OpType::Tx => {
                        if let Some(txid) = items.first() {
                            let txid_hex = HEXLOWER.encode(txid);
                            if !GLOBAL_MEMORY_POOL.contains(&txid_hex) {
                                send_get_data(&addr_from, OpType::Tx, txid, &config);
                            }
                        }
                    }
                }
            }

            Message::GetData { addr_from, op_type, id } => {
                match op_type {
                    OpType::Block => {
                        if let Some(block) = blockchain.get_block(&id) {
                            send_block(&addr_from, &block, &config);
                        }
                    }
                    OpType::Tx => {
                        let txid_hex = HEXLOWER.encode(&id);
                        if let Some(tx) = GLOBAL_MEMORY_POOL.get(&txid_hex) {
                            send_tx(&addr_from, &tx, &config);
                        }
                    }
                }
            }

            Message::Block { addr_from, block } => {
                let block = Block::deserialize(&block);
                blockchain.add_block(&block);
                info!("Added block {}", block.get_hash());

                if !GLOBAL_BLOCKS_IN_TRANSIT.is_empty() {
                    if let Some(hash) = GLOBAL_BLOCKS_IN_TRANSIT.first() {
                        send_get_data(&addr_from, OpType::Block, &hash, &config);
                        GLOBAL_BLOCKS_IN_TRANSIT.remove(&hash);
                    }
                } else {
                    let utxo_set = UTXOSet::new(blockchain.get_db().clone());
                    let utxo_map = blockchain.find_utxo();
                    utxo_set.reindex(&utxo_map);
                }
            }

            Message::Tx { addr_from, transaction } => {
                let tx = Transaction::deserialize(&transaction);
                let txid = tx.get_id_bytes();
                GLOBAL_MEMORY_POOL.add(tx);

                let node_addr = config.get_node_addr();

                // Broadcast to other nodes if we're the central node
                if node_addr == CENTRAL_NODE {
                    for node in GLOBAL_NODES.get_nodes() {
                        if node.get_addr() == node_addr || node.get_addr() == addr_from {
                            continue;
                        }
                        send_inv(node.get_addr(), OpType::Tx, &[txid.clone()], &config);
                    }
                }

                // Mine if we have enough transactions and are a miner
                if GLOBAL_MEMORY_POOL.len() >= TRANSACTION_THRESHOLD {
                    if let Some(mining_addr) = config.get_mining_addr() {
                        let coinbase_tx = Transaction::new_coinbase_tx(&mining_addr);
                        let mut txs = GLOBAL_MEMORY_POOL.get_all();
                        txs.push(coinbase_tx);

                        let new_block = blockchain.mine_block(&txs);
                        let utxo_set = UTXOSet::new(blockchain.get_db().clone());
                        let utxo_map = blockchain.find_utxo();
                        utxo_set.reindex(&utxo_map);
                        info!("Mined new block: {}", new_block.get_hash());

                        // Clear mined transactions
                        for tx in &txs {
                            let txid_hex = HEXLOWER.encode(tx.get_id());
                            GLOBAL_MEMORY_POOL.remove(&txid_hex);
                        }

                        // Broadcast new block
                        for node in GLOBAL_NODES.get_nodes() {
                            if node.get_addr() == node_addr {
                                continue;
                            }
                            send_inv(
                                node.get_addr(),
                                OpType::Block,
                                &[new_block.get_hash_bytes()],
                                &config,
                            );
                        }
                    }
                }
            }
        }
    }

    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

// Helper functions for sending messages
fn send_data(addr: &str, msg: Message, _config: &Config) {
    let socket_addr: SocketAddr = match addr.parse() {
        Ok(a) => a,
        Err(_) => {
            error!("Invalid address: {}", addr);
            return;
        }
    };

    info!("Sending to {}: {:?}", addr, msg);

    match TcpStream::connect(socket_addr) {
        Ok(mut stream) => {
            let _ = stream.set_write_timeout(Some(Duration::from_millis(TCP_WRITE_TIMEOUT)));
            let _ = serde_json::to_writer(&stream, &msg);
            let _ = stream.flush();
        }
        Err(e) => {
            error!("Failed to connect to {}: {}", addr, e);
            GLOBAL_NODES.evict_node(addr);
        }
    }
}

fn send_version(addr: &str, height: usize, config: &Config) {
    let msg = Message::Version {
        addr_from: config.get_node_addr(),
        version: NODE_VERSION,
        best_height: height,
    };
    send_data(addr, msg, config);
}

fn send_get_blocks(addr: &str, config: &Config) {
    let msg = Message::GetBlocks {
        addr_from: config.get_node_addr(),
    };
    send_data(addr, msg, config);
}

fn send_inv(addr: &str, op_type: OpType, items: &[Vec<u8>], config: &Config) {
    let msg = Message::Inv {
        addr_from: config.get_node_addr(),
        op_type,
        items: items.to_vec(),
    };
    send_data(addr, msg, config);
}

fn send_get_data(addr: &str, op_type: OpType, id: &[u8], config: &Config) {
    let msg = Message::GetData {
        addr_from: config.get_node_addr(),
        op_type,
        id: id.to_vec(),
    };
    send_data(addr, msg, config);
}

fn send_block(addr: &str, block: &Block, config: &Config) {
    let msg = Message::Block {
        addr_from: config.get_node_addr(),
        block: block.serialize(),
    };
    send_data(addr, msg, config);
}

/// Send a transaction to a node (public API)
pub fn send_tx(addr: &str, tx: &Transaction, config: &Config) {
    let msg = Message::Tx {
        addr_from: config.get_node_addr(),
        transaction: tx.serialize(),
    };
    send_data(addr, msg, config);
}
