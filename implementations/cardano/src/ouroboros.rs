//! Ouroboros Praos Consensus
//!
//! ## Consensus Comparison
//!
//! | Aspect | Nakamoto PoW | Casper PoS | Ouroboros Praos |
//! |--------|--------------|------------|-----------------|
//! | Leader Selection | Hash puzzle | RANDAO | VRF lottery |
//! | Finality | Probabilistic | Economic | Probabilistic |
//! | Security Proof | None | Partial | Full (in Praos) |
//! | Slot Duration | ~10 min | ~12 sec | 1 sec |
//! | Empty Slots | No | No | Yes (~95%) |
//!
//! ## Slot and Epoch Structure
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                           Epoch N                                   │
//! │                     (432,000 slots = ~5 days)                       │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ Slot 0  │ Slot 1  │ Slot 2  │ ... │ Slot 431,999                   │
//! │ [Block] │ [empty] │ [empty] │     │ [Block]                        │
//! │         │         │         │     │                                │
//! │ ← 1 second per slot →                                              │
//! └─────────────────────────────────────────────────────────────────────┘
//!
//! Active Slot Coefficient (f) = 0.05
//! → ~5% of slots have blocks
//! → ~21,600 blocks per epoch
//! ```
//!
//! ## VRF-based Leader Selection
//!
//! ```text
//! For each slot:
//!   1. Pool computes: VRF_output = VRF(pool_key, slot_nonce || slot_no)
//!   2. Compare: VRF_output < threshold(pool_stake / total_stake)
//!   3. If true → Pool is slot leader, can produce block
//!
//! Threshold calculation:
//!   threshold = 1 - (1 - f)^(pool_stake / total_stake)
//!   where f = active_slot_coefficient
//! ```
//!
//! ## Chain Selection Rule
//!
//! ```text
//! If multiple valid chains:
//!   1. Prefer chain with more blocks (longest chain)
//!   2. If equal length, prefer lower VRF output
//!   3. Never switch more than k blocks back (security parameter)
//! ```

use crate::constants::*;
use crate::eutxo::TxHash;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Slot number (absolute)
pub type SlotNo = u64;

/// Epoch number
pub type EpochNo = u64;

/// Block number (height)
pub type BlockNo = u64;

/// Pool ID (28 bytes - hash of VRF key)
pub type PoolId = [u8; 28];

/// VRF output (32 bytes)
pub type VrfOutput = [u8; 32];

/// Block hash
pub type BlockHash = [u8; 32];

// =============================================================================
// Slot and Epoch
// =============================================================================

/// Get epoch number from slot
pub fn slot_to_epoch(slot: SlotNo) -> EpochNo {
    slot / SLOTS_PER_EPOCH
}

/// Get first slot of epoch
pub fn epoch_first_slot(epoch: EpochNo) -> SlotNo {
    epoch * SLOTS_PER_EPOCH
}

/// Get slot within epoch
pub fn slot_in_epoch(slot: SlotNo) -> SlotNo {
    slot % SLOTS_PER_EPOCH
}

/// Check if slot is at epoch boundary
pub fn is_epoch_boundary(slot: SlotNo) -> bool {
    slot_in_epoch(slot) == 0
}

// =============================================================================
// Stake Pool
// =============================================================================

/// Stake pool registration
#[derive(Debug, Clone)]
pub struct StakePool {
    /// Pool ID
    pub pool_id: PoolId,
    /// VRF verification key
    pub vrf_vkey: [u8; 32],
    /// Pool pledge (owner's stake)
    pub pledge: u64,
    /// Pool cost per epoch
    pub cost: u64,
    /// Pool margin (0.0 - 1.0)
    pub margin: f64,
    /// Pool metadata URL
    pub metadata_url: Option<String>,
    /// Reward account
    pub reward_account: [u8; 28],
    /// Pool owners
    pub owners: Vec<[u8; 28]>,
}

