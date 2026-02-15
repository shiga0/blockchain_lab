//! XCM (Cross-Consensus Messaging)
//!
//! ## XCM vs Other Cross-Chain Protocols
//!
//! | Aspect | IBC (Cosmos) | Bridges | XCM (Polkadot) |
//! |--------|--------------|---------|----------------|
//! | Trust | Light clients | Varies | Shared security |
//! | Scope | Cosmos chains | Any | Polkadot ecosystem |
//! | Format | Protobuf | Various | SCALE/versioned |
//! | Execution | On receive | On receive | Programmatic |
//!
//! ## XCM Design Philosophy
//!
//! ```text
//! XCM is NOT a messaging protocol, it's a language for:
//!   - Expressing what should happen (instructions)
//!   - NOT how to transport messages (XCMP/VMP handle transport)
//!
//! Properties:
//!   - Asynchronous: Fire and forget
//!   - Absolute: Instructions are executed or fail
//!   - Asymmetric: No built-in response mechanism
//!   - Agnostic: Works across consensus systems
//! ```
//!
//! ## MultiLocation - Universal Addressing
//!
//! ```text
//! Location examples:
//!
//! From Parachain 1000's perspective:
//!   Here                     = self
//!   ../                      = relay chain (parent)
//!   ../Parachain(1001)       = sibling parachain
//!   ./PalletInstance(5)      = local pallet
//!   ../Parachain(1001)/AccountId32(0x123...)
//!                            = account on sibling chain
//!
//! Interior junctions:
//! ┌─────────────────────────────────────────────────┐
//! │ Parachain(id)      - A specific parachain       │
//! │ AccountId32(key)   - 32-byte account            │
//! │ AccountKey20(key)  - Ethereum-style account     │
//! │ PalletInstance(n)  - Pallet index               │
//! │ GeneralIndex(n)    - Generic index              │
//! │ GeneralKey(bytes)  - Generic key                │
//! │ Plurality(id,part) - Group/collective           │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! ## XCM Instructions
//!
//! ```text
//! Asset Operations:
//!   WithdrawAsset(assets)      - Remove from holding
//!   DepositAsset(assets, dest) - Add to destination
//!   TransferAsset(assets, dest)- Withdraw + Deposit
//!
//! Execution Control:
//!   BuyExecution(fees, weight) - Pay for execution
//!   RefundSurplus              - Refund unused fees
//!   SetErrorHandler(xcm)       - On error, do this
//!   SetAppendix(xcm)           - Always run at end
//!
//! Flow Control:
//!   Transact(origin, call)     - Execute runtime call
//!   QueryResponse(...)         - Return query result
//!   ReportError(dest)          - Report error to dest
//! ```
//!
//! ## Example: Cross-Chain Transfer
//!
//! ```text
//! Alice (Para 1000) → Bob (Para 1001): 10 DOT
//!
//! Step 1: Para 1000 sends to Relay Chain
//! ┌─────────────────────────────────────────┐
//! │ ReserveAssetDeposited([DOT: 10])        │
//! │ ClearOrigin                             │
//! │ BuyExecution(DOT: 1, Unlimited)         │
//! │ DepositReserveAsset(                    │
//! │   assets: [DOT: 9],                     │
//! │   dest: Parachain(1001),                │
//! │   xcm: [                                │
//! │     DepositAsset(All, AccountId32(Bob)) │
//! │   ]                                     │
//! │ )                                       │
//! └─────────────────────────────────────────┘
//!
//! Step 2: Relay Chain forwards to Para 1001
//! ┌─────────────────────────────────────────┐
//! │ ReserveAssetDeposited([DOT: 9])         │
//! │ ClearOrigin                             │
//! │ DepositAsset(All, AccountId32(Bob))     │
//! └─────────────────────────────────────────┘
//! ```

/// MultiLocation - Universal addressing system
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiLocation {
    /// Number of parents (../ in path)
    pub parents: u8,
    /// Interior junctions (path components)
    pub interior: Junctions,
}

impl MultiLocation {
    /// Create "Here" location (self)
    pub fn here() -> Self {
        Self {
            parents: 0,
            interior: Junctions::Here,
        }
    }

    /// Create parent location (relay chain from parachain)
    pub fn parent() -> Self {
        Self {
            parents: 1,
            interior: Junctions::Here,
        }
    }

    /// Create parachain location relative to relay
    pub fn parachain(id: u32) -> Self {
        Self {
            parents: 0,
            interior: Junctions::X1(Junction::Parachain(id)),
        }
    }

