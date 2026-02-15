//! Sui Object Model
//!
//! ## Object-Centric vs Account Model
//!
//! | Aspect | Account (Ethereum) | Object (Sui) |
//! |--------|-------------------|--------------|
//! | State | Global account map | Individual objects |
//! | Ownership | Implicit (sender) | Explicit (Owner enum) |
//! | Parallelism | Limited (state conflicts) | High (object independence) |
//! | Execution | Sequential | Fastpath/Consensus |
//!
//! ## Object Ownership Types
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ AddressOwner(address)     - Single owner, mutable, fastpath   │
//! │ ObjectOwner(object_id)    - Owned by another object (child)   │
//! │ Shared { version }        - Multi-user, requires consensus    │
//! │ Immutable                 - Frozen forever, anyone can read   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Object Lifecycle
//!
//! ```text
//! Create → Mutate (version++) → Transfer/Share/Freeze → Delete/Wrap
//!
//! Each mutation increments the Lamport timestamp (version).
//! ObjectRef = (ObjectID, Version, Digest) uniquely identifies state.
//! ```

use crate::constants::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Object ID (32 bytes, derived from transaction or random)
pub type ObjectId = [u8; OBJECT_ID_LENGTH];

/// Sui address (32 bytes, derived from public key)
pub type SuiAddress = [u8; 32];

/// Object digest (32 bytes BLAKE2b256 hash)
pub type ObjectDigest = [u8; DIGEST_LENGTH];

/// Transaction digest
pub type TransactionDigest = [u8; DIGEST_LENGTH];

/// Sequence number (Lamport timestamp)
pub type SequenceNumber = u64;

/// Epoch ID
pub type EpochId = u64;

// =============================================================================
// Object Reference
// =============================================================================

/// Reference to a specific version of an object
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectRef {
    /// Object ID
    pub object_id: ObjectId,
    /// Version (Lamport timestamp)
    pub version: SequenceNumber,
    /// Content digest
    pub digest: ObjectDigest,
}

impl ObjectRef {
    pub fn new(object_id: ObjectId, version: SequenceNumber, digest: ObjectDigest) -> Self {
        Self {
            object_id,
            version,
            digest,
        }
    }
}

// =============================================================================
// Object Ownership
// =============================================================================

/// Object ownership type - determines execution path
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    /// Single address owns this object (fastpath execution)
    AddressOwner(SuiAddress),

    /// Another object owns this object (child object)
    ObjectOwner(ObjectId),

    /// Shared object - anyone can use, requires consensus
    Shared {
        /// Version when object became shared
        initial_shared_version: SequenceNumber,
    },

    /// Immutable - frozen forever, no owner
    Immutable,
}

impl Owner {
    /// Check if this object requires consensus for mutations
    pub fn requires_consensus(&self) -> bool {
        matches!(self, Owner::Shared { .. })
    }

    /// Check if object can be used in fastpath
    pub fn is_fastpath(&self) -> bool {
        matches!(self, Owner::AddressOwner(_) | Owner::ObjectOwner(_))
    }

    /// Check if object is immutable
    pub fn is_immutable(&self) -> bool {
        matches!(self, Owner::Immutable)
    }

    /// Get owner address if AddressOwner
    pub fn get_owner_address(&self) -> Option<&SuiAddress> {
        match self {
            Owner::AddressOwner(addr) => Some(addr),
            _ => None,
        }
    }
}

// =============================================================================
// Object Data
// =============================================================================

/// Move object type identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveObjectType {
    /// Package address
    pub package: SuiAddress,
    /// Module name
    pub module: String,
    /// Struct name
    pub name: String,
    /// Type parameters
    pub type_params: Vec<String>,
}

impl MoveObjectType {
    pub fn new(package: SuiAddress, module: &str, name: &str) -> Self {
        Self {
            package,
            module: module.to_string(),
            name: name.to_string(),
            type_params: Vec::new(),
        }
    }

    /// Check if this is a Coin type
    pub fn is_coin(&self) -> bool {
        self.module == "coin" && self.name == "Coin"
    }

    /// Format as string
    pub fn to_string(&self) -> String {
        format!(
            "{}::{}::{}",
            hex::encode(&self.package[..8]),
            self.module,
            self.name
        )
    }
}

