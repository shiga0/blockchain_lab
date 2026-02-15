//! Block validation logic
//!
//! This module handles validation of blocks beyond consensus rules,
//! including transaction validation and structural checks.
//!
//! ## Validation layers:
//! 1. **Structural** - Block format, field sizes
//! 2. **Contextual** - Links to previous block, timestamps
//! 3. **Consensus** - PoW/PoS requirements (delegated to Consensus trait)
//! 4. **Transaction** - All transactions are valid

use crate::primitives::Block;

/// Block validation result
pub type ValidationResult = Result<(), ValidationError>;

/// Errors that can occur during block validation
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Block structure is invalid
    InvalidStructure(String),
    /// Block timestamp is invalid
    InvalidTimestamp(String),
    /// Transaction validation failed
    InvalidTransaction(String),
    /// Merkle root mismatch
    InvalidMerkleRoot,
    /// Empty block (no transactions)
    EmptyBlock,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidStructure(msg) => write!(f, "Invalid structure: {}", msg),
            ValidationError::InvalidTimestamp(msg) => write!(f, "Invalid timestamp: {}", msg),
            ValidationError::InvalidTransaction(msg) => write!(f, "Invalid transaction: {}", msg),
            ValidationError::InvalidMerkleRoot => write!(f, "Merkle root mismatch"),
            ValidationError::EmptyBlock => write!(f, "Block has no transactions"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Block validator
pub struct BlockValidator {
    /// Maximum allowed timestamp drift (seconds into the future)
    max_future_time: i64,
}

impl Default for BlockValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockValidator {
    /// Create a new validator with default settings
    pub fn new() -> Self {
        BlockValidator {
            max_future_time: 7200, // 2 hours (Bitcoin's rule)
        }
    }

    /// Create validator with custom settings
    pub fn with_max_future_time(max_future_time: i64) -> Self {
        BlockValidator { max_future_time }
    }

    /// Validate block structure (without context)
    pub fn validate_structure(&self, block: &Block) -> ValidationResult {
        // Check hash is not empty
        if block.get_hash().is_empty() {
            return Err(ValidationError::InvalidStructure(
                "Block hash is empty".to_string(),
            ));
        }

        // Check hash format (should be 64 hex characters)
        if block.get_hash().len() != 64 {
            return Err(ValidationError::InvalidStructure(
                "Block hash has invalid length".to_string(),
            ));
        }

        // Validate hash contains only hex characters
        if !block.get_hash().chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ValidationError::InvalidStructure(
                "Block hash contains non-hex characters".to_string(),
            ));
        }

        // Check block has transactions
        if block.get_transactions().is_empty() {
            return Err(ValidationError::EmptyBlock);
        }

        Ok(())
    }

    /// Validate block timestamp
    pub fn validate_timestamp(&self, block: &Block, prev_block: Option<&Block>) -> ValidationResult {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Block timestamp should not be too far in the future
        if block.get_timestamp() > current_time + (self.max_future_time * 1000) {
            return Err(ValidationError::InvalidTimestamp(
                "Block timestamp too far in the future".to_string(),
            ));
        }

        // Block timestamp should be greater than previous block
        if let Some(prev) = prev_block {
            if block.get_timestamp() <= prev.get_timestamp() {
                return Err(ValidationError::InvalidTimestamp(
                    "Block timestamp not greater than previous block".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validate all transactions in a block
    pub fn validate_transactions(&self, block: &Block) -> ValidationResult {
        let transactions = block.get_transactions();

        // First transaction must be coinbase
        if transactions.is_empty() {
            return Err(ValidationError::EmptyBlock);
        }

        // Check for duplicate transactions
        let mut seen_ids = std::collections::HashSet::new();
        for tx in transactions {
            let tx_id = tx.get_id();
            if !seen_ids.insert(tx_id.to_vec()) {
                return Err(ValidationError::InvalidTransaction(
                    "Duplicate transaction in block".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Perform full block validation
    pub fn validate(&self, block: &Block, prev_block: Option<&Block>) -> ValidationResult {
        self.validate_structure(block)?;
        self.validate_timestamp(block, prev_block)?;
        self.validate_transactions(block)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require Block to be fully implemented
    // Placeholder for now
    #[test]
    fn test_validator_creation() {
        let validator = BlockValidator::new();
        assert_eq!(validator.max_future_time, 7200);
    }
}
