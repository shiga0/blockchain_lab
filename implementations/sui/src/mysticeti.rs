//! Mysticeti Consensus (DAG-based BFT)
//!
//! ## Mysticeti vs Other Consensus
//!
//! | Aspect | Tendermint | PBFT | Mysticeti |
//! |--------|------------|------|-----------|
//! | Structure | Linear | Linear | DAG |
//! | Leader | Per-block | Per-view | Per-wave |
//! | Latency | 2 RTT | 3 RTT | ~1-2 RTT |
//! | Parallelism | Low | Low | High |
//!
//! ## DAG Structure
//!
//! ```text
//! Unlike linear chains, Mysticeti forms a DAG where each block
//! references multiple ancestors from the previous round:
//!
//!    Round 3:    [B3_0]─────[B3_1]─────[B3_2]
//!                  │ ╲       │ ╲       │
//!    Round 2:    [B2_0]─────[B2_1]─────[B2_2]
//!                  │ ╲       │ ╲       │
//!    Round 1:    [B1_0]─────[B1_1]─────[B1_2]
//!
//! Each block includes:
//! - Author (validator index)
//! - Round number
//! - References to ancestor blocks
//! - Transactions
//! - Commit votes for previous leaders
//! ```
//!
//! ## Wave-based Commitment
//!
//! ```text
//! Commitment happens in waves of 3 rounds:
//!
//! Wave W:
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Round 3W   (Leader):    Leader proposes anchor block       │
//! │ Round 3W+1 (Voting):    Validators reference leader block  │
//! │ Round 3W+2 (Decision):  If 2f+1 reference, leader commits  │
//! └─────────────────────────────────────────────────────────────┘
//!
//! Direct Rule: 2f+1 validators in round R+1 reference round R leader
//! Indirect Rule: Commit via ancestor chain if direct fails
//! ```

use crate::constants::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Authority index (validator ID)
pub type AuthorityIndex = u32;

/// Round number
pub type Round = u64;

/// Block digest (32 bytes)
pub type BlockDigest = [u8; DIGEST_LENGTH];

/// Commit index (sequential)
pub type CommitIndex = u64;

/// Timestamp in milliseconds
pub type TimestampMs = u64;

// =============================================================================
// Block Reference
// =============================================================================

/// Reference to a block in the DAG
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockRef {
    /// Round number
    pub round: Round,
    /// Author (validator index)
    pub author: AuthorityIndex,
    /// Block digest
    pub digest: BlockDigest,
}

impl BlockRef {
    pub fn new(round: Round, author: AuthorityIndex, digest: BlockDigest) -> Self {
        Self {
            round,
            author,
            digest,
        }
    }

    /// Create genesis block reference
    pub fn genesis(author: AuthorityIndex) -> Self {
        Self {
            round: 0,
            author,
            digest: [0u8; 32],
        }
    }
}

// =============================================================================
// Consensus Transaction
// =============================================================================

/// Transaction submitted to consensus
#[derive(Debug, Clone)]
pub struct ConsensusTransaction {
    /// Serialized transaction data
    pub data: Vec<u8>,
    /// Transaction tracking ID
    pub tracking_id: [u8; 16],
}

impl ConsensusTransaction {
    pub fn new(data: Vec<u8>) -> Self {
        let mut tracking_id = [0u8; 16];
        let hash = Sha256::digest(&data);
        tracking_id.copy_from_slice(&hash[..16]);
        Self { data, tracking_id }
    }
}

// =============================================================================
// Commit Vote
// =============================================================================

/// Vote to commit a leader block
#[derive(Debug, Clone)]
pub struct CommitVote {
    /// Reference to the leader block being voted on
    pub leader: BlockRef,
    /// Vote outcome
    pub vote: VoteDecision,
}

/// Vote decision for a leader
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteDecision {
    /// Accept this leader
    Accept,
    /// Reject this leader (skip)
    Skip,
}

// =============================================================================
// Block
// =============================================================================

/// Consensus block in the DAG
#[derive(Debug, Clone)]
pub struct Block {
    /// Epoch number
    pub epoch: u64,
    /// Round number
    pub round: Round,
    /// Author (validator index)
    pub author: AuthorityIndex,
    /// Timestamp when block was created
    pub timestamp_ms: TimestampMs,
    /// References to ancestor blocks
    pub ancestors: Vec<BlockRef>,
    /// Transactions in this block
    pub transactions: Vec<ConsensusTransaction>,
    /// Commit votes for previous leaders
    pub commit_votes: Vec<CommitVote>,
    /// Block signature (simplified)
    pub signature: [u8; 64],
}

