//! Network message types
//!
//! This module defines the protocol messages exchanged between nodes.
//!
//! ## Message flow:
//! ```text
//! Node A                    Node B
//!   |                          |
//!   |------ Version ---------->|
//!   |<----- Version -----------|
//!   |                          |
//!   |------ GetBlocks -------->|
//!   |<----- Inv (blocks) ------|
//!   |                          |
//!   |------ GetData ---------->|
//!   |<----- Block -------------|
//! ```
//!
//! ## Comparison:
//! - **Bitcoin**: Similar message types (version, inv, getdata, block, tx)
//! - **Ethereum**: devp2p protocol with RLPx encoding
//! - **Kaspa**: gRPC-based protocol

use serde::{Deserialize, Serialize};

/// Type of data being referenced
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpType {
    /// Transaction
    Tx,
    /// Block
    Block,
}

/// Network protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Exchange version information
    Version {
        addr_from: String,
        version: usize,
        best_height: usize,
    },

    /// Request block hashes
    GetBlocks {
        addr_from: String,
    },

    /// Announce available data (blocks or transactions)
    Inv {
        addr_from: String,
        op_type: OpType,
        items: Vec<Vec<u8>>,
    },

    /// Request specific data
    GetData {
        addr_from: String,
        op_type: OpType,
        id: Vec<u8>,
    },

    /// Send a block
    Block {
        addr_from: String,
        block: Vec<u8>,
    },

    /// Send a transaction
    Tx {
        addr_from: String,
        transaction: Vec<u8>,
    },
}

impl Message {
    /// Get the sender address
    pub fn get_addr_from(&self) -> &str {
        match self {
            Message::Version { addr_from, .. } => addr_from,
            Message::GetBlocks { addr_from } => addr_from,
            Message::Inv { addr_from, .. } => addr_from,
            Message::GetData { addr_from, .. } => addr_from,
            Message::Block { addr_from, .. } => addr_from,
            Message::Tx { addr_from, .. } => addr_from,
        }
    }

    /// Serialize message to bytes
    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }

    /// Deserialize message from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_message() {
        let msg = Message::Version {
            addr_from: "127.0.0.1:8080".to_string(),
            version: 1,
            best_height: 100,
        };

        let serialized = msg.serialize();
        let deserialized = Message::deserialize(&serialized).unwrap();

        if let Message::Version { version, best_height, .. } = deserialized {
            assert_eq!(version, 1);
            assert_eq!(best_height, 100);
        } else {
            panic!("Wrong message type");
        }
    }

    #[test]
    fn test_inv_message() {
        let msg = Message::Inv {
            addr_from: "127.0.0.1:8080".to_string(),
            op_type: OpType::Block,
            items: vec![vec![1, 2, 3], vec![4, 5, 6]],
        };

        let serialized = msg.serialize();
        let deserialized = Message::deserialize(&serialized).unwrap();

        if let Message::Inv { items, .. } = deserialized {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Wrong message type");
        }
    }
}
