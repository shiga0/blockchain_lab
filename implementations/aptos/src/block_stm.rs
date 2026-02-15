//! # Block-STM: Parallel Transaction Execution
//!
//! Block-STM is an optimistic parallel execution engine that:
//! - Executes transactions speculatively in parallel
//! - Uses multi-version data structure for conflict detection
//! - Re-executes conflicting transactions (incarnation++)
//!
//! ## Key Components
//!
//! - **MVHashMap**: Multi-version data structure storing writes per transaction
//! - **Scheduler**: Coordinates execution and validation tasks
//! - **ReadSet/WriteSet**: Tracks transaction dependencies
//! - **Incarnation**: Re-execution count for a transaction

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};

/// Transaction index in the block
pub type TxnIndex = u32;

/// Incarnation number (re-execution count)
pub type IncarnationNumber = u32;

/// Version = (transaction index, incarnation)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version {
    pub txn_idx: TxnIndex,
    pub incarnation: IncarnationNumber,
}

impl Version {
    pub fn new(txn_idx: TxnIndex, incarnation: IncarnationNumber) -> Self {
        Self { txn_idx, incarnation }
    }
}

/// Storage key for multi-version data
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StorageKey {
    /// Account address
    pub address: [u8; 32],
    /// Resource type or path
    pub path: Vec<u8>,
}

impl StorageKey {
    pub fn new(address: [u8; 32], path: Vec<u8>) -> Self {
        Self { address, path }
    }
}

/// Value stored in MVHashMap
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MVValue {
    /// Actual value written
    Value(Vec<u8>),
    /// Deleted
    Deleted,
    /// Estimate marker (transaction is being re-executed)
    Estimate,
}

/// Entry in the multi-version map
#[derive(Clone, Debug)]
pub struct MVEntry {
    pub version: Version,
    pub value: MVValue,
}

/// Multi-Version HashMap for parallel execution
///
/// For each key, stores values written by different transactions.
/// Reads return the value from the highest transaction < reader's index.
#[derive(Debug, Default)]
pub struct MVHashMap {
    /// Map from storage key to version -> value
    data: HashMap<StorageKey, BTreeMap<TxnIndex, MVEntry>>,
}

impl MVHashMap {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Write a value for a key at a specific version
    pub fn write(&mut self, key: StorageKey, version: Version, value: MVValue) {
        let entries = self.data.entry(key).or_default();
        entries.insert(
            version.txn_idx,
            MVEntry {
                version,
                value,
            },
        );
    }

    /// Read the latest value written by a transaction < txn_idx
    /// Returns None if no prior write exists (should read from storage)
    pub fn read(&self, key: &StorageKey, txn_idx: TxnIndex) -> Option<ReadResult> {
        let entries = self.data.get(key)?;

        // Find the largest txn_idx < reader's txn_idx
        entries
            .range(..txn_idx)
            .next_back()
            .map(|(_, entry)| {
                if matches!(entry.value, MVValue::Estimate) {
                    ReadResult::Dependency(entry.version.txn_idx)
                } else {
                    ReadResult::Value(entry.clone())
                }
            })
    }

    /// Mark all writes from a version as estimates (on abort)
    pub fn mark_estimates(&mut self, txn_idx: TxnIndex, incarnation: IncarnationNumber) {
        for entries in self.data.values_mut() {
            if let Some(entry) = entries.get_mut(&txn_idx) {
                if entry.version.incarnation == incarnation {
                    entry.value = MVValue::Estimate;
                }
            }
        }
    }

    /// Remove estimates that weren't overwritten (after re-execution)
    pub fn remove_stale_estimates(&mut self, txn_idx: TxnIndex) {
        for entries in self.data.values_mut() {
            if let Some(entry) = entries.get(&txn_idx) {
                if matches!(entry.value, MVValue::Estimate) {
                    entries.remove(&txn_idx);
                }
            }
        }
    }

