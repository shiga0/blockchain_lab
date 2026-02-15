//! Validator Set and Stake-weighted Sampling
//!
//! ## Avalanche Validator Selection
//!
//! Unlike traditional BFT (all validators participate), Avalanche samples
//! a small subset of validators each round. This enables O(k) message complexity.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Validator Set (N=1000)                   │
//! │  ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐ ...     │
//! │  │V1 │ │V2 │ │V3 │ │V4 │ │V5 │ │V6 │ │V7 │ │V8 │          │
//! │  │10%│ │8% │ │7% │ │6% │ │5% │ │4% │ │3% │ │2% │          │
//! │  └───┘ └───┘ └───┘ └───┘ └───┘ └───┘ └───┘ └───┘          │
//! └─────────────────────────────────────────────────────────────┘
//!                            ↓
//!              Stake-weighted random sample (k=20)
//!                            ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Sampled Validators                       │
//! │  ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐ ... (20 total)             │
//! │  │V1 │ │V3 │ │V5 │ │V12│ │V47│                             │
//! │  └───┘ └───┘ └───┘ └───┘ └───┘                             │
//! │  (Higher stake = higher probability of selection)          │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Comparison with Other Systems
//!
//! | System | Validator Selection | Message Complexity |
//! |--------|--------------------|--------------------|
//! | Tendermint | All validators | O(n²) |
//! | Avalanche | Random sample k | O(k) per round |
//! | Solana | Leader rotation | O(n) per slot |

use crate::constants::*;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Validator node ID (20 bytes like Ethereum address)
pub type NodeId = [u8; 20];

/// Stake amount (in nAVAX, 10^9 nAVAX = 1 AVAX)
pub type Stake = u64;

/// Validator in the network
#[derive(Debug, Clone)]
pub struct Validator {
    /// Node identifier
    pub node_id: NodeId,
    /// Stake amount
    pub stake: Stake,
    /// Whether the validator is currently active
    pub is_active: bool,
    /// Start time of validation period
    pub start_time: u64,
    /// End time of validation period
    pub end_time: u64,
}

impl Validator {
    pub fn new(node_id: NodeId, stake: Stake) -> Self {
        Self {
            node_id,
            stake,
            is_active: true,
            start_time: 0,
            end_time: u64::MAX,
        }
    }

    /// Create with specific validation period
    pub fn with_period(node_id: NodeId, stake: Stake, start_time: u64, end_time: u64) -> Self {
        Self {
            node_id,
            stake,
            is_active: true,
            start_time,
            end_time,
        }
    }
}

/// Set of validators with stake-weighted sampling
#[derive(Debug, Clone)]
pub struct ValidatorSet {
    /// Validators indexed by node ID
    validators: HashMap<NodeId, Validator>,
    /// Total stake in the set
    total_stake: Stake,
    /// Cached sorted list for sampling (node_id, cumulative_stake)
    cumulative_weights: Vec<(NodeId, Stake)>,
}

