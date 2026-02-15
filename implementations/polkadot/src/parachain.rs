//! Parachain Primitives
//!
//! ## Parachain vs Rollup Comparison
//!
//! | Aspect | Polkadot Parachain | Ethereum L2 Rollup |
//! |--------|-------------------|-------------------|
//! | Security | Shared (relay validators) | Posted to L1 |
//! | Execution | WASM runtime | EVM/custom |
//! | Data | PoV + erasure coding | Calldata/blobs |
//! | Finality | GRANDPA (~12 sec) | L1 dependent |
//! | Messaging | XCM (native) | Bridges |
//!
//! ## Candidate Lifecycle
//!
//! ```text
//! 1. Collator produces block
//!    ┌─────────────────────────────────────┐
//!    │ Candidate {                         │
//!    │   para_id,                          │
//!    │   relay_parent,                     │
//!    │   pov_hash,                         │
//!    │   head_data,                        │
//!    │   commitments,                      │
//!    │ }                                   │
//!    └─────────────────────────────────────┘
//!
//! 2. Backing group validates (MIN_BACKING_VOTES)
//!    ┌─────────────────────────────────────┐
//!    │ BackedCandidate {                   │
//!    │   candidate,                        │
//!    │   validity_votes: [sig1, sig2],    │
//!    │   validator_indices: 0b110,         │
//!    │ }                                   │
//!    └─────────────────────────────────────┘
//!
//! 3. Availability (erasure coded PoV)
//!    ┌─────────────────────────────────────┐
//!    │ 2/3+ validators hold chunks        │
//!    │ AvailabilityBitfield: 0b11110111   │
//!    └─────────────────────────────────────┘
//!
//! 4. Inclusion in relay block
//! ```

use sha2::{Digest, Sha256};

/// Parachain ID
pub type ParaId = u32;

/// Block hash (32 bytes)
pub type Hash = [u8; 32];

/// Block number
pub type BlockNumber = u64;

/// Validator index
pub type ValidatorIndex = u32;

/// Session index
pub type SessionIndex = u32;

/// Group index (validator group)
pub type GroupIndex = u32;

/// Core index (availability core)
pub type CoreIndex = u32;

// =============================================================================
// Head Data
// =============================================================================

/// Parachain head data (state root)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeadData(pub Vec<u8>);

impl HeadData {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.0);
        hasher.finalize().into()
    }
}

// =============================================================================
// Validation Code
// =============================================================================

/// Parachain WASM runtime code
#[derive(Debug, Clone)]
pub struct ValidationCode(pub Vec<u8>);

impl ValidationCode {
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.0);
        hasher.finalize().into()
    }
}

// =============================================================================
// Candidate Descriptor
// =============================================================================

/// Describes a parachain block candidate
#[derive(Debug, Clone)]
pub struct CandidateDescriptor {
    /// Parachain ID
    pub para_id: ParaId,
    /// Relay chain block this is built on
    pub relay_parent: Hash,
    /// Collator public key
    pub collator: [u8; 32],
    /// Hash of persisted validation data
    pub persisted_validation_data_hash: Hash,
    /// Hash of Proof of Validity
    pub pov_hash: Hash,
    /// Erasure coding root
    pub erasure_root: Hash,
    /// Signature by collator
    pub signature: [u8; 64],
    /// New head data hash
    pub para_head: Hash,
    /// Validation code hash
    pub validation_code_hash: Hash,
}

impl CandidateDescriptor {
    /// Compute hash of descriptor
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.para_id.to_le_bytes());
        hasher.update(&self.relay_parent);
        hasher.update(&self.collator);
        hasher.update(&self.pov_hash);
        hasher.update(&self.para_head);
        hasher.finalize().into()
    }
}

// =============================================================================
// Candidate Commitments
// =============================================================================

/// Commitments made by a candidate (outputs)
#[derive(Debug, Clone, Default)]
pub struct CandidateCommitments {
    /// Upward messages (to relay chain)
    pub upward_messages: Vec<Vec<u8>>,
    /// Horizontal messages (to other parachains)
    pub horizontal_messages: Vec<OutboundHrmpMessage>,
    /// New validation code (if upgraded)
    pub new_validation_code: Option<ValidationCode>,
    /// New head data
    pub head_data: HeadData,
    /// Number of processed downward messages
    pub processed_downward_messages: u32,
    /// HRMP watermark
    pub hrmp_watermark: BlockNumber,
}

/// Outbound HRMP message
#[derive(Debug, Clone)]
pub struct OutboundHrmpMessage {
    /// Destination parachain
    pub recipient: ParaId,
    /// Message data
    pub data: Vec<u8>,
}

