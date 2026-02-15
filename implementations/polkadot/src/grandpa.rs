//! GRANDPA (GHOST-based Recursive ANcestor Deriving Prefix Agreement)
//!
//! ## GRANDPA vs Tendermint
//!
//! | Aspect | Tendermint | GRANDPA |
//! |--------|------------|---------|
//! | Finality | Per-block | Per-chain (batch) |
//! | Rounds | Per block | Per finalization |
//! | Voting | Block hash | (Block, Number) |
//! | Liveness | May halt | Continues with BABE |
//!
//! ## Finalization Process
//!
//! ```text
//! Round N:
//!
//! 1. Primary Propose (optional):
//!    Primary validator proposes best chain
//!
//! 2. Prevote:
//!    Each validator broadcasts prevote for best chain
//!    ┌─────────────────────────────────────────┐
//!    │ Prevote { target_hash, target_number }  │
//!    └─────────────────────────────────────────┘
//!
//! 3. Precommit:
//!    If 2/3+ prevotes for chain C, precommit to ghost(C)
//!    ┌─────────────────────────────────────────┐
//!    │ Precommit { target_hash, target_number }│
//!    └─────────────────────────────────────────┘
//!
//! 4. Finalize:
//!    If 2/3+ precommits for block B, B is FINAL
//! ```
//!
//! ## Chain Finalization (not just blocks)
//!
//! ```text
//! ┌────┬────┬────┬────┬────┬────┐
//! │ B1 │ B2 │ B3 │ B4 │ B5 │ B6 │
//! └────┴────┴────┴────┴────┴────┘
//!   ↑                        ↑
//!   Last finalized           GRANDPA vote
//!
//! When GRANDPA finalizes B6, all of B2-B6 become final.
//! This is more efficient than finalizing each block.
//! ```

use std::collections::HashMap;

/// Block hash
pub type BlockHash = [u8; 32];

/// Block number
pub type BlockNumber = u64;

/// Authority set ID (monotonically increasing)
pub type SetId = u64;

/// Round number
pub type RoundNumber = u64;

/// Authority ID (ed25519 public key)
pub type AuthorityId = [u8; 32];

/// Authority signature (ed25519)
pub type AuthoritySignature = [u8; 64];

/// Authority weight
pub type AuthorityWeight = u64;

// =============================================================================
// Authority Set
// =============================================================================

/// GRANDPA authority set
#[derive(Debug, Clone)]
pub struct AuthoritySet {
    /// Set ID
    pub set_id: SetId,
    /// Authorities with weights
    pub authorities: Vec<(AuthorityId, AuthorityWeight)>,
}

impl AuthoritySet {
    pub fn new(set_id: SetId, authorities: Vec<(AuthorityId, AuthorityWeight)>) -> Self {
        Self { set_id, authorities }
    }

    /// Total voting weight
    pub fn total_weight(&self) -> AuthorityWeight {
        self.authorities.iter().map(|(_, w)| w).sum()
    }

    /// Threshold for supermajority (2/3+)
    pub fn threshold(&self) -> AuthorityWeight {
        let total = self.total_weight();
        // Need strictly more than 2/3
        (total * 2) / 3 + 1
    }

    /// Check if weight meets threshold
    pub fn is_supermajority(&self, weight: AuthorityWeight) -> bool {
        weight >= self.threshold()
    }

    /// Get authority index by ID
    pub fn get_index(&self, id: &AuthorityId) -> Option<usize> {
        self.authorities.iter().position(|(auth_id, _)| auth_id == id)
    }

    /// Get authority weight by ID
    pub fn get_weight(&self, id: &AuthorityId) -> AuthorityWeight {
        self.authorities
            .iter()
            .find(|(auth_id, _)| auth_id == id)
            .map(|(_, w)| *w)
            .unwrap_or(0)
    }
}

// =============================================================================
// Votes
// =============================================================================

/// Vote target (block hash and number)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VoteTarget {
    pub hash: BlockHash,
    pub number: BlockNumber,
}

impl VoteTarget {
    pub fn new(hash: BlockHash, number: BlockNumber) -> Self {
        Self { hash, number }
    }
}

/// Prevote message
#[derive(Debug, Clone)]
pub struct Prevote {
    pub target: VoteTarget,
}