    /// Get all keys written by a transaction
    pub fn get_write_keys(&self, txn_idx: TxnIndex) -> Vec<StorageKey> {
        self.data
            .iter()
            .filter(|(_, entries)| entries.contains_key(&txn_idx))
            .map(|(key, _)| key.clone())
            .collect()
    }
}

/// Result of reading from MVHashMap
#[derive(Clone, Debug)]
pub enum ReadResult {
    /// Found a value
    Value(MVEntry),
    /// Encountered an estimate, need to wait for dependency
    Dependency(TxnIndex),
}

/// Read set entry tracking what a transaction read
#[derive(Clone, Debug)]
pub struct ReadSetEntry {
    pub key: StorageKey,
    pub version: Option<Version>, // None if read from storage
}

/// Write set entry tracking what a transaction wrote
#[derive(Clone, Debug)]
pub struct WriteSetEntry {
    pub key: StorageKey,
    pub value: MVValue,
}

/// Transaction's read and write sets
#[derive(Clone, Debug, Default)]
pub struct TxnInputOutput {
    pub read_set: Vec<ReadSetEntry>,
    pub write_set: Vec<WriteSetEntry>,
}

impl TxnInputOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_read(&mut self, key: StorageKey, version: Option<Version>) {
        self.read_set.push(ReadSetEntry { key, version });
    }

    pub fn add_write(&mut self, key: StorageKey, value: MVValue) {
        self.write_set.push(WriteSetEntry { key, value });
    }

    pub fn write_keys(&self) -> HashSet<StorageKey> {
        self.write_set.iter().map(|w| w.key.clone()).collect()
    }
}

/// Execution status for a transaction
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Ready to execute
    Ready(IncarnationNumber),
    /// Currently executing
    Executing(IncarnationNumber),
    /// Waiting on a dependency
    Suspended(IncarnationNumber, TxnIndex),
    /// Finished execution, pending validation
    Executed(IncarnationNumber),
    /// Validation passed, can be committed
    Validated(IncarnationNumber),
    /// Being aborted
    Aborting(IncarnationNumber),
}

/// Scheduler task
#[derive(Clone, Debug)]
pub enum SchedulerTask {
    /// Execute transaction at index with incarnation
    Execute(TxnIndex, IncarnationNumber),
    /// Validate transaction at index with incarnation
    Validate(TxnIndex, IncarnationNumber),
    /// No task available, retry later
    Retry,
    /// All done
    Done,
}

/// Collaborative scheduler for Block-STM
#[derive(Debug)]
pub struct Scheduler {
    /// Number of transactions
    num_txns: TxnIndex,
    /// Status of each transaction
    status: Vec<ExecutionStatus>,
    /// Execution index (next transaction to try executing)
    execution_idx: AtomicU32,
    /// Validation index (next transaction to try validating)
    validation_idx: AtomicU32,
    /// Dependencies: txn_idx -> list of transactions waiting on it
    dependencies: HashMap<TxnIndex, Vec<TxnIndex>>,
    /// Transaction input/output sets
    txn_io: Vec<TxnInputOutput>,
}

impl Scheduler {
    pub fn new(num_txns: TxnIndex) -> Self {
        Self {
            num_txns,
            status: (0..num_txns).map(|_| ExecutionStatus::Ready(0)).collect(),
            execution_idx: AtomicU32::new(0),
            validation_idx: AtomicU32::new(0),
            dependencies: HashMap::new(),
            txn_io: (0..num_txns).map(|_| TxnInputOutput::new()).collect(),
        }
    }

    /// Get the next task to perform
    pub fn next_task(&mut self) -> SchedulerTask {
        let exec_idx = self.execution_idx.load(Ordering::Acquire);
        let val_idx = self.validation_idx.load(Ordering::Acquire);

        // Prioritize lower indices
        if exec_idx < self.num_txns && exec_idx <= val_idx {
            if let Some(task) = self.try_execute(exec_idx) {
                return task;
            }
        }

        if val_idx < self.num_txns {
            if let Some(task) = self.try_validate(val_idx) {
                return task;
            }
        }

        // Check if all done
        if self.all_validated() {
            return SchedulerTask::Done;
        }

        SchedulerTask::Retry
    }