    /// Create sibling parachain location
    pub fn sibling_parachain(id: u32) -> Self {
        Self {
            parents: 1,
            interior: Junctions::X1(Junction::Parachain(id)),
        }
    }

    /// Add a junction to the interior
    pub fn push(&mut self, junction: Junction) -> Result<(), &'static str> {
        self.interior = self.interior.clone().push(junction)?;
        Ok(())
    }

    /// Check if this is the "Here" location
    pub fn is_here(&self) -> bool {
        self.parents == 0 && self.interior == Junctions::Here
    }
}

/// Interior junctions (up to 8 levels)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Junctions {
    Here,
    X1(Junction),
    X2(Junction, Junction),
    X3(Junction, Junction, Junction),
}

impl Junctions {
    /// Push a new junction
    pub fn push(self, junction: Junction) -> Result<Self, &'static str> {
        match self {
            Junctions::Here => Ok(Junctions::X1(junction)),
            Junctions::X1(a) => Ok(Junctions::X2(a, junction)),
            Junctions::X2(a, b) => Ok(Junctions::X3(a, b, junction)),
            Junctions::X3(_, _, _) => Err("Max junctions reached"),
        }
    }

    /// Get number of junctions
    pub fn len(&self) -> usize {
        match self {
            Junctions::Here => 0,
            Junctions::X1(_) => 1,
            Junctions::X2(_, _) => 2,
            Junctions::X3(_, _, _) => 3,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Junctions::Here)
    }
}

/// Individual junction component
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Junction {
    /// Parachain with given ID
    Parachain(u32),
    /// 32-byte account ID
    AccountId32 { network: Option<NetworkId>, id: [u8; 32] },
    /// 20-byte account key (Ethereum-style)
    AccountKey20 { network: Option<NetworkId>, key: [u8; 20] },
    /// Pallet instance index
    PalletInstance(u8),
    /// General index
    GeneralIndex(u128),
    /// General key (up to 32 bytes)
    GeneralKey(Vec<u8>),
    /// Plurality (governance body)
    Plurality { id: BodyId, part: BodyPart },
}

/// Network identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkId {
    /// Any network
    Any,
    /// Named network
    Named(Vec<u8>),
    /// Polkadot relay
    Polkadot,
    /// Kusama relay
    Kusama,
}

/// Body identifier for Plurality
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyId {
    Unit,
    Named(Vec<u8>),
    Index(u32),
    Executive,
    Technical,
    Legislative,
    Judicial,
}

/// Body part for Plurality
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyPart {
    Voice,
    Members { count: u32 },
    Fraction { nom: u32, denom: u32 },
    AtLeastProportion { nom: u32, denom: u32 },
    MoreThanProportion { nom: u32, denom: u32 },
}

// =============================================================================
// MultiAsset
// =============================================================================

/// Asset identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetId(pub MultiLocation);

/// Asset amount (fungible or non-fungible)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fungibility {
    Fungible(u128),
    NonFungible(AssetInstance),
}

/// Non-fungible asset instance
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetInstance {
    Undefined,
    Index(u128),
    Array4([u8; 4]),
    Array8([u8; 8]),
    Array16([u8; 16]),
    Array32([u8; 32]),
}

/// A single asset (location + fungibility)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiAsset {
    pub id: AssetId,
    pub fun: Fungibility,
}

impl MultiAsset {
    /// Create fungible asset
    pub fn fungible(location: MultiLocation, amount: u128) -> Self {
        Self {
            id: AssetId(location),
            fun: Fungibility::Fungible(amount),
        }
    }

    /// Create native token asset
    pub fn native(amount: u128) -> Self {
        Self::fungible(MultiLocation::here(), amount)
    }

    /// Get amount if fungible
    pub fn amount(&self) -> Option<u128> {
        match &self.fun {
            Fungibility::Fungible(amount) => Some(*amount),
            Fungibility::NonFungible(_) => None,
        }
    }
}

/// Multiple assets
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultiAssets(pub Vec<MultiAsset>);

impl MultiAssets {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, asset: MultiAsset) {
        self.0.push(asset);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

// =============================================================================
// Asset Filter
// =============================================================================

/// Filter for selecting assets
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiAssetFilter {
    /// Select all assets in holding
    Wild(WildMultiAsset),
    /// Select specific assets
    Definite(MultiAssets),
}

/// Wildcard asset selection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WildMultiAsset {
    /// All assets
    All,
    /// All of a specific asset type
    AllOf { id: AssetId, fun: WildFungibility },
}

/// Wildcard fungibility
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WildFungibility {
    Fungible,
    NonFungible,
}

// =============================================================================
// XCM Instructions
// =============================================================================

