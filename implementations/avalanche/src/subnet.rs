//! Subnet and Chain Management
//!
//! ## Avalanche Subnet Architecture
//!
//! Subnets are independent networks with their own validators and consensus.
//! Each subnet can run multiple blockchains with different VMs.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        Primary Network                              │
//! │  (All validators must participate)                                  │
//! │                                                                     │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
//! │  │  P-Chain    │  │  X-Chain    │  │  C-Chain    │                 │
//! │  │  Platform   │  │  Exchange   │  │   EVM       │                 │
//! │  │  (Linear)   │  │   (DAG)     │  │  (Linear)   │                 │
//! │  │             │  │             │  │             │                 │
//! │  │ • Staking   │  │ • Assets    │  │ • Smart     │                 │
//! │  │ • Subnets   │  │ • Transfers │  │   Contracts │                 │
//! │  │ • Chains    │  │ • NFTs      │  │ • DeFi      │                 │
//! │  └─────────────┘  └─────────────┘  └─────────────┘                 │
//! └─────────────────────────────────────────────────────────────────────┘
//!                              ↓
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        Custom Subnets                               │
//! │  (Subset of validators, custom rules)                               │
//! │                                                                     │
//! │  ┌────────────────────┐      ┌────────────────────┐                │
//! │  │     Subnet A       │      │     Subnet B       │                │
//! │  │  (Gaming focused)  │      │  (DeFi focused)    │                │
//! │  │                    │      │                    │                │
//! │  │  Validators:       │      │  Validators:       │                │
//! │  │  V1, V3, V7        │      │  V2, V4, V5        │                │
//! │  │                    │      │                    │                │
//! │  │  ┌──────┐ ┌──────┐│      │  ┌──────────────┐  │                │
//! │  │  │Game  │ │NFT   ││      │  │  Lending     │  │                │
//! │  │  │Chain │ │Chain ││      │  │  Chain       │  │                │
//! │  │  │(EVM) │ │(custom)│     │  │  (custom VM) │  │                │
//! │  │  └──────┘ └──────┘│      │  └──────────────┘  │                │
//! │  └────────────────────┘      └────────────────────┘                │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Chain Types
//!
//! | Chain Type | Description | Consensus |
//! |------------|-------------|-----------|
//! | Platform (P) | Manages validators, subnets | Snowman (linear) |
//! | Exchange (X) | Asset transfers | Avalanche (DAG) |
//! | Contract (C) | EVM smart contracts | Snowman (linear) |
//! | Custom | User-defined VM | Snowman or Avalanche |

use crate::validator::{NodeId, ValidatorSet};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Subnet ID (32 bytes)
pub type SubnetId = [u8; 32];

/// Chain ID (32 bytes)
pub type ChainId = [u8; 32];

/// Primary Network subnet ID (all zeros represents primary network)
pub const PRIMARY_NETWORK_ID: SubnetId = [0u8; 32];

/// Built-in chain IDs for the Primary Network
pub mod chains {
    use super::ChainId;

    /// P-Chain (Platform Chain) - manages validators and subnets
    pub fn p_chain_id() -> ChainId {
        let mut id = [0u8; 32];
        id[0] = 0x50; // 'P'
        id
    }

    /// X-Chain (Exchange Chain) - asset transfers
    pub fn x_chain_id() -> ChainId {
        let mut id = [0u8; 32];
        id[0] = 0x58; // 'X'
        id
    }

    /// C-Chain (Contract Chain) - EVM compatible
    pub fn c_chain_id() -> ChainId {
        let mut id = [0u8; 32];
        id[0] = 0x43; // 'C'
        id
    }
}

/// Virtual Machine type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmType {
    /// Platform VM (P-Chain)
    Platform,
    /// Avalanche VM (X-Chain, DAG-based)
    Avm,
    /// Ethereum VM (C-Chain)
    Evm,
    /// Subnet EVM (customized EVM)
    SubnetEvm,
    /// Custom VM (user-defined)
    Custom(String),
}

