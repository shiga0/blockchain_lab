//! Tendermint BFT Consensus
//!
//! ## Consensus Comparison
//!
//! | Aspect | PoW (Bitcoin) | PoS (Ethereum) | BFT (Cosmos) |
//! |--------|--------------|----------------|--------------|
//! | Finality | Probabilistic | Economic | Instant |
//! | Forks | Possible | Possible | Impossible |
//! | Byzantine Tolerance | 50% hashrate | 33% stake | 33% voting power |
//! | Block Time | 10 min | 12 sec | 1-7 sec |
//! | Validators | Unbounded | ~900K | Bounded (~200) |
//!
//! ## Tendermint BFT Protocol
//!
//! Tendermint BFT is a round-based protocol where each round has:
//!
//! 1. **Propose**: Designated proposer broadcasts block
//! 2. **Prevote**: Validators prevote for valid block
//! 3. **Precommit**: If 2/3+ prevotes, validators precommit
//! 4. **Commit**: If 2/3+ precommits, block is committed
//!
//! ```text
//! Round Phases:
//!
//! Time ─────────────────────────────────────────────────────────→
//!
//!     Propose      Prevote       Precommit      Commit
//!   ┌─────────┬─────────────┬──────────────┬──────────────┐
//!   │Proposer │  Validators │  Validators  │    Block     │
//!   │ sends   │   vote on   │   vote if    │   finalized  │
//!   │  block  │  proposal   │  2/3+ prevote│   instantly  │
//!   └─────────┴─────────────┴──────────────┴──────────────┘
//!        │          │              │               │
//!   Timeout     Need 2/3+      Need 2/3+        Done!
//!   → Round+1   prevotes      precommits    (no forks)
//! ```
//!
//! ## Proof of Lock (POL)
//!
//! If a validator sees 2/3+ prevotes for a block in round R,
//! they are "locked" on that block and must prevote for it
//! in subsequent rounds until they see a POL for a different block.
//!
//! This ensures safety: once 2/3+ precommit, no other block can
//! get 2/3+ prevotes.

use std::collections::HashMap;
use crate::types::*;
use crate::constants::*;

/// Round step in the consensus state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundStep {
    /// Waiting for new height to start
    NewHeight,
    /// Setting up new round
    NewRound,
    /// Waiting for/sending proposal
    Propose,
    /// Waiting for/sending prevotes
    Prevote,
    /// Waiting for 2/3+ prevotes
    PrevoteWait,
    /// Waiting for/sending precommits
    Precommit,
    /// Waiting for 2/3+ precommits
    PrecommitWait,
    /// Committing the block
    Commit,
}

/// Collected votes for a round
#[derive(Debug, Clone, Default)]
pub struct VoteSet {
    /// Height
    pub height: i64,
    /// Round
    pub round: i32,
    /// Vote type
    pub vote_type: Option<VoteType>,
    /// Votes by validator index
    pub votes: HashMap<i32, Vote>,
    /// Total voting power that voted
    pub sum: i64,
    /// Block with 2/3+ votes (if any)
    pub maj23: Option<BlockId>,
}

impl VoteSet {
    pub fn new(height: i64, round: i32, vote_type: VoteType) -> Self {
        Self {
            height,
            round,
            vote_type: Some(vote_type),
            votes: HashMap::new(),
            sum: 0,
            maj23: None,
        }
    }

    /// Add a vote to the set
    pub fn add_vote(&mut self, vote: Vote, validator_power: i64) -> bool {
        // Check vote matches this set
        if vote.height != self.height || vote.round != self.round {
            return false;
        }
        if Some(vote.vote_type) != self.vote_type {
            return false;
        }

        // Check for duplicate
        if self.votes.contains_key(&vote.validator_index) {
            return false;
        }

        // Add vote
        self.votes.insert(vote.validator_index, vote);
        self.sum += validator_power;

        true
    }

    /// Check if we have 2/3+ for any block
    pub fn has_two_thirds_any(&self, total_power: i64) -> bool {
        self.sum * 3 > total_power * 2
    }

    /// Check if we have 2/3+ for a specific block
    pub fn has_two_thirds_for(&self, block_id: &BlockId, validator_set: &ValidatorSet) -> bool {
        let mut power_for_block: i64 = 0;

        for vote in self.votes.values() {
            if let Some(ref vid) = vote.block_id {
                if vid == block_id {
                    if let Some(v) = validator_set.validators.get(vote.validator_index as usize) {
                        power_for_block += v.voting_power;
                    }
                }
            }
        }

        power_for_block * 3 > validator_set.total_voting_power() * 2
    }

