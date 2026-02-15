//! Programmable Transaction Blocks (PTB)
//!
//! ## PTB vs Traditional Transactions
//!
//! | Aspect | Ethereum TX | Solana IX | Sui PTB |
//! |--------|-------------|-----------|---------|
//! | Composability | Single call | Multiple IXs | Commands with results |
//! | Result passing | No | Accounts | Explicit Result refs |
//! | Atomicity | Per TX | Per TX | Per PTB |
//! | Flexibility | Low | Medium | High |
//!
//! ## PTB Structure
//!
//! ```text
//! ProgrammableTransaction {
//!   inputs: [              // Input arguments
//!     Pure(bytes),         // Primitive value (u64, address, etc.)
//!     Object(ObjectArg),   // Object reference
//!   ],
//!   commands: [            // Sequential commands
//!     MoveCall(...),       // Call Move function
//!     SplitCoins(...),     // Split coins
//!     MergeCoins(...),     // Merge coins
//!     TransferObjects(...),// Transfer objects
//!     Publish(...),        // Publish package
//!     Upgrade(...),        // Upgrade package
//!   ],
//! }
//! ```
//!
//! ## Result Passing
//!
//! ```text
//! Command outputs can be used as inputs to subsequent commands:
//!
//!   inputs: [Pure(100), Object(coin_ref)]
//!   commands:
//!     [0] SplitCoins(Input(1), [Input(0)])  // Split coin, output new coin
//!     [1] TransferObjects([Result(0)], recipient)  // Use split result
//!
//! Result(0) refers to output of command 0 (the split coin)
//! ```

use crate::object::{ObjectDigest, ObjectId, ObjectRef, SequenceNumber, SuiAddress};
use sha2::{Digest, Sha256};

/// Transaction digest
pub type TransactionDigest = [u8; 32];

/// Gas price in MIST (1 SUI = 10^9 MIST)
pub type GasPrice = u64;

// =============================================================================
// Transaction Input Arguments
// =============================================================================

/// Argument to a command
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    /// The gas coin (special)
    GasCoin,
    /// Reference to an input object/value
    Input(u16),
    /// Result from a previous command
    Result(u16),
    /// Nested result (for commands returning multiple values)
    NestedResult(u16, u16),
}

/// Call argument (input to transaction)
#[derive(Debug, Clone)]
pub enum CallArg {
    /// Pure value (BCS-encoded primitive)
    Pure(Vec<u8>),
    /// Object argument
    Object(ObjectArg),
}

/// Object argument types
#[derive(Debug, Clone)]
pub enum ObjectArg {
    /// Immutable or owned object (fastpath)
    ImmOrOwnedObject(ObjectRef),
    /// Shared object (requires consensus)
    SharedObject {
        id: ObjectId,
        initial_shared_version: SequenceNumber,
        mutable: bool,
    },
    /// Receiving object (transferred to this transaction)
    Receiving(ObjectRef),
}

impl ObjectArg {
    /// Check if this object requires consensus
    pub fn requires_consensus(&self) -> bool {
        matches!(self, ObjectArg::SharedObject { .. })
    }

    /// Get object ID
    pub fn object_id(&self) -> ObjectId {
        match self {
            ObjectArg::ImmOrOwnedObject(r) => r.object_id,
            ObjectArg::SharedObject { id, .. } => *id,
            ObjectArg::Receiving(r) => r.object_id,
        }
    }
}

// =============================================================================
// PTB Commands
// =============================================================================

/// Move function target
#[derive(Debug, Clone)]
pub struct MoveCallTarget {
    /// Package ID
    pub package: ObjectId,
    /// Module name
    pub module: String,
    /// Function name
    pub function: String,
}

impl MoveCallTarget {
    pub fn new(package: ObjectId, module: &str, function: &str) -> Self {
        Self {
            package,
            module: module.to_string(),
            function: function.to_string(),
        }
    }
}

/// PTB Command
#[derive(Debug, Clone)]
pub enum Command {
    /// Call a Move function
    MoveCall {
        target: MoveCallTarget,
        type_arguments: Vec<String>,
        arguments: Vec<Argument>,
    },