    fn try_execute(&mut self, txn_idx: TxnIndex) -> Option<SchedulerTask> {
        if txn_idx >= self.num_txns {
            return None;
        }

        let status = &self.status[txn_idx as usize];
        if let ExecutionStatus::Ready(incarnation) = status {
            let incarnation = *incarnation;
            self.status[txn_idx as usize] = ExecutionStatus::Executing(incarnation);
            self.execution_idx.fetch_add(1, Ordering::Release);
            return Some(SchedulerTask::Execute(txn_idx, incarnation));
        }

        None
    }

    fn try_validate(&mut self, txn_idx: TxnIndex) -> Option<SchedulerTask> {
        if txn_idx >= self.num_txns {
            return None;
        }

        let status = &self.status[txn_idx as usize];
        if let ExecutionStatus::Executed(incarnation) = status {
            let incarnation = *incarnation;
            self.validation_idx.fetch_add(1, Ordering::Release);
            return Some(SchedulerTask::Validate(txn_idx, incarnation));
        }

        None
    }

    /// Mark execution as finished
    pub fn finish_execution(&mut self, txn_idx: TxnIndex, incarnation: IncarnationNumber, io: TxnInputOutput) {
        if txn_idx < self.num_txns {
            self.status[txn_idx as usize] = ExecutionStatus::Executed(incarnation);
            self.txn_io[txn_idx as usize] = io;

            // Wake up dependencies
            if let Some(deps) = self.dependencies.remove(&txn_idx) {
                for dep_idx in deps {
                    if let ExecutionStatus::Suspended(inc, _) = self.status[dep_idx as usize] {
                        self.status[dep_idx as usize] = ExecutionStatus::Ready(inc);
                        // Reset execution index to re-execute
                        let _ = self.execution_idx.fetch_min(dep_idx, Ordering::Release);
                    }
                }
            }
        }
    }

    /// Mark transaction as suspended waiting on dependency
    pub fn suspend(&mut self, txn_idx: TxnIndex, incarnation: IncarnationNumber, dep_idx: TxnIndex) {
        if txn_idx < self.num_txns {
            self.status[txn_idx as usize] = ExecutionStatus::Suspended(incarnation, dep_idx);
            self.dependencies.entry(dep_idx).or_default().push(txn_idx);
        }
    }

    /// Validation succeeded
    pub fn finish_validation(&mut self, txn_idx: TxnIndex, incarnation: IncarnationNumber) {
        if txn_idx < self.num_txns {
            if let ExecutionStatus::Executed(inc) = self.status[txn_idx as usize] {
                if inc == incarnation {
                    self.status[txn_idx as usize] = ExecutionStatus::Validated(incarnation);
                }
            }
        }
    }

    /// Abort transaction and schedule re-execution
    pub fn abort(&mut self, txn_idx: TxnIndex, incarnation: IncarnationNumber) {
        if txn_idx < self.num_txns {
            self.status[txn_idx as usize] = ExecutionStatus::Ready(incarnation + 1);
            // Reset execution index
            let _ = self.execution_idx.fetch_min(txn_idx, Ordering::Release);
            // Reset validation index for higher transactions
            let _ = self.validation_idx.fetch_min(txn_idx + 1, Ordering::Release);
        }
    }

    fn all_validated(&self) -> bool {
        self.status.iter().all(|s| matches!(s, ExecutionStatus::Validated(_)))
    }

    pub fn get_status(&self, txn_idx: TxnIndex) -> Option<&ExecutionStatus> {
        self.status.get(txn_idx as usize)
    }

    pub fn get_io(&self, txn_idx: TxnIndex) -> Option<&TxnInputOutput> {
        self.txn_io.get(txn_idx as usize)
    }
}

/// Block-STM executor
#[derive(Debug)]
pub struct BlockSTMExecutor {
    /// Multi-version data
    mv_data: MVHashMap,
    /// Scheduler
    scheduler: Scheduler,
}