/// Weight for execution
pub type Weight = u64;

/// XCM instruction set (v3-like)
#[derive(Debug, Clone)]
pub enum Instruction {
    // === Asset Operations ===
    /// Withdraw assets from origin account
    WithdrawAsset(MultiAssets),

    /// Reserve assets were deposited
    ReserveAssetDeposited(MultiAssets),

    /// Receive teleported assets
    ReceiveTeleportedAsset(MultiAssets),

    /// Deposit assets to beneficiary
    DepositAsset {
        assets: MultiAssetFilter,
        beneficiary: MultiLocation,
    },

    /// Deposit reserve assets and forward XCM
    DepositReserveAsset {
        assets: MultiAssetFilter,
        dest: MultiLocation,
        xcm: Xcm,
    },

    /// Transfer reserve assets
    TransferReserveAsset {
        assets: MultiAssets,
        dest: MultiLocation,
        xcm: Xcm,
    },

    /// Initiate teleport
    InitiateTeleport {
        assets: MultiAssetFilter,
        dest: MultiLocation,
        xcm: Xcm,
    },

    // === Execution Control ===
    /// Buy execution weight with fees
    BuyExecution {
        fees: MultiAsset,
        weight_limit: WeightLimit,
    },

    /// Refund unused execution fees
    RefundSurplus,

    /// Set error handler
    SetErrorHandler(Xcm),

    /// Set appendix (always runs at end)
    SetAppendix(Xcm),

    /// Clear origin
    ClearOrigin,

    /// Descend into child location
    DescendOrigin(Junctions),

    // === Flow Control ===
    /// Execute encoded call
    Transact {
        origin_kind: OriginKind,
        require_weight_at_most: Weight,
        call: Vec<u8>,
    },

    /// Report holding state
    ReportHolding {
        response_info: QueryResponseInfo,
        assets: MultiAssetFilter,
    },

    /// Burn assets
    BurnAsset(MultiAssets),

    /// Expect specific assets in holding
    ExpectAsset(MultiAssets),

    /// Expect specific origin
    ExpectOrigin(Option<MultiLocation>),

    /// Trap (unconditional error)
    Trap(u64),
}

/// XCM program (sequence of instructions)
#[derive(Debug, Clone, Default)]
pub struct Xcm(pub Vec<Instruction>);

impl Xcm {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Self(instructions)
    }

    pub fn push(&mut self, instruction: Instruction) {
        self.0.push(instruction);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Weight limit for execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeightLimit {
    Unlimited,
    Limited(Weight),
}

/// Origin kind for Transact
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginKind {
    /// Native origin (as if from origin chain)
    Native,
    /// Sovereign origin (from parachain sovereign account)
    SovereignAccount,
    /// Superuser origin (root)
    Superuser,
    /// Xcm origin (generic)
    Xcm,
}

/// Response info for queries
#[derive(Debug, Clone)]
pub struct QueryResponseInfo {
    pub destination: MultiLocation,
    pub query_id: u64,
    pub max_weight: Weight,
}

// =============================================================================
// XCM Executor
// =============================================================================

/// Error during XCM execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XcmError {
    /// Asset not found
    AssetNotFound,
    /// Insufficient assets
    Overflow,
    /// Unhandled XCM version
    UnhandledXcmVersion,
    /// Invalid location
    InvalidLocation,
    /// Weight limit exceeded
    WeightLimitReached,
    /// Barrier check failed
    Barrier,
    /// Execution not allowed
    NotWithdrawable,
    /// Unknown instruction
    Unimplemented,
    /// Transact call failed
    FailedToDecode,
    /// Max weight exceeded
    MaxWeightInvalid,
    /// Not holding requested assets
    NotHoldingFees,
    /// Trap instruction
    Trap(u64),
}

/// Outcome of XCM execution
#[derive(Debug, Clone)]
pub enum Outcome {
    /// Completely executed
    Complete(Weight),
    /// Partially executed, then error
    Incomplete(Weight, XcmError),
    /// Failed at barrier
    Error(XcmError),
}

impl Outcome {
    pub fn is_complete(&self) -> bool {
        matches!(self, Outcome::Complete(_))
    }

    pub fn is_error(&self) -> bool {
        !self.is_complete()
    }
}

/// XCM execution context
#[derive(Debug)]
pub struct XcmContext {
    /// Current origin
    pub origin: Option<MultiLocation>,
    /// Holding register (assets being processed)
    pub holding: MultiAssets,
    /// Weight used so far
    pub weight_used: Weight,
    /// Weight limit
    pub weight_limit: Weight,
    /// Error handler
    pub error_handler: Xcm,
    /// Appendix (runs at end)
    pub appendix: Xcm,
}