    /// Get the block with 2/3+ votes
    pub fn two_thirds_majority(&self, validator_set: &ValidatorSet) -> Option<BlockId> {
        // Group votes by block
        let mut block_power: HashMap<Hash, (BlockId, i64)> = HashMap::new();

        for vote in self.votes.values() {
            if let Some(ref block_id) = vote.block_id {
                let power = validator_set.validators
                    .get(vote.validator_index as usize)
                    .map(|v| v.voting_power)
                    .unwrap_or(0);

                let entry = block_power.entry(block_id.hash).or_insert((block_id.clone(), 0));
                entry.1 += power;
            }
        }

        let threshold = validator_set.two_thirds_threshold();

        for (_, (block_id, power)) in block_power {
            if power >= threshold {
                return Some(block_id);
            }
        }

        None
    }
}

/// Consensus state for current height and round
#[derive(Debug)]
pub struct ConsensusState {
    /// Current height
    pub height: i64,
    /// Current round
    pub round: i32,
    /// Current step
    pub step: RoundStep,
    /// Start time of current round
    pub start_time: u64,

    /// Validator set
    pub validators: ValidatorSet,

    /// Current proposal
    pub proposal: Option<Block>,
    /// Proposal block ID
    pub proposal_block_id: Option<BlockId>,

    /// Prevotes for current round
    pub prevotes: VoteSet,
    /// Precommits for current round
    pub precommits: VoteSet,

    /// Locked block (have seen 2/3+ prevotes)
    pub locked_block: Option<Block>,
    /// Round at which we locked
    pub locked_round: i32,

    /// Valid block (have seen 2/3+ precommits in some round)
    pub valid_block: Option<Block>,
    /// Round at which valid block was seen
    pub valid_round: i32,

    /// Commits for the block
    pub commit: Option<Commit>,
}

impl ConsensusState {
    /// Create new consensus state for height
    pub fn new(height: i64, validators: ValidatorSet) -> Self {
        Self {
            height,
            round: 0,
            step: RoundStep::NewHeight,
            start_time: now_unix(),
            validators,
            proposal: None,
            proposal_block_id: None,
            prevotes: VoteSet::new(height, 0, VoteType::Prevote),
            precommits: VoteSet::new(height, 0, VoteType::Precommit),
            locked_block: None,
            locked_round: -1,
            valid_block: None,
            valid_round: -1,
            commit: None,
        }
    }

    /// Start a new round
    pub fn start_round(&mut self, round: i32) {
        self.round = round;
        self.step = RoundStep::NewRound;
        self.start_time = now_unix();
        self.proposal = None;
        self.proposal_block_id = None;
        self.prevotes = VoteSet::new(self.height, round, VoteType::Prevote);
        self.precommits = VoteSet::new(self.height, round, VoteType::Precommit);

        // Update proposer for new round
        for _ in 0..=round {
            self.validators.increment_proposer_priority();
        }
    }

    /// Get current proposer
    pub fn get_proposer(&self) -> Option<&Validator> {
        self.validators.get_proposer()
    }

    /// Check if address is current proposer
    pub fn is_proposer(&self, address: &Address) -> bool {
        self.get_proposer()
            .map(|p| &p.address == address)
            .unwrap_or(false)
    }

    // =========================================================================
    // State Transitions
    // =========================================================================

    /// Enter propose step
    pub fn enter_propose(&mut self) {
        self.step = RoundStep::Propose;
        // Proposer should create and broadcast proposal
    }

    /// Receive proposal
    pub fn set_proposal(&mut self, block: Block) {
        let block_id = BlockId {
            hash: block.hash(),
            part_set_hash: [0u8; 32], // Simplified
            part_set_total: 1,
        };

        self.proposal = Some(block);
        self.proposal_block_id = Some(block_id);
    }

    /// Enter prevote step
    pub fn enter_prevote(&mut self) {
        self.step = RoundStep::Prevote;
    }

    /// Decide what to prevote
    ///
    /// Returns the block_id to prevote for, or None for nil vote.
    pub fn decide_prevote(&self) -> Option<BlockId> {
        // If locked, prevote for locked block
        if let Some(ref locked) = self.locked_block {
            return Some(BlockId {
                hash: locked.hash(),
                part_set_hash: [0u8; 32],
                part_set_total: 1,
            });
        }

        // If we have a valid proposal, prevote for it
        self.proposal_block_id.clone()
    }

    /// Add a prevote
    pub fn add_prevote(&mut self, vote: Vote) -> bool {
        let power = self.validators.validators
            .get(vote.validator_index as usize)
            .map(|v| v.voting_power)
            .unwrap_or(0);

        self.prevotes.add_vote(vote, power)
    }

    /// Check if we have 2/3+ prevotes and should enter precommit
    pub fn should_enter_precommit(&self) -> bool {
        self.prevotes.has_two_thirds_any(self.validators.total_voting_power())
    }

    /// Enter precommit step
    pub fn enter_precommit(&mut self) {
        self.step = RoundStep::Precommit;

        // If 2/3+ prevoted for a block, lock on it
        if let Some(block_id) = self.prevotes.two_thirds_majority(&self.validators) {
            if let Some(ref proposal) = self.proposal {
                if proposal.hash() == block_id.hash {
                    self.locked_block = Some(proposal.clone());
                    self.locked_round = self.round;
                }
            }
        }
    }