impl BlockSTMExecutor {
    pub fn new(num_txns: TxnIndex) -> Self {
        Self {
            mv_data: MVHashMap::new(),
            scheduler: Scheduler::new(num_txns),
        }
    }

    /// Read a value, returning dependency if estimate encountered
    pub fn read(&self, key: &StorageKey, txn_idx: TxnIndex) -> Option<ReadResult> {
        self.mv_data.read(key, txn_idx)
    }

    /// Write a value
    pub fn write(&mut self, key: StorageKey, version: Version, value: MVValue) {
        self.mv_data.write(key, version, value);
    }

    /// Get next task
    pub fn next_task(&mut self) -> SchedulerTask {
        self.scheduler.next_task()
    }

    /// Finish execution
    pub fn finish_execution(&mut self, txn_idx: TxnIndex, incarnation: IncarnationNumber, io: TxnInputOutput) {
        // Apply writes to mv_data
        for write in &io.write_set {
            self.mv_data.write(
                write.key.clone(),
                Version::new(txn_idx, incarnation),
                write.value.clone(),
            );
        }
        self.scheduler.finish_execution(txn_idx, incarnation, io);
    }

    /// Suspend waiting on dependency
    pub fn suspend(&mut self, txn_idx: TxnIndex, incarnation: IncarnationNumber, dep_idx: TxnIndex) {
        self.scheduler.suspend(txn_idx, incarnation, dep_idx);
    }

    /// Validate transaction by checking read set
    pub fn validate(&mut self, txn_idx: TxnIndex, incarnation: IncarnationNumber) -> bool {
        let io = match self.scheduler.get_io(txn_idx) {
            Some(io) => io.clone(),
            None => return false,
        };

        // Check each read in read set
        for read in &io.read_set {
            let current = self.mv_data.read(&read.key, txn_idx);
            let matches = match (&current, &read.version) {
                (None, None) => true,
                (Some(ReadResult::Value(entry)), Some(ver)) => entry.version == *ver,
                _ => false,
            };

            if !matches {
                // Validation failed - abort
                self.mv_data.mark_estimates(txn_idx, incarnation);
                self.scheduler.abort(txn_idx, incarnation);
                return false;
            }
        }

        // Validation succeeded
        self.scheduler.finish_validation(txn_idx, incarnation);
        true
    }
}

/// Gas metering for transactions
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GasMeter {
    /// Gas used
    pub gas_used: u64,
    /// Max gas
    pub max_gas: u64,
}

impl GasMeter {
    pub fn new(max_gas: u64) -> Self {
        Self { gas_used: 0, max_gas }
    }

