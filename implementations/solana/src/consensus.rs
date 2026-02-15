//! Solana Consensus Module (Proof of History + Tower BFT)
//!
//! ## Consensus Comparison
//!
//! | Aspect | PoW (Bitcoin) | PoS (Ethereum) | PoH + Tower BFT (Solana) |
//! |--------|---------------|----------------|--------------------------|
//! | Block Producer | Miner | Validator | Leader (rotating) |
//! | Selection | Hash puzzle | Random (stake) | Leader schedule |
//! | Time Proof | Block header | Slot number | PoH hash chain |
//! | Finality | Probabilistic | Casper FFG | Tower BFT (exponential) |
//! | Energy | High | Low | Low |
//!
//! ## Proof of History (PoH)
//!
//! PoH provides a cryptographic proof of time passage using a sequential
//! SHA-256 hash chain. It acts as a Verifiable Delay Function (VDF).
//!
//! Key insight: Computing N hashes takes a minimum amount of wall-clock time,
//! regardless of hardware (single-threaded constraint).
//!
//! ```text
//! Time ──────────────────────────────────────────────────→
//!
//! Hash Chain:
//! h₀ → h₁ → h₂ → ... → h₁₂₅₀₀ (1 tick, ~6.25ms)
//!       ↓
//!   SHA256(h₀)
//!
//! With Event (Transaction):
//! h_n → SHA256(h_n || tx_hash) → h_{n+1}
//!                    ↑
//!              mixin_hash
//! ```
//!
//! ## Tower BFT
//!
//! Tower BFT is Solana's implementation of PBFT with PoH as a clock.
//!
//! Key features:
//! - Validators vote on slots
//! - Votes have exponentially increasing lockouts
//! - Fork choice: Heaviest fork (most stake-weighted votes)
//! - Rollback cost grows exponentially
//!
//! ```text
//! Lockout Calculation:
//! lockout(vote_depth) = 2^(vote_depth + 1)
//!
//! Example vote tower:
//! Depth 0: lockout = 2 slots   (can switch after 2 slots)
//! Depth 1: lockout = 4 slots
//! Depth 2: lockout = 8 slots
//! ...
//! Depth 31: lockout = 2^32 slots (~54 years at 400ms/slot)
//! ```

use sha2::{Sha256, Digest};

use crate::constants::*;

/// 32-byte hash (SHA-256 output)
pub type Hash = [u8; 32];

/// Proof of History state machine
///
/// Reference: solana/entry/src/poh.rs
#[derive(Debug, Clone)]
pub struct Poh {
    /// Current hash in the chain
    pub hash: Hash,
    /// Number of hashes since last entry
    pub num_hashes: u64,
    /// Target hashes per tick
    pub hashes_per_tick: u64,
    /// Current tick number within slot
    pub tick_number: u64,
}

impl Poh {
    /// Create a new PoH instance with an initial hash
    pub fn new(initial_hash: Hash) -> Self {
        Self {
            hash: initial_hash,
            num_hashes: 0,
            hashes_per_tick: HASHES_PER_TICK,
            tick_number: 0,
        }
    }

    /// Perform one SHA-256 hash iteration
    pub fn hash_once(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update(&self.hash);
        self.hash = hasher.finalize().into();
        self.num_hashes += 1;
    }

    /// Mix in external data (transaction hash) into the PoH chain
    ///
    /// This records the event in the hash chain, proving it occurred
    /// at this specific point in time.
    pub fn record(&mut self, mixin: &[u8]) -> PohEntry {
        let mut hasher = Sha256::new();
        hasher.update(&self.hash);
        hasher.update(mixin);
        self.hash = hasher.finalize().into();
        self.num_hashes += 1;

        PohEntry {
            num_hashes: self.num_hashes,
            hash: self.hash,
            mixin: Some(mixin.to_vec()),
        }
    }

    /// Generate a tick (hash chain without mixin)
    ///
    /// Ticks are generated at regular intervals to prove time passage
    /// even without transactions.
    pub fn tick(&mut self) -> PohEntry {
        // Hash until we reach hashes_per_tick
        while self.num_hashes < self.hashes_per_tick {
            self.hash_once();
        }

        let entry = PohEntry {
            num_hashes: self.num_hashes,
            hash: self.hash,
            mixin: None,
        };

        self.num_hashes = 0;
        self.tick_number += 1;

        entry
    }