impl ValidatorSet {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            total_stake: 0,
            cumulative_weights: Vec::new(),
        }
    }

    /// Add a validator
    pub fn add(&mut self, validator: Validator) {
        self.total_stake += validator.stake;
        self.validators.insert(validator.node_id, validator);
        self.rebuild_weights();
    }

    /// Remove a validator
    pub fn remove(&mut self, node_id: &NodeId) -> Option<Validator> {
        if let Some(v) = self.validators.remove(node_id) {
            self.total_stake -= v.stake;
            self.rebuild_weights();
            Some(v)
        } else {
            None
        }
    }

    /// Get a validator by ID
    pub fn get(&self, node_id: &NodeId) -> Option<&Validator> {
        self.validators.get(node_id)
    }

    /// Get total stake
    pub fn total_stake(&self) -> Stake {
        self.total_stake
    }

    /// Get number of validators
    pub fn len(&self) -> usize {
        self.validators.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }

    /// Rebuild cumulative weights for sampling
    fn rebuild_weights(&mut self) {
        self.cumulative_weights.clear();
        let mut cumulative: Stake = 0;

        // Sort by node_id for deterministic ordering
        let mut validators: Vec<_> = self.validators.values().collect();
        validators.sort_by_key(|v| v.node_id);

        for v in validators {
            if v.is_active {
                cumulative += v.stake;
                self.cumulative_weights.push((v.node_id, cumulative));
            }
        }
    }

    /// Sample k validators using stake-weighted random selection
    ///
    /// Higher stake = higher probability of being selected.
    /// Returns unique validators (no duplicates).
    pub fn sample<R: Rng>(&self, k: usize, rng: &mut R) -> Vec<NodeId> {
        if self.cumulative_weights.is_empty() {
            return Vec::new();
        }

        let mut selected = Vec::with_capacity(k);
        let mut selected_set = std::collections::HashSet::new();

        // Sample k unique validators
        let max_attempts = k * 10; // Prevent infinite loop
        let mut attempts = 0;

        while selected.len() < k && attempts < max_attempts {
            attempts += 1;

            // Generate random stake value
            let target = rng.gen_range(0..self.total_stake);

            // Binary search for validator
            let idx = self
                .cumulative_weights
                .partition_point(|(_, cum)| *cum <= target);

            if idx < self.cumulative_weights.len() {
                let node_id = self.cumulative_weights[idx].0;
                if selected_set.insert(node_id) {
                    selected.push(node_id);
                }
            }
        }

        selected
    }

    /// Sample k validators with default sample size
    pub fn sample_k<R: Rng>(&self, rng: &mut R) -> Vec<NodeId> {
        self.sample(K, rng)
    }

    /// Get stake weight as fraction (0.0 - 1.0)
    pub fn weight(&self, node_id: &NodeId) -> f64 {
        if self.total_stake == 0 {
            return 0.0;
        }
        self.validators
            .get(node_id)
            .map(|v| v.stake as f64 / self.total_stake as f64)
            .unwrap_or(0.0)
    }

    /// Check if a set of validators has quorum (> 50% stake)
    pub fn has_quorum(&self, validators: &[NodeId]) -> bool {
        let stake: Stake = validators
            .iter()
            .filter_map(|id| self.validators.get(id))
            .map(|v| v.stake)
            .sum();

        stake * 2 > self.total_stake
    }

    /// Iterate over all validators
    pub fn iter(&self) -> impl Iterator<Item = &Validator> {
        self.validators.values()
    }
}

impl Default for ValidatorSet {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Delegator Support
// =============================================================================

/// Delegator who stakes to a validator
#[derive(Debug, Clone)]
pub struct Delegator {
    /// Delegator's address
    pub address: [u8; 20],
    /// Validator being delegated to
    pub validator_node_id: NodeId,
    /// Delegated stake amount
    pub stake: Stake,
    /// Start time
    pub start_time: u64,
    /// End time
    pub end_time: u64,
}

/// Validator with delegations
#[derive(Debug, Clone)]
pub struct ValidatorWithDelegators {
    /// The validator
    pub validator: Validator,
    /// Delegators to this validator
    pub delegators: Vec<Delegator>,
}

impl ValidatorWithDelegators {
    pub fn new(validator: Validator) -> Self {
        Self {
            validator,
            delegators: Vec::new(),
        }
    }

    /// Get total stake (validator + delegators)
    pub fn total_stake(&self) -> Stake {
        self.validator.stake + self.delegators.iter().map(|d| d.stake).sum::<Stake>()
    }

    /// Add a delegator
    pub fn add_delegator(&mut self, delegator: Delegator) {
        self.delegators.push(delegator);
    }
}

// =============================================================================
// Validator Manager (P-Chain style)
// =============================================================================

/// Manages validator staking operations
#[derive(Debug)]
pub struct ValidatorManager {
    /// Current validator set
    pub current_validators: ValidatorSet,
    /// Pending validators (waiting to be activated)
    pending_validators: Vec<Validator>,
    /// Minimum stake required (2000 AVAX for primary network)
    pub min_stake: Stake,
    /// Maximum stake allowed
    pub max_stake: Stake,
    /// Minimum delegation amount (25 AVAX)
    pub min_delegation: Stake,
}

impl ValidatorManager {
    /// Create with Avalanche mainnet parameters
    pub fn new() -> Self {
        Self {
            current_validators: ValidatorSet::new(),
            pending_validators: Vec::new(),
            min_stake: 2_000_000_000_000, // 2000 AVAX in nAVAX
            max_stake: 3_000_000_000_000_000, // 3M AVAX in nAVAX
            min_delegation: 25_000_000_000, // 25 AVAX in nAVAX
        }
    }

    /// Create with custom parameters (for testing)
    pub fn with_params(min_stake: Stake, max_stake: Stake, min_delegation: Stake) -> Self {
        Self {
            current_validators: ValidatorSet::new(),
            pending_validators: Vec::new(),
            min_stake,
            max_stake,
            min_delegation,
        }
    }

