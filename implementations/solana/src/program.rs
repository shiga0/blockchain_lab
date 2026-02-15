//! Solana Program Module (BPF Smart Contracts)
//!
//! ## Smart Contract Comparison
//!
//! | Aspect | Ethereum (EVM) | Solana (BPF) |
//! |--------|---------------|--------------|
//! | VM | EVM (stack-based) | eBPF/SBF (register-based) |
//! | Language | Solidity, Vyper | Rust, C |
//! | State | Contract storage | Account data |
//! | Code Storage | Contract account | Separate program account |
//! | Upgradability | Proxy pattern | Native (upgradeable loader) |
//! | Cross-calls | CALL opcode | CPI (Cross-Program Invocation) |
//!
//! ## Solana Program Model
//!
//! In Solana, programs (smart contracts) are:
//! - Stateless: No internal storage, use accounts
//! - BPF bytecode: Compiled Rust/C code
//! - Invoked via instructions with accounts
//!
//! ```text
//! Program Invocation:
//!
//! ┌─────────────────────────────────────────────────────────┐
//! │                     Instruction                         │
//! ├─────────────────────────────────────────────────────────┤
//! │ program_id: Token Program                               │
//! │ accounts: [                                             │
//! │   { source_token_account, writable }                   │
//! │   { dest_token_account, writable }                     │
//! │   { owner, signer }                                    │
//! │ ]                                                       │
//! │ data: Transfer { amount: 100 }                         │
//! └─────────────────────────────────────────────────────────┘
//!                           │
//!                           ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │                    Token Program                        │
//! │  fn process_instruction(                                │
//! │      program_id: &Pubkey,                               │
//! │      accounts: &[AccountInfo],                          │
//! │      instruction_data: &[u8]                            │
//! │  ) -> ProgramResult                                     │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Program Types
//!
//! 1. **Native Programs**: Built into the runtime
//!    - System Program: Create accounts, transfer SOL
//!    - Vote Program: Validator voting
//!    - Stake Program: Stake delegation
//!
//! 2. **BPF Programs**: User-deployed
//!    - BPFLoader (deprecated)
//!    - BPFLoaderUpgradeable (current)
//!    - LoaderV4 (latest)
//!
//! ## Cross-Program Invocation (CPI)
//!
//! Programs can call other programs:
//!
//! ```text
//! User TX
//!    │
//!    ▼
//! ┌─────────────┐      CPI       ┌─────────────┐
//! │  My DeFi    │ ──────────────→│   Token     │
//! │  Program    │                │   Program   │
//! └─────────────┘                └─────────────┘
//!        │                              │
//!        │                              ▼
//!        │                       Token Transfer
//!        │
//!        ▼
//!  Custom Logic
//! ```

use crate::account::Pubkey;

/// Program entrypoint signature
///
/// Every Solana program implements this function
pub type ProcessInstruction = fn(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult;

/// Result type for program execution
pub type ProgramResult = Result<(), ProgramError>;

/// Program execution errors
#[derive(Debug, Clone)]
pub enum ProgramError {
    /// Invalid instruction data
    InvalidInstructionData,
    /// Invalid account data
    InvalidAccountData,
    /// Account not signer
    MissingRequiredSignature,
    /// Account not writable
    AccountNotWritable,
    /// Insufficient funds
    InsufficientFunds,
    /// Account already initialized
    AccountAlreadyInitialized,
    /// Account not initialized
    UninitializedAccount,
    /// Owner mismatch
    IncorrectProgramId,
    /// Custom error with code
    Custom(u32),
}

/// Account information passed to programs
///
/// This is a reference to account data, not the account itself.
/// The runtime manages the actual account storage.
#[derive(Debug)]
pub struct AccountInfo<'a> {
    /// Account public key
    pub key: &'a Pubkey,
    /// Is this account a signer?
    pub is_signer: bool,
    /// Is this account writable?
    pub is_writable: bool,
    /// Account lamports (mutable through interior mutability)
    pub lamports: &'a mut u64,
    /// Account data (mutable through interior mutability)
    pub data: &'a mut Vec<u8>,
    /// Owner program
    pub owner: &'a Pubkey,
    /// Is executable?
    pub executable: bool,
    /// Rent epoch
    pub rent_epoch: u64,
}

impl<'a> AccountInfo<'a> {
    /// Check if this account is owned by the given program
    pub fn is_owned_by(&self, program_id: &Pubkey) -> bool {
        self.owner == program_id
    }

    /// Borrow lamports (for reading)
    pub fn lamports(&self) -> u64 {
        *self.lamports
    }

    /// Borrow data (for reading)
    pub fn data(&self) -> &[u8] {
        self.data
    }
}

/// System Program - creates accounts and transfers SOL
///
/// Program ID: 11111111111111111111111111111111
pub mod system_program {
    use super::*;