impl VmType {
    /// Get VM identifier
    pub fn id(&self) -> &str {
        match self {
            VmType::Platform => "platformvm",
            VmType::Avm => "avm",
            VmType::Evm => "evm",
            VmType::SubnetEvm => "subnetevm",
            VmType::Custom(name) => name,
        }
    }
}

/// Blockchain configuration
#[derive(Debug, Clone)]
pub struct ChainConfig {
    /// Chain identifier
    pub chain_id: ChainId,
    /// Human-readable name
    pub name: String,
    /// Subnet this chain belongs to
    pub subnet_id: SubnetId,
    /// VM type
    pub vm_type: VmType,
    /// Genesis data
    pub genesis: Vec<u8>,
    /// Custom VM configuration
    pub vm_config: Option<Vec<u8>>,
}

impl ChainConfig {
    pub fn new(name: &str, subnet_id: SubnetId, vm_type: VmType, genesis: Vec<u8>) -> Self {
        let chain_id = Self::compute_id(name, &subnet_id, &vm_type);
        Self {
            chain_id,
            name: name.to_string(),
            subnet_id,
            vm_type,
            genesis,
            vm_config: None,
        }
    }

    /// Compute chain ID from parameters
    fn compute_id(name: &str, subnet_id: &SubnetId, vm_type: &VmType) -> ChainId {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(subnet_id);
        hasher.update(vm_type.id().as_bytes());
        hasher.finalize().into()
    }

    /// Check if this is a primary network chain
    pub fn is_primary_network(&self) -> bool {
        self.subnet_id == PRIMARY_NETWORK_ID
    }
}

/// Subnet configuration
#[derive(Debug, Clone)]
pub struct Subnet {
    /// Subnet identifier
    pub subnet_id: SubnetId,
    /// Human-readable name
    pub name: String,
    /// Owner address (for control operations)
    pub owner: [u8; 20],
    /// Validator node IDs
    validators: Vec<NodeId>,
    /// Chains in this subnet
    chains: Vec<ChainId>,
    /// Whether this is the primary network
    is_primary: bool,
}

impl Subnet {
    /// Create a new subnet
    pub fn new(name: &str, owner: [u8; 20]) -> Self {
        let subnet_id = Self::compute_id(name, &owner);
        Self {
            subnet_id,
            name: name.to_string(),
            owner,
            validators: Vec::new(),
            chains: Vec::new(),
            is_primary: false,
        }
    }

    /// Create the primary network subnet
    pub fn primary_network() -> Self {
        Self {
            subnet_id: PRIMARY_NETWORK_ID,
            name: "Primary Network".to_string(),
            owner: [0u8; 20],
            validators: Vec::new(),
            chains: vec![
                chains::p_chain_id(),
                chains::x_chain_id(),
                chains::c_chain_id(),
            ],
            is_primary: true,
        }
    }

    /// Compute subnet ID
    fn compute_id(name: &str, owner: &[u8; 20]) -> SubnetId {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(owner);
        hasher.finalize().into()
    }

    /// Add a validator to the subnet
    pub fn add_validator(&mut self, node_id: NodeId) {
        if !self.validators.contains(&node_id) {
            self.validators.push(node_id);
        }
    }

    /// Remove a validator
    pub fn remove_validator(&mut self, node_id: &NodeId) {
        self.validators.retain(|id| id != node_id);
    }

    /// Check if a node is a validator in this subnet
    pub fn is_validator(&self, node_id: &NodeId) -> bool {
        self.validators.contains(node_id)
    }

    /// Get validator count
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Get all validators
    pub fn validators(&self) -> &[NodeId] {
        &self.validators
    }

    /// Add a chain to the subnet
    pub fn add_chain(&mut self, chain_id: ChainId) {
        if !self.chains.contains(&chain_id) {
            self.chains.push(chain_id);
        }
    }

    /// Get all chains
    pub fn chains(&self) -> &[ChainId] {
        &self.chains
    }

    /// Check if this is the primary network
    pub fn is_primary_network(&self) -> bool {
        self.is_primary
    }
}