impl XcmContext {
    pub fn new(origin: MultiLocation, weight_limit: Weight) -> Self {
        Self {
            origin: Some(origin),
            holding: MultiAssets::new(),
            weight_used: 0,
            weight_limit,
            error_handler: Xcm::default(),
            appendix: Xcm::default(),
        }
    }

    /// Execute an XCM program
    pub fn execute(&mut self, xcm: Xcm) -> Outcome {
        for instruction in xcm.0 {
            if let Err(e) = self.execute_instruction(instruction) {
                return Outcome::Incomplete(self.weight_used, e);
            }
        }
        Outcome::Complete(self.weight_used)
    }

    /// Execute a single instruction
    fn execute_instruction(&mut self, instruction: Instruction) -> Result<(), XcmError> {
        // Check weight limit
        let instruction_weight = self.instruction_weight(&instruction);
        if self.weight_used + instruction_weight > self.weight_limit {
            return Err(XcmError::WeightLimitReached);
        }
        self.weight_used += instruction_weight;

        match instruction {
            Instruction::WithdrawAsset(assets) => {
                // In real impl, would withdraw from origin's account
                // Here we just add to holding
                for asset in assets.0 {
                    self.holding.push(asset);
                }
                Ok(())
            }

            Instruction::ReserveAssetDeposited(assets) => {
                // Assets deposited by reserve
                for asset in assets.0 {
                    self.holding.push(asset);
                }
                Ok(())
            }

            Instruction::ReceiveTeleportedAsset(assets) => {
                // Assets teleported from origin
                for asset in assets.0 {
                    self.holding.push(asset);
                }
                Ok(())
            }

            Instruction::DepositAsset { assets, beneficiary } => {
                // In real impl, would deposit to beneficiary
                let _ = (assets, beneficiary);
                self.holding = MultiAssets::new();
                Ok(())
            }

            Instruction::ClearOrigin => {
                self.origin = None;
                Ok(())
            }

            Instruction::DescendOrigin(interior) => {
                if let Some(ref mut origin) = self.origin {
                    let junctions: Vec<Junction> = match &interior {
                        Junctions::Here => vec![],
                        Junctions::X1(a) => vec![a.clone()],
                        Junctions::X2(a, b) => vec![a.clone(), b.clone()],
                        Junctions::X3(a, b, c) => vec![a.clone(), b.clone(), c.clone()],
                    };
                    for junction in junctions {
                        origin.push(junction).map_err(|_| XcmError::InvalidLocation)?;
                    }
                }
                Ok(())
            }

            Instruction::BuyExecution { fees, weight_limit } => {
                // Check if we have the fees in holding
                let has_fees = self.holding.0.iter().any(|a| {
                    a.id == fees.id && a.amount().unwrap_or(0) >= fees.amount().unwrap_or(0)
                });
                if !has_fees {
                    return Err(XcmError::NotHoldingFees);
                }
                // Set weight limit if specified
                if let WeightLimit::Limited(w) = weight_limit {
                    self.weight_limit = w;
                }
                Ok(())
            }

            Instruction::RefundSurplus => {
                // Would refund unused fees to holding
                Ok(())
            }

            Instruction::SetErrorHandler(handler) => {
                self.error_handler = handler;
                Ok(())
            }

            Instruction::SetAppendix(appendix) => {
                self.appendix = appendix;
                Ok(())
            }

            Instruction::Trap(code) => {
                Err(XcmError::Trap(code))
            }

            Instruction::ExpectAsset(_) | Instruction::ExpectOrigin(_) => {
                // Validation instructions
                Ok(())
            }

            _ => Err(XcmError::Unimplemented),
        }
    }

    /// Estimate weight for an instruction
    fn instruction_weight(&self, instruction: &Instruction) -> Weight {
        match instruction {
            Instruction::WithdrawAsset(_) => 1000,
            Instruction::DepositAsset { .. } => 1000,
            Instruction::BuyExecution { .. } => 500,
            Instruction::ClearOrigin => 100,
            Instruction::Transact { require_weight_at_most, .. } => *require_weight_at_most,
            _ => 200,
        }
    }

}

// =============================================================================
// XCMP and VMP
// =============================================================================

/// Upward message (parachain → relay)
#[derive(Debug, Clone)]
pub struct UpwardMessage {
    pub origin: u32, // para_id
    pub data: Vec<u8>,
}

/// Downward message (relay → parachain)
#[derive(Debug, Clone)]
pub struct DownwardMessage {
    pub sent_at: u64, // relay block number
    pub msg: Xcm,
}

