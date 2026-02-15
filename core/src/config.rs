//! Configuration management
//!
//! This module handles node configuration including network address
//! and mining settings.

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::sync::RwLock;

/// Global configuration instance
pub static GLOBAL_CONFIG: Lazy<Config> = Lazy::new(Config::new);

/// Default node address
const DEFAULT_NODE_ADDR: &str = "127.0.0.1:2001";

/// Environment variable for node address
const NODE_ADDRESS_KEY: &str = "NODE_ADDRESS";

/// Configuration key for mining address
const MINING_ADDRESS_KEY: &str = "MINING_ADDRESS";

/// Node configuration
#[derive(Clone)]
pub struct Config {
    inner: std::sync::Arc<RwLock<HashMap<String, String>>>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Create a new configuration
    pub fn new() -> Self {
        let mut map = HashMap::new();

        // Get node address from environment or use default
        let node_addr = env::var(NODE_ADDRESS_KEY).unwrap_or_else(|_| DEFAULT_NODE_ADDR.to_string());
        map.insert(NODE_ADDRESS_KEY.to_string(), node_addr);

        Config {
            inner: std::sync::Arc::new(RwLock::new(map)),
        }
    }

    /// Get the node's network address
    pub fn get_node_addr(&self) -> String {
        let inner = self.inner.read().unwrap();
        inner.get(NODE_ADDRESS_KEY).unwrap().clone()
    }

    /// Set the mining address (enables mining mode)
    pub fn set_mining_addr(&self, addr: String) {
        let mut inner = self.inner.write().unwrap();
        inner.insert(MINING_ADDRESS_KEY.to_string(), addr);
    }

    /// Get the mining address if configured
    pub fn get_mining_addr(&self) -> Option<String> {
        let inner = self.inner.read().unwrap();
        inner.get(MINING_ADDRESS_KEY).cloned()
    }

    /// Check if this node is configured for mining
    pub fn is_miner(&self) -> bool {
        let inner = self.inner.read().unwrap();
        inner.contains_key(MINING_ADDRESS_KEY)
    }

    /// Set a custom configuration value
    pub fn set(&self, key: &str, value: String) {
        let mut inner = self.inner.write().unwrap();
        inner.insert(key.to_string(), value);
    }

    /// Get a configuration value
    pub fn get(&self, key: &str) -> Option<String> {
        let inner = self.inner.read().unwrap();
        inner.get(key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::new();
        assert!(!config.get_node_addr().is_empty());
    }

    #[test]
    fn test_mining_config() {
        let config = Config::new();
        assert!(!config.is_miner());

        config.set_mining_addr("test_address".to_string());
        assert!(config.is_miner());
        assert_eq!(config.get_mining_addr(), Some("test_address".to_string()));
    }

    #[test]
    fn test_custom_config() {
        let config = Config::new();
        config.set("custom_key", "custom_value".to_string());
        assert_eq!(config.get("custom_key"), Some("custom_value".to_string()));
    }
}