impl Block {
    /// Create a new block
    pub fn new(
        epoch: u64,
        round: Round,
        author: AuthorityIndex,
        ancestors: Vec<BlockRef>,
        transactions: Vec<ConsensusTransaction>,
        commit_votes: Vec<CommitVote>,
    ) -> Self {
        Self {
            epoch,
            round,
            author,
            timestamp_ms: 0, // Set by proposer
            ancestors,
            transactions,
            commit_votes,
            signature: [0u8; 64],
        }
    }

    /// Compute block digest
    pub fn digest(&self) -> BlockDigest {
        let mut hasher = Sha256::new();
        hasher.update(&self.epoch.to_le_bytes());
        hasher.update(&self.round.to_le_bytes());
        hasher.update(&self.author.to_le_bytes());
        hasher.update(&self.timestamp_ms.to_le_bytes());
        for ancestor in &self.ancestors {
            hasher.update(&ancestor.digest);
        }
        for tx in &self.transactions {
            hasher.update(&tx.tracking_id);
        }
        hasher.finalize().into()
    }

    /// Get block reference
    pub fn reference(&self) -> BlockRef {
        BlockRef::new(self.round, self.author, self.digest())
    }

    /// Check if this block references a specific block
    pub fn references(&self, block_ref: &BlockRef) -> bool {
        self.ancestors.contains(block_ref)
    }

    /// Check if this is a genesis block
    pub fn is_genesis(&self) -> bool {
        self.round == 0 && self.ancestors.is_empty()
    }
}

// =============================================================================
// Commit
// =============================================================================

/// Committed subdag (consensus output)
#[derive(Debug, Clone)]
pub struct Commit {
    /// Sequential commit index
    pub index: CommitIndex,
    /// Previous commit digest (chain linkage)
    pub previous_digest: BlockDigest,
    /// Consensus timestamp
    pub timestamp_ms: TimestampMs,
    /// Leader block that was committed
    pub leader: BlockRef,
    /// All blocks in the committed subdag
    pub blocks: Vec<BlockRef>,
}

impl Commit {
    /// Compute commit digest
    pub fn digest(&self) -> BlockDigest {
        let mut hasher = Sha256::new();
        hasher.update(&self.index.to_le_bytes());
        hasher.update(&self.previous_digest);
        hasher.update(&self.leader.digest);
        for block in &self.blocks {
            hasher.update(&block.digest);
        }
        hasher.finalize().into()
    }
}

// =============================================================================
// Leader Schedule
// =============================================================================

/// Leader schedule for waves
#[derive(Debug, Clone)]
pub struct LeaderSchedule {
    /// Validators with their scores
    validators: Vec<(AuthorityIndex, u64)>,
    /// Leader offset per wave
    pub leader_offset: u32,
}

impl LeaderSchedule {
    pub fn new(num_validators: u32) -> Self {
        let validators = (0..num_validators).map(|i| (i, 100)).collect();
        Self {
            validators,
            leader_offset: 0,
        }
    }

    /// Get leader for a given round
    pub fn leader_for_round(&self, round: Round) -> AuthorityIndex {
        // Wave-based leader selection
        // Each wave is 3 rounds, leader round is first of each wave
        let wave = round / 3;
        let leader_idx = (wave as u32 + self.leader_offset) % self.validators.len() as u32;
        self.validators[leader_idx as usize].0
    }

    /// Check if round is a leader round
    pub fn is_leader_round(&self, round: Round) -> bool {
        round % 3 == 0
    }

    /// Update validator scores (reputation)
    pub fn update_scores(&mut self, scores: HashMap<AuthorityIndex, u64>) {
        for (idx, score) in scores {
            if let Some(validator) = self.validators.iter_mut().find(|(i, _)| *i == idx) {
                validator.1 = score;
            }
        }
    }
}

// =============================================================================
// DAG State
// =============================================================================

