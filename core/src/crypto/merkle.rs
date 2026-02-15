//! Merkle Tree implementation
//!
//! Merkle trees provide efficient and secure verification of data integrity.
//! They allow proving that a transaction is included in a block without
//! downloading the entire block (SPV - Simplified Payment Verification).
//!
//! ## Structure:
//! ```text
//!            Root Hash
//!           /         \
//!       Hash01       Hash23
//!       /    \       /    \
//!    Hash0  Hash1  Hash2  Hash3
//!      |      |      |      |
//!    Tx0    Tx1    Tx2    Tx3
//! ```
//!
//! ## Comparison with other blockchains:
//! - **Bitcoin**: Binary Merkle tree with SHA256
//! - **Ethereum**: Modified Merkle Patricia Trie
//! - **Kaspa**: Merkle tree for block transactions
//! - **This implementation**: Binary Merkle tree (Bitcoin-style)

use crate::crypto::hash::sha256;

/// Compute Merkle root from a list of transaction hashes
///
/// # Algorithm:
/// 1. If odd number of hashes, duplicate the last one
/// 2. Pair adjacent hashes and combine them
/// 3. Repeat until only one hash remains (the root)
pub fn compute_merkle_root(hashes: &[Vec<u8>]) -> Vec<u8> {
    if hashes.is_empty() {
        return vec![0u8; 32];
    }

    if hashes.len() == 1 {
        return hashes[0].clone();
    }

    let mut current_level: Vec<Vec<u8>> = hashes.to_vec();

    while current_level.len() > 1 {
        // If odd number of nodes, duplicate the last one
        if current_level.len() % 2 == 1 {
            let last = current_level.last().unwrap().clone();
            current_level.push(last);
        }

        let mut next_level = Vec::new();
        for i in (0..current_level.len()).step_by(2) {
            let combined = combine_hashes(&current_level[i], &current_level[i + 1]);
            next_level.push(combined);
        }
        current_level = next_level;
    }

    current_level.pop().unwrap()
}

/// Combine two hashes into one (concatenate and hash)
fn combine_hashes(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut combined = Vec::with_capacity(left.len() + right.len());
    combined.extend_from_slice(left);
    combined.extend_from_slice(right);
    sha256(&combined)
}

/// Merkle proof for verifying transaction inclusion
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// The path of hashes needed to reconstruct the root
    pub proof: Vec<(Vec<u8>, bool)>, // (hash, is_right)
}

/// Generate a Merkle proof for a specific transaction
///
/// # Arguments
/// * `hashes` - All transaction hashes in the block
/// * `index` - Index of the transaction to prove
///
/// # Returns
/// Proof that can be used to verify the transaction is in the tree
pub fn generate_merkle_proof(hashes: &[Vec<u8>], index: usize) -> Option<MerkleProof> {
    if hashes.is_empty() || index >= hashes.len() {
        return None;
    }

    let mut proof = Vec::new();
    let mut current_level: Vec<Vec<u8>> = hashes.to_vec();
    let mut current_index = index;

    while current_level.len() > 1 {
        // Duplicate last if odd
        if current_level.len() % 2 == 1 {
            let last = current_level.last().unwrap().clone();
            current_level.push(last);
        }

        // Find sibling
        let sibling_index = if current_index % 2 == 0 {
            current_index + 1
        } else {
            current_index - 1
        };

        let is_right = current_index % 2 == 0;
        proof.push((current_level[sibling_index].clone(), is_right));

        // Build next level
        let mut next_level = Vec::new();
        for i in (0..current_level.len()).step_by(2) {
            let combined = combine_hashes(&current_level[i], &current_level[i + 1]);
            next_level.push(combined);
        }

        current_level = next_level;
        current_index /= 2;
    }

    Some(MerkleProof { proof })
}

/// Verify a Merkle proof
///
/// # Arguments
/// * `tx_hash` - Hash of the transaction to verify
/// * `proof` - The Merkle proof
/// * `root` - The expected Merkle root
///
/// # Returns
/// `true` if the proof is valid, `false` otherwise
pub fn verify_merkle_proof(tx_hash: &[u8], proof: &MerkleProof, root: &[u8]) -> bool {
    let mut current_hash = tx_hash.to_vec();

    for (sibling_hash, is_right) in &proof.proof {
        current_hash = if *is_right {
            combine_hashes(&current_hash, sibling_hash)
        } else {
            combine_hashes(sibling_hash, &current_hash)
        };
    }

    current_hash == root
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hash(data: &str) -> Vec<u8> {
        sha256(data.as_bytes())
    }

    #[test]
    fn test_single_transaction() {
        let hashes = vec![make_hash("tx1")];
        let root = compute_merkle_root(&hashes);
        assert_eq!(root, hashes[0]);
    }

    #[test]
    fn test_two_transactions() {
        let hashes = vec![make_hash("tx1"), make_hash("tx2")];
        let root = compute_merkle_root(&hashes);

        // Root should be hash of combined hashes
        let expected = combine_hashes(&hashes[0], &hashes[1]);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_four_transactions() {
        let hashes = vec![
            make_hash("tx1"),
            make_hash("tx2"),
            make_hash("tx3"),
            make_hash("tx4"),
        ];
        let root = compute_merkle_root(&hashes);

        // Manual calculation
        let hash01 = combine_hashes(&hashes[0], &hashes[1]);
        let hash23 = combine_hashes(&hashes[2], &hashes[3]);
        let expected_root = combine_hashes(&hash01, &hash23);

        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_odd_number_transactions() {
        let hashes = vec![make_hash("tx1"), make_hash("tx2"), make_hash("tx3")];
        let root = compute_merkle_root(&hashes);

        // With 3 txs, tx3 should be duplicated
        let hash01 = combine_hashes(&hashes[0], &hashes[1]);
        let hash23 = combine_hashes(&hashes[2], &hashes[2]); // duplicated
        let expected_root = combine_hashes(&hash01, &hash23);

        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_merkle_proof_verification() {
        let hashes = vec![
            make_hash("tx1"),
            make_hash("tx2"),
            make_hash("tx3"),
            make_hash("tx4"),
        ];
        let root = compute_merkle_root(&hashes);

        // Generate and verify proof for each transaction
        for (i, hash) in hashes.iter().enumerate() {
            let proof = generate_merkle_proof(&hashes, i).unwrap();
            assert!(verify_merkle_proof(hash, &proof, &root));

            // Wrong hash should fail
            let wrong_hash = make_hash("wrong");
            assert!(!verify_merkle_proof(&wrong_hash, &proof, &root));
        }
    }

    #[test]
    fn test_empty_transactions() {
        let hashes: Vec<Vec<u8>> = vec![];
        let root = compute_merkle_root(&hashes);
        assert_eq!(root.len(), 32);
    }
}