    /// System Program ID
    pub const ID: Pubkey = [0u8; 32];

    /// System instruction types
    #[derive(Debug, Clone)]
    pub enum SystemInstruction {
        /// Create a new account
        CreateAccount {
            /// Lamports to fund the account
            lamports: u64,
            /// Space to allocate (bytes)
            space: u64,
            /// Owner program
            owner: Pubkey,
        },
        /// Transfer lamports
        Transfer {
            lamports: u64,
        },
        /// Allocate space in an account
        Allocate {
            space: u64,
        },
        /// Assign account to a program
        Assign {
            owner: Pubkey,
        },
    }

    /// Process a system instruction
    pub fn process_instruction(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        // Parse instruction (simplified)
        if instruction_data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        match instruction_data[0] {
            // Transfer instruction
            2 => {
                if accounts.len() < 2 {
                    return Err(ProgramError::InvalidAccountData);
                }

                // Validate accounts
                if !accounts[0].is_signer {
                    return Err(ProgramError::MissingRequiredSignature);
                }
                if !accounts[0].is_writable || !accounts[1].is_writable {
                    return Err(ProgramError::AccountNotWritable);
                }

                // Parse lamports (simplified: assume next 8 bytes)
                if instruction_data.len() < 9 {
                    return Err(ProgramError::InvalidInstructionData);
                }
                let lamports = u64::from_le_bytes(
                    instruction_data[1..9].try_into().unwrap()
                );

                if *accounts[0].lamports < lamports {
                    return Err(ProgramError::InsufficientFunds);
                }

                // Note: In real Solana, AccountInfo uses RefCell for interior mutability.
                // This simplified version shows the logic but would need RefCell
                // to actually compile with proper mutation semantics.
                // The transfer would be:
                // **accounts[0].lamports.borrow_mut() -= lamports;
                // **accounts[1].lamports.borrow_mut() += lamports;

                // For this educational implementation, we just validate
                // and return success (actual mutation requires runtime support)
                let _ = lamports; // Suppress unused warning

                Ok(())
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

/// Program-Derived Address (PDA)
///
/// PDAs are addresses derived from a program ID and seeds.
/// They can only be signed by the program, enabling trustless escrows.
///
/// ```text
/// PDA = hash(seeds, program_id, "ProgramDerivedAddress")
///
/// If PDA is on curve → add bump seed and retry
/// Result: Off-curve point that only the program can "sign"
/// ```
pub fn find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    // Try bump seeds from 255 down to 0
    for bump in (0..=255).rev() {
        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;
        for seed in seeds {
            hasher.update(seed);
        }
        hasher.update(&[bump]);
        hasher.update(program_id);
        hasher.update(b"ProgramDerivedAddress");

        let hash: [u8; 32] = hasher.finalize().into();

        // In reality, we check if this is off the Ed25519 curve
        // For simplicity, we just return the first result
        return (hash, bump);
    }

    // Should never reach here
    ([0u8; 32], 0)
}

/// Cross-Program Invocation (CPI) context
///
/// Used when a program calls another program
#[derive(Debug)]
pub struct CpiContext<'a> {
    /// Program to invoke
    pub program_id: &'a Pubkey,
    /// Accounts to pass
    pub accounts: Vec<AccountInfo<'a>>,
}

/// Invoke another program (CPI)
///
/// This is the mechanism for composability in Solana
pub fn invoke(
    _instruction: &crate::runtime::Instruction,
    _account_infos: &[AccountInfo],
) -> ProgramResult {
    // In real Solana, this:
    // 1. Validates signer and writable permissions
    // 2. Invokes the target program's entrypoint
    // 3. Propagates any errors
    // 4. Updates account state

    todo!("CPI implementation requires runtime integration")
}

/// Invoke with signer seeds (for PDAs)
///
/// Allows a program to sign for PDA accounts it owns
pub fn invoke_signed(
    _instruction: &crate::runtime::Instruction,
    _account_infos: &[AccountInfo],
    _signer_seeds: &[&[&[u8]]],
) -> ProgramResult {
    // Similar to invoke, but also validates PDA signatures
    todo!("CPI with signer seeds requires runtime integration")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pda_derivation() {
        let program_id: Pubkey = [1u8; 32];
        let seeds: &[&[u8]] = &[b"user", b"account"];

        let (pda, bump) = find_program_address(seeds, &program_id);

        // PDA should be deterministic
        let (pda2, bump2) = find_program_address(seeds, &program_id);
        assert_eq!(pda, pda2);
        assert_eq!(bump, bump2);

        // Different seeds should give different PDA
        let (pda3, _) = find_program_address(&[b"other"], &program_id);
        assert_ne!(pda, pda3);
    }
}
