//! BABE (Blind Assignment for Blockchain Extension)
//!
//! ## BABE vs Other Block Production
//!
//! | Aspect | Bitcoin PoW | Ouroboros Praos | BABE |
//! |--------|-------------|-----------------|------|
//! | Selection | Hash puzzle | VRF + stake | VRF + stake |
//! | Slot time | ~10 min | 1 sec | 6 sec |
//! | Empty slots | Rare | Common (~95%) | Common |
//! | Secondary slots | No | No | Yes (fallback) |
//!
//! ## Slot Types
//!
//! ```text
//! Primary Slot (VRF win):
//!   VRF(key, slot_randomness) < threshold(stake)
//!   → Strong randomness, may have multiple winners
//!
//! Secondary Slot (deterministic fallback):
//!   If no primary winner, use round-robin
//!   → Ensures liveness (no empty slots)
//!
//! ┌─────┬─────┬─────┬─────┬─────┬─────┐
//! │ S0  │ S1  │ S2  │ S3  │ S4  │ S5  │
//! │ Pri │ Sec │ Pri │ Pri │ Sec │ Pri │
//! │ V3  │ V1  │ V2  │V1,V4│ V2  │ V3  │
//! └─────┴─────┴─────┴─────┴─────┴─────┘
//!   ↑                   ↑
//!   VRF winner    Multiple VRF winners (fork)
//! ```
//!
//! ## Epoch and Randomness
//!
//! ```text
//! Epoch N:
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Uses randomness from Epoch N-2                              │
//! │ Accumulates VRF outputs for Epoch N+2                      │
//! └─────────────────────────────────────────────────────────────┘
//!
//! Why N-2? Prevents stake grinding:
//!   - Randomness fixed before staking decisions
//!   - 2 epoch lookahead for security
//! ```

use crate::constants::*;
use sha2::{Digest, Sha256};

/// Slot number
pub type Slot = u64;

/// Epoch index
pub type EpochIndex = u64;

/// Authority index
pub type AuthorityIndex = u32;

/// Authority weight (voting power)
pub type AuthorityWeight = u64;

/// VRF output (32 bytes)
pub type VrfOutput = [u8; 32];

/// Randomness seed (32 bytes)
pub type Randomness = [u8; 32];

/// Authority ID (public key, 32 bytes)
pub type AuthorityId = [u8; 32];

// =============================================================================
// BABE Configuration
// =============================================================================

/// BABE configuration for an epoch
#[derive(Debug, Clone)]
pub struct BabeConfiguration {
    /// Slot duration in milliseconds
    pub slot_duration: u64,
    /// Epoch length in slots
    pub epoch_length: u64,
    /// Threshold constant (c in paper) as ratio (numerator, denominator)
    pub c: (u64, u64),
    /// Authorities with weights
    pub authorities: Vec<(AuthorityId, AuthorityWeight)>,
    /// Randomness for this epoch
    pub randomness: Randomness,
    /// Allowed slot types
    pub allowed_slots: AllowedSlots,
}

impl Default for BabeConfiguration {
    fn default() -> Self {
        Self {
            slot_duration: SLOT_DURATION_MS,
            epoch_length: EPOCH_DURATION_SLOTS,
            c: (1, 4), // 25% primary slot probability
            authorities: Vec::new(),
            randomness: [0u8; 32],
            allowed_slots: AllowedSlots::PrimaryAndSecondaryVrf,
        }
    }
}

/// Allowed slot assignment types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedSlots {
    /// Only primary (VRF) slots
    PrimarySlots,
    /// Primary with plain secondary fallback
    PrimaryAndSecondaryPlain,
    /// Primary with VRF-based secondary
    PrimaryAndSecondaryVrf,
}

// =============================================================================
// Epoch
// =============================================================================

/// Epoch information
#[derive(Debug, Clone)]
pub struct Epoch {
    /// Epoch index
    pub epoch_index: EpochIndex,
    /// Start slot
    pub start_slot: Slot,
    /// Duration in slots
    pub duration: u64,
    /// Authorities for this epoch
    pub authorities: Vec<(AuthorityId, AuthorityWeight)>,
    /// Randomness for this epoch
    pub randomness: Randomness,
    /// Configuration
    pub config: BabeEpochConfig,
}

/// Per-epoch configuration
#[derive(Debug, Clone)]
pub struct BabeEpochConfig {
    /// Threshold constant
    pub c: (u64, u64),
    /// Allowed slots
    pub allowed_slots: AllowedSlots,
}

impl Epoch {
    /// Get end slot (exclusive)
    pub fn end_slot(&self) -> Slot {
        self.start_slot + self.duration
    }

    /// Check if slot belongs to this epoch
    pub fn contains(&self, slot: Slot) -> bool {
        slot >= self.start_slot && slot < self.end_slot()
    }

    /// Total authority weight
    pub fn total_weight(&self) -> AuthorityWeight {
        self.authorities.iter().map(|(_, w)| w).sum()
    }
}

// =============================================================================
// VRF and Slot Assignment
// =============================================================================