/// In-memory DAG state
#[derive(Debug)]
pub struct DagState {
    /// Blocks by reference
    blocks: HashMap<BlockRef, Block>,
    /// Blocks by round
    blocks_by_round: BTreeMap<Round, Vec<BlockRef>>,
    /// Genesis blocks (one per authority)
    genesis: HashMap<AuthorityIndex, BlockRef>,
    /// Last block from each authority
    last_block: HashMap<AuthorityIndex, BlockRef>,
    /// Committed rounds per authority
    committed_rounds: HashMap<AuthorityIndex, Round>,
    /// Current round
    current_round: Round,
    /// Number of authorities
    num_authorities: u32,
}

impl DagState {
    pub fn new(num_authorities: u32) -> Self {
        Self {
            blocks: HashMap::new(),
            blocks_by_round: BTreeMap::new(),
            genesis: HashMap::new(),
            last_block: HashMap::new(),
            committed_rounds: HashMap::new(),
            current_round: 0,
            num_authorities,
        }
    }

    /// Initialize with genesis blocks
    pub fn initialize_genesis(&mut self, epoch: u64) {
        for author in 0..self.num_authorities {
            let genesis_block = Block::new(epoch, 0, author, vec![], vec![], vec![]);
            let block_ref = genesis_block.reference();
            self.genesis.insert(author, block_ref.clone());
            self.last_block.insert(author, block_ref.clone());
            self.add_block(genesis_block);
        }
    }

    /// Add a block to the DAG
    pub fn add_block(&mut self, block: Block) {
        let block_ref = block.reference();
        let round = block.round;
        let author = block.author;

        // Update last block for author
        if let Some(last) = self.last_block.get(&author) {
            if round > last.round {
                self.last_block.insert(author, block_ref.clone());
            }
        } else {
            self.last_block.insert(author, block_ref.clone());
        }

        // Add to round index
        self.blocks_by_round
            .entry(round)
            .or_insert_with(Vec::new)
            .push(block_ref.clone());

        // Store block
        self.blocks.insert(block_ref, block);

        // Update current round
        if round > self.current_round {
            self.current_round = round;
        }
    }

    /// Get block by reference
    pub fn get_block(&self, block_ref: &BlockRef) -> Option<&Block> {
        self.blocks.get(block_ref)
    }

    /// Get all blocks in a round
    pub fn get_blocks_at_round(&self, round: Round) -> Vec<&Block> {
        self.blocks_by_round
            .get(&round)
            .map(|refs| refs.iter().filter_map(|r| self.blocks.get(r)).collect())
            .unwrap_or_default()
    }

    /// Get ancestors for proposing a new block
    pub fn get_ancestors_for_round(&self, round: Round) -> Vec<BlockRef> {
        if round == 0 {
            return vec![];
        }

        // Include all blocks from previous round
        self.blocks_by_round
            .get(&(round - 1))
            .cloned()
            .unwrap_or_default()
    }

    /// Count support for a leader block
    pub fn count_leader_support(&self, leader_ref: &BlockRef) -> u32 {
        let voting_round = leader_ref.round + 1;
        let mut support = 0;

        if let Some(refs) = self.blocks_by_round.get(&voting_round) {
            for block_ref in refs {
                if let Some(block) = self.blocks.get(block_ref) {
                    if block.references(leader_ref) {
                        support += 1;
                    }
                }
            }
        }

        support
    }

    /// Check if leader has quorum support
    pub fn has_quorum_support(&self, leader_ref: &BlockRef) -> bool {
        let support = self.count_leader_support(leader_ref);
        let threshold = (self.num_authorities * 2 + 2) / 3; // 2f+1
        support >= threshold
    }

    /// Get current round
    pub fn current_round(&self) -> Round {
        self.current_round
    }
}

// =============================================================================
// Universal Committer
// =============================================================================

/// Decision for a leader
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderDecision {
    /// Commit this leader
    Commit,
    /// Skip this leader
    Skip,
    /// Undecided (not enough info)
    Undecided,
}

/// Committer that decides on leaders
#[derive(Debug)]
pub struct UniversalCommitter {
    /// Number of authorities
    num_authorities: u32,
    /// Quorum threshold (2f+1)
    quorum_threshold: u32,
    /// Last committed round per authority
    last_committed: HashMap<AuthorityIndex, Round>,
    /// Last commit index
    last_commit_index: CommitIndex,
    /// Leader schedule
    leader_schedule: LeaderSchedule,
}

