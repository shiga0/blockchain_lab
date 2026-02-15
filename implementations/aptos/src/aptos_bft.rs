//! # AptosBFT Consensus
//!
//! DAG-based BFT consensus derived from DiemBFT/Jolteon.
//!
//! ## Key Concepts
//!
//! - **Node**: A vertex in the DAG containing transactions
//! - **NodeId**: Unique identifier (epoch, round, author)
//! - **NodeCertificate**: Quorum signatures over node metadata
//! - **CertifiedNode**: Node with its certificate
//!
//! ## Consensus Flow
//!
//! 1. Each validator creates a Node for each round
//! 2. Node references 2f+1 parent NodeCertificates from previous round
//! 3. Validators vote on nodes, creating NodeCertificates
//! 4. Certified nodes are committed based on anchor rules

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Round number in the DAG
pub type Round = u64;

/// Epoch number
pub type Epoch = u64;

/// Incarnation number for re-execution
pub type Incarnation = u32;

/// Author/Validator index
pub type AuthorityIndex = u32;

/// Timestamp in microseconds (Aptos uses microseconds)
pub type TimestampUsecs = u64;

/// 32-byte hash value
pub type HashValue = [u8; 32];

/// Account address (32 bytes)
pub type AccountAddress = [u8; 32];

/// Signature (64 bytes for Ed25519, variable for BLS)
pub type Signature = Vec<u8>;

/// Aggregate signature from multiple validators
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AggregateSignature {
    /// Bitmap of signers
    pub signers_bitmap: Vec<bool>,
    /// Aggregated signature bytes
    pub signature: Vec<u8>,
}

impl AggregateSignature {
    pub fn new(num_validators: usize) -> Self {
        Self {
            signers_bitmap: vec![false; num_validators],
            signature: Vec::new(),
        }
    }

    pub fn add_signature(&mut self, validator_idx: usize, _signature: &[u8]) {
        if validator_idx < self.signers_bitmap.len() {
            self.signers_bitmap[validator_idx] = true;
        }
    }

    pub fn signers_count(&self) -> usize {
        self.signers_bitmap.iter().filter(|&&b| b).count()
    }

    pub fn has_signer(&self, idx: usize) -> bool {
        self.signers_bitmap.get(idx).copied().unwrap_or(false)
    }
}

/// Unique identifier for a node in the DAG
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId {
    pub epoch: Epoch,
    pub round: Round,
    pub author: AuthorityIndex,
}

impl NodeId {
    pub fn new(epoch: Epoch, round: Round, author: AuthorityIndex) -> Self {
        Self {
            epoch,
            round,
            author,
        }
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NodeId(epoch={}, round={}, author={})",
            self.epoch, self.round, self.author
        )
    }
}

/// Metadata about a node (without payload)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeMetadata {
    pub node_id: NodeId,
    pub timestamp_usecs: TimestampUsecs,
    pub digest: HashValue,
}

impl NodeMetadata {
    pub fn epoch(&self) -> Epoch {
        self.node_id.epoch
    }

    pub fn round(&self) -> Round {
        self.node_id.round
    }

    pub fn author(&self) -> AuthorityIndex {
        self.node_id.author
    }
}

/// Certificate proving quorum agreement on a node
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeCertificate {
    pub metadata: NodeMetadata,
    pub signatures: AggregateSignature,
}

impl NodeCertificate {
    pub fn new(metadata: NodeMetadata, signatures: AggregateSignature) -> Self {
        Self {
            metadata,
            signatures,
        }
    }

    pub fn verify(&self, num_validators: usize) -> bool {
        let threshold = (num_validators * 2 + 2) / 3; // 2f+1 out of 3f+1
        self.signatures.signers_count() >= threshold
    }
}

/// Payload containing transactions
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Payload {
    /// Transaction hashes included in this payload
    pub transaction_hashes: Vec<HashValue>,
    /// Total gas limit for this payload
    pub gas_limit: u64,
}

impl Payload {
    pub fn new(transaction_hashes: Vec<HashValue>, gas_limit: u64) -> Self {
        Self {
            transaction_hashes,
            gas_limit,
        }
    }

    pub fn empty() -> Self {
        Self {
            transaction_hashes: Vec::new(),
            gas_limit: 0,
        }
    }