    /// Add a validator (staking transaction)
    pub fn add_validator(&mut self, validator: Validator) -> Result<(), &'static str> {
        if validator.stake < self.min_stake {
            return Err("Stake below minimum");
        }
        if validator.stake > self.max_stake {
            return Err("Stake above maximum");
        }
        if self.current_validators.get(&validator.node_id).is_some() {
            return Err("Validator already exists");
        }

        self.current_validators.add(validator);
        Ok(())
    }

    /// Remove a validator (unstaking)
    pub fn remove_validator(&mut self, node_id: &NodeId) -> Result<Validator, &'static str> {
        self.current_validators
            .remove(node_id)
            .ok_or("Validator not found")
    }

    /// Get the validator set for consensus
    pub fn validator_set(&self) -> &ValidatorSet {
        &self.current_validators
    }

    /// Compute validator set hash
    pub fn validators_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();

        let mut validators: Vec<_> = self.current_validators.iter().collect();
        validators.sort_by_key(|v| v.node_id);

        for v in validators {
            hasher.update(&v.node_id);
            hasher.update(&v.stake.to_le_bytes());
        }

        hasher.finalize().into()
    }
}

impl Default for ValidatorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn make_node_id(id: u8) -> NodeId {
        let mut arr = [0u8; 20];
        arr[0] = id;
        arr
    }

    #[test]
    fn test_validator_set_basic() {
        let mut vs = ValidatorSet::new();

        let v1 = Validator::new(make_node_id(1), 1000);
        let v2 = Validator::new(make_node_id(2), 2000);

        vs.add(v1);
        vs.add(v2);

        assert_eq!(vs.len(), 2);
        assert_eq!(vs.total_stake(), 3000);
        assert!(vs.get(&make_node_id(1)).is_some());
    }

    #[test]
    fn test_stake_weighted_sampling() {
        let mut vs = ValidatorSet::new();

        // V1 has 90% stake, V2 has 10%
        vs.add(Validator::new(make_node_id(1), 9000));
        vs.add(Validator::new(make_node_id(2), 1000));

        let mut rng = StdRng::seed_from_u64(42);

        // Sample many times and count
        let mut v1_count = 0;
        let mut v2_count = 0;

        for _ in 0..1000 {
            let sampled = vs.sample(1, &mut rng);
            if sampled[0] == make_node_id(1) {
                v1_count += 1;
            } else {
                v2_count += 1;
            }
        }

        // V1 should be selected much more often (roughly 9x)
        assert!(v1_count > v2_count * 5, "V1: {}, V2: {}", v1_count, v2_count);
    }

    #[test]
    fn test_sample_unique() {
        let mut vs = ValidatorSet::new();

        for i in 0..10 {
            vs.add(Validator::new(make_node_id(i), 1000));
        }

        let mut rng = StdRng::seed_from_u64(123);
        let sampled = vs.sample(5, &mut rng);

        // Should have 5 unique validators
        assert_eq!(sampled.len(), 5);

        let unique: std::collections::HashSet<_> = sampled.iter().collect();
        assert_eq!(unique.len(), 5);
    }

    #[test]
    fn test_quorum() {
        let mut vs = ValidatorSet::new();

        vs.add(Validator::new(make_node_id(1), 100));
        vs.add(Validator::new(make_node_id(2), 100));
        vs.add(Validator::new(make_node_id(3), 100));
        vs.add(Validator::new(make_node_id(4), 100));

        // 2 validators = 200 stake, not > 50% of 400
        assert!(!vs.has_quorum(&[make_node_id(1), make_node_id(2)]));

        // 3 validators = 300 stake, > 50% of 400
        assert!(vs.has_quorum(&[make_node_id(1), make_node_id(2), make_node_id(3)]));
    }

    #[test]
    fn test_validator_manager() {
        let mut manager = ValidatorManager::with_params(100, 10000, 10);

        let v1 = Validator::new(make_node_id(1), 1000);
        assert!(manager.add_validator(v1).is_ok());

        // Below minimum stake
        let v2 = Validator::new(make_node_id(2), 50);
        assert!(manager.add_validator(v2).is_err());

        // Duplicate
        let v3 = Validator::new(make_node_id(1), 1000);
        assert!(manager.add_validator(v3).is_err());
    }
}