/// Move object data
#[derive(Debug, Clone)]
pub struct MoveObject {
    /// Type information
    pub type_: MoveObjectType,
    /// Whether object has public transfer ability
    pub has_public_transfer: bool,
    /// Version (Lamport timestamp)
    pub version: SequenceNumber,
    /// BCS-encoded contents
    pub contents: Vec<u8>,
}

impl MoveObject {
    pub fn new(
        type_: MoveObjectType,
        has_public_transfer: bool,
        version: SequenceNumber,
        contents: Vec<u8>,
    ) -> Self {
        Self {
            type_,
            has_public_transfer,
            version,
            contents,
        }
    }

    /// Compute content hash
    pub fn content_hash(&self) -> ObjectDigest {
        let mut hasher = Sha256::new();
        hasher.update(&self.contents);
        hasher.update(&self.version.to_le_bytes());
        hasher.finalize().into()
    }
}

/// Move package (published modules)
#[derive(Debug, Clone)]
pub struct MovePackage {
    /// Package ID
    pub id: ObjectId,
    /// Version
    pub version: SequenceNumber,
    /// Module bytecode (module_name -> bytecode)
    pub modules: BTreeMap<String, Vec<u8>>,
    /// Dependencies (other package IDs)
    pub dependencies: Vec<ObjectId>,
}