    pub fn num_transactions(&self) -> usize {
        self.transaction_hashes.len()
    }
}

/// A node (vertex) in the DAG
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    /// Node metadata
    pub metadata: NodeMetadata,
    /// Validator transactions (system transactions)
    pub validator_txns: Vec<HashValue>,
    /// User transaction payload
    pub payload: Payload,
    /// Parent node certificates (2f+1 from previous round)
    pub parents: Vec<NodeCertificate>,
    /// Author's signature
    pub signature: Signature,
}

impl Node {
    pub fn new(
        epoch: Epoch,
        round: Round,
        author: AuthorityIndex,
        timestamp_usecs: TimestampUsecs,
        validator_txns: Vec<HashValue>,
        payload: Payload,
        parents: Vec<NodeCertificate>,
    ) -> Self {
        // Calculate digest
        let digest = Self::calculate_digest(
            epoch,
            round,
            author,
            timestamp_usecs,
            &validator_txns,
            &payload,
            &parents,
        );

        Self {
            metadata: NodeMetadata {
                node_id: NodeId::new(epoch, round, author),
                timestamp_usecs,
                digest,
            },
            validator_txns,
            payload,
            parents,
            signature: Vec::new(),
        }
    }

    fn calculate_digest(
        epoch: Epoch,
        round: Round,
        author: AuthorityIndex,
        timestamp_usecs: TimestampUsecs,
        validator_txns: &[HashValue],
        payload: &Payload,
        parents: &[NodeCertificate],
    ) -> HashValue {
        let mut hasher = Sha256::new();
        hasher.update(epoch.to_le_bytes());
        hasher.update(round.to_le_bytes());
        hasher.update(author.to_le_bytes());
        hasher.update(timestamp_usecs.to_le_bytes());

        for txn in validator_txns {
            hasher.update(txn);
        }

        for hash in &payload.transaction_hashes {
            hasher.update(hash);
        }

        for parent in parents {
            hasher.update(&parent.metadata.digest);
        }

        hasher.finalize().into()
    }

    pub fn id(&self) -> &NodeId {
        &self.metadata.node_id
    }

    pub fn epoch(&self) -> Epoch {
        self.metadata.epoch()
    }

    pub fn round(&self) -> Round {
        self.metadata.round()
    }

    pub fn author(&self) -> AuthorityIndex {
        self.metadata.author()
    }

    pub fn digest(&self) -> &HashValue {
        &self.metadata.digest
    }

    pub fn verify(&self, num_validators: usize) -> Result<(), &'static str> {
        // Round 1 should have no parents
        if self.round() == 1 {
            if !self.parents.is_empty() {
                return Err("Round 1 should have no parents");
            }
            return Ok(());
        }

        // Check parent round
        let expected_parent_round = self.round() - 1;
        for parent in &self.parents {
            if parent.metadata.round() != expected_parent_round {
                return Err("Parent round mismatch");
            }
        }

        // Check parent quorum (2f+1)
        let threshold = (num_validators * 2 + 2) / 3;
        let unique_parents: HashSet<_> = self.parents.iter().map(|p| p.metadata.author()).collect();
        if unique_parents.len() < threshold {
            return Err("Not enough unique parent authors");
        }

        Ok(())
    }
}

/// Certified node = Node + its certificate
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CertifiedNode {
    pub node: Node,
    pub certificate: AggregateSignature,
}

impl CertifiedNode {
    pub fn new(node: Node, certificate: AggregateSignature) -> Self {
        Self { node, certificate }
    }

    pub fn to_certificate(&self) -> NodeCertificate {
        NodeCertificate::new(self.node.metadata.clone(), self.certificate.clone())
    }
}

/// Vote for a node
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vote {
    pub metadata: NodeMetadata,
    pub voter: AuthorityIndex,
    pub signature: Signature,
}

impl Vote {
    pub fn new(metadata: NodeMetadata, voter: AuthorityIndex) -> Self {
        Self {
            metadata,
            voter,
            signature: Vec::new(),
        }
    }
}