impl UniversalCommitter {
    pub fn new(num_authorities: u32) -> Self {
        let f = (num_authorities - 1) / 3;
        let quorum_threshold = 2 * f + 1;

        Self {
            num_authorities,
            quorum_threshold,
            last_committed: HashMap::new(),
            last_commit_index: 0,
            leader_schedule: LeaderSchedule::new(num_authorities),
        }
    }

    /// Try to commit leaders up to the given round
    pub fn try_commit(&mut self, dag: &DagState, up_to_round: Round) -> Vec<Commit> {
        let mut commits = Vec::new();

        // Process leader rounds (every 3 rounds starting from 0)
        // Start from 0 if no commits yet, otherwise next wave
        let start_round = if self.last_committed.is_empty() {
            0
        } else {
            self.last_committed_round() + 3
        };
        let mut round = start_round;
        while round <= up_to_round {
            if !self.leader_schedule.is_leader_round(round) {
                round += 1;
                continue;
            }

            let leader = self.leader_schedule.leader_for_round(round);
            let leader_blocks = dag.get_blocks_at_round(round);

            // Find the leader's block
            if let Some(leader_block) = leader_blocks.iter().find(|b| b.author == leader) {
                let leader_ref = leader_block.reference();

                // Apply direct decision rule
                let decision = self.decide_leader(dag, &leader_ref);

                if decision == LeaderDecision::Commit {
                    // Collect all blocks in the subdag
                    let subdag = self.collect_subdag(dag, &leader_ref);

                    let commit = Commit {
                        index: self.last_commit_index + 1,
                        previous_digest: [0u8; 32], // Simplified
                        timestamp_ms: leader_block.timestamp_ms,
                        leader: leader_ref.clone(),
                        blocks: subdag,
                    };

                    self.last_commit_index = commit.index;
                    self.last_committed.insert(leader, round);
                    commits.push(commit);
                }
            }

            round += 3;
        }

        commits
    }

    /// Decide on a leader using direct rule
    fn decide_leader(&self, dag: &DagState, leader_ref: &BlockRef) -> LeaderDecision {
        // Direct rule: 2f+1 validators reference the leader in round+1
        if dag.has_quorum_support(leader_ref) {
            LeaderDecision::Commit
        } else {
            // Check decision round (round+2)
            let decision_round = leader_ref.round + 2;
            let decision_blocks = dag.get_blocks_at_round(decision_round);

            if decision_blocks.len() >= self.quorum_threshold as usize {
                // If we have enough decision round blocks but no quorum support, skip
                LeaderDecision::Skip
            } else {
                LeaderDecision::Undecided
            }
        }
    }

    /// Collect all blocks in the subdag to commit
    fn collect_subdag(&self, dag: &DagState, leader_ref: &BlockRef) -> Vec<BlockRef> {
        let mut subdag = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = vec![leader_ref.clone()];

        while let Some(block_ref) = queue.pop() {
            if visited.contains(&block_ref) {
                continue;
            }
            visited.insert(block_ref.clone());

            if let Some(block) = dag.get_block(&block_ref) {
                subdag.push(block_ref.clone());

                // Add uncommitted ancestors
                for ancestor in &block.ancestors {
                    if !self.is_committed(ancestor) && !visited.contains(ancestor) {
                        queue.push(ancestor.clone());
                    }
                }
            }
        }

        // Sort by (round, author) for deterministic ordering
        subdag.sort_by(|a, b| (a.round, a.author).cmp(&(b.round, b.author)));
        subdag
    }

    /// Check if a block has been committed
    fn is_committed(&self, block_ref: &BlockRef) -> bool {
        self.last_committed
            .get(&block_ref.author)
            .map(|&r| block_ref.round <= r)
            .unwrap_or(false)
    }