/// Pool distribution snapshot
#[derive(Debug, Clone, Default)]
pub struct PoolDistr {
    /// Pool stake: pool_id -> (stake, vrf_key_hash)
    pub pools: HashMap<PoolId, (u64, [u8; 32])>,
    /// Total stake
    pub total_stake: u64,
}

impl PoolDistr {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update pool stake
    pub fn set_pool_stake(&mut self, pool_id: PoolId, stake: u64, vrf_hash: [u8; 32]) {
        if let Some((old_stake, _)) = self.pools.get(&pool_id) {
            self.total_stake -= old_stake;
        }
        self.total_stake += stake;
        self.pools.insert(pool_id, (stake, vrf_hash));
    }

    /// Get pool's relative stake
    pub fn relative_stake(&self, pool_id: &PoolId) -> f64 {
        if self.total_stake == 0 {
            return 0.0;
        }
        self.pools
            .get(pool_id)
            .map(|(stake, _)| *stake as f64 / self.total_stake as f64)
            .unwrap_or(0.0)
    }

    /// Get number of pools
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}

// =============================================================================
// VRF and Leader Election
// =============================================================================

/// VRF proof (simplified)
#[derive(Debug, Clone)]
pub struct VrfProof {
    /// VRF output
    pub output: VrfOutput,
    /// Proof bytes
    pub proof: Vec<u8>,
}

/// Leader election result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderCheck {
    /// Not a leader for this slot
    NotLeader,
    /// Leader for this slot
    Leader(VrfOutput),
}

/// Check if pool is slot leader (simplified VRF)
pub fn check_leader(
    pool_id: &PoolId,
    vrf_key: &[u8; 32],
    slot: SlotNo,
    epoch_nonce: &[u8; 32],
    relative_stake: f64,
) -> LeaderCheck {
    // Compute VRF output (simplified: just hash)
    let mut hasher = Sha256::new();
    hasher.update(vrf_key);
    hasher.update(&slot.to_le_bytes());
    hasher.update(epoch_nonce);
    let output: VrfOutput = hasher.finalize().into();

    // Convert output to threshold value (0.0 - 1.0)
    let vrf_value = u64::from_be_bytes(output[..8].try_into().unwrap()) as f64 / u64::MAX as f64;

    // Calculate threshold based on stake
    // threshold = 1 - (1 - f)^sigma where sigma = relative_stake
    let threshold = 1.0 - (1.0 - ACTIVE_SLOT_COEFF).powf(relative_stake);

    if vrf_value < threshold {
        LeaderCheck::Leader(output)
    } else {
        LeaderCheck::NotLeader
    }
}

// =============================================================================
// Block Header
// =============================================================================

/// Block header
#[derive(Debug, Clone)]
pub struct BlockHeader {
    /// Slot number
    pub slot: SlotNo,
    /// Block number (height)
    pub block_no: BlockNo,
    /// Previous block hash
    pub prev_hash: BlockHash,
    /// Issuer pool ID
    pub issuer_id: PoolId,
    /// VRF result for leader election
    pub vrf_result: VrfOutput,
    /// Block body hash
    pub body_hash: [u8; 32],
    /// Operational certificate
    pub op_cert: OpCert,
    /// Protocol version
    pub protocol_version: (u16, u16),
}

impl BlockHeader {
    /// Compute block hash
    pub fn hash(&self) -> BlockHash {
        let mut hasher = Sha256::new();
        hasher.update(&self.slot.to_le_bytes());
        hasher.update(&self.block_no.to_le_bytes());
        hasher.update(&self.prev_hash);
        hasher.update(&self.issuer_id);
        hasher.update(&self.vrf_result);
        hasher.update(&self.body_hash);
        hasher.finalize().into()
    }
}

/// Operational certificate (for hot key rotation)
#[derive(Debug, Clone)]
pub struct OpCert {
    /// Hot verification key
    pub hot_vkey: [u8; 32],
    /// Sequence number
    pub sequence: u64,
    /// KES period
    pub kes_period: u64,
    /// Signature by cold key
    pub sigma: [u8; 64],
}

