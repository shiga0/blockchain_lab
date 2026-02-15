//! Application Blockchain Interface (ABCI)
//!
//! ABCI is the interface between CometBFT (consensus) and the application.
//!
//! ## ABCI vs Other Approaches
//!
//! | Aspect | Bitcoin | Ethereum | Cosmos (ABCI) |
//! |--------|---------|----------|---------------|
//! | State Machine | Hardcoded | EVM | Any (via ABCI) |
//! | Modularity | Low | Medium | High |
//! | Language | C++ | Go/Rust/etc | Any |
//! | Customization | Fork | Smart contracts | ABCI + modules |
//!
//! ## ABCI Methods
//!
//! ### Consensus Connection (Block Execution)
//!
//! ```text
//! Consensus          ABCI            Application
//!    │                │                  │
//!    │ InitChain      │                  │
//!    │───────────────→│──────────────────→│ (Genesis setup)
//!    │                │                  │
//!    │ PrepareProposal│                  │
//!    │───────────────→│──────────────────→│ (Proposer builds block)
//!    │                │                  │
//!    │ ProcessProposal│                  │
//!    │───────────────→│──────────────────→│ (Validators validate)
//!    │                │                  │
//!    │ FinalizeBlock  │                  │
//!    │───────────────→│──────────────────→│ (Execute all txs)
//!    │                │                  │
//!    │ Commit         │                  │
//!    │───────────────→│──────────────────→│ (Persist state)
//!    │                │                  │
//!    │                │←─ AppHash ───────│ (State root)
//!    │                │                  │
//! ```
//!
//! ## Mempool Connection (Transaction Validation)
//!
//! ```text
//! User TX → CheckTx → (valid?) → Add to mempool
//!                         │
//!                         └→ (invalid?) → Reject
//! ```

use crate::types::Hash;

/// Transaction result code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// Success
    Ok = 0,
    /// Unknown error
    UnknownError = 1,
    /// Invalid transaction
    InvalidTx = 2,
    /// Insufficient funds
    InsufficientFunds = 3,
    /// Invalid signature
    InvalidSignature = 4,
    /// Invalid nonce
    InvalidNonce = 5,
}

// =============================================================================
// Request Types
// =============================================================================

/// Initialize chain at genesis
#[derive(Debug, Clone)]
pub struct RequestInitChain {
    /// Chain ID
    pub chain_id: String,
    /// Genesis time
    pub genesis_time: u64,
    /// Initial validators
    pub validators: Vec<ValidatorUpdate>,
    /// Application-specific genesis state
    pub app_state_bytes: Vec<u8>,
}

/// Check if transaction is valid for mempool
#[derive(Debug, Clone)]
pub struct RequestCheckTx {
    /// Transaction bytes
    pub tx: Vec<u8>,
    /// Check type (new or recheck)
    pub check_type: CheckTxType,
}

/// Check type for CheckTx
#[derive(Debug, Clone, Copy)]
pub enum CheckTxType {
    /// New transaction
    New,
    /// Rechecking after commit
    Recheck,
}

/// Proposer prepares block proposal
#[derive(Debug, Clone)]
pub struct RequestPrepareProposal {
    /// Maximum bytes for transactions
    pub max_tx_bytes: i64,
    /// Transactions from mempool
    pub txs: Vec<Vec<u8>>,
    /// Block height
    pub height: i64,
}

/// Validators process (validate) proposal
#[derive(Debug, Clone)]
pub struct RequestProcessProposal {
    /// Proposed transactions
    pub txs: Vec<Vec<u8>>,
    /// Block hash
    pub hash: Hash,
    /// Block height
    pub height: i64,
}

/// Finalize and execute block
#[derive(Debug, Clone)]
pub struct RequestFinalizeBlock {
    /// Transactions to execute
    pub txs: Vec<Vec<u8>>,
    /// Block hash
    pub hash: Hash,
    /// Block height
    pub height: i64,
    /// Block time
    pub time: u64,
    /// Proposer address
    pub proposer_address: [u8; 20],
}

/// Commit state
#[derive(Debug, Clone)]
pub struct RequestCommit {}

/// Query application state
#[derive(Debug, Clone)]
pub struct RequestQuery {
    /// Query path
    pub path: String,
    /// Query data
    pub data: Vec<u8>,
    /// Block height (0 = latest)
    pub height: i64,
    /// Return proof?
    pub prove: bool,
}

// =============================================================================
// Response Types
// =============================================================================

