//! Cosmos/CometBFT Type Definitions
//!
//! ## Block Structure Comparison
//!
//! | Component | Bitcoin | Ethereum | Cosmos |
//! |-----------|---------|----------|--------|
//! | Header | prev_hash, merkle, nonce | parent, state_root | last_block_id, validators_hash, app_hash |
//! | Body | transactions | transactions | transactions + evidence + last_commit |
//! | Finality Proof | None (PoW) | None (PoS votes in next block) | LastCommit (2/3+ signatures) |
//!
//! ## Validator Set
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │                   Validator Set                        │
//! ├────────────────────────────────────────────────────────┤
//! │ Validator 1: Address=0x1234, VotingPower=1000         │
//! │ Validator 2: Address=0x5678, VotingPower=800          │
//! │ Validator 3: Address=0xABCD, VotingPower=500          │
//! │ Validator 4: Address=0xEF01, VotingPower=200          │
//! │                                                        │
//! │ Total Voting Power: 2500                               │
//! │ 2/3 Threshold: 1667                                    │
//! │ Current Proposer: Validator 1 (highest priority)       │
//! └────────────────────────────────────────────────────────┘
//! ```

use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

/// 32-byte hash
pub type Hash = [u8; 32];

/// 20-byte address (like Ethereum)
pub type Address = [u8; 20];

/// Block ID - uniquely identifies a block
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlockId {
    /// Block hash
    pub hash: Hash,
    /// Part set header (for block part gossipping)
    pub part_set_hash: Hash,
    /// Number of parts
    pub part_set_total: u32,
}

impl BlockId {
    pub fn is_zero(&self) -> bool {
        self.hash == [0u8; 32]
    }
}

/// Block header
///
/// Reference: cometbft/types/block.go
#[derive(Debug, Clone)]
pub struct Header {
    /// Chain identifier
    pub chain_id: String,
    /// Block height
    pub height: i64,
    /// Block timestamp
    pub time: u64,

    // Previous block info
    /// Previous block ID
    pub last_block_id: BlockId,

    // Hashes of block data
    /// Hash of LastCommit
    pub last_commit_hash: Hash,
    /// Merkle root of transactions
    pub data_hash: Hash,

    // Hashes from app (result of previous block)
    /// Hash of current validators
    pub validators_hash: Hash,
    /// Hash of next block's validators
    pub next_validators_hash: Hash,
    /// Hash of consensus parameters
    pub consensus_hash: Hash,
    /// Application state root hash (from Commit)
    pub app_hash: Hash,
    /// Merkle root of transaction results
    pub last_results_hash: Hash,

    // Consensus info
    /// Hash of evidence
    pub evidence_hash: Hash,
    /// Address of block proposer
    pub proposer_address: Address,
}

impl Header {
    /// Compute header hash
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();

        hasher.update(self.chain_id.as_bytes());
        hasher.update(&self.height.to_le_bytes());
        hasher.update(&self.time.to_le_bytes());
        hasher.update(&self.last_block_id.hash);
        hasher.update(&self.last_commit_hash);
        hasher.update(&self.data_hash);
        hasher.update(&self.validators_hash);
        hasher.update(&self.next_validators_hash);
        hasher.update(&self.consensus_hash);
        hasher.update(&self.app_hash);
        hasher.update(&self.last_results_hash);
        hasher.update(&self.evidence_hash);
        hasher.update(&self.proposer_address);

        hasher.finalize().into()
    }
}

/// Transaction data in a block
#[derive(Debug, Clone, Default)]
pub struct Data {
    /// Transactions
    pub txs: Vec<Vec<u8>>,
}

impl Data {
    /// Compute data hash (Merkle root of transactions)
    pub fn hash(&self) -> Hash {
        if self.txs.is_empty() {
            return [0u8; 32];
        }

        // Simplified: just hash all txs together
        let mut hasher = Sha256::new();
        for tx in &self.txs {
            let tx_hash: [u8; 32] = Sha256::digest(tx).into();
            hasher.update(&tx_hash);
        }
        hasher.finalize().into()
    }
}

/// Evidence of Byzantine behavior
#[derive(Debug, Clone)]
pub struct Evidence {
    /// Type of evidence
    pub evidence_type: EvidenceType,
    /// Height at which evidence occurred
    pub height: i64,
    /// Validator address
    pub validator: Address,
}