    /// Decide what to precommit
    ///
    /// Returns the block_id to precommit for, or None for nil.
    pub fn decide_precommit(&self) -> Option<BlockId> {
        // Only precommit if we have 2/3+ prevotes for a block
        self.prevotes.two_thirds_majority(&self.validators)
    }

    /// Add a precommit
    pub fn add_precommit(&mut self, vote: Vote) -> bool {
        let power = self.validators.validators
            .get(vote.validator_index as usize)
            .map(|v| v.voting_power)
            .unwrap_or(0);

        self.precommits.add_vote(vote, power)
    }

    /// Check if we have 2/3+ precommits and should commit
    pub fn should_commit(&self) -> bool {
        self.precommits.two_thirds_majority(&self.validators).is_some()
    }

    /// Enter commit step - block is finalized!
    pub fn enter_commit(&mut self) -> Option<Block> {
        self.step = RoundStep::Commit;

        if let Some(block_id) = self.precommits.two_thirds_majority(&self.validators) {
            // Build commit from precommits
            let mut signatures = Vec::new();
            for i in 0..self.validators.validators.len() {
                if let Some(vote) = self.precommits.votes.get(&(i as i32)) {
                    signatures.push(CommitSig::Commit {
                        validator_address: vote.validator_address,
                        timestamp: vote.timestamp,
                        signature: vote.signature.clone(),
                    });
                } else {
                    signatures.push(CommitSig::Absent);
                }
            }

            self.commit = Some(Commit {
                height: self.height,
                round: self.round,
                block_id,
                signatures,
            });

            // Return the committed block
            self.proposal.clone()
        } else {
            None
        }
    }

    // =========================================================================
    // Timeout Calculations
    // =========================================================================

    /// Get propose timeout for current round
    pub fn propose_timeout(&self) -> u64 {
        TIMEOUT_PROPOSE_MS + (self.round as u64) * TIMEOUT_PROPOSE_DELTA_MS
    }

    /// Get prevote timeout for current round
    pub fn prevote_timeout(&self) -> u64 {
        TIMEOUT_PREVOTE_MS + (self.round as u64) * TIMEOUT_PREVOTE_DELTA_MS
    }

    /// Get precommit timeout for current round
    pub fn precommit_timeout(&self) -> u64 {
        TIMEOUT_PRECOMMIT_MS + (self.round as u64) * TIMEOUT_PRECOMMIT_DELTA_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_validator_set() -> ValidatorSet {
        ValidatorSet::new(vec![
            Validator::new([1u8; 20], vec![], 100),
            Validator::new([2u8; 20], vec![], 100),
            Validator::new([3u8; 20], vec![], 100),
            Validator::new([4u8; 20], vec![], 100),
        ])
    }

    #[test]
    fn test_vote_set_two_thirds() {
        let vs = make_validator_set();
        let mut vote_set = VoteSet::new(1, 0, VoteType::Prevote);

        let block_id = BlockId {
            hash: [1u8; 32],
            part_set_hash: [0u8; 32],
            part_set_total: 1,
        };

        // Add 3 votes (300 power out of 400)
        for i in 0..3 {
            let vote = Vote {
                vote_type: VoteType::Prevote,
                height: 1,
                round: 0,
                block_id: Some(block_id.clone()),
                timestamp: 0,
                validator_address: [(i + 1) as u8; 20],
                validator_index: i,
                signature: vec![],
            };
            vote_set.add_vote(vote, 100);
        }

        // 300 * 3 = 900 > 400 * 2 = 800 ✓
        assert!(vote_set.has_two_thirds_any(vs.total_voting_power()));
        assert!(vote_set.two_thirds_majority(&vs).is_some());
    }

    #[test]
    fn test_consensus_state_round_progression() {
        let vs = make_validator_set();
        let mut state = ConsensusState::new(1, vs);

        assert_eq!(state.step, RoundStep::NewHeight);

        state.start_round(0);
        assert_eq!(state.step, RoundStep::NewRound);
        assert_eq!(state.round, 0);

        state.enter_propose();
        assert_eq!(state.step, RoundStep::Propose);

        state.enter_prevote();
        assert_eq!(state.step, RoundStep::Prevote);

        state.enter_precommit();
        assert_eq!(state.step, RoundStep::Precommit);
    }

    #[test]
    fn test_timeout_increases_with_round() {
        let vs = make_validator_set();
        let mut state = ConsensusState::new(1, vs);

        state.start_round(0);
        let timeout_r0 = state.propose_timeout();

        state.start_round(1);
        let timeout_r1 = state.propose_timeout();

        state.start_round(2);
        let timeout_r2 = state.propose_timeout();

        assert!(timeout_r1 > timeout_r0);
        assert!(timeout_r2 > timeout_r1);
    }
}