/// Horizontal message (parachain → parachain via relay)
#[derive(Debug, Clone)]
pub struct HrmpMessage {
    pub sender: u32,
    pub recipient: u32,
    pub data: Vec<u8>,
}

/// Channel between parachains
#[derive(Debug, Clone)]
pub struct HrmpChannel {
    pub sender: u32,
    pub recipient: u32,
    pub max_capacity: u32,
    pub max_total_size: u32,
    pub mqc_head: Option<[u8; 32]>,
}

impl HrmpChannel {
    pub fn new(sender: u32, recipient: u32) -> Self {
        Self {
            sender,
            recipient,
            max_capacity: 100,
            max_total_size: 10 * 1024 * 1024, // 10 MB
            mqc_head: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_location_here() {
        let here = MultiLocation::here();
        assert!(here.is_here());
        assert_eq!(here.parents, 0);
    }

    #[test]
    fn test_multi_location_parent() {
        let parent = MultiLocation::parent();
        assert!(!parent.is_here());
        assert_eq!(parent.parents, 1);
    }

    #[test]
    fn test_multi_location_sibling() {
        let sibling = MultiLocation::sibling_parachain(1001);
        assert_eq!(sibling.parents, 1);
        assert_eq!(
            sibling.interior,
            Junctions::X1(Junction::Parachain(1001))
        );
    }

    #[test]
    fn test_junctions_push() {
        let j = Junctions::Here;
        let j = j.push(Junction::Parachain(1000)).unwrap();
        assert_eq!(j.len(), 1);

        let j = j.push(Junction::PalletInstance(5)).unwrap();
        assert_eq!(j.len(), 2);
    }

    #[test]
    fn test_multi_asset_fungible() {
        let asset = MultiAsset::native(1_000_000);
        assert_eq!(asset.amount(), Some(1_000_000));
    }

    #[test]
    fn test_xcm_execution_withdraw() {
        let mut ctx = XcmContext::new(MultiLocation::here(), 10000);

        let xcm = Xcm::new(vec![
            Instruction::WithdrawAsset(MultiAssets(vec![
                MultiAsset::native(100),
            ])),
        ]);

        let outcome = ctx.execute(xcm);
        assert!(outcome.is_complete());
        assert_eq!(ctx.holding.len(), 1);
    }

    #[test]
    fn test_xcm_execution_clear_origin() {
        let mut ctx = XcmContext::new(MultiLocation::here(), 10000);
        assert!(ctx.origin.is_some());

        let xcm = Xcm::new(vec![Instruction::ClearOrigin]);
        let outcome = ctx.execute(xcm);

        assert!(outcome.is_complete());
        assert!(ctx.origin.is_none());
    }

    #[test]
    fn test_xcm_trap() {
        let mut ctx = XcmContext::new(MultiLocation::here(), 10000);

        let xcm = Xcm::new(vec![Instruction::Trap(42)]);
        let outcome = ctx.execute(xcm);

        assert!(outcome.is_error());
        match outcome {
            Outcome::Incomplete(_, XcmError::Trap(code)) => assert_eq!(code, 42),
            _ => panic!("Expected Trap error"),
        }
    }

    #[test]
    fn test_xcm_weight_limit() {
        let mut ctx = XcmContext::new(MultiLocation::here(), 100); // Very low limit

        let xcm = Xcm::new(vec![
            Instruction::WithdrawAsset(MultiAssets(vec![MultiAsset::native(100)])),
        ]);

        let outcome = ctx.execute(xcm);
        // WithdrawAsset costs 1000, but limit is 100
        match outcome {
            Outcome::Incomplete(_, XcmError::WeightLimitReached) => {}
            _ => panic!("Expected WeightLimitReached"),
        }
    }

    #[test]
    fn test_xcm_buy_execution() {
        let mut ctx = XcmContext::new(MultiLocation::here(), 10000);

        // First add some assets to holding
        ctx.holding.push(MultiAsset::native(1000));

        let xcm = Xcm::new(vec![
            Instruction::BuyExecution {
                fees: MultiAsset::native(100),
                weight_limit: WeightLimit::Limited(5000),
            },
        ]);

        let outcome = ctx.execute(xcm);
        assert!(outcome.is_complete());
        assert_eq!(ctx.weight_limit, 5000);
    }

    #[test]
    fn test_hrmp_channel() {
        let channel = HrmpChannel::new(1000, 1001);
        assert_eq!(channel.sender, 1000);
        assert_eq!(channel.recipient, 1001);
        assert_eq!(channel.max_capacity, 100);
    }
}