    pub fn charge(&mut self, amount: u64) -> Result<(), &'static str> {
        if self.gas_used + amount > self.max_gas {
            return Err("Out of gas");
        }
        self.gas_used += amount;
        Ok(())
    }

    pub fn remaining(&self) -> u64 {
        self.max_gas.saturating_sub(self.gas_used)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key(addr: u8, path: u8) -> StorageKey {
        StorageKey::new([addr; 32], vec![path])
    }

    #[test]
    fn test_version() {
        let v1 = Version::new(1, 0);
        let v2 = Version::new(1, 1);
        assert_ne!(v1, v2);
        assert!(v1 < v2);
    }

    #[test]
    fn test_mv_hashmap_write_read() {
        let mut mv = MVHashMap::new();
        let key = test_key(1, 1);

        // Write from txn 0
        mv.write(key.clone(), Version::new(0, 0), MVValue::Value(vec![1, 2, 3]));

        // Read from txn 1 should see txn 0's write
        let result = mv.read(&key, 1);
        assert!(matches!(result, Some(ReadResult::Value(_))));

        // Read from txn 0 should see nothing
        let result = mv.read(&key, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_mv_hashmap_estimate() {
        let mut mv = MVHashMap::new();
        let key = test_key(1, 1);

        // Write and then mark as estimate
        mv.write(key.clone(), Version::new(0, 0), MVValue::Value(vec![1]));
        mv.mark_estimates(0, 0);

        // Read should return dependency
        let result = mv.read(&key, 1);
        assert!(matches!(result, Some(ReadResult::Dependency(0))));
    }

    #[test]
    fn test_scheduler_basic() {
        let mut scheduler = Scheduler::new(3);

        // First task should be execute txn 0
        let task = scheduler.next_task();
        assert!(matches!(task, SchedulerTask::Execute(0, 0)));

        // Finish execution
        scheduler.finish_execution(0, 0, TxnInputOutput::new());

        // Next should be either execute 1 or validate 0
        let task = scheduler.next_task();
        // The exact order depends on implementation
        assert!(matches!(
            task,
            SchedulerTask::Execute(1, 0) | SchedulerTask::Validate(0, 0)
        ));
    }

    #[test]
    fn test_scheduler_abort() {
        let mut scheduler = Scheduler::new(2);

        // Execute txn 0
        scheduler.next_task(); // Execute(0, 0)
        scheduler.finish_execution(0, 0, TxnInputOutput::new());

        // Execute txn 1
        scheduler.next_task(); // Execute(1, 0)
        scheduler.finish_execution(1, 0, TxnInputOutput::new());

        // Abort txn 0 - should increment incarnation
        scheduler.abort(0, 0);
        let status = scheduler.get_status(0);
        assert!(matches!(status, Some(ExecutionStatus::Ready(1))));
    }

    #[test]
    fn test_txn_input_output() {
        let mut io = TxnInputOutput::new();
        let key = test_key(1, 1);

        io.add_read(key.clone(), Some(Version::new(0, 0)));
        io.add_write(key.clone(), MVValue::Value(vec![1, 2, 3]));

        assert_eq!(io.read_set.len(), 1);
        assert_eq!(io.write_set.len(), 1);
        assert!(io.write_keys().contains(&key));
    }

    #[test]
    fn test_block_stm_executor() {
        let mut executor = BlockSTMExecutor::new(2);
        let key = test_key(1, 1);

        // Execute txn 0
        let task = executor.next_task();
        assert!(matches!(task, SchedulerTask::Execute(0, 0)));

        // Txn 0 writes
        let mut io0 = TxnInputOutput::new();
        io0.add_write(key.clone(), MVValue::Value(vec![1]));
        executor.finish_execution(0, 0, io0);

        // Next task could be Execute(1, 0) or Validate(0, 0)
        let mut executed_1 = false;
        for _ in 0..3 {
            let task = executor.next_task();
            match task {
                SchedulerTask::Execute(1, 0) => {
                    executed_1 = true;
                    // Txn 1 reads txn 0's write
                    let result = executor.read(&key, 1);
                    assert!(matches!(result, Some(ReadResult::Value(_))));

                    let mut io1 = TxnInputOutput::new();
                    io1.add_read(key.clone(), Some(Version::new(0, 0)));
                    executor.finish_execution(1, 0, io1);
                    break;
                }
                SchedulerTask::Validate(0, 0) => {
                    executor.validate(0, 0);
                }
                _ => {}
            }
        }
        assert!(executed_1, "Should have executed txn 1");
    }

    #[test]
    fn test_gas_meter() {
        let mut meter = GasMeter::new(100);
        assert_eq!(meter.remaining(), 100);

        meter.charge(30).unwrap();
        assert_eq!(meter.gas_used, 30);
        assert_eq!(meter.remaining(), 70);

        assert!(meter.charge(80).is_err()); // Would exceed
        assert_eq!(meter.gas_used, 30); // Unchanged
    }

    #[test]
    fn test_validation_success() {
        let mut executor = BlockSTMExecutor::new(2);
        let key = test_key(1, 1);

        // Execute txn 0
        executor.next_task();
        let mut io0 = TxnInputOutput::new();
        io0.add_write(key.clone(), MVValue::Value(vec![1]));
        executor.finish_execution(0, 0, io0);

        // Validate txn 0
        let valid = executor.validate(0, 0);
        assert!(valid);
    }
}