/// Precommit message
#[derive(Debug, Clone)]
pub struct Precommit {
    pub target: VoteTarget,
}

/// Signed vote
#[derive(Debug, Clone)]
pub struct SignedVote<V> {
    pub vote: V,
    pub id: AuthorityId,
    pub signature: AuthoritySignature,
}

/// Primary propose (optional first step)
#[derive(Debug, Clone)]
pub struct PrimaryPropose {
    pub target: VoteTarget,
}

// =============================================================================
// Commit
// =============================================================================

/// Commit message (finalization proof)
#[derive(Debug, Clone)]
pub struct Commit {
    /// Finalized block
    pub target: VoteTarget,
    /// Precommits that justify finalization
    pub precommits: Vec<SignedVote<Precommit>>,
}

impl Commit {
    /// Verify commit has sufficient precommits
    pub fn verify(&self, authority_set: &AuthoritySet) -> bool {
        let mut weight = 0u64;

        for signed in &self.precommits {
            // Check signature is from authority
            if let Some(_) = authority_set.get_index(&signed.id) {
                // Check vote is for our target or ancestor
                if signed.vote.target.number <= self.target.number {
                    weight += authority_set.get_weight(&signed.id);
                }
            }
        }

        authority_set.is_supermajority(weight)
    }
}

/// GRANDPA justification (commit + ancestry)
#[derive(Debug, Clone)]
pub struct GrandpaJustification {
    /// Round that produced this justification
    pub round: RoundNumber,
    /// Commit data
    pub commit: Commit,
    /// Ancestry headers (for verification)
    pub votes_ancestries: Vec<BlockHash>,
}

// =============================================================================
// Round State
// =============================================================================

/// State of a GRANDPA round
#[derive(Debug)]
pub struct RoundState {
    /// Round number
    pub round: RoundNumber,
    /// Authority set for this round
    pub authority_set: AuthoritySet,
    /// Collected prevotes
    pub prevotes: HashMap<AuthorityId, SignedVote<Prevote>>,
    /// Collected precommits
    pub precommits: HashMap<AuthorityId, SignedVote<Precommit>>,
    /// Primary proposer index
    pub primary_index: usize,
    /// Whether we've prevoted
    pub prevoted: bool,
    /// Whether we've precommitted
    pub precommitted: bool,
    /// Finalized target (if any)
    pub finalized: Option<VoteTarget>,
}

impl RoundState {
    pub fn new(round: RoundNumber, authority_set: AuthoritySet) -> Self {
        let primary_index = (round as usize) % authority_set.authorities.len();
        Self {
            round,
            authority_set,
            prevotes: HashMap::new(),
            precommits: HashMap::new(),
            primary_index,
            prevoted: false,
            precommitted: false,
            finalized: None,
        }
    }

    /// Get primary proposer for this round
    pub fn primary(&self) -> &AuthorityId {
        &self.authority_set.authorities[self.primary_index].0
    }

    /// Add a prevote
    pub fn add_prevote(&mut self, vote: SignedVote<Prevote>) -> bool {
        if self.authority_set.get_index(&vote.id).is_some() {
            self.prevotes.insert(vote.id, vote);
            true
        } else {
            false
        }
    }

    /// Add a precommit
    pub fn add_precommit(&mut self, vote: SignedVote<Precommit>) -> bool {
        if self.authority_set.get_index(&vote.id).is_some() {
            self.precommits.insert(vote.id, vote);
            true
        } else {
            false
        }
    }

    /// Calculate total prevote weight for a target
    pub fn prevote_weight(&self, target: &VoteTarget) -> AuthorityWeight {
        self.prevotes
            .values()
            .filter(|v| v.vote.target == *target)
            .map(|v| self.authority_set.get_weight(&v.id))
            .sum()
    }

    /// Calculate total precommit weight for a target
    pub fn precommit_weight(&self, target: &VoteTarget) -> AuthorityWeight {
        self.precommits
            .values()
            .filter(|v| v.vote.target == *target)
            .map(|v| self.authority_set.get_weight(&v.id))
            .sum()
    }