// =============================================================================
// Block Body
// =============================================================================

/// Block body
#[derive(Debug, Clone)]
pub struct BlockBody {
    /// Transactions in the block
    pub txs: Vec<crate::eutxo::Tx>,
}

impl BlockBody {
    /// Compute body hash
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for tx in &self.txs {
            hasher.update(&tx.hash());
        }
        hasher.finalize().into()
    }
}

/// Complete block
#[derive(Debug, Clone)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// Block body
    pub body: BlockBody,
}

impl Block {
    /// Get block hash
    pub fn hash(&self) -> BlockHash {
        self.header.hash()
    }

    /// Get slot number
    pub fn slot(&self) -> SlotNo {
        self.header.slot
    }

    /// Get block number
    pub fn block_no(&self) -> BlockNo {
        self.header.block_no
    }

    /// Transaction count
    pub fn tx_count(&self) -> usize {
        self.body.txs.len()
    }
}

// =============================================================================
// Chain State
// =============================================================================

/// Chain tip info
#[derive(Debug, Clone)]
pub struct ChainTip {
    /// Block hash
    pub hash: BlockHash,
    /// Slot number
    pub slot: SlotNo,
    /// Block number
    pub block_no: BlockNo,
}

/// Consensus state
#[derive(Debug)]
pub struct ConsensusState {
    /// Current chain tip
    pub tip: ChainTip,
    /// Current epoch nonce
    pub epoch_nonce: [u8; 32],
    /// Pool distribution for current epoch
    pub pool_distr: PoolDistr,
    /// Stability window (slots)
    pub stability_window: u64,
    /// Security parameter k
    pub security_param: u64,
}

impl ConsensusState {
    pub fn new() -> Self {
        Self {
            tip: ChainTip {
                hash: [0u8; 32],
                slot: 0,
                block_no: 0,
            },
            epoch_nonce: [0u8; 32],
            pool_distr: PoolDistr::new(),
            stability_window: SECURITY_PARAMETER * 3,
            security_param: SECURITY_PARAMETER,
        }
    }

    /// Check if block can be rolled back
    pub fn can_rollback(&self, block_no: BlockNo) -> bool {
        self.tip.block_no.saturating_sub(block_no) < self.security_param
    }

    /// Update tip
    pub fn update_tip(&mut self, hash: BlockHash, slot: SlotNo, block_no: BlockNo) {
        self.tip = ChainTip {
            hash,
            slot,
            block_no,
        };
    }

    /// Update epoch nonce (at epoch boundary)
    pub fn update_epoch_nonce(&mut self, new_nonce: [u8; 32]) {
        self.epoch_nonce = new_nonce;
    }

    /// Current epoch
    pub fn current_epoch(&self) -> EpochNo {
        slot_to_epoch(self.tip.slot)
    }
}

