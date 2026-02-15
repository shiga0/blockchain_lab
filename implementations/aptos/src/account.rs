//! # Aptos Account and Resource Model
//!
//! Aptos uses Move's account-based resource model:
//!
//! - **Account**: Identified by 32-byte address
//! - **Resource**: Move struct stored under an account
//! - **Module**: Move bytecode published under an account
//!
//! Unlike Sui's object-centric model, Aptos resources live under accounts
//! and are accessed via (address, type) pairs.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Account address (32 bytes)
pub type Address = [u8; 32];

/// Authentication key (32 bytes)
pub type AuthenticationKey = [u8; 32];

/// Sequence number for replay protection
pub type SequenceNumber = u64;

/// Type tag identifying a Move type
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeTag {
    /// Module address
    pub address: Address,
    /// Module name
    pub module: String,
    /// Struct name
    pub name: String,
    /// Type parameters
    pub type_params: Vec<TypeTag>,
}

impl TypeTag {
    pub fn new(address: Address, module: &str, name: &str) -> Self {
        Self {
            address,
            module: module.to_string(),
            name: name.to_string(),
            type_params: Vec::new(),
        }
    }

    pub fn with_type_params(mut self, params: Vec<TypeTag>) -> Self {
        self.type_params = params;
        self
    }

    /// Create the standard Coin type tag
    pub fn coin(coin_type: TypeTag) -> Self {
        let framework_addr = [0u8; 32]; // 0x1
        Self::new(framework_addr, "coin", "Coin").with_type_params(vec![coin_type])
    }

    /// Create AptosCoin type tag
    pub fn aptos_coin() -> Self {
        let framework_addr = [0u8; 32];
        Self::new(framework_addr, "aptos_coin", "AptosCoin")
    }
}

impl std::fmt::Display for TypeTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}::{}::{}", hex::encode(self.address), self.module, self.name)?;
        if !self.type_params.is_empty() {
            write!(f, "<")?;
            for (i, param) in self.type_params.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", param)?;
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

/// Resource key = (Address, TypeTag)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceKey {
    pub address: Address,
    pub type_tag: TypeTag,
}

impl ResourceKey {
    pub fn new(address: Address, type_tag: TypeTag) -> Self {
        Self { address, type_tag }
    }
}

/// Module identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleId {
    pub address: Address,
    pub name: String,
}

impl ModuleId {
    pub fn new(address: Address, name: &str) -> Self {
        Self {
            address,
            name: name.to_string(),
        }
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}::{}", hex::encode(self.address), self.name)
    }
}

/// Account resource (0x1::account::Account)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountResource {
    /// Authentication key for signature verification
    pub authentication_key: AuthenticationKey,
    /// Sequence number (nonce) for replay protection
    pub sequence_number: SequenceNumber,
    /// GUID creation number
    pub guid_creation_num: u64,
    /// Coin register events
    pub coin_register_events: EventHandle,
    /// Key rotation events
    pub key_rotation_events: EventHandle,
    /// Rotation capability offer
    pub rotation_capability_offer: CapabilityOffer,
    /// Signer capability offer
    pub signer_capability_offer: CapabilityOffer,
}

impl AccountResource {
    pub fn new(authentication_key: AuthenticationKey) -> Self {
        Self {
            authentication_key,
            sequence_number: 0,
            guid_creation_num: 0,
            coin_register_events: EventHandle::new(0, 0),
            key_rotation_events: EventHandle::new(0, 1),
            rotation_capability_offer: CapabilityOffer::default(),
            signer_capability_offer: CapabilityOffer::default(),
        }
    }

    pub fn increment_sequence_number(&mut self) {
        self.sequence_number += 1;
    }
}

/// Event handle for tracking events
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventHandle {
    /// Counter for events emitted
    pub counter: u64,
    /// GUID for this handle
    pub guid: u64,
}

impl EventHandle {
    pub fn new(counter: u64, guid: u64) -> Self {
        Self { counter, guid }
    }
}

/// Capability offer
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CapabilityOffer {
    pub for_address: Option<Address>,
}