    /// Transfer objects to an address
    TransferObjects {
        objects: Vec<Argument>,
        recipient: Argument,
    },

    /// Split a coin into multiple coins
    SplitCoins {
        coin: Argument,
        amounts: Vec<Argument>,
    },

    /// Merge multiple coins into one
    MergeCoins {
        target: Argument,
        sources: Vec<Argument>,
    },

    /// Publish a new Move package
    Publish {
        modules: Vec<Vec<u8>>,
        dependencies: Vec<ObjectId>,
    },

    /// Upgrade an existing package
    Upgrade {
        modules: Vec<Vec<u8>>,
        dependencies: Vec<ObjectId>,
        package: ObjectId,
        ticket: Argument,
    },

    /// Create a Move vector
    MakeMoveVec {
        type_: Option<String>,
        elements: Vec<Argument>,
    },
}

impl Command {
    /// Get all arguments used by this command
    pub fn arguments(&self) -> Vec<&Argument> {
        match self {
            Command::MoveCall { arguments, .. } => arguments.iter().collect(),
            Command::TransferObjects { objects, recipient } => {
                let mut args: Vec<&Argument> = objects.iter().collect();
                args.push(recipient);
                args
            }
            Command::SplitCoins { coin, amounts } => {
                let mut args = vec![coin];
                args.extend(amounts.iter());
                args
            }
            Command::MergeCoins { target, sources } => {
                let mut args = vec![target];
                args.extend(sources.iter());
                args
            }
            Command::Publish { .. } => vec![],
            Command::Upgrade { ticket, .. } => vec![ticket],
            Command::MakeMoveVec { elements, .. } => elements.iter().collect(),
        }
    }
}

// =============================================================================
// Programmable Transaction
// =============================================================================

/// Programmable Transaction Block
#[derive(Debug, Clone)]
pub struct ProgrammableTransaction {
    /// Input objects and values
    pub inputs: Vec<CallArg>,
    /// Commands to execute
    pub commands: Vec<Command>,
}