/// Types of Byzantine evidence
#[derive(Debug, Clone)]
pub enum EvidenceType {
    /// Validator signed two different blocks at same height
    DuplicateVote,
    /// Light client attack evidence
    LightClientAttack,
}

/// Evidence data in a block
#[derive(Debug, Clone, Default)]
pub struct EvidenceData {
    pub evidence: Vec<Evidence>,
}

/// Commit contains 2/3+ signatures for a block
///
/// This provides instant finality proof.
#[derive(Debug, Clone)]
pub struct Commit {
    /// Block height
    pub height: i64,
    /// Round number
    pub round: i32,
    /// Block being committed
    pub block_id: BlockId,
    /// Signatures from validators
    pub signatures: Vec<CommitSig>,
}

impl Commit {
    /// Compute commit hash
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.height.to_le_bytes());
        hasher.update(&self.round.to_le_bytes());
        hasher.update(&self.block_id.hash);
        for sig in &self.signatures {
            match sig {
                CommitSig::Commit { validator_address, signature, .. } => {
                    hasher.update(validator_address);
                    hasher.update(signature);
                }
                CommitSig::Nil { validator_address, .. } => {
                    hasher.update(validator_address);
                }
                CommitSig::Absent => {}
            }
        }
        hasher.finalize().into()
    }

    /// Check if commit has 2/3+ voting power
    pub fn has_two_thirds_majority(&self, validator_set: &ValidatorSet) -> bool {
        let mut signed_power: i64 = 0;

        for (i, sig) in self.signatures.iter().enumerate() {
            if let CommitSig::Commit { .. } = sig {
                if let Some(validator) = validator_set.validators.get(i) {
                    signed_power += validator.voting_power;
                }
            }
        }

        signed_power * 3 > validator_set.total_voting_power() * 2
    }
}

/// Commit signature
#[derive(Debug, Clone)]
pub enum CommitSig {
    /// Validator was absent
    Absent,
    /// Validator voted nil
    Nil {
        validator_address: Address,
        timestamp: u64,
    },
    /// Validator committed
    Commit {
        validator_address: Address,
        timestamp: u64,
        signature: Vec<u8>,
    },
}

/// Complete block structure
#[derive(Debug, Clone)]
pub struct Block {
    /// Block header
    pub header: Header,
    /// Transaction data
    pub data: Data,
    /// Evidence of Byzantine behavior
    pub evidence: EvidenceData,
    /// Commit from previous block
    pub last_commit: Option<Commit>,
}

impl Block {
    /// Get block hash
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    /// Get block height
    pub fn height(&self) -> i64 {
        self.header.height
    }
}

// =============================================================================
// Validator Types
// =============================================================================

/// Validator in the validator set
#[derive(Debug, Clone)]
pub struct Validator {
    /// Validator address
    pub address: Address,
    /// Public key (simplified as bytes)
    pub pub_key: Vec<u8>,
    /// Voting power (based on stake)
    pub voting_power: i64,
    /// Proposer priority (for round-robin with weighted)
    pub proposer_priority: i64,
}

impl Validator {
    pub fn new(address: Address, pub_key: Vec<u8>, voting_power: i64) -> Self {
        Self {
            address,
            pub_key,
            voting_power,
            proposer_priority: 0,
        }
    }
}

/// Set of validators
#[derive(Debug, Clone)]
pub struct ValidatorSet {
    /// Validators sorted by voting power (descending)
    pub validators: Vec<Validator>,
    /// Current proposer
    proposer_index: usize,
}

impl ValidatorSet {
    pub fn new(validators: Vec<Validator>) -> Self {
        let mut vs = Self {
            validators,
            proposer_index: 0,
        };
        vs.validators.sort_by(|a, b| b.voting_power.cmp(&a.voting_power));
        vs
    }

    /// Get total voting power
    pub fn total_voting_power(&self) -> i64 {
        self.validators.iter().map(|v| v.voting_power).sum()
    }

    /// Get current proposer
    pub fn get_proposer(&self) -> Option<&Validator> {
        self.validators.get(self.proposer_index)
    }

    /// Increment proposer priority and select next proposer
    ///
    /// This implements the weighted round-robin algorithm.
    pub fn increment_proposer_priority(&mut self) {
        if self.validators.is_empty() {
            return;
        }

        let total = self.total_voting_power();

        // Increment each validator's priority by their voting power
        for v in &mut self.validators {
            v.proposer_priority += v.voting_power;
        }

        // Select validator with highest priority as proposer
        let (max_idx, _) = self.validators
            .iter()
            .enumerate()
            .max_by_key(|(_, v)| v.proposer_priority)
            .unwrap();

        // Subtract total from selected proposer to balance
        self.validators[max_idx].proposer_priority -= total;
        self.proposer_index = max_idx;
    }

