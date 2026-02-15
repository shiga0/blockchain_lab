//! Solana Runtime Module (Sealevel Parallel Execution)
//!
//! ## Runtime Comparison
//!
//! | Aspect | Ethereum (EVM) | Solana (Sealevel) |
//! |--------|---------------|-------------------|
//! | Execution | Sequential | Parallel |
//! | Account Access | Runtime determined | Declared upfront |
//! | State Conflicts | Handled by EVM | Prevented by scheduler |
//! | Throughput | ~15-30 TPS | ~65,000 TPS |
//!
//! ## Sealevel Runtime
//!
//! Sealevel enables parallel transaction execution by requiring transactions
//! to declare all accounts they will access upfront.
//!
//! ```text
//! Transaction Declaration:
//! ┌─────────────────────────────────────────────────────────┐
//! │ accounts: [                                             │
//! │   { pubkey: A, is_writable: true,  is_signer: true  }  │
//! │   { pubkey: B, is_writable: true,  is_signer: false }  │
//! │   { pubkey: C, is_writable: false, is_signer: false }  │
//! │ ]                                                       │
//! └─────────────────────────────────────────────────────────┘
//!
//! Conflict Detection:
//! TX1: write(A), read(B)   ─┐
//!                           ├─→ Conflict on A (both write)
//! TX2: write(A), read(C)   ─┘
//!
//! TX3: read(A), write(D)   ─┐
//!                           ├─→ No conflict (parallel OK)
//! TX4: read(B), write(E)   ─┘
//! ```
//!
//! ## Execution Flow
//!
//! ```text
//! Entry (from PoH)
//!     │
//!     ▼
//! ┌─────────────────────────────────────┐
//! │     Transaction Scheduler           │
//! │  (group non-conflicting TXs)        │
//! └───────────────┬─────────────────────┘
//!                 │
//!     ┌───────────┼───────────┐
//!     ▼           ▼           ▼
//! ┌───────┐  ┌───────┐  ┌───────┐
//! │ TX1   │  │ TX2   │  │ TX3   │   (Parallel execution)
//! │ Core1 │  │ Core2 │  │ Core3 │
//! └───────┘  └───────┘  └───────┘
//!     │           │           │
//!     └───────────┼───────────┘
//!                 ▼
//! ┌─────────────────────────────────────┐
//! │        Commit Results               │
//! └─────────────────────────────────────┘
//! ```

use crate::account::{AccountMeta, AccountStore, Pubkey};
use crate::constants::MAX_TX_ACCOUNT_LOCKS;
use std::collections::HashSet;

/// Transaction instruction
#[derive(Debug, Clone)]
pub struct Instruction {
    /// Program to invoke
    pub program_id: Pubkey,
    /// Accounts to pass to the program
    pub accounts: Vec<AccountMeta>,
    /// Instruction data
    pub data: Vec<u8>,
}

/// A Solana transaction
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Signatures (Ed25519)
    pub signatures: Vec<[u8; 64]>,
    /// Message containing instructions
    pub message: Message,
}

/// Transaction message
#[derive(Debug, Clone)]
pub struct Message {
    /// Message header
    pub header: MessageHeader,
    /// All account pubkeys used in this transaction
    pub account_keys: Vec<Pubkey>,
    /// Recent blockhash (for replay protection)
    pub recent_blockhash: [u8; 32],
    /// Instructions to execute
    pub instructions: Vec<CompiledInstruction>,
}

/// Message header
#[derive(Debug, Clone, Copy)]
pub struct MessageHeader {
    /// Number of required signatures
    pub num_required_signatures: u8,
    /// Number of readonly signed accounts
    pub num_readonly_signed_accounts: u8,
    /// Number of readonly unsigned accounts
    pub num_readonly_unsigned_accounts: u8,
}

/// Compiled instruction (indexes into account_keys)
#[derive(Debug, Clone)]
pub struct CompiledInstruction {
    /// Index into account_keys for the program
    pub program_id_index: u8,
    /// Indexes into account_keys for accounts
    pub accounts: Vec<u8>,
    /// Instruction data
    pub data: Vec<u8>,
}

/// Account lock types for conflict detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    Read,
    Write,
}