    /// Check if we have supermajority prevotes for any target
    pub fn has_prevote_supermajority(&self) -> Option<VoteTarget> {
        let threshold = self.authority_set.threshold();

        // Group votes by target and find one with supermajority
        let mut weights: HashMap<VoteTarget, AuthorityWeight> = HashMap::new();
        for vote in self.prevotes.values() {
            let weight = self.authority_set.get_weight(&vote.id);
            *weights.entry(vote.vote.target.clone()).or_insert(0) += weight;
        }

        weights
            .into_iter()
            .find(|(_, w)| *w >= threshold)
            .map(|(t, _)| t)
    }

    /// Check if we have supermajority precommits for any target
    pub fn has_precommit_supermajority(&self) -> Option<VoteTarget> {
        let threshold = self.authority_set.threshold();

        let mut weights: HashMap<VoteTarget, AuthorityWeight> = HashMap::new();
        for vote in self.precommits.values() {
            let weight = self.authority_set.get_weight(&vote.id);
            *weights.entry(vote.vote.target.clone()).or_insert(0) += weight;
        }

        weights
            .into_iter()
            .find(|(_, w)| *w >= threshold)
            .map(|(t, _)| t)
    }

    /// Try to finalize
    pub fn try_finalize(&mut self) -> Option<VoteTarget> {
        if self.finalized.is_some() {
            return self.finalized.clone();
        }

        if let Some(target) = self.has_precommit_supermajority() {
            self.finalized = Some(target.clone());
            Some(target)
        } else {
            None
        }
    }

    /// Create commit message
    pub fn create_commit(&self) -> Option<Commit> {
        let target = self.finalized.clone()?;

        // Collect precommits for the target
        let precommits: Vec<_> = self
            .precommits
            .values()
            .filter(|v| v.vote.target == target)
            .cloned()
            .collect();

        Some(Commit { target, precommits })
    }
}

// =============================================================================
// Equivocation
// =============================================================================

/// Evidence of double voting
#[derive(Debug, Clone)]
pub struct Equivocation<V> {
    /// Round number
    pub round: RoundNumber,
    /// First vote
    pub first: SignedVote<V>,
    /// Second vote (conflicting)
    pub second: SignedVote<V>,
}

/// Check if two prevotes are equivocating
pub fn check_prevote_equivocation(
    a: &SignedVote<Prevote>,
    b: &SignedVote<Prevote>,
) -> bool {
    a.id == b.id && a.vote.target != b.vote.target
}

/// Check if two precommits are equivocating
pub fn check_precommit_equivocation(
    a: &SignedVote<Precommit>,
    b: &SignedVote<Precommit>,
) -> bool {
    a.id == b.id && a.vote.target != b.vote.target
}

// =============================================================================
// GRANDPA State
// =============================================================================

/// GRANDPA consensus state
#[derive(Debug)]
pub struct GrandpaState {
    /// Current authority set
    pub authority_set: AuthoritySet,
    /// Current round
    pub current_round: RoundState,
    /// Last finalized block
    pub last_finalized: VoteTarget,
    /// Pending authority set change
    pub pending_change: Option<ScheduledChange>,
}

/// Scheduled authority set change
#[derive(Debug, Clone)]
pub struct ScheduledChange {
    /// New authorities
    pub next_authorities: Vec<(AuthorityId, AuthorityWeight)>,
    /// Delay in blocks
    pub delay: BlockNumber,
    /// Scheduled at block
    pub scheduled_at: BlockNumber,
}

impl GrandpaState {
    pub fn new(genesis_authorities: Vec<(AuthorityId, AuthorityWeight)>) -> Self {
        let authority_set = AuthoritySet::new(0, genesis_authorities);
        let last_finalized = VoteTarget::new([0u8; 32], 0);
        let current_round = RoundState::new(0, authority_set.clone());

        Self {
            authority_set,
            current_round,
            last_finalized,
            pending_change: None,
        }
    }

    /// Advance to next round
    pub fn next_round(&mut self) {
        let next_round = self.current_round.round + 1;
        self.current_round = RoundState::new(next_round, self.authority_set.clone());
    }