/// VRF transcript for slot assignment
#[derive(Debug, Clone)]
pub struct VrfTranscript {
    pub epoch_randomness: Randomness,
    pub slot: Slot,
    pub epoch_index: EpochIndex,
}

impl VrfTranscript {
    /// Create transcript for a slot
    pub fn new(randomness: Randomness, slot: Slot, epoch_index: EpochIndex) -> Self {
        Self {
            epoch_randomness: randomness,
            slot,
            epoch_index,
        }
    }

    /// Compute VRF input hash
    pub fn to_vrf_input(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"BabeVRFInOutContext");
        hasher.update(&self.epoch_randomness);
        hasher.update(&self.slot.to_le_bytes());
        hasher.update(&self.epoch_index.to_le_bytes());
        hasher.finalize().into()
    }
}

/// Slot claim type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotClaim {
    /// Primary slot (VRF winner)
    Primary {
        authority_index: AuthorityIndex,
        vrf_output: VrfOutput,
    },
    /// Secondary plain slot (deterministic)
    SecondaryPlain {
        authority_index: AuthorityIndex,
    },
    /// Secondary VRF slot
    SecondaryVrf {
        authority_index: AuthorityIndex,
        vrf_output: VrfOutput,
    },
}

impl SlotClaim {
    pub fn authority_index(&self) -> AuthorityIndex {
        match self {
            SlotClaim::Primary { authority_index, .. } => *authority_index,
            SlotClaim::SecondaryPlain { authority_index } => *authority_index,
            SlotClaim::SecondaryVrf { authority_index, .. } => *authority_index,
        }
    }

    pub fn is_primary(&self) -> bool {
        matches!(self, SlotClaim::Primary { .. })
    }
}

/// Check if authority wins primary slot
pub fn check_primary_slot(
    authority_id: &AuthorityId,
    authority_weight: AuthorityWeight,
    total_weight: AuthorityWeight,
    transcript: &VrfTranscript,
    c: (u64, u64),
) -> Option<VrfOutput> {
    // Compute VRF output (simplified: use hash)
    let vrf_input = transcript.to_vrf_input();
    let mut hasher = Sha256::new();
    hasher.update(&vrf_input);
    hasher.update(authority_id);
    let vrf_output: VrfOutput = hasher.finalize().into();

    // Convert to threshold value
    let vrf_value = u128::from_be_bytes(vrf_output[..16].try_into().unwrap());
    let max_value = u128::MAX;

    // Calculate threshold: 1 - (1 - c)^(weight/total_weight)
    // Simplified: threshold ≈ c * weight / total_weight * max_value
    // Compute in parts to avoid overflow:
    // threshold = (max_value / total_weight) * authority_weight * c_num / c_denom
    let (c_num, c_denom) = c;
    let per_weight = max_value / total_weight as u128;
    let threshold = per_weight
        .saturating_mul(authority_weight as u128)
        .saturating_mul(c_num as u128)
        / c_denom as u128;

    if vrf_value < threshold {
        Some(vrf_output)
    } else {
        None
    }
}

/// Get secondary slot authority (round-robin)
pub fn secondary_slot_authority(slot: Slot, num_authorities: u32) -> AuthorityIndex {
    (slot % num_authorities as u64) as AuthorityIndex
}

// =============================================================================
// Pre-digest and Seal
// =============================================================================

/// BABE pre-runtime digest (included in block header)
#[derive(Debug, Clone)]
pub enum PreDigest {
    /// Primary slot claim
    Primary(PrimaryPreDigest),
    /// Secondary plain slot
    SecondaryPlain(SecondaryPlainPreDigest),
    /// Secondary VRF slot
    SecondaryVrf(SecondaryVrfPreDigest),
}

/// Primary slot pre-digest
#[derive(Debug, Clone)]
pub struct PrimaryPreDigest {
    /// Authority index
    pub authority_index: AuthorityIndex,
    /// Slot number
    pub slot: Slot,
    /// VRF output
    pub vrf_output: VrfOutput,
}

/// Secondary plain pre-digest
#[derive(Debug, Clone)]
pub struct SecondaryPlainPreDigest {
    /// Authority index
    pub authority_index: AuthorityIndex,
    /// Slot number
    pub slot: Slot,
}

/// Secondary VRF pre-digest
#[derive(Debug, Clone)]
pub struct SecondaryVrfPreDigest {
    /// Authority index
    pub authority_index: AuthorityIndex,
    /// Slot number
    pub slot: Slot,
    /// VRF output
    pub vrf_output: VrfOutput,
}

impl PreDigest {
    pub fn slot(&self) -> Slot {
        match self {
            PreDigest::Primary(d) => d.slot,
            PreDigest::SecondaryPlain(d) => d.slot,
            PreDigest::SecondaryVrf(d) => d.slot,
        }
    }

    pub fn authority_index(&self) -> AuthorityIndex {
        match self {
            PreDigest::Primary(d) => d.authority_index,
            PreDigest::SecondaryPlain(d) => d.authority_index,
            PreDigest::SecondaryVrf(d) => d.authority_index,
        }
    }
}

// =============================================================================
// BABE State
// =============================================================================