/// Response to InitChain
#[derive(Debug, Clone, Default)]
pub struct ResponseInitChain {
    /// Initial app hash
    pub app_hash: Hash,
    /// Validator updates (if different from request)
    pub validators: Vec<ValidatorUpdate>,
}

/// Response to CheckTx
#[derive(Debug, Clone)]
pub struct ResponseCheckTx {
    /// Result code
    pub code: Code,
    /// Error message (if any)
    pub log: String,
    /// Gas wanted
    pub gas_wanted: i64,
    /// Gas used
    pub gas_used: i64,
}

impl Default for ResponseCheckTx {
    fn default() -> Self {
        Self {
            code: Code::Ok,
            log: String::new(),
            gas_wanted: 0,
            gas_used: 0,
        }
    }
}

/// Response to PrepareProposal
#[derive(Debug, Clone)]
pub struct ResponsePrepareProposal {
    /// Transactions to include in block
    pub txs: Vec<Vec<u8>>,
}

/// Response to ProcessProposal
#[derive(Debug, Clone, Copy)]
pub enum ProposalStatus {
    /// Accept proposal
    Accept,
    /// Reject proposal
    Reject,
}

#[derive(Debug, Clone)]
pub struct ResponseProcessProposal {
    /// Accept or reject
    pub status: ProposalStatus,
}

/// Transaction execution result
#[derive(Debug, Clone)]
pub struct ExecTxResult {
    /// Result code
    pub code: Code,
    /// Result data
    pub data: Vec<u8>,
    /// Log message
    pub log: String,
    /// Gas wanted
    pub gas_wanted: i64,
    /// Gas used
    pub gas_used: i64,
    /// Events emitted
    pub events: Vec<Event>,
}

impl Default for ExecTxResult {
    fn default() -> Self {
        Self {
            code: Code::Ok,
            data: Vec::new(),
            log: String::new(),
            gas_wanted: 0,
            gas_used: 0,
            events: Vec::new(),
        }
    }
}

/// Event emitted by transaction
#[derive(Debug, Clone)]
pub struct Event {
    /// Event type
    pub event_type: String,
    /// Event attributes
    pub attributes: Vec<EventAttribute>,
}

/// Event attribute (key-value)
#[derive(Debug, Clone)]
pub struct EventAttribute {
    pub key: String,
    pub value: String,
}

/// Response to FinalizeBlock
#[derive(Debug, Clone)]
pub struct ResponseFinalizeBlock {
    /// Results for each transaction
    pub tx_results: Vec<ExecTxResult>,
    /// Validator updates
    pub validator_updates: Vec<ValidatorUpdate>,
    /// App hash after executing block
    pub app_hash: Hash,
    /// Events from begin/end block
    pub events: Vec<Event>,
}

/// Response to Commit
#[derive(Debug, Clone)]
pub struct ResponseCommit {
    /// Retain height (for state sync)
    pub retain_height: i64,
}

/// Response to Query
#[derive(Debug, Clone)]
pub struct ResponseQuery {
    /// Result code
    pub code: Code,
    /// Result data
    pub value: Vec<u8>,
    /// Log message
    pub log: String,
    /// Block height
    pub height: i64,
}

// =============================================================================
// Validator Updates
// =============================================================================

/// Validator update (for adding/removing/changing validators)
#[derive(Debug, Clone)]
pub struct ValidatorUpdate {
    /// Public key
    pub pub_key: Vec<u8>,
    /// Voting power (0 = remove)
    pub power: i64,
}

// =============================================================================
// ABCI Application Trait
// =============================================================================

/// ABCI Application interface
///
/// Implement this trait to create a Cosmos-compatible blockchain application.
pub trait Application {
    /// Initialize chain at genesis
    fn init_chain(&mut self, req: RequestInitChain) -> ResponseInitChain;

    /// Check if transaction is valid for mempool
    fn check_tx(&self, req: RequestCheckTx) -> ResponseCheckTx;

    /// Proposer prepares block proposal
    fn prepare_proposal(&self, req: RequestPrepareProposal) -> ResponsePrepareProposal;

    /// Process (validate) proposal
    fn process_proposal(&self, req: RequestProcessProposal) -> ResponseProcessProposal;

    /// Finalize and execute block
    fn finalize_block(&mut self, req: RequestFinalizeBlock) -> ResponseFinalizeBlock;

    /// Commit state
    fn commit(&mut self, req: RequestCommit) -> ResponseCommit;

    /// Query state
    fn query(&self, req: RequestQuery) -> ResponseQuery;
}

// =============================================================================
// Simple Application Example
// =============================================================================