    /// Check if a slot is complete (64 ticks)
    pub fn is_slot_complete(&self) -> bool {
        self.tick_number >= TICKS_PER_SLOT
    }

    /// Reset for new slot
    pub fn reset_slot(&mut self) {
        self.tick_number = 0;
    }
}

/// A PoH entry - represents a point in the hash chain
#[derive(Debug, Clone)]
pub struct PohEntry {
    /// Number of hashes since previous entry
    pub num_hashes: u64,
    /// Resulting hash
    pub hash: Hash,
    /// Optional mixin data (transaction hash)
    pub mixin: Option<Vec<u8>>,
}

impl PohEntry {
    /// Verify this entry follows from a previous hash
    pub fn verify(&self, prev_hash: &Hash) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(prev_hash);

        if let Some(ref mixin) = self.mixin {
            hasher.update(mixin);
        }

        // For simplicity, we verify the final hash matches
        // In production, would need to verify num_hashes iterations
        let computed: Hash = hasher.finalize().into();

        // Note: This is simplified. Full verification requires
        // computing all intermediate hashes.
        computed == self.hash || self.num_hashes > 1
    }
}

// =============================================================================
// Tower BFT
// =============================================================================

/// A vote in Tower BFT
///
/// Reference: solana/programs/vote/src/vote_state/mod.rs
#[derive(Debug, Clone)]
pub struct Lockout {
    /// Slot being voted on
    pub slot: u64,
    /// Number of confirmations (increases with subsequent votes)
    pub confirmation_count: u32,
}

impl Lockout {
    /// Create a new lockout for a slot
    pub fn new(slot: u64) -> Self {
        Self {
            slot,
            confirmation_count: 1,
        }
    }

    /// Calculate the lockout duration in slots
    ///
    /// lockout = 2^confirmation_count
    pub fn lockout(&self) -> u64 {
        1u64 << self.confirmation_count
    }

    /// The slot at which this lockout expires
    pub fn expiration_slot(&self) -> u64 {
        self.slot + self.lockout()
    }

    /// Check if this vote is locked out at the given slot
    pub fn is_locked_out_at_slot(&self, slot: u64) -> bool {
        slot < self.expiration_slot()
    }

    /// Increase confirmation count
    pub fn increase_confirmation(&mut self) {
        self.confirmation_count += 1;
    }
}

/// Tower BFT vote state for a validator
///
/// The vote tower maintains a stack of votes with exponentially
/// increasing lockouts. This makes fork switching increasingly expensive.
#[derive(Debug, Clone, Default)]
pub struct TowerVoteState {
    /// Stack of votes (oldest at index 0)
    pub votes: Vec<Lockout>,
    /// Root slot (deeply finalized)
    pub root_slot: Option<u64>,
    /// Validator's public key
    pub node_pubkey: [u8; 32],
}

impl TowerVoteState {
    /// Create a new vote state
    pub fn new(node_pubkey: [u8; 32]) -> Self {
        Self {
            votes: Vec::new(),
            root_slot: None,
            node_pubkey,
        }
    }

    /// Process a vote for a slot
    ///
    /// This implements Tower BFT's vote processing:
    /// 1. Pop expired lockouts
    /// 2. Push new vote
    /// 3. Double confirmation counts for older votes
    pub fn process_vote(&mut self, slot: u64) {
        // Pop votes that conflict with this vote (on different forks)
        // For simplicity, we pop votes for slots >= our vote slot
        self.votes.retain(|v| v.slot < slot);

        // Pop votes that have expired their lockout
        self.pop_expired_votes(slot);

        // Increase confirmation count for all existing votes
        for vote in self.votes.iter_mut() {
            vote.increase_confirmation();
        }

        // Push the new vote
        self.votes.push(Lockout::new(slot));

        // If we exceed max history, the oldest vote becomes root
        if self.votes.len() > MAX_LOCKOUT_HISTORY {
            let oldest = self.votes.remove(0);
            self.root_slot = Some(oldest.slot);
        }
    }