impl ProgrammableTransaction {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            commands: Vec::new(),
        }
    }

    /// Add a pure value input
    pub fn add_pure_input(&mut self, value: Vec<u8>) -> Argument {
        let idx = self.inputs.len() as u16;
        self.inputs.push(CallArg::Pure(value));
        Argument::Input(idx)
    }

    /// Add an object input
    pub fn add_object_input(&mut self, obj: ObjectArg) -> Argument {
        let idx = self.inputs.len() as u16;
        self.inputs.push(CallArg::Object(obj));
        Argument::Input(idx)
    }

    /// Add a command and return its result reference
    pub fn add_command(&mut self, command: Command) -> Argument {
        let idx = self.commands.len() as u16;
        self.commands.push(command);
        Argument::Result(idx)
    }

    /// Check if this PTB requires consensus (has shared objects)
    pub fn requires_consensus(&self) -> bool {
        self.inputs.iter().any(|input| {
            matches!(
                input,
                CallArg::Object(ObjectArg::SharedObject { .. })
            )
        })
    }

    /// Validate the PTB structure
    pub fn validate(&self) -> Result<(), PtbError> {
        // Check command count
        if self.commands.len() > crate::constants::MAX_PTB_COMMANDS {
            return Err(PtbError::TooManyCommands);
        }

        // Validate argument references
        for (cmd_idx, command) in self.commands.iter().enumerate() {
            for arg in command.arguments() {
                self.validate_argument(arg, cmd_idx)?;
            }
        }

        Ok(())
    }

    /// Validate a single argument reference
    fn validate_argument(&self, arg: &Argument, current_cmd: usize) -> Result<(), PtbError> {
        match arg {
            Argument::GasCoin => Ok(()),
            Argument::Input(idx) => {
                if (*idx as usize) >= self.inputs.len() {
                    Err(PtbError::InvalidInputIndex(*idx))
                } else {
                    Ok(())
                }
            }
            Argument::Result(idx) => {
                if (*idx as usize) >= current_cmd {
                    Err(PtbError::InvalidResultIndex(*idx))
                } else {
                    Ok(())
                }
            }
            Argument::NestedResult(cmd_idx, _) => {
                if (*cmd_idx as usize) >= current_cmd {
                    Err(PtbError::InvalidResultIndex(*cmd_idx))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl Default for ProgrammableTransaction {
    fn default() -> Self {
        Self::new()
    }
}

/// PTB validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtbError {
    TooManyCommands,
    InvalidInputIndex(u16),
    InvalidResultIndex(u16),
    TypeMismatch,
    MissingInput,
}

// =============================================================================
// Transaction Data
// =============================================================================

/// Gas data for transaction
#[derive(Debug, Clone)]
pub struct GasData {
    /// Gas payment objects
    pub payment: Vec<ObjectRef>,
    /// Gas owner
    pub owner: SuiAddress,
    /// Gas price
    pub price: GasPrice,
    /// Gas budget
    pub budget: u64,
}

/// Transaction kind
#[derive(Debug, Clone)]
pub enum TransactionKind {
    /// Programmable transaction
    ProgrammableTransaction(ProgrammableTransaction),
    /// System transaction (consensus prologue, etc.)
    System(SystemTransaction),
}

/// System transaction types
#[derive(Debug, Clone)]
pub enum SystemTransaction {
    /// Consensus commit prologue
    ConsensusCommitPrologue {
        epoch: u64,
        round: u64,
        commit_timestamp_ms: u64,
    },
    /// End of epoch
    EndOfEpoch {
        epoch: u64,
    },
}

/// Transaction data (unsigned)
#[derive(Debug, Clone)]
pub struct TransactionData {
    /// Transaction kind
    pub kind: TransactionKind,
    /// Sender address
    pub sender: SuiAddress,
    /// Gas data
    pub gas_data: GasData,
    /// Expiration epoch (optional)
    pub expiration: Option<u64>,
}

impl TransactionData {
    /// Compute transaction digest
    pub fn digest(&self) -> TransactionDigest {
        let mut hasher = Sha256::new();
        hasher.update(&self.sender);
        hasher.update(&self.gas_data.budget.to_le_bytes());
        hasher.update(&self.gas_data.price.to_le_bytes());
        // Simplified: in practice, hash all fields
        hasher.finalize().into()
    }

    /// Check if transaction requires consensus
    pub fn requires_consensus(&self) -> bool {
        match &self.kind {
            TransactionKind::ProgrammableTransaction(ptb) => ptb.requires_consensus(),
            TransactionKind::System(_) => true,
        }
    }
}

// =============================================================================
// Transaction Effects
// =============================================================================

/// Object change type
#[derive(Debug, Clone)]
pub enum ObjectChange {
    /// Object was created
    Created {
        id: ObjectId,
        digest: ObjectDigest,
        owner: crate::object::Owner,
    },
    /// Object was mutated
    Mutated {
        id: ObjectId,
        version: SequenceNumber,
        digest: ObjectDigest,
    },
    /// Object was deleted
    Deleted {
        id: ObjectId,
        version: SequenceNumber,
    },
    /// Object was wrapped (stored in another object)
    Wrapped {
        id: ObjectId,
        version: SequenceNumber,
    },
    /// Object was unwrapped
    Unwrapped {
        id: ObjectId,
        version: SequenceNumber,
        digest: ObjectDigest,
    },
}

/// Gas cost summary
#[derive(Debug, Clone, Default)]
pub struct GasCostSummary {
    /// Computation cost
    pub computation_cost: u64,
    /// Storage cost
    pub storage_cost: u64,
    /// Storage rebate
    pub storage_rebate: u64,
    /// Non-refundable storage fee
    pub non_refundable_storage_fee: u64,
}

impl GasCostSummary {
    /// Total gas used
    pub fn gas_used(&self) -> u64 {
        self.computation_cost + self.storage_cost
    }

    /// Net gas cost (after rebate)
    pub fn net_gas_cost(&self) -> i64 {
        (self.computation_cost + self.storage_cost) as i64 - self.storage_rebate as i64
    }
}

/// Execution status
#[derive(Debug, Clone)]
pub enum ExecutionStatus {
    Success,
    Failure { error: String },
}

/// Transaction effects
#[derive(Debug, Clone)]
pub struct TransactionEffects {
    /// Transaction digest
    pub transaction_digest: TransactionDigest,
    /// Execution status
    pub status: ExecutionStatus,
    /// Gas cost summary
    pub gas_cost: GasCostSummary,
    /// Object changes
    pub object_changes: Vec<ObjectChange>,
    /// Events emitted
    pub events_digest: Option<[u8; 32]>,
    /// Dependencies (input object versions)
    pub dependencies: Vec<TransactionDigest>,
}

impl TransactionEffects {
    /// Compute effects digest
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.transaction_digest);
        hasher.update(&self.gas_cost.computation_cost.to_le_bytes());
        // Simplified
        hasher.finalize().into()
    }

    /// Check if execution succeeded
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success)
    }
}