impl MovePackage {
    pub fn new(id: ObjectId, version: SequenceNumber) -> Self {
        Self {
            id,
            version,
            modules: BTreeMap::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn add_module(&mut self, name: &str, bytecode: Vec<u8>) {
        self.modules.insert(name.to_string(), bytecode);
    }
}

/// Object data type
#[derive(Debug, Clone)]
pub enum Data {
    /// Move struct instance
    Move(MoveObject),
    /// Move package (modules)
    Package(MovePackage),
}

// =============================================================================
// Object
// =============================================================================

/// Sui object
#[derive(Debug, Clone)]
pub struct Object {
    /// Object data (Move object or Package)
    pub data: Data,
    /// Ownership
    pub owner: Owner,
    /// Transaction that last mutated this object
    pub previous_transaction: TransactionDigest,
    /// Storage rebate (refund when deleted)
    pub storage_rebate: u64,
}

impl Object {
    /// Create a new Move object
    pub fn new_move(
        move_object: MoveObject,
        owner: Owner,
        previous_transaction: TransactionDigest,
    ) -> Self {
        Self {
            data: Data::Move(move_object),
            owner,
            previous_transaction,
            storage_rebate: 0,
        }
    }

    /// Create a new Package object
    pub fn new_package(package: MovePackage, previous_transaction: TransactionDigest) -> Self {
        Self {
            data: Data::Package(package),
            owner: Owner::Immutable, // Packages are always immutable
            previous_transaction,
            storage_rebate: 0,
        }
    }

    /// Get object ID
    pub fn id(&self) -> ObjectId {
        match &self.data {
            Data::Move(obj) => {
                // Derive ID from content hash (simplified)
                let mut hasher = Sha256::new();
                hasher.update(&obj.contents);
                hasher.finalize().into()
            }
            Data::Package(pkg) => pkg.id,
        }
    }

    /// Get version
    pub fn version(&self) -> SequenceNumber {
        match &self.data {
            Data::Move(obj) => obj.version,
            Data::Package(pkg) => pkg.version,
        }
    }

    /// Compute object digest
    pub fn digest(&self) -> ObjectDigest {
        let mut hasher = Sha256::new();
        match &self.data {
            Data::Move(obj) => {
                hasher.update(&obj.contents);
                hasher.update(&obj.version.to_le_bytes());
            }
            Data::Package(pkg) => {
                hasher.update(&pkg.id);
                hasher.update(&pkg.version.to_le_bytes());
            }
        }
        hasher.finalize().into()
    }

    /// Get ObjectRef for this object
    pub fn compute_object_ref(&self) -> ObjectRef {
        ObjectRef {
            object_id: self.id(),
            version: self.version(),
            digest: self.digest(),
        }
    }

    /// Check if object is a Move object
    pub fn is_move_object(&self) -> bool {
        matches!(self.data, Data::Move(_))
    }

    /// Check if object is a package
    pub fn is_package(&self) -> bool {
        matches!(self.data, Data::Package(_))
    }

    /// Check if object requires consensus
    pub fn requires_consensus(&self) -> bool {
        self.owner.requires_consensus()
    }

    /// Get Move object data
    pub fn as_move_object(&self) -> Option<&MoveObject> {
        match &self.data {
            Data::Move(obj) => Some(obj),
            _ => None,
        }
    }

    /// Increment version (mutate object)
    pub fn increment_version(&mut self) {
        match &mut self.data {
            Data::Move(obj) => obj.version += 1,
            Data::Package(_) => {} // Packages are immutable
        }
    }

    /// Share this object
    pub fn share(&mut self) {
        if let Owner::AddressOwner(_) = self.owner {
            self.owner = Owner::Shared {
                initial_shared_version: self.version(),
            };
        }
    }

    /// Freeze this object (make immutable)
    pub fn freeze(&mut self) {
        self.owner = Owner::Immutable;
    }

    /// Transfer to new owner
    pub fn transfer(&mut self, new_owner: SuiAddress) {
        if !matches!(self.owner, Owner::Immutable | Owner::Shared { .. }) {
            self.owner = Owner::AddressOwner(new_owner);
        }
    }
}

// =============================================================================
// Coin (Special Object)
// =============================================================================

/// Coin object (SUI or other tokens)
#[derive(Debug, Clone)]
pub struct Coin {
    /// Coin type (e.g., "0x2::sui::SUI")
    pub coin_type: MoveObjectType,
    /// Balance
    pub balance: u64,
}

impl Coin {
    /// Create a new SUI coin
    pub fn new_sui(balance: u64) -> Self {
        let sui_package = [0u8; 32]; // Framework package
        Self {
            coin_type: MoveObjectType::new(sui_package, "sui", "SUI"),
            balance,
        }
    }

    /// Split coin into two
    pub fn split(&mut self, amount: u64) -> Option<Coin> {
        if amount > self.balance {
            return None;
        }
        self.balance -= amount;
        Some(Coin {
            coin_type: self.coin_type.clone(),
            balance: amount,
        })
    }

    /// Merge another coin into this one
    pub fn merge(&mut self, other: Coin) -> Result<(), &'static str> {
        if self.coin_type != other.coin_type {
            return Err("Coin type mismatch");
        }
        self.balance += other.balance;
        Ok(())
    }
}

// =============================================================================
// Object Store
// =============================================================================

/// In-memory object store
#[derive(Debug, Default)]
pub struct ObjectStore {
    /// Objects by ID (latest version)
    objects: BTreeMap<ObjectId, Object>,
    /// Version history
    versions: BTreeMap<ObjectId, Vec<SequenceNumber>>,
}

impl ObjectStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get object by ID
    pub fn get(&self, id: &ObjectId) -> Option<&Object> {
        self.objects.get(id)
    }

    /// Get object by reference (checks version and digest)
    pub fn get_by_ref(&self, obj_ref: &ObjectRef) -> Option<&Object> {
        let obj = self.objects.get(&obj_ref.object_id)?;
        if obj.version() == obj_ref.version && obj.digest() == obj_ref.digest {
            Some(obj)
        } else {
            None
        }
    }

    /// Insert or update object
    pub fn insert(&mut self, id: ObjectId, object: Object) {
        let version = object.version();
        self.versions
            .entry(id)
            .or_insert_with(Vec::new)
            .push(version);
        self.objects.insert(id, object);
    }

    /// Delete object
    pub fn delete(&mut self, id: &ObjectId) -> Option<Object> {
        self.objects.remove(id)
    }

    /// Check if object exists
    pub fn contains(&self, id: &ObjectId) -> bool {
        self.objects.contains_key(id)
    }

    /// Get all objects owned by address
    pub fn get_owned_objects(&self, owner: &SuiAddress) -> Vec<&Object> {
        self.objects
            .values()
            .filter(|obj| obj.owner.get_owner_address() == Some(owner))
            .collect()
    }