/// Manages subnets and chains
#[derive(Debug)]
pub struct SubnetManager {
    /// All subnets
    subnets: HashMap<SubnetId, Subnet>,
    /// All chains
    chains: HashMap<ChainId, ChainConfig>,
    /// Primary network validator set
    primary_validators: ValidatorSet,
}

impl SubnetManager {
    pub fn new() -> Self {
        let mut manager = Self {
            subnets: HashMap::new(),
            chains: HashMap::new(),
            primary_validators: ValidatorSet::new(),
        };

        // Initialize primary network
        let primary = Subnet::primary_network();
        manager.subnets.insert(primary.subnet_id, primary);

        // Add primary network chains
        manager.chains.insert(
            chains::p_chain_id(),
            ChainConfig::new("P-Chain", PRIMARY_NETWORK_ID, VmType::Platform, vec![]),
        );
        manager.chains.insert(
            chains::x_chain_id(),
            ChainConfig::new("X-Chain", PRIMARY_NETWORK_ID, VmType::Avm, vec![]),
        );
        manager.chains.insert(
            chains::c_chain_id(),
            ChainConfig::new("C-Chain", PRIMARY_NETWORK_ID, VmType::Evm, vec![]),
        );

        manager
    }

    /// Get the primary network
    pub fn primary_network(&self) -> Option<&Subnet> {
        self.subnets.get(&PRIMARY_NETWORK_ID)
    }

    /// Create a new subnet
    pub fn create_subnet(&mut self, name: &str, owner: [u8; 20]) -> SubnetId {
        let subnet = Subnet::new(name, owner);
        let subnet_id = subnet.subnet_id;
        self.subnets.insert(subnet_id, subnet);
        subnet_id
    }

    /// Get a subnet by ID
    pub fn get_subnet(&self, subnet_id: &SubnetId) -> Option<&Subnet> {
        self.subnets.get(subnet_id)
    }

    /// Get a mutable subnet by ID
    pub fn get_subnet_mut(&mut self, subnet_id: &SubnetId) -> Option<&mut Subnet> {
        self.subnets.get_mut(subnet_id)
    }

    /// Create a chain in a subnet
    pub fn create_chain(
        &mut self,
        name: &str,
        subnet_id: SubnetId,
        vm_type: VmType,
        genesis: Vec<u8>,
    ) -> Result<ChainId, &'static str> {
        // Check subnet exists
        let subnet = self
            .subnets
            .get_mut(&subnet_id)
            .ok_or("Subnet not found")?;

        // Create chain config
        let config = ChainConfig::new(name, subnet_id, vm_type, genesis);
        let chain_id = config.chain_id;

        // Add to subnet
        subnet.add_chain(chain_id);

        // Store chain config
        self.chains.insert(chain_id, config);

        Ok(chain_id)
    }

    /// Get a chain config
    pub fn get_chain(&self, chain_id: &ChainId) -> Option<&ChainConfig> {
        self.chains.get(chain_id)
    }

    /// Add validator to primary network
    pub fn add_primary_validator(&mut self, validator: crate::validator::Validator) {
        let node_id = validator.node_id;
        self.primary_validators.add(validator);

        if let Some(primary) = self.subnets.get_mut(&PRIMARY_NETWORK_ID) {
            primary.add_validator(node_id);
        }
    }

    /// Get primary network validators
    pub fn primary_validators(&self) -> &ValidatorSet {
        &self.primary_validators
    }

    /// Add validator to a subnet (must already be primary validator)
    pub fn add_subnet_validator(
        &mut self,
        subnet_id: &SubnetId,
        node_id: NodeId,
    ) -> Result<(), &'static str> {
        // Must be primary network validator
        if self.primary_validators.get(&node_id).is_none() {
            return Err("Must be primary network validator first");
        }

        let subnet = self
            .subnets
            .get_mut(subnet_id)
            .ok_or("Subnet not found")?;

        subnet.add_validator(node_id);
        Ok(())
    }

    /// List all subnets
    pub fn list_subnets(&self) -> Vec<&Subnet> {
        self.subnets.values().collect()
    }

    /// List all chains in a subnet
    pub fn list_chains(&self, subnet_id: &SubnetId) -> Vec<&ChainConfig> {
        self.chains
            .values()
            .filter(|c| c.subnet_id == *subnet_id)
            .collect()
    }
}