/// DAG state tracking all nodes
#[derive(Debug)]
pub struct DagState {
    /// All certified nodes indexed by NodeId
    nodes: HashMap<NodeId, CertifiedNode>,
    /// Nodes by round
    nodes_by_round: BTreeMap<Round, Vec<NodeId>>,
    /// Latest round per author
    latest_round: HashMap<AuthorityIndex, Round>,
    /// Number of validators
    num_validators: usize,
    /// Current epoch
    epoch: Epoch,
}

impl DagState {
    pub fn new(num_validators: usize, epoch: Epoch) -> Self {
        Self {
            nodes: HashMap::new(),
            nodes_by_round: BTreeMap::new(),
            latest_round: HashMap::new(),
            num_validators,
            epoch,
        }
    }

    pub fn add_certified_node(&mut self, certified_node: CertifiedNode) -> Result<(), &'static str> {
        let node_id = certified_node.node.id().clone();

        // Check epoch
        if certified_node.node.epoch() != self.epoch {
            return Err("Epoch mismatch");
        }

        // Check if already exists
        if self.nodes.contains_key(&node_id) {
            return Err("Node already exists");
        }

        // Verify certificate
        let threshold = (self.num_validators * 2 + 2) / 3;
        if certified_node.certificate.signers_count() < threshold {
            return Err("Insufficient certificate signers");
        }

        // Update indices
        self.nodes_by_round
            .entry(node_id.round)
            .or_default()
            .push(node_id.clone());

        self.latest_round
            .entry(node_id.author)
            .and_modify(|r| *r = (*r).max(node_id.round))
            .or_insert(node_id.round);

        self.nodes.insert(node_id, certified_node);
        Ok(())
    }

    pub fn get_node(&self, node_id: &NodeId) -> Option<&CertifiedNode> {
        self.nodes.get(node_id)
    }

    pub fn get_nodes_at_round(&self, round: Round) -> Vec<&CertifiedNode> {
        self.nodes_by_round
            .get(&round)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn latest_round(&self) -> Round {
        self.nodes_by_round.keys().last().copied().unwrap_or(0)
    }

    pub fn get_strong_links(&self, round: Round) -> Vec<NodeCertificate> {
        self.get_nodes_at_round(round)
            .into_iter()
            .map(|cn| cn.to_certificate())
            .collect()
    }

    /// Check if we have 2f+1 nodes at a round
    pub fn has_quorum_at_round(&self, round: Round) -> bool {
        let threshold = (self.num_validators * 2 + 2) / 3;
        self.get_nodes_at_round(round).len() >= threshold
    }
}

/// Order rule for committing nodes
#[derive(Debug)]
pub struct OrderRule {
    /// Committed node IDs in order
    committed: Vec<NodeId>,
    /// Last committed round
    last_committed_round: Round,
}

impl OrderRule {
    pub fn new() -> Self {
        Self {
            committed: Vec::new(),
            last_committed_round: 0,
        }
    }

    /// Try to commit nodes based on anchor rules
    /// Returns newly committed nodes
    pub fn try_commit(&mut self, dag: &DagState) -> Vec<NodeId> {
        let mut newly_committed = Vec::new();

        // Find rounds with quorum that haven't been committed
        let latest = dag.latest_round();

        for round in (self.last_committed_round + 1)..=latest {
            if dag.has_quorum_at_round(round) {
                // Get nodes at this round and commit them
                for node in dag.get_nodes_at_round(round) {
                    let node_id = node.node.id().clone();
                    if !self.committed.contains(&node_id) {
                        self.committed.push(node_id.clone());
                        newly_committed.push(node_id);
                    }
                }
                self.last_committed_round = round;
            }
        }

        newly_committed
    }

    pub fn committed_nodes(&self) -> &[NodeId] {
        &self.committed
    }

    pub fn last_committed_round(&self) -> Round {
        self.last_committed_round
    }
}

impl Default for OrderRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Block metadata for execution
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockMetadata {
    pub id: HashValue,
    pub epoch: Epoch,
    pub round: Round,
    pub proposer: AccountAddress,
    pub previous_block_votes: Vec<bool>,
    pub failed_proposer_indices: Vec<u32>,
    pub timestamp_usecs: TimestampUsecs,
}