impl Default for ConsensusState {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Chain Selection
// =============================================================================

/// Chain selection between two chains
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainPreference {
    /// Prefer the first chain
    PreferFirst,
    /// Prefer the second chain
    PreferSecond,
    /// Chains are equivalent
    Equivalent,
}

/// Compare two chains for selection
pub fn compare_chains(
    chain1: &ChainTip,
    chain2: &ChainTip,
    fork_point_block_no: BlockNo,
    security_param: u64,
) -> ChainPreference {
    // Cannot switch if fork point is beyond security parameter
    let depth1 = chain1.block_no.saturating_sub(fork_point_block_no);
    let depth2 = chain2.block_no.saturating_sub(fork_point_block_no);

    if depth1 > security_param || depth2 > security_param {
        // Prefer the one we're currently on (assumed to be chain1)
        return ChainPreference::PreferFirst;
    }

    // Prefer longer chain
    match chain1.block_no.cmp(&chain2.block_no) {
        std::cmp::Ordering::Greater => ChainPreference::PreferFirst,
        std::cmp::Ordering::Less => ChainPreference::PreferSecond,
        std::cmp::Ordering::Equal => {
            // Same length - prefer lower slot (earlier)
            match chain1.slot.cmp(&chain2.slot) {
                std::cmp::Ordering::Less => ChainPreference::PreferFirst,
                std::cmp::Ordering::Greater => ChainPreference::PreferSecond,
                std::cmp::Ordering::Equal => ChainPreference::Equivalent,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_epoch_conversion() {
        assert_eq!(slot_to_epoch(0), 0);
        assert_eq!(slot_to_epoch(431_999), 0);
        assert_eq!(slot_to_epoch(432_000), 1);
        assert_eq!(slot_to_epoch(864_000), 2);

        assert_eq!(epoch_first_slot(0), 0);
        assert_eq!(epoch_first_slot(1), 432_000);
        assert_eq!(epoch_first_slot(2), 864_000);

        assert_eq!(slot_in_epoch(432_100), 100);
    }

    #[test]
    fn test_epoch_boundary() {
        assert!(is_epoch_boundary(0));
        assert!(is_epoch_boundary(432_000));
        assert!(!is_epoch_boundary(1));
        assert!(!is_epoch_boundary(431_999));
    }

    #[test]
    fn test_pool_distribution() {
        let mut distr = PoolDistr::new();

        let pool1 = [1u8; 28];
        let pool2 = [2u8; 28];

        distr.set_pool_stake(pool1, 1000, [0u8; 32]);
        distr.set_pool_stake(pool2, 3000, [0u8; 32]);

        assert_eq!(distr.total_stake, 4000);
        assert!((distr.relative_stake(&pool1) - 0.25).abs() < 0.001);
        assert!((distr.relative_stake(&pool2) - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_leader_check_probability() {
        let pool_id = [1u8; 28];
        let vrf_key = [2u8; 32];
        let epoch_nonce = [0u8; 32];

        // With 100% stake, should be leader most of the time
        let mut leader_count = 0;
        for slot in 0..1000 {
            if let LeaderCheck::Leader(_) = check_leader(&pool_id, &vrf_key, slot, &epoch_nonce, 1.0) {
                leader_count += 1;
            }
        }
        // With f=0.05 and 100% stake, expect ~50 leaders per 1000 slots
        // (because threshold = 1 - (1-0.05)^1 = 0.05)
        // Actually for 100% stake, threshold is exactly 0.05
        assert!(
            leader_count > 20 && leader_count < 100,
            "Leader count: {}",
            leader_count
        );
    }

    #[test]
    fn test_chain_selection() {
        let chain1 = ChainTip {
            hash: [1u8; 32],
            slot: 100,
            block_no: 50,
        };
        let chain2 = ChainTip {
            hash: [2u8; 32],
            slot: 100,
            block_no: 45,
        };

        // Longer chain wins
        assert_eq!(
            compare_chains(&chain1, &chain2, 40, 2160),
            ChainPreference::PreferFirst
        );

        // Same length, earlier slot wins
        let chain3 = ChainTip {
            hash: [3u8; 32],
            slot: 90,
            block_no: 50,
        };
        assert_eq!(
            compare_chains(&chain1, &chain3, 40, 2160),
            ChainPreference::PreferSecond
        );
    }

    #[test]
    fn test_consensus_state_rollback() {
        let mut state = ConsensusState::new();
        state.update_tip([1u8; 32], 1000, 100);

        // Can rollback recent blocks
        assert!(state.can_rollback(99));
        assert!(state.can_rollback(50));

        // Cannot rollback beyond security parameter
        // With k=2160, block 100 - 2160 = negative, so all should be rollbackable
        // Let's test with a higher block number
        state.update_tip([2u8; 32], 5000, 3000);
        assert!(state.can_rollback(2999)); // depth = 1
        assert!(state.can_rollback(1000)); // depth = 2000
        assert!(!state.can_rollback(500)); // depth = 2500 > 2160
    }
}