    /// Get the last committed round
    fn last_committed_round(&self) -> Round {
        self.last_committed.values().max().copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_ref() {
        let digest = [0xab; 32];
        let block_ref = BlockRef::new(5, 2, digest);

        assert_eq!(block_ref.round, 5);
        assert_eq!(block_ref.author, 2);
        assert_eq!(block_ref.digest, digest);
    }

    #[test]
    fn test_block_creation() {
        let ancestors = vec![BlockRef::genesis(0), BlockRef::genesis(1)];
        let tx = ConsensusTransaction::new(vec![1, 2, 3]);

        let block = Block::new(0, 1, 0, ancestors.clone(), vec![tx], vec![]);

        assert_eq!(block.round, 1);
        assert_eq!(block.author, 0);
        assert_eq!(block.ancestors.len(), 2);
        assert_eq!(block.transactions.len(), 1);
        assert!(!block.is_genesis());
    }

    #[test]
    fn test_genesis_block() {
        let block = Block::new(0, 0, 0, vec![], vec![], vec![]);
        assert!(block.is_genesis());
    }

    #[test]
    fn test_leader_schedule() {
        let schedule = LeaderSchedule::new(4);

        // Wave 0 (rounds 0,1,2) -> leader 0
        assert_eq!(schedule.leader_for_round(0), 0);
        assert!(schedule.is_leader_round(0));
        assert!(!schedule.is_leader_round(1));

        // Wave 1 (rounds 3,4,5) -> leader 1
        assert_eq!(schedule.leader_for_round(3), 1);
        assert!(schedule.is_leader_round(3));
    }

    #[test]
    fn test_dag_state() {
        let mut dag = DagState::new(3);
        dag.initialize_genesis(0);

        // Should have 3 genesis blocks
        let genesis_blocks = dag.get_blocks_at_round(0);
        assert_eq!(genesis_blocks.len(), 3);

        // Add round 1 block
        let ancestors = dag.get_ancestors_for_round(1);
        assert_eq!(ancestors.len(), 3);

        let block = Block::new(0, 1, 0, ancestors, vec![], vec![]);
        dag.add_block(block);

        assert_eq!(dag.current_round(), 1);
    }

    #[test]
    fn test_commit_vote() {
        let leader = BlockRef::new(0, 0, [0u8; 32]);
        let vote = CommitVote {
            leader: leader.clone(),
            vote: VoteDecision::Accept,
        };

        assert_eq!(vote.vote, VoteDecision::Accept);
    }

    #[test]
    fn test_quorum_support() {
        let mut dag = DagState::new(4);
        dag.initialize_genesis(0);

        // Add round 0 leader block
        let ancestors = vec![];
        let leader_block = Block::new(0, 0, 0, ancestors, vec![], vec![]);
        let leader_ref = leader_block.reference();
        dag.add_block(leader_block);

        // Add round 1 blocks that reference the leader
        for author in 0..3 {
            // 3 out of 4 = quorum
            let block = Block::new(0, 1, author, vec![leader_ref.clone()], vec![], vec![]);
            dag.add_block(block);
        }

        // Should have quorum (3 >= 2*1+1 = 3 for n=4)
        assert!(dag.has_quorum_support(&leader_ref));
    }

    #[test]
    fn test_universal_committer() {
        let mut dag = DagState::new(4);
        dag.initialize_genesis(0);
        let mut committer = UniversalCommitter::new(4);

        // Create wave 0
        // Round 0: Leader block
        let leader_ref = dag.genesis.get(&0).unwrap().clone();

        // Round 1: Voting blocks (3 validators reference leader)
        for author in 0..3 {
            let block = Block::new(0, 1, author, vec![leader_ref.clone()], vec![], vec![]);
            dag.add_block(block);
        }

        // Round 2: Decision blocks
        let round_1_blocks = dag.get_ancestors_for_round(2);
        for author in 0..4 {
            let block = Block::new(0, 2, author, round_1_blocks.clone(), vec![], vec![]);
            dag.add_block(block);
        }

        // Try to commit
        let commits = committer.try_commit(&dag, 2);

        // Should commit the genesis leader
        assert!(!commits.is_empty());
    }

    #[test]
    fn test_commit_structure() {
        let commit = Commit {
            index: 1,
            previous_digest: [0u8; 32],
            timestamp_ms: 1234567890,
            leader: BlockRef::new(0, 0, [1u8; 32]),
            blocks: vec![
                BlockRef::new(0, 0, [1u8; 32]),
                BlockRef::new(0, 1, [2u8; 32]),
            ],
        };

        assert_eq!(commit.index, 1);
        assert_eq!(commit.blocks.len(), 2);

        // Digest should be deterministic
        let digest1 = commit.digest();
        let digest2 = commit.digest();
        assert_eq!(digest1, digest2);
    }
}