    /// Get validator by address
    pub fn get_by_address(&self, address: &Address) -> Option<&Validator> {
        self.validators.iter().find(|v| &v.address == address)
    }

    /// Check if address is a validator
    pub fn is_validator(&self, address: &Address) -> bool {
        self.get_by_address(address).is_some()
    }

    /// Get 2/3 threshold
    pub fn two_thirds_threshold(&self) -> i64 {
        (self.total_voting_power() * 2) / 3 + 1
    }

    /// Compute hash of validator set
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        for v in &self.validators {
            hasher.update(&v.address);
            hasher.update(&v.voting_power.to_le_bytes());
        }
        hasher.finalize().into()
    }
}

// =============================================================================
// Vote Types
// =============================================================================

/// Vote type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteType {
    /// Prevote phase
    Prevote,
    /// Precommit phase
    Precommit,
}

/// A vote from a validator
#[derive(Debug, Clone)]
pub struct Vote {
    /// Vote type (prevote or precommit)
    pub vote_type: VoteType,
    /// Block height
    pub height: i64,
    /// Consensus round
    pub round: i32,
    /// Block being voted on (nil if voting nil)
    pub block_id: Option<BlockId>,
    /// Timestamp
    pub timestamp: u64,
    /// Validator address
    pub validator_address: Address,
    /// Validator index in set
    pub validator_index: i32,
    /// Signature
    pub signature: Vec<u8>,
}

impl Vote {
    /// Create signing bytes for vote
    pub fn sign_bytes(&self, chain_id: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(chain_id.as_bytes());
        bytes.push(self.vote_type as u8);
        bytes.extend_from_slice(&self.height.to_le_bytes());
        bytes.extend_from_slice(&self.round.to_le_bytes());
        if let Some(ref block_id) = self.block_id {
            bytes.extend_from_slice(&block_id.hash);
        }
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.validator_address);
        bytes
    }

    /// Check if this is a nil vote
    pub fn is_nil(&self) -> bool {
        self.block_id.is_none()
    }
}

/// Utility function to get current timestamp
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_set_proposer() {
        let validators = vec![
            Validator::new([1u8; 20], vec![], 100),
            Validator::new([2u8; 20], vec![], 200),
            Validator::new([3u8; 20], vec![], 50),
        ];

        let mut vs = ValidatorSet::new(validators);

        // First proposer should be highest voting power
        assert_eq!(vs.get_proposer().unwrap().voting_power, 200);

        // Increment and check rotation
        vs.increment_proposer_priority();
        // After one round, priorities shift
    }

    #[test]
    fn test_two_thirds_threshold() {
        let validators = vec![
            Validator::new([1u8; 20], vec![], 100),
            Validator::new([2u8; 20], vec![], 100),
            Validator::new([3u8; 20], vec![], 100),
        ];

        let vs = ValidatorSet::new(validators);

        // Total = 300, 2/3 = 200, threshold = 201
        assert_eq!(vs.two_thirds_threshold(), 201);
    }

    #[test]
    fn test_commit_has_majority() {
        let validators = vec![
            Validator::new([1u8; 20], vec![], 100),
            Validator::new([2u8; 20], vec![], 100),
            Validator::new([3u8; 20], vec![], 100),
        ];

        let vs = ValidatorSet::new(validators);
        // Total power = 300, need > 200 (2/3)

        // All three validators commit - should pass
        let commit = Commit {
            height: 1,
            round: 0,
            block_id: BlockId::default(),
            signatures: vec![
                CommitSig::Commit {
                    validator_address: [1u8; 20], // 100 power
                    timestamp: 0,
                    signature: vec![],
                },
                CommitSig::Commit {
                    validator_address: [2u8; 20], // 100 power
                    timestamp: 0,
                    signature: vec![],
                },
                CommitSig::Commit {
                    validator_address: [3u8; 20], // 100 power
                    timestamp: 0,
                    signature: vec![],
                },
            ],
        };

        // 300 * 3 = 900 > 300 * 2 = 600 ✓
        assert!(commit.has_two_thirds_majority(&vs));
    }
}