/// Account locks for a transaction
#[derive(Debug, Clone)]
pub struct AccountLocks {
    /// Read locks
    pub read_locks: HashSet<Pubkey>,
    /// Write locks
    pub write_locks: HashSet<Pubkey>,
}

impl AccountLocks {
    /// Create from a transaction's message
    pub fn from_message(message: &Message) -> Result<Self, &'static str> {
        let mut read_locks = HashSet::new();
        let mut write_locks = HashSet::new();

        let num_signed = message.header.num_required_signatures as usize;
        let num_readonly_signed = message.header.num_readonly_signed_accounts as usize;
        let num_readonly_unsigned = message.header.num_readonly_unsigned_accounts as usize;

        for (i, pubkey) in message.account_keys.iter().enumerate() {
            let is_writable = if i < num_signed {
                // Signed accounts
                i >= num_readonly_signed
            } else {
                // Unsigned accounts
                i < message.account_keys.len() - num_readonly_unsigned
            };

            if is_writable {
                write_locks.insert(*pubkey);
            } else {
                read_locks.insert(*pubkey);
            }
        }

        if read_locks.len() + write_locks.len() > MAX_TX_ACCOUNT_LOCKS {
            return Err("too many account locks");
        }

        Ok(Self {
            read_locks,
            write_locks,
        })
    }

    /// Check if this transaction conflicts with another
    ///
    /// Conflicts occur when:
    /// - Both transactions write to the same account
    /// - One writes and the other reads the same account
    pub fn conflicts_with(&self, other: &AccountLocks) -> bool {
        // Write-write conflict
        if !self.write_locks.is_disjoint(&other.write_locks) {
            return true;
        }

        // Write-read conflict (either direction)
        if !self.write_locks.is_disjoint(&other.read_locks) {
            return true;
        }
        if !self.read_locks.is_disjoint(&other.write_locks) {
            return true;
        }

        false
    }
}

/// Transaction batch for parallel execution
#[derive(Debug)]
pub struct TransactionBatch {
    /// Transactions that can execute in parallel
    pub transactions: Vec<Transaction>,
    /// Combined locks for this batch
    pub locks: AccountLocks,
}

/// Sealevel scheduler - groups transactions for parallel execution
#[derive(Debug, Default)]
pub struct Scheduler {
    /// Pending transactions
    pending: Vec<(Transaction, AccountLocks)>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a transaction to the scheduler
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), &'static str> {
        let locks = AccountLocks::from_message(&tx.message)?;
        self.pending.push((tx, locks));
        Ok(())
    }

    /// Schedule transactions into parallel batches
    ///
    /// This is a greedy algorithm that groups non-conflicting transactions.
    pub fn schedule(&mut self) -> Vec<TransactionBatch> {
        let mut batches = Vec::new();
        let mut pending = std::mem::take(&mut self.pending);

        while !pending.is_empty() {
            let mut batch_txs = Vec::new();
            let mut batch_locks = AccountLocks {
                read_locks: HashSet::new(),
                write_locks: HashSet::new(),
            };
            let mut remaining = Vec::new();

            for (tx, locks) in pending {
                if !batch_locks.conflicts_with(&locks) {
                    // Add to batch
                    batch_locks.read_locks.extend(&locks.read_locks);
                    batch_locks.write_locks.extend(&locks.write_locks);
                    batch_txs.push(tx);
                } else {
                    // Keep for next batch
                    remaining.push((tx, locks));
                }
            }

            if !batch_txs.is_empty() {
                batches.push(TransactionBatch {
                    transactions: batch_txs,
                    locks: batch_locks,
                });
            }

            pending = remaining;
        }

        batches
    }
}

/// Transaction execution result
#[derive(Debug)]
pub enum ExecutionResult {
    Success {
        /// Logs from program execution
        logs: Vec<String>,
        /// Compute units consumed
        compute_units: u64,
    },
    Failure {
        /// Error message
        error: String,
    },
}