impl Default for SubnetManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Cross-Chain Messaging (Warp)
// =============================================================================

/// Cross-subnet message
#[derive(Debug, Clone)]
pub struct WarpMessage {
    /// Source chain
    pub source_chain: ChainId,
    /// Destination chain
    pub destination_chain: ChainId,
    /// Message payload
    pub payload: Vec<u8>,
    /// Aggregated signature from source subnet validators
    pub signature: Vec<u8>,
}

impl WarpMessage {
    pub fn new(source: ChainId, destination: ChainId, payload: Vec<u8>) -> Self {
        Self {
            source_chain: source,
            destination_chain: destination,
            payload,
            signature: Vec::new(),
        }
    }

    /// Compute message hash for signing
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.source_chain);
        hasher.update(&self.destination_chain);
        hasher.update(&self.payload);
        hasher.finalize().into()
    }

    /// Add aggregated signature
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = signature;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node_id(id: u8) -> NodeId {
        let mut arr = [0u8; 20];
        arr[0] = id;
        arr
    }

    #[test]
    fn test_primary_network() {
        let manager = SubnetManager::new();

        let primary = manager.primary_network().unwrap();
        assert!(primary.is_primary_network());
        assert_eq!(primary.chains().len(), 3);
    }

    #[test]
    fn test_create_subnet() {
        let mut manager = SubnetManager::new();

        let owner = [1u8; 20];
        let subnet_id = manager.create_subnet("Gaming Subnet", owner);

        let subnet = manager.get_subnet(&subnet_id).unwrap();
        assert_eq!(subnet.name, "Gaming Subnet");
        assert_eq!(subnet.owner, owner);
        assert!(!subnet.is_primary_network());
    }

    #[test]
    fn test_create_chain() {
        let mut manager = SubnetManager::new();

        let owner = [1u8; 20];
        let subnet_id = manager.create_subnet("DeFi Subnet", owner);

        let chain_id = manager
            .create_chain("Lending Chain", subnet_id, VmType::SubnetEvm, vec![1, 2, 3])
            .unwrap();

        let chain = manager.get_chain(&chain_id).unwrap();
        assert_eq!(chain.name, "Lending Chain");
        assert_eq!(chain.subnet_id, subnet_id);
        assert_eq!(chain.vm_type, VmType::SubnetEvm);
    }

    #[test]
    fn test_subnet_validators() {
        let mut manager = SubnetManager::new();

        // Add primary validator
        let validator = crate::validator::Validator::new(make_node_id(1), 1000);
        manager.add_primary_validator(validator);

        // Create subnet
        let subnet_id = manager.create_subnet("Test Subnet", [2u8; 20]);

        // Add to subnet (must be primary validator)
        assert!(manager
            .add_subnet_validator(&subnet_id, make_node_id(1))
            .is_ok());

        // Non-primary validator should fail
        assert!(manager
            .add_subnet_validator(&subnet_id, make_node_id(99))
            .is_err());

        let subnet = manager.get_subnet(&subnet_id).unwrap();
        assert_eq!(subnet.validator_count(), 1);
    }

    #[test]
    fn test_vm_types() {
        assert_eq!(VmType::Platform.id(), "platformvm");
        assert_eq!(VmType::Avm.id(), "avm");
        assert_eq!(VmType::Evm.id(), "evm");
        assert_eq!(VmType::Custom("myvm".to_string()).id(), "myvm");
    }

    #[test]
    fn test_warp_message() {
        let msg = WarpMessage::new(
            chains::c_chain_id(),
            chains::x_chain_id(),
            b"transfer".to_vec(),
        );

        assert_eq!(msg.source_chain, chains::c_chain_id());
        assert_eq!(msg.destination_chain, chains::x_chain_id());

        let hash1 = msg.hash();
        let hash2 = msg.hash();
        assert_eq!(hash1, hash2); // Deterministic
    }
}