// =============================================================================
// Candidate Receipt
// =============================================================================

/// Candidate receipt (descriptor + commitments hash)
#[derive(Debug, Clone)]
pub struct CandidateReceipt {
    /// Candidate descriptor
    pub descriptor: CandidateDescriptor,
    /// Hash of commitments
    pub commitments_hash: Hash,
}

impl CandidateReceipt {
    pub fn hash(&self) -> Hash {
        self.descriptor.hash()
    }
}

/// Committed candidate receipt
#[derive(Debug, Clone)]
pub struct CommittedCandidateReceipt {
    /// Descriptor
    pub descriptor: CandidateDescriptor,
    /// Full commitments
    pub commitments: CandidateCommitments,
}

// =============================================================================
// Backed Candidate
// =============================================================================

/// Validity attestation by a validator
#[derive(Debug, Clone)]
pub enum ValidityAttestation {
    /// Implicit attestation (from backing)
    Implicit([u8; 64]),
    /// Explicit attestation (from approval)
    Explicit([u8; 64]),
}

/// Backed candidate (signed by backing group)
#[derive(Debug, Clone)]
pub struct BackedCandidate {
    /// Committed candidate
    pub candidate: CommittedCandidateReceipt,
    /// Validity votes from backing validators
    pub validity_votes: Vec<ValidityAttestation>,
    /// Bitfield of which validators backed
    pub validator_indices: Vec<bool>,
}

impl BackedCandidate {
    /// Get number of backing votes
    pub fn backing_count(&self) -> usize {
        self.validity_votes.len()
    }

    /// Check if minimum backing is met
    pub fn has_minimum_backing(&self, min_votes: u32) -> bool {
        self.backing_count() >= min_votes as usize
    }
}

// =============================================================================
// Availability
// =============================================================================

/// Availability bitfield (per validator)
#[derive(Debug, Clone)]
pub struct AvailabilityBitfield(pub Vec<bool>);

impl AvailabilityBitfield {
    /// Create bitfield for N cores
    pub fn new(num_cores: usize) -> Self {
        Self(vec![false; num_cores])
    }

    /// Set availability for a core
    pub fn set(&mut self, core: CoreIndex, available: bool) {
        if (core as usize) < self.0.len() {
            self.0[core as usize] = available;
        }
    }

    /// Check if core is available
    pub fn is_available(&self, core: CoreIndex) -> bool {
        self.0.get(core as usize).copied().unwrap_or(false)
    }

    /// Count available cores
    pub fn count_available(&self) -> usize {
        self.0.iter().filter(|&&b| b).count()
    }
}

/// Signed availability bitfield
#[derive(Debug, Clone)]
pub struct SignedAvailabilityBitfield {
    /// The bitfield
    pub payload: AvailabilityBitfield,
    /// Validator index
    pub validator_index: ValidatorIndex,
    /// Signature
    pub signature: [u8; 64],
}

// =============================================================================
// Persisted Validation Data
// =============================================================================

/// Data available to the parachain during validation
#[derive(Debug, Clone)]
pub struct PersistedValidationData {
    /// Parent head data
    pub parent_head: HeadData,
    /// Relay parent block number
    pub relay_parent_number: BlockNumber,
    /// Relay parent storage root
    pub relay_parent_storage_root: Hash,
    /// Max PoV size
    pub max_pov_size: u32,
}

impl PersistedValidationData {
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(&self.parent_head.0);
        hasher.update(&self.relay_parent_number.to_le_bytes());
        hasher.update(&self.relay_parent_storage_root);
        hasher.update(&self.max_pov_size.to_le_bytes());
        hasher.finalize().into()
    }
}

// =============================================================================
// Session Info
// =============================================================================

/// Session information
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Active validators
    pub validators: Vec<[u8; 32]>,
    /// Validator groups assignment
    pub validator_groups: Vec<Vec<ValidatorIndex>>,
    /// Number of availability cores
    pub n_cores: u32,
    /// Needed approvals
    pub needed_approvals: u32,
    /// Random seed
    pub random_seed: Hash,
}

impl SessionInfo {
    /// Get group for a parachain
    pub fn group_for_para(&self, para_id: ParaId, num_paras: u32) -> Option<GroupIndex> {
        if num_paras == 0 {
            return None;
        }
        Some((para_id % num_paras) as GroupIndex)
    }

    /// Get validators in a group
    pub fn validators_in_group(&self, group: GroupIndex) -> Option<&Vec<ValidatorIndex>> {
        self.validator_groups.get(group as usize)
    }
}