    /// Get all shared objects
    pub fn get_shared_objects(&self) -> Vec<&Object> {
        self.objects
            .values()
            .filter(|obj| obj.owner.requires_consensus())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_address(seed: u8) -> SuiAddress {
        let mut addr = [0u8; 32];
        addr[0] = seed;
        addr
    }

    fn make_object_id(seed: u8) -> ObjectId {
        let mut id = [0u8; 32];
        id[0] = seed;
        id
    }

    #[test]
    fn test_owner_types() {
        let addr = make_address(1);
        let obj_id = make_object_id(2);

        let address_owner = Owner::AddressOwner(addr);
        assert!(address_owner.is_fastpath());
        assert!(!address_owner.requires_consensus());
        assert_eq!(address_owner.get_owner_address(), Some(&addr));

        let object_owner = Owner::ObjectOwner(obj_id);
        assert!(object_owner.is_fastpath());

        let shared = Owner::Shared {
            initial_shared_version: 5,
        };
        assert!(shared.requires_consensus());
        assert!(!shared.is_fastpath());

        let immutable = Owner::Immutable;
        assert!(immutable.is_immutable());
    }

    #[test]
    fn test_object_ref() {
        let id = make_object_id(1);
        let digest = [0xab; 32];
        let obj_ref = ObjectRef::new(id, 42, digest);

        assert_eq!(obj_ref.object_id, id);
        assert_eq!(obj_ref.version, 42);
        assert_eq!(obj_ref.digest, digest);
    }

    #[test]
    fn test_move_object() {
        let package = make_address(0);
        let obj_type = MoveObjectType::new(package, "coin", "Coin");

        assert!(obj_type.is_coin());

        let move_obj = MoveObject::new(obj_type, true, 1, vec![1, 2, 3]);
        assert_eq!(move_obj.version, 1);
        assert!(move_obj.has_public_transfer);
    }

    #[test]
    fn test_object_mutations() {
        let package = make_address(0);
        let obj_type = MoveObjectType::new(package, "test", "TestObj");
        let move_obj = MoveObject::new(obj_type, true, 1, vec![1, 2, 3]);

        let tx_digest = [0u8; 32];
        let owner = Owner::AddressOwner(make_address(1));

        let mut obj = Object::new_move(move_obj, owner, tx_digest);

        assert_eq!(obj.version(), 1);
        assert!(!obj.requires_consensus());

        // Increment version
        obj.increment_version();
        assert_eq!(obj.version(), 2);

        // Share object
        obj.share();
        assert!(obj.requires_consensus());

        // Transfer (should not work on shared object)
        let new_owner = make_address(2);
        obj.transfer(new_owner);
        assert!(matches!(obj.owner, Owner::Shared { .. }));
    }

    #[test]
    fn test_coin_operations() {
        let mut coin = Coin::new_sui(1000);
        assert_eq!(coin.balance, 1000);

        // Split
        let split = coin.split(300).unwrap();
        assert_eq!(coin.balance, 700);
        assert_eq!(split.balance, 300);

        // Split too much
        assert!(coin.split(1000).is_none());

        // Merge
        let mut coin2 = Coin::new_sui(500);
        coin2.merge(split).unwrap();
        assert_eq!(coin2.balance, 800);
    }

    #[test]
    fn test_object_store() {
        let mut store = ObjectStore::new();

        let owner = make_address(1);
        let tx_digest = [0u8; 32];

        // Create objects
        for i in 0..3 {
            let obj_type = MoveObjectType::new(make_address(0), "test", "Obj");
            let move_obj = MoveObject::new(obj_type, true, 1, vec![i]);
            let obj = Object::new_move(move_obj, Owner::AddressOwner(owner), tx_digest);
            let id = obj.id();
            store.insert(id, obj);
        }

        // Get owned objects
        let owned = store.get_owned_objects(&owner);
        assert_eq!(owned.len(), 3);
    }

    #[test]
    fn test_package_object() {
        let id = make_object_id(1);
        let mut package = MovePackage::new(id, 1);
        package.add_module("my_module", vec![0x00, 0x61, 0x73, 0x6d]);

        let tx_digest = [0u8; 32];
        let obj = Object::new_package(package, tx_digest);

        assert!(obj.is_package());
        assert!(obj.owner.is_immutable());
    }
}
