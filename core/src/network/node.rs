//! P2P node management
//!
//! This module handles peer discovery and connection management.

use std::net::SocketAddr;
use std::sync::RwLock;

/// Represents a peer node in the network
#[derive(Clone, Debug)]
pub struct Node {
    /// Network address of the node
    addr: String,
}

impl Node {
    /// Create a new node with the given address
    pub fn new(addr: String) -> Self {
        Node { addr }
    }

    /// Get the node's address string
    pub fn get_addr(&self) -> &str {
        &self.addr
    }

    /// Parse the address as a SocketAddr
    pub fn parse_socket_addr(&self) -> Result<SocketAddr, std::net::AddrParseError> {
        self.addr.parse()
    }
}

/// Collection of known peer nodes
pub struct Nodes {
    inner: RwLock<Vec<Node>>,
}

impl Default for Nodes {
    fn default() -> Self {
        Self::new()
    }
}

impl Nodes {
    /// Create a new empty node collection
    pub fn new() -> Self {
        Nodes {
            inner: RwLock::new(vec![]),
        }
    }

    /// Add a new node if not already known
    pub fn add_node(&self, addr: String) {
        let mut inner = self.inner.write().unwrap();
        if !inner.iter().any(|x| x.get_addr() == addr) {
            inner.push(Node::new(addr));
        }
    }

    /// Remove a node by address
    pub fn evict_node(&self, addr: &str) {
        let mut inner = self.inner.write().unwrap();
        if let Some(idx) = inner.iter().position(|x| x.get_addr() == addr) {
            inner.remove(idx);
        }
    }

    /// Get the first node in the list
    pub fn first(&self) -> Option<Node> {
        self.inner.read().unwrap().first().cloned()
    }

    /// Get all known nodes
    pub fn get_nodes(&self) -> Vec<Node> {
        self.inner.read().unwrap().clone()
    }

    /// Get number of known nodes
    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    /// Check if there are no known nodes
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }

    /// Check if a node address is known
    pub fn node_is_known(&self, addr: &str) -> bool {
        self.inner.read().unwrap().iter().any(|x| x.get_addr() == addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new("127.0.0.1:8080".to_string());
        assert_eq!(node.get_addr(), "127.0.0.1:8080");
    }

    #[test]
    fn test_nodes_add_and_remove() {
        let nodes = Nodes::new();
        nodes.add_node("127.0.0.1:8080".to_string());
        nodes.add_node("127.0.0.1:8081".to_string());

        assert_eq!(nodes.len(), 2);
        assert!(nodes.node_is_known("127.0.0.1:8080"));

        nodes.evict_node("127.0.0.1:8080");
        assert_eq!(nodes.len(), 1);
        assert!(!nodes.node_is_known("127.0.0.1:8080"));
    }

    #[test]
    fn test_no_duplicate_nodes() {
        let nodes = Nodes::new();
        nodes.add_node("127.0.0.1:8080".to_string());
        nodes.add_node("127.0.0.1:8080".to_string());

        assert_eq!(nodes.len(), 1);
    }
}