/// Simple runtime executor
pub struct Runtime {
    /// Account storage
    pub accounts: AccountStore,
    /// Compute budget per transaction
    pub compute_budget: u64,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            accounts: AccountStore::new(),
            compute_budget: 200_000, // Default compute budget
        }
    }

    /// Execute a batch of transactions in parallel
    ///
    /// Note: This is a simplified version. Real Solana uses:
    /// - BPF interpreter for program execution
    /// - Rayon for parallel execution
    /// - Complex error handling and rollback
    pub fn execute_batch(&mut self, batch: &TransactionBatch) -> Vec<ExecutionResult> {
        // In production, this would use rayon::par_iter()
        batch
            .transactions
            .iter()
            .map(|tx| self.execute_transaction(tx))
            .collect()
    }

    /// Execute a single transaction
    fn execute_transaction(&mut self, tx: &Transaction) -> ExecutionResult {
        let mut logs = Vec::new();
        let mut compute_units = 0u64;

        // Process each instruction
        for instruction in &tx.message.instructions {
            logs.push(format!(
                "Program {} invoke",
                bs58::encode(&tx.message.account_keys[instruction.program_id_index as usize])
                    .into_string()
            ));

            // Simplified: just consume some compute units
            compute_units += 1000;

            if compute_units > self.compute_budget {
                return ExecutionResult::Failure {
                    error: "exceeded compute budget".to_string(),
                };
            }

            logs.push(format!("Program {} success",
                bs58::encode(&tx.message.account_keys[instruction.program_id_index as usize])
                    .into_string()
            ));
        }

        ExecutionResult::Success {
            logs,
            compute_units,
        }
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_tx(write_accounts: Vec<Pubkey>, read_accounts: Vec<Pubkey>) -> Transaction {
        let mut account_keys = write_accounts.clone();
        account_keys.extend(read_accounts.clone());

        Transaction {
            signatures: vec![],
            message: Message {
                header: MessageHeader {
                    num_required_signatures: 1,
                    num_readonly_signed_accounts: 0,
                    num_readonly_unsigned_accounts: read_accounts.len() as u8,
                },
                account_keys,
                recent_blockhash: [0u8; 32],
                instructions: vec![],
            },
        }
    }

    #[test]
    fn test_no_conflict() {
        let tx1 = make_test_tx(vec![[1u8; 32]], vec![[2u8; 32]]);
        let tx2 = make_test_tx(vec![[3u8; 32]], vec![[4u8; 32]]);

        let locks1 = AccountLocks::from_message(&tx1.message).unwrap();
        let locks2 = AccountLocks::from_message(&tx2.message).unwrap();

        assert!(!locks1.conflicts_with(&locks2));
    }

    #[test]
    fn test_write_write_conflict() {
        let shared: Pubkey = [1u8; 32];
        let tx1 = make_test_tx(vec![shared], vec![]);
        let tx2 = make_test_tx(vec![shared], vec![]);

        let locks1 = AccountLocks::from_message(&tx1.message).unwrap();
        let locks2 = AccountLocks::from_message(&tx2.message).unwrap();

        assert!(locks1.conflicts_with(&locks2));
    }

    #[test]
    fn test_write_read_conflict() {
        let shared: Pubkey = [1u8; 32];
        let tx1 = make_test_tx(vec![shared], vec![]);
        let tx2 = make_test_tx(vec![], vec![shared]);

        let locks1 = AccountLocks::from_message(&tx1.message).unwrap();
        let locks2 = AccountLocks::from_message(&tx2.message).unwrap();

        assert!(locks1.conflicts_with(&locks2));
    }

    #[test]
    fn test_scheduler_batching() {
        let mut scheduler = Scheduler::new();

        // TX1 and TX2 don't conflict
        scheduler
            .add_transaction(make_test_tx(vec![[1u8; 32]], vec![]))
            .unwrap();
        scheduler
            .add_transaction(make_test_tx(vec![[2u8; 32]], vec![]))
            .unwrap();

        // TX3 conflicts with TX1
        scheduler
            .add_transaction(make_test_tx(vec![[1u8; 32]], vec![]))
            .unwrap();

        let batches = scheduler.schedule();

        // Should create 2 batches
        assert_eq!(batches.len(), 2);
        // First batch has 2 transactions
        assert_eq!(batches[0].transactions.len(), 2);
        // Second batch has 1 transaction
        assert_eq!(batches[1].transactions.len(), 1);
    }
}