// =============================================================================
// PTB Builder (convenience)
// =============================================================================

/// Builder for constructing PTBs
#[derive(Debug, Default)]
pub struct PtbBuilder {
    ptb: ProgrammableTransaction,
}

impl PtbBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pure u64 value
    pub fn pure_u64(&mut self, value: u64) -> Argument {
        self.ptb.add_pure_input(value.to_le_bytes().to_vec())
    }

    /// Add a pure address
    pub fn pure_address(&mut self, addr: SuiAddress) -> Argument {
        self.ptb.add_pure_input(addr.to_vec())
    }

    /// Add an owned object
    pub fn object(&mut self, obj_ref: ObjectRef) -> Argument {
        self.ptb
            .add_object_input(ObjectArg::ImmOrOwnedObject(obj_ref))
    }

    /// Add a shared object
    pub fn shared_object(
        &mut self,
        id: ObjectId,
        initial_shared_version: SequenceNumber,
        mutable: bool,
    ) -> Argument {
        self.ptb.add_object_input(ObjectArg::SharedObject {
            id,
            initial_shared_version,
            mutable,
        })
    }

    /// Split coins
    pub fn split_coins(&mut self, coin: Argument, amounts: Vec<Argument>) -> Argument {
        self.ptb.add_command(Command::SplitCoins { coin, amounts })
    }

    /// Merge coins
    pub fn merge_coins(&mut self, target: Argument, sources: Vec<Argument>) -> Argument {
        self.ptb.add_command(Command::MergeCoins { target, sources })
    }

    /// Transfer objects
    pub fn transfer_objects(&mut self, objects: Vec<Argument>, recipient: Argument) -> Argument {
        self.ptb.add_command(Command::TransferObjects { objects, recipient })
    }

    /// Move call
    pub fn move_call(
        &mut self,
        package: ObjectId,
        module: &str,
        function: &str,
        type_arguments: Vec<String>,
        arguments: Vec<Argument>,
    ) -> Argument {
        self.ptb.add_command(Command::MoveCall {
            target: MoveCallTarget::new(package, module, function),
            type_arguments,
            arguments,
        })
    }

    /// Build the PTB
    pub fn build(self) -> ProgrammableTransaction {
        self.ptb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_object_ref(seed: u8) -> ObjectRef {
        ObjectRef {
            object_id: [seed; 32],
            version: 1,
            digest: [seed; 32],
        }
    }

    #[test]
    fn test_argument_types() {
        assert_eq!(Argument::GasCoin, Argument::GasCoin);
        assert_eq!(Argument::Input(0), Argument::Input(0));
        assert_ne!(Argument::Input(0), Argument::Input(1));
        assert_eq!(Argument::Result(5), Argument::Result(5));
    }

    #[test]
    fn test_object_arg() {
        let obj_ref = make_object_ref(1);
        let owned = ObjectArg::ImmOrOwnedObject(obj_ref.clone());
        assert!(!owned.requires_consensus());

        let shared = ObjectArg::SharedObject {
            id: [2u8; 32],
            initial_shared_version: 5,
            mutable: true,
        };
        assert!(shared.requires_consensus());
    }

    #[test]
    fn test_ptb_construction() {
        let mut ptb = ProgrammableTransaction::new();

        // Add inputs
        let amount = ptb.add_pure_input(100u64.to_le_bytes().to_vec());
        let coin = ptb.add_object_input(ObjectArg::ImmOrOwnedObject(make_object_ref(1)));

        // Add commands
        let split_result = ptb.add_command(Command::SplitCoins {
            coin,
            amounts: vec![amount],
        });

        let recipient = ptb.add_pure_input([3u8; 32].to_vec());
        ptb.add_command(Command::TransferObjects {
            objects: vec![split_result],
            recipient,
        });

        assert_eq!(ptb.inputs.len(), 3);
        assert_eq!(ptb.commands.len(), 2);
        assert!(!ptb.requires_consensus());
    }

    #[test]
    fn test_ptb_with_shared_object() {
        let mut ptb = ProgrammableTransaction::new();

        ptb.add_object_input(ObjectArg::SharedObject {
            id: [1u8; 32],
            initial_shared_version: 10,
            mutable: true,
        });

        assert!(ptb.requires_consensus());
    }

    #[test]
    fn test_ptb_validation() {
        let mut ptb = ProgrammableTransaction::new();
        let input = ptb.add_pure_input(vec![1, 2, 3]);

        // Valid: reference existing input
        ptb.add_command(Command::MoveCall {
            target: MoveCallTarget::new([0u8; 32], "module", "func"),
            type_arguments: vec![],
            arguments: vec![input],
        });

        assert!(ptb.validate().is_ok());

        // Invalid: reference future result
        ptb.commands.push(Command::MoveCall {
            target: MoveCallTarget::new([0u8; 32], "module", "func"),
            type_arguments: vec![],
            arguments: vec![Argument::Result(10)], // Invalid
        });

        assert!(ptb.validate().is_err());
    }

    #[test]
    fn test_ptb_builder() {
        let mut builder = PtbBuilder::new();

        let amount = builder.pure_u64(500);
        let coin = builder.object(make_object_ref(1));
        let recipient = builder.pure_address([2u8; 32]);

        let split = builder.split_coins(coin, vec![amount]);
        builder.transfer_objects(vec![split], recipient);

        let ptb = builder.build();
        assert_eq!(ptb.commands.len(), 2);
    }

    #[test]
    fn test_gas_cost_summary() {
        let gas = GasCostSummary {
            computation_cost: 1000,
            storage_cost: 500,
            storage_rebate: 200,
            non_refundable_storage_fee: 50,
        };

        assert_eq!(gas.gas_used(), 1500);
        assert_eq!(gas.net_gas_cost(), 1300);
    }

    #[test]
    fn test_execution_status() {
        let success = ExecutionStatus::Success;
        assert!(matches!(success, ExecutionStatus::Success));

        let failure = ExecutionStatus::Failure {
            error: "out of gas".to_string(),
        };
        assert!(matches!(failure, ExecutionStatus::Failure { .. }));
    }

    #[test]
    fn test_transaction_effects() {
        let effects = TransactionEffects {
            transaction_digest: [1u8; 32],
            status: ExecutionStatus::Success,
            gas_cost: GasCostSummary::default(),
            object_changes: vec![
                ObjectChange::Created {
                    id: [2u8; 32],
                    digest: [3u8; 32],
                    owner: crate::object::Owner::AddressOwner([4u8; 32]),
                },
            ],
            events_digest: None,
            dependencies: vec![],
        };

        assert!(effects.is_success());
        assert_eq!(effects.object_changes.len(), 1);
    }

    #[test]
    fn test_move_call_target() {
        let target = MoveCallTarget::new([0u8; 32], "coin", "transfer");
        assert_eq!(target.module, "coin");
        assert_eq!(target.function, "transfer");
    }
}