/// CoinStore resource (0x1::coin::CoinStore<CoinType>)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoinStore {
    /// Coin balance
    pub coin: Coin,
    /// Whether deposits are frozen
    pub frozen: bool,
    /// Deposit events
    pub deposit_events: EventHandle,
    /// Withdraw events
    pub withdraw_events: EventHandle,
}

impl CoinStore {
    pub fn new(value: u64) -> Self {
        Self {
            coin: Coin { value },
            frozen: false,
            deposit_events: EventHandle::new(0, 0),
            withdraw_events: EventHandle::new(0, 1),
        }
    }

    pub fn deposit(&mut self, amount: u64) -> Result<(), &'static str> {
        if self.frozen {
            return Err("Account is frozen");
        }
        self.coin.value = self.coin.value.checked_add(amount).ok_or("Overflow")?;
        self.deposit_events.counter += 1;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: u64) -> Result<Coin, &'static str> {
        if self.frozen {
            return Err("Account is frozen");
        }
        if self.coin.value < amount {
            return Err("Insufficient balance");
        }
        self.coin.value -= amount;
        self.withdraw_events.counter += 1;
        Ok(Coin { value: amount })
    }

    pub fn balance(&self) -> u64 {
        self.coin.value
    }
}

/// Coin resource
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coin {
    pub value: u64,
}

impl Coin {
    pub fn new(value: u64) -> Self {
        Self { value }
    }

    pub fn merge(&mut self, other: Coin) {
        self.value += other.value;
    }

    pub fn split(&mut self, amount: u64) -> Result<Coin, &'static str> {
        if self.value < amount {
            return Err("Insufficient balance");
        }
        self.value -= amount;
        Ok(Coin { value: amount })
    }
}

/// Move module (bytecode)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Module {
    /// Module bytecode
    pub bytecode: Vec<u8>,
    /// Module hash
    pub hash: [u8; 32],
}

impl Module {
    pub fn new(bytecode: Vec<u8>) -> Self {
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(&bytecode);
            hasher.finalize().into()
        };
        Self { bytecode, hash }
    }
}

/// Global state storage
#[derive(Debug, Default)]
pub struct GlobalState {
    /// Resources: (address, type) -> serialized value
    resources: HashMap<ResourceKey, Vec<u8>>,
    /// Modules: (address, module_name) -> Module
    modules: HashMap<ModuleId, Module>,
}

impl GlobalState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a resource
    pub fn get_resource(&self, key: &ResourceKey) -> Option<&Vec<u8>> {
        self.resources.get(key)
    }

    /// Set a resource
    pub fn set_resource(&mut self, key: ResourceKey, value: Vec<u8>) {
        self.resources.insert(key, value);
    }

    /// Delete a resource
    pub fn delete_resource(&mut self, key: &ResourceKey) -> Option<Vec<u8>> {
        self.resources.remove(key)
    }

    /// Check if resource exists
    pub fn exists_resource(&self, key: &ResourceKey) -> bool {
        self.resources.contains_key(key)
    }

    /// Get a module
    pub fn get_module(&self, id: &ModuleId) -> Option<&Module> {
        self.modules.get(id)
    }

    /// Publish a module
    pub fn publish_module(&mut self, id: ModuleId, module: Module) -> Result<(), &'static str> {
        if self.modules.contains_key(&id) {
            return Err("Module already exists");
        }
        self.modules.insert(id, module);
        Ok(())
    }

    /// Upgrade a module (simplified - real Aptos has compatibility checks)
    pub fn upgrade_module(&mut self, id: ModuleId, module: Module) -> Result<(), &'static str> {
        if !self.modules.contains_key(&id) {
            return Err("Module does not exist");
        }
        self.modules.insert(id, module);
        Ok(())
    }
}

/// Transaction authenticator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionAuthenticator {
    /// Ed25519 signature
    Ed25519 {
        public_key: [u8; 32],
        signature: Vec<u8>,
    },
    /// Multi-Ed25519
    MultiEd25519 {
        public_keys: Vec<[u8; 32]>,
        signatures: Vec<Vec<u8>>,
        bitmap: Vec<u8>,
        threshold: u8,
    },
    /// Secp256k1 ECDSA
    Secp256k1Ecdsa {
        public_key: Vec<u8>,
        signature: Vec<u8>,
    },
}