// =============================================================================
// Core State
// =============================================================================

/// State of an availability core
#[derive(Debug, Clone)]
pub enum CoreState {
    /// Core is free
    Free,
    /// Core is occupied with a candidate
    Occupied(OccupiedCore),
}

/// Occupied core info
#[derive(Debug, Clone)]
pub struct OccupiedCore {
    /// Parachain using this core
    pub para_id: ParaId,
    /// Responsible validator group
    pub group_responsible: GroupIndex,
    /// Candidate hash
    pub candidate_hash: Hash,
    /// Availability bitfield
    pub availability: AvailabilityBitfield,
    /// Timeout block
    pub time_out_at: BlockNumber,
}

impl OccupiedCore {
    /// Check if availability threshold met
    pub fn is_available(&self, threshold: usize) -> bool {
        self.availability.count_available() >= threshold
    }
}

// =============================================================================
// Relay Chain Block Data
// =============================================================================

/// Inherent data for relay block
#[derive(Debug, Clone)]
pub struct ParachainsInherentData {
    /// Signed availability bitfields
    pub bitfields: Vec<SignedAvailabilityBitfield>,
    /// Backed candidates to include
    pub backed_candidates: Vec<BackedCandidate>,
    /// Dispute statements
    pub disputes: Vec<DisputeStatement>,
    /// Parent header hash
    pub parent_header: Hash,
}

/// Dispute statement
#[derive(Debug, Clone)]
pub struct DisputeStatement {
    /// Candidate hash
    pub candidate_hash: Hash,
    /// Session
    pub session: SessionIndex,
    /// Valid statement (true = valid, false = invalid)
    pub valid: bool,
    /// Validator index
    pub validator_index: ValidatorIndex,
    /// Signature
    pub signature: [u8; 64],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_head_data_hash() {
        let head1 = HeadData::new(vec![1, 2, 3]);
        let head2 = HeadData::new(vec![1, 2, 3]);
        let head3 = HeadData::new(vec![1, 2, 4]);

        assert_eq!(head1.hash(), head2.hash());
        assert_ne!(head1.hash(), head3.hash());
    }

    #[test]
    fn test_availability_bitfield() {
        let mut bitfield = AvailabilityBitfield::new(5);

        assert_eq!(bitfield.count_available(), 0);

        bitfield.set(0, true);
        bitfield.set(2, true);
        bitfield.set(4, true);

        assert!(bitfield.is_available(0));
        assert!(!bitfield.is_available(1));
        assert!(bitfield.is_available(2));
        assert_eq!(bitfield.count_available(), 3);
    }

    #[test]
    fn test_backed_candidate_minimum() {
        let candidate = CommittedCandidateReceipt {
            descriptor: CandidateDescriptor {
                para_id: 1000,
                relay_parent: [0u8; 32],
                collator: [0u8; 32],
                persisted_validation_data_hash: [0u8; 32],
                pov_hash: [0u8; 32],
                erasure_root: [0u8; 32],
                signature: [0u8; 64],
                para_head: [0u8; 32],
                validation_code_hash: [0u8; 32],
            },
            commitments: CandidateCommitments::default(),
        };

        let backed = BackedCandidate {
            candidate,
            validity_votes: vec![
                ValidityAttestation::Implicit([0u8; 64]),
                ValidityAttestation::Implicit([0u8; 64]),
            ],
            validator_indices: vec![true, true, false],
        };

        assert!(backed.has_minimum_backing(2));
        assert!(!backed.has_minimum_backing(3));
    }

    #[test]
    fn test_session_group_assignment() {
        let session = SessionInfo {
            validators: vec![[1u8; 32], [2u8; 32], [3u8; 32]],
            validator_groups: vec![vec![0], vec![1], vec![2]],
            n_cores: 3,
            needed_approvals: 2,
            random_seed: [0u8; 32],
        };

        assert_eq!(session.group_for_para(1000, 3), Some(1));
        assert_eq!(session.group_for_para(1001, 3), Some(2));
        assert_eq!(session.group_for_para(1002, 3), Some(0));
    }

    #[test]
    fn test_occupied_core_availability() {
        let occupied = OccupiedCore {
            para_id: 1000,
            group_responsible: 0,
            candidate_hash: [1u8; 32],
            availability: AvailabilityBitfield(vec![true, true, true, false, false]),
            time_out_at: 100,
        };

        assert!(occupied.is_available(3));
        assert!(!occupied.is_available(4));
    }
}