impl BlockMetadata {
    pub fn new(
        id: HashValue,
        epoch: Epoch,
        round: Round,
        proposer: AccountAddress,
        timestamp_usecs: TimestampUsecs,
    ) -> Self {
        Self {
            id,
            epoch,
            round,
            proposer,
            previous_block_votes: Vec::new(),
            failed_proposer_indices: Vec::new(),
            timestamp_usecs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node(epoch: Epoch, round: Round, author: AuthorityIndex) -> Node {
        Node::new(
            epoch,
            round,
            author,
            1000000 * round, // timestamp
            Vec::new(),
            Payload::empty(),
            Vec::new(),
        )
    }

    fn certify_node(node: Node, num_validators: usize) -> CertifiedNode {
        let mut sig = AggregateSignature::new(num_validators);
        // Add 2f+1 signatures
        let threshold = (num_validators * 2 + 2) / 3;
        for i in 0..threshold {
            sig.add_signature(i, &[]);
        }
        CertifiedNode::new(node, sig)
    }

    #[test]
    fn test_node_id() {
        let id = NodeId::new(1, 2, 3);
        assert_eq!(id.epoch, 1);
        assert_eq!(id.round, 2);
        assert_eq!(id.author, 3);
    }

    #[test]
    fn test_aggregate_signature() {
        let mut sig = AggregateSignature::new(4);
        assert_eq!(sig.signers_count(), 0);

        sig.add_signature(0, &[]);
        sig.add_signature(2, &[]);
        assert_eq!(sig.signers_count(), 2);
        assert!(sig.has_signer(0));
        assert!(!sig.has_signer(1));
        assert!(sig.has_signer(2));
    }

    #[test]
    fn test_node_creation() {
        let node = create_test_node(1, 1, 0);
        assert_eq!(node.epoch(), 1);
        assert_eq!(node.round(), 1);
        assert_eq!(node.author(), 0);
    }

    #[test]
    fn test_node_certificate() {
        let node = create_test_node(1, 1, 0);
        let certified = certify_node(node, 4);
        let cert = certified.to_certificate();
        assert!(cert.verify(4));
    }

    #[test]
    fn test_dag_state() {
        let mut dag = DagState::new(4, 1);

        // Add nodes for round 1
        for author in 0..4 {
            let node = create_test_node(1, 1, author);
            let certified = certify_node(node, 4);
            dag.add_certified_node(certified).unwrap();
        }

        assert_eq!(dag.latest_round(), 1);
        assert!(dag.has_quorum_at_round(1));
        assert_eq!(dag.get_nodes_at_round(1).len(), 4);
    }

    #[test]
    fn test_order_rule() {
        let mut dag = DagState::new(4, 1);
        let mut order = OrderRule::new();

        // Add nodes for round 1
        for author in 0..4 {
            let node = create_test_node(1, 1, author);
            let certified = certify_node(node, 4);
            dag.add_certified_node(certified).unwrap();
        }

        // Try to commit
        let committed = order.try_commit(&dag);
        assert_eq!(committed.len(), 4);
        assert_eq!(order.last_committed_round(), 1);
    }

    #[test]
    fn test_block_metadata() {
        let id = [0u8; 32];
        let proposer = [1u8; 32];
        let meta = BlockMetadata::new(id, 1, 1, proposer, 1000000);

        assert_eq!(meta.epoch, 1);
        assert_eq!(meta.round, 1);
        assert_eq!(meta.timestamp_usecs, 1000000);
    }

    #[test]
    fn test_payload() {
        let payload = Payload::new(vec![[1u8; 32], [2u8; 32]], 1000);
        assert_eq!(payload.num_transactions(), 2);
        assert_eq!(payload.gas_limit, 1000);

        let empty = Payload::empty();
        assert_eq!(empty.num_transactions(), 0);
    }

    #[test]
    fn test_node_with_parents() {
        let mut dag = DagState::new(4, 1);

        // Add round 1 nodes
        for author in 0..4 {
            let node = create_test_node(1, 1, author);
            let certified = certify_node(node, 4);
            dag.add_certified_node(certified).unwrap();
        }

        // Create round 2 node with parents
        let parents = dag.get_strong_links(1);
        let node2 = Node::new(
            1,
            2,
            0,
            2000000,
            Vec::new(),
            Payload::empty(),
            parents,
        );

        assert_eq!(node2.round(), 2);
        assert!(node2.verify(4).is_ok());
    }
}