impl TransactionAuthenticator {
    pub fn ed25519(public_key: [u8; 32], signature: Vec<u8>) -> Self {
        Self::Ed25519 { public_key, signature }
    }

    /// Derive address from authenticator
    pub fn derive_address(&self) -> Address {
        let mut hasher = Sha256::new();
        match self {
            Self::Ed25519 { public_key, .. } => {
                hasher.update(public_key);
                hasher.update(&[0u8]); // scheme byte
            }
            Self::MultiEd25519 { public_keys, threshold, .. } => {
                for pk in public_keys {
                    hasher.update(pk);
                }
                hasher.update(&[*threshold]);
                hasher.update(&[1u8]); // scheme byte
            }
            Self::Secp256k1Ecdsa { public_key, .. } => {
                hasher.update(public_key);
                hasher.update(&[2u8]); // scheme byte
            }
        }
        hasher.finalize().into()
    }
}

/// Signed transaction
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedTransaction {
    /// Raw transaction
    pub raw_txn: RawTransaction,
    /// Authenticator
    pub authenticator: TransactionAuthenticator,
}

impl SignedTransaction {
    pub fn new(raw_txn: RawTransaction, authenticator: TransactionAuthenticator) -> Self {
        Self { raw_txn, authenticator }
    }

    pub fn sender(&self) -> Address {
        self.raw_txn.sender
    }

    pub fn sequence_number(&self) -> SequenceNumber {
        self.raw_txn.sequence_number
    }
}

/// Raw transaction payload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawTransaction {
    /// Sender address
    pub sender: Address,
    /// Sequence number
    pub sequence_number: SequenceNumber,
    /// Payload
    pub payload: TransactionPayload,
    /// Max gas amount
    pub max_gas_amount: u64,
    /// Gas unit price
    pub gas_unit_price: u64,
    /// Expiration timestamp (seconds)
    pub expiration_timestamp_secs: u64,
    /// Chain ID
    pub chain_id: u8,
}

impl RawTransaction {
    pub fn new(
        sender: Address,
        sequence_number: SequenceNumber,
        payload: TransactionPayload,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiration_timestamp_secs: u64,
        chain_id: u8,
    ) -> Self {
        Self {
            sender,
            sequence_number,
            payload,
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
        }
    }
}

/// Transaction payload types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionPayload {
    /// Script function call
    EntryFunction(EntryFunction),
    /// Module bundle (publish modules)
    ModuleBundle(Vec<Vec<u8>>),
    /// Multisig transaction
    Multisig(MultisigPayload),
}

/// Entry function call
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntryFunction {
    /// Module ID
    pub module: ModuleId,
    /// Function name
    pub function: String,
    /// Type arguments
    pub ty_args: Vec<TypeTag>,
    /// Arguments (BCS-encoded)
    pub args: Vec<Vec<u8>>,
}

impl EntryFunction {
    pub fn new(module: ModuleId, function: &str, ty_args: Vec<TypeTag>, args: Vec<Vec<u8>>) -> Self {
        Self {
            module,
            function: function.to_string(),
            ty_args,
            args,
        }
    }
}

/// Multisig payload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultisigPayload {
    pub multisig_address: Address,
    pub transaction_payload: Option<Box<TransactionPayload>>,
}

/// Transaction output
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionOutput {
    /// Write set (state changes)
    pub write_set: Vec<(ResourceKey, Option<Vec<u8>>)>,
    /// Events emitted
    pub events: Vec<ContractEvent>,
    /// Gas used
    pub gas_used: u64,
    /// Status
    pub status: TransactionStatus,
}

/// Contract event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractEvent {
    /// Event key
    pub key: EventHandle,
    /// Sequence number
    pub sequence_number: u64,
    /// Event type
    pub type_tag: TypeTag,
    /// Event data (BCS-encoded)
    pub data: Vec<u8>,
}