/// Simple counter application (for demonstration)
///
/// This implements a basic ABCI app that maintains a counter.
#[derive(Debug, Default)]
pub struct CounterApp {
    /// Counter value
    counter: u64,
    /// Block height
    height: i64,
    /// App hash
    app_hash: Hash,
}

impl CounterApp {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Application for CounterApp {
    fn init_chain(&mut self, _req: RequestInitChain) -> ResponseInitChain {
        self.counter = 0;
        self.height = 0;
        ResponseInitChain::default()
    }

    fn check_tx(&self, req: RequestCheckTx) -> ResponseCheckTx {
        // Valid if tx is a single byte <= counter + 1
        if req.tx.len() == 1 && req.tx[0] as u64 <= self.counter + 1 {
            ResponseCheckTx::default()
        } else {
            ResponseCheckTx {
                code: Code::InvalidTx,
                log: "invalid counter value".to_string(),
                ..Default::default()
            }
        }
    }

    fn prepare_proposal(&self, req: RequestPrepareProposal) -> ResponsePrepareProposal {
        // Include all valid transactions
        let txs: Vec<Vec<u8>> = req.txs.into_iter()
            .filter(|tx| tx.len() == 1 && tx[0] as u64 <= self.counter + 1)
            .collect();

        ResponsePrepareProposal { txs }
    }

    fn process_proposal(&self, req: RequestProcessProposal) -> ResponseProcessProposal {
        // Validate all transactions
        for tx in &req.txs {
            if tx.len() != 1 || tx[0] as u64 > self.counter + 1 {
                return ResponseProcessProposal {
                    status: ProposalStatus::Reject,
                };
            }
        }

        ResponseProcessProposal {
            status: ProposalStatus::Accept,
        }
    }

    fn finalize_block(&mut self, req: RequestFinalizeBlock) -> ResponseFinalizeBlock {
        self.height = req.height;

        let mut tx_results = Vec::new();

        for tx in req.txs {
            if tx.len() == 1 {
                let value = tx[0] as u64;
                if value == self.counter + 1 {
                    self.counter = value;
                    tx_results.push(ExecTxResult::default());
                } else {
                    tx_results.push(ExecTxResult {
                        code: Code::InvalidTx,
                        log: "wrong counter value".to_string(),
                        ..Default::default()
                    });
                }
            }
        }

        // Compute app hash from counter
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&self.counter.to_le_bytes());
        hasher.update(&self.height.to_le_bytes());
        self.app_hash = hasher.finalize().into();

        ResponseFinalizeBlock {
            tx_results,
            validator_updates: Vec::new(),
            app_hash: self.app_hash,
            events: Vec::new(),
        }
    }

    fn commit(&mut self, _req: RequestCommit) -> ResponseCommit {
        ResponseCommit { retain_height: 0 }
    }

    fn query(&self, req: RequestQuery) -> ResponseQuery {
        if req.path == "counter" {
            ResponseQuery {
                code: Code::Ok,
                value: self.counter.to_le_bytes().to_vec(),
                log: String::new(),
                height: self.height,
            }
        } else {
            ResponseQuery {
                code: Code::UnknownError,
                value: Vec::new(),
                log: "unknown query path".to_string(),
                height: self.height,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_app() {
        let mut app = CounterApp::new();

        // Init chain
        app.init_chain(RequestInitChain {
            chain_id: "test".to_string(),
            genesis_time: 0,
            validators: vec![],
            app_state_bytes: vec![],
        });

        // Check valid tx
        let check = app.check_tx(RequestCheckTx {
            tx: vec![1],
            check_type: CheckTxType::New,
        });
        assert_eq!(check.code, Code::Ok);

        // Check invalid tx
        let check = app.check_tx(RequestCheckTx {
            tx: vec![5],
            check_type: CheckTxType::New,
        });
        assert_eq!(check.code, Code::InvalidTx);

        // Finalize block with counter=1
        let result = app.finalize_block(RequestFinalizeBlock {
            txs: vec![vec![1]],
            hash: [0u8; 32],
            height: 1,
            time: 0,
            proposer_address: [0u8; 20],
        });

        assert_eq!(result.tx_results.len(), 1);
        assert_eq!(result.tx_results[0].code, Code::Ok);

        // Query counter
        let query = app.query(RequestQuery {
            path: "counter".to_string(),
            data: vec![],
            height: 0,
            prove: false,
        });
        assert_eq!(query.value, 1u64.to_le_bytes().to_vec());
    }
}