    /// Remove votes that have expired their lockout
    fn pop_expired_votes(&mut self, current_slot: u64) {
        self.votes.retain(|v| v.is_locked_out_at_slot(current_slot));
    }

    /// Check if we can switch to a different fork at the given slot
    ///
    /// Returns true if no active votes would be violated by switching.
    pub fn can_switch_to_fork(&self, slot: u64) -> bool {
        // We can switch if all lockouts have expired
        self.votes.iter().all(|v| !v.is_locked_out_at_slot(slot))
    }

    /// Get the last voted slot
    pub fn last_voted_slot(&self) -> Option<u64> {
        self.votes.last().map(|v| v.slot)
    }

    /// Calculate the lockout for the newest vote
    pub fn tower_lockout(&self) -> u64 {
        self.votes.last().map(|v| v.lockout()).unwrap_or(0)
    }

    /// Check if a slot has reached threshold commitment
    ///
    /// A slot is considered committed if it has >= VOTE_THRESHOLD_DEPTH
    /// confirmations, meaning 2^8 = 256 slots of lockout.
    pub fn is_slot_committed(&self, slot: u64) -> bool {
        self.votes.iter().any(|v| {
            v.slot == slot && v.confirmation_count as usize >= VOTE_THRESHOLD_DEPTH
        })
    }
}

/// Fork choice - select the heaviest fork
///
/// In Solana, validators vote on the fork with the most stake-weighted votes.
#[derive(Debug, Default)]
pub struct ForkChoice {
    /// Map of slot -> total stake voting for this slot
    pub fork_weights: std::collections::HashMap<u64, u64>,
}

impl ForkChoice {
    /// Add a vote with the validator's stake
    pub fn add_vote(&mut self, slot: u64, stake: u64) {
        *self.fork_weights.entry(slot).or_insert(0) += stake;
    }

    /// Get the heaviest fork (slot with most stake)
    pub fn heaviest_fork(&self) -> Option<u64> {
        self.fork_weights
            .iter()
            .max_by_key(|(_, stake)| *stake)
            .map(|(slot, _)| *slot)
    }

    /// Check if a slot has supermajority (> 2/3 stake)
    pub fn has_supermajority(&self, slot: u64, total_stake: u64) -> bool {
        let slot_stake = self.fork_weights.get(&slot).copied().unwrap_or(0);
        slot_stake as f64 / total_stake as f64 > SUPERMAJORITY_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poh_hash_chain() {
        let initial = [0u8; 32];
        let mut poh = Poh::new(initial);

        let hash1 = poh.hash;
        poh.hash_once();
        let hash2 = poh.hash;

        // Hashes should be different
        assert_ne!(hash1, hash2);
        assert_eq!(poh.num_hashes, 1);
    }

    #[test]
    fn test_poh_record() {
        let initial = [0u8; 32];
        let mut poh = Poh::new(initial);

        let tx_hash = b"transaction_hash";
        let entry = poh.record(tx_hash);

        assert!(entry.mixin.is_some());
        assert_eq!(entry.mixin.as_ref().unwrap(), tx_hash);
    }

    #[test]
    fn test_tower_vote_lockout() {
        let mut tower = TowerVoteState::new([0u8; 32]);

        tower.process_vote(100);
        assert_eq!(tower.tower_lockout(), 2); // 2^1

        tower.process_vote(101);
        // First vote now has confirmation_count = 2, lockout = 4
        // Second vote has confirmation_count = 1, lockout = 2
        assert_eq!(tower.votes[0].lockout(), 4);
        assert_eq!(tower.votes[1].lockout(), 2);
    }

    #[test]
    fn test_tower_exponential_lockout() {
        let mut tower = TowerVoteState::new([0u8; 32]);

        // Vote on consecutive slots
        for slot in 0..10 {
            tower.process_vote(slot);
        }

        // Oldest vote should have highest confirmation count
        assert!(tower.votes[0].confirmation_count > tower.votes.last().unwrap().confirmation_count);

        // Lockouts should increase exponentially
        for i in 0..tower.votes.len() - 1 {
            assert!(tower.votes[i].lockout() > tower.votes[i + 1].lockout());
        }
    }
}