/// Transaction execution status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    /// Transaction executed successfully
    Success,
    /// Transaction failed
    Failed(String),
    /// Transaction aborted by Move
    MoveAbort { location: String, code: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_address(seed: u8) -> Address {
        [seed; 32]
    }

    #[test]
    fn test_type_tag() {
        let tag = TypeTag::new(test_address(1), "coin", "Coin");
        assert_eq!(tag.module, "coin");
        assert_eq!(tag.name, "Coin");

        let coin_tag = TypeTag::coin(TypeTag::aptos_coin());
        assert_eq!(coin_tag.type_params.len(), 1);
    }

    #[test]
    fn test_account_resource() {
        let mut account = AccountResource::new([0u8; 32]);
        assert_eq!(account.sequence_number, 0);

        account.increment_sequence_number();
        assert_eq!(account.sequence_number, 1);
    }

    #[test]
    fn test_coin_store() {
        let mut store = CoinStore::new(1000);
        assert_eq!(store.balance(), 1000);

        store.deposit(500).unwrap();
        assert_eq!(store.balance(), 1500);

        let withdrawn = store.withdraw(300).unwrap();
        assert_eq!(withdrawn.value, 300);
        assert_eq!(store.balance(), 1200);
    }

    #[test]
    fn test_coin_store_frozen() {
        let mut store = CoinStore::new(1000);
        store.frozen = true;

        assert!(store.deposit(100).is_err());
        assert!(store.withdraw(100).is_err());
    }

    #[test]
    fn test_coin_operations() {
        let mut coin = Coin::new(100);

        let split = coin.split(30).unwrap();
        assert_eq!(split.value, 30);
        assert_eq!(coin.value, 70);

        coin.merge(split);
        assert_eq!(coin.value, 100);
    }

    #[test]
    fn test_global_state() {
        let mut state = GlobalState::new();
        let key = ResourceKey::new(
            test_address(1),
            TypeTag::new(test_address(0), "coin", "CoinStore"),
        );

        assert!(!state.exists_resource(&key));

        state.set_resource(key.clone(), vec![1, 2, 3]);
        assert!(state.exists_resource(&key));
        assert_eq!(state.get_resource(&key), Some(&vec![1, 2, 3]));

        state.delete_resource(&key);
        assert!(!state.exists_resource(&key));
    }

    #[test]
    fn test_module_publish() {
        let mut state = GlobalState::new();
        let id = ModuleId::new(test_address(1), "my_module");
        let module = Module::new(vec![0, 1, 2, 3]);

        state.publish_module(id.clone(), module.clone()).unwrap();
        assert!(state.get_module(&id).is_some());

        // Can't publish twice
        assert!(state.publish_module(id.clone(), module).is_err());
    }

    #[test]
    fn test_entry_function() {
        let func = EntryFunction::new(
            ModuleId::new(test_address(1), "coin"),
            "transfer",
            vec![TypeTag::aptos_coin()],
            vec![vec![1, 2, 3], vec![4, 5, 6]],
        );

        assert_eq!(func.function, "transfer");
        assert_eq!(func.ty_args.len(), 1);
        assert_eq!(func.args.len(), 2);
    }

    #[test]
    fn test_raw_transaction() {
        let payload = TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(test_address(1), "coin"),
            "transfer",
            vec![],
            vec![],
        ));

        let txn = RawTransaction::new(
            test_address(1),
            0,
            payload,
            1000,
            1,
            3600,
            1,
        );

        assert_eq!(txn.sender, test_address(1));
        assert_eq!(txn.sequence_number, 0);
        assert_eq!(txn.max_gas_amount, 1000);
    }

    #[test]
    fn test_transaction_authenticator() {
        let auth = TransactionAuthenticator::ed25519([1u8; 32], vec![2u8; 64]);
        let addr = auth.derive_address();
        assert_ne!(addr, [0u8; 32]); // Should be a hash
    }

    #[test]
    fn test_transaction_output() {
        let output = TransactionOutput {
            write_set: vec![],
            events: vec![],
            gas_used: 100,
            status: TransactionStatus::Success,
        };

        assert_eq!(output.status, TransactionStatus::Success);
        assert_eq!(output.gas_used, 100);
    }
}