    /// Handle finalization
    pub fn finalize(&mut self, target: VoteTarget, justification: GrandpaJustification) -> bool {
        // Verify justification
        if !justification.commit.verify(&self.authority_set) {
            return false;
        }

        // Update finalized
        if target.number > self.last_finalized.number {
            self.last_finalized = target;

            // Check for authority set change
            if let Some(change) = &self.pending_change {
                if self.last_finalized.number >= change.scheduled_at + change.delay {
                    self.authority_set = AuthoritySet::new(
                        self.authority_set.set_id + 1,
                        change.next_authorities.clone(),
                    );
                    self.pending_change = None;
                }
            }

            true
        } else {
            false
        }
    }

    /// Schedule authority set change
    pub fn schedule_change(
        &mut self,
        next_authorities: Vec<(AuthorityId, AuthorityWeight)>,
        delay: BlockNumber,
        at_block: BlockNumber,
    ) {
        self.pending_change = Some(ScheduledChange {
            next_authorities,
            delay,
            scheduled_at: at_block,
        });
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

    fn make_signature() -> AuthoritySignature {
        [0u8; 64]
    }

    fn make_test_authorities() -> Vec<(AuthorityId, AuthorityWeight)> {
        vec![
            (make_authority(1), 100),
            (make_authority(2), 100),
            (make_authority(3), 100),
        ]
    }

    #[test]
    fn test_authority_set_threshold() {
        let set = AuthoritySet::new(0, make_test_authorities());

        assert_eq!(set.total_weight(), 300);
        assert_eq!(set.threshold(), 201); // 300 * 2/3 + 1

        assert!(!set.is_supermajority(200));
        assert!(set.is_supermajority(201));
        assert!(set.is_supermajority(300));
    }

    #[test]
    fn test_round_state_prevotes() {
        let authorities = make_test_authorities();
        let mut round = RoundState::new(0, AuthoritySet::new(0, authorities));

        let target = VoteTarget::new([1u8; 32], 10);

        // Add prevotes from 2/3+ authorities
        round.add_prevote(SignedVote {
            vote: Prevote { target: target.clone() },
            id: make_authority(1),
            signature: make_signature(),
        });
        round.add_prevote(SignedVote {
            vote: Prevote { target: target.clone() },
            id: make_authority(2),
            signature: make_signature(),
        });
        round.add_prevote(SignedVote {
            vote: Prevote { target: target.clone() },
            id: make_authority(3),
            signature: make_signature(),
        });

        let supermajority = round.has_prevote_supermajority();
        assert!(supermajority.is_some());
        assert_eq!(supermajority.unwrap(), target);
    }

    #[test]
    fn test_round_finalization() {
        let authorities = make_test_authorities();
        let mut round = RoundState::new(0, AuthoritySet::new(0, authorities));

        let target = VoteTarget::new([1u8; 32], 10);

        // Add precommits from all authorities
        for i in 1..=3 {
            round.add_precommit(SignedVote {
                vote: Precommit { target: target.clone() },
                id: make_authority(i),
                signature: make_signature(),
            });
        }

        let finalized = round.try_finalize();
        assert!(finalized.is_some());
        assert_eq!(finalized.unwrap(), target);
    }

    #[test]
    fn test_commit_verification() {
        let authorities = make_test_authorities();
        let set = AuthoritySet::new(0, authorities);

        let target = VoteTarget::new([1u8; 32], 10);

        let commit = Commit {
            target: target.clone(),
            precommits: vec![
                SignedVote {
                    vote: Precommit { target: target.clone() },
                    id: make_authority(1),
                    signature: make_signature(),
                },
                SignedVote {
                    vote: Precommit { target: target.clone() },
                    id: make_authority(2),
                    signature: make_signature(),
                },
                SignedVote {
                    vote: Precommit { target: target.clone() },
                    id: make_authority(3),
                    signature: make_signature(),
                },
            ],
        };

        assert!(commit.verify(&set));
    }

    #[test]
    fn test_equivocation_detection() {
        let target1 = VoteTarget::new([1u8; 32], 10);
        let target2 = VoteTarget::new([2u8; 32], 10);

        let vote1 = SignedVote {
            vote: Prevote { target: target1 },
            id: make_authority(1),
            signature: make_signature(),
        };

        let vote2 = SignedVote {
            vote: Prevote { target: target2 },
            id: make_authority(1),
            signature: make_signature(),
        };

        assert!(check_prevote_equivocation(&vote1, &vote2));
    }
}