/// BABE consensus state
#[derive(Debug)]
pub struct BabeState {
    /// Current epoch
    pub current_epoch: Epoch,
    /// Next epoch (if known)
    pub next_epoch: Option<Epoch>,
    /// Randomness accumulator for future epoch
    pub randomness_accumulator: Vec<VrfOutput>,
    /// Current slot
    pub current_slot: Slot,
}

impl BabeState {
    pub fn new(genesis_epoch: Epoch) -> Self {
        Self {
            current_slot: genesis_epoch.start_slot,
            current_epoch: genesis_epoch,
            next_epoch: None,
            randomness_accumulator: Vec::new(),
        }
    }

    /// Advance to next slot
    pub fn advance_slot(&mut self) {
        self.current_slot += 1;
    }

    /// Accumulate VRF output for future randomness
    pub fn accumulate_randomness(&mut self, vrf_output: VrfOutput) {
        self.randomness_accumulator.push(vrf_output);
    }

    /// Compute next epoch randomness from accumulator
    pub fn compute_next_randomness(&self) -> Randomness {
        let mut hasher = Sha256::new();
        hasher.update(&self.current_epoch.randomness);
        for vrf in &self.randomness_accumulator {
            hasher.update(vrf);
        }
        hasher.finalize().into()
    }

    /// Transition to next epoch
    pub fn transition_epoch(&mut self, next_authorities: Vec<(AuthorityId, AuthorityWeight)>) {
        let new_randomness = self.compute_next_randomness();
        let new_epoch = Epoch {
            epoch_index: self.current_epoch.epoch_index + 1,
            start_slot: self.current_epoch.end_slot(),
            duration: self.current_epoch.duration,
            authorities: next_authorities,
            randomness: new_randomness,
            config: self.current_epoch.config.clone(),
        };

        self.current_epoch = new_epoch;
        self.randomness_accumulator.clear();
    }

    /// Get epoch for a slot
    pub fn epoch_for_slot(&self, slot: Slot) -> Option<&Epoch> {
        if self.current_epoch.contains(slot) {
            Some(&self.current_epoch)
        } else if let Some(ref next) = self.next_epoch {
            if next.contains(slot) {
                Some(next)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_authority(id: u8) -> AuthorityId {
        let mut arr = [0u8; 32];
        arr[0] = id;
        arr
    }

    fn make_test_epoch() -> Epoch {
        Epoch {
            epoch_index: 0,
            start_slot: 0,
            duration: 100,
            authorities: vec![
                (make_authority(1), 100),
                (make_authority(2), 100),
                (make_authority(3), 100),
            ],
            randomness: [0u8; 32],
            config: BabeEpochConfig {
                c: (1, 4),
                allowed_slots: AllowedSlots::PrimaryAndSecondaryVrf,
            },
        }
    }

    #[test]
    fn test_epoch_contains() {
        let epoch = make_test_epoch();

        assert!(epoch.contains(0));
        assert!(epoch.contains(50));
        assert!(epoch.contains(99));
        assert!(!epoch.contains(100));
    }

    #[test]
    fn test_secondary_slot_authority() {
        assert_eq!(secondary_slot_authority(0, 3), 0);
        assert_eq!(secondary_slot_authority(1, 3), 1);
        assert_eq!(secondary_slot_authority(2, 3), 2);
        assert_eq!(secondary_slot_authority(3, 3), 0);
    }

    #[test]
    fn test_vrf_transcript() {
        let randomness = [1u8; 32];
        let transcript1 = VrfTranscript::new(randomness, 100, 5);
        let transcript2 = VrfTranscript::new(randomness, 100, 5);
        let transcript3 = VrfTranscript::new(randomness, 101, 5);

        assert_eq!(transcript1.to_vrf_input(), transcript2.to_vrf_input());
        assert_ne!(transcript1.to_vrf_input(), transcript3.to_vrf_input());
    }

    #[test]
    fn test_primary_slot_probability() {
        let epoch = make_test_epoch();
        let transcript = VrfTranscript::new(epoch.randomness, 50, epoch.epoch_index);

        // Count primary wins over many "random" authority keys
        let mut wins = 0;
        for i in 0..1000 {
            let auth_id = make_authority(i as u8);
            if check_primary_slot(&auth_id, 100, 300, &transcript, epoch.config.c).is_some() {
                wins += 1;
            }
        }

        // With c = 1/4 and 1/3 stake, expect ~8% win rate
        // (Actually ~8.3% = 1 - (1 - 0.25)^(1/3))
        assert!(wins > 30 && wins < 150, "Wins: {}", wins);
    }

    #[test]
    fn test_babe_state_transition() {
        let epoch = make_test_epoch();
        let mut state = BabeState::new(epoch);

        // Accumulate some randomness
        state.accumulate_randomness([1u8; 32]);
        state.accumulate_randomness([2u8; 32]);

        // Transition
        let new_authorities = vec![(make_authority(10), 200)];
        state.transition_epoch(new_authorities.clone());

        assert_eq!(state.current_epoch.epoch_index, 1);
        assert_eq!(state.current_epoch.start_slot, 100);
        assert_eq!(state.current_epoch.authorities, new_authorities);
        assert!(state.randomness_accumulator.is_empty());
    }
}
