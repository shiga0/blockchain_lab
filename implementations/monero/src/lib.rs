//! # Monero-style Blockchain Implementation
//!
//! Monero is a privacy-focused cryptocurrency based on the CryptoNote protocol.
//! Key privacy features:
//!
//! 1. **Ring Signatures**: Hide sender among decoys
//! 2. **Stealth Addresses**: One-time addresses for receivers
//! 3. **RingCT**: Hide transaction amounts
//! 4. **Bulletproofs**: Efficient range proofs
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         Monero Privacy Stack                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                    Ring Signatures                               │   │
//! │  │  Sender hidden among ring of decoys (ring size = 16)            │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                    Stealth Addresses                             │   │
//! │  │  One-time public keys derived from receiver's address           │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                    RingCT (Pedersen Commitments)                 │   │
//! │  │  Amount = mask * G + amount * H (hidden but verifiable)         │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                          │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                    Bulletproofs (Range Proofs)                   │   │
//! │  │  Prove amount is in valid range without revealing it            │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Concepts
//!
//! ### Dual-Key System
//!
//! ```text
//! Account:
//!   - View Key (a, A): Can see incoming transactions
//!   - Spend Key (b, B): Required to spend funds
//!
//! Address = (A, B) where A = a*G, B = b*G
//!
//! Stealth Address Generation (by sender):
//!   1. Generate random r
//!   2. R = r*G (transaction public key)
//!   3. P = Hs(r*A)*G + B (one-time public key)
//!
//! Receiver scans:
//!   1. Compute P' = Hs(a*R)*G + B
//!   2. If P' == P, this output belongs to us
//! ```
//!
//! ### Ring Signatures
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         Ring Signature                                   │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  Real Input: Output #5                                                  │
//! │  Ring Members: [Output #12, Output #47, ..., Output #5, ..., Output #89]│
//! │                                                                         │
//! │  Signature proves:                                                      │
//! │    - One of the ring members is being spent                            │
//! │    - The signer knows the private key for that member                  │
//! │    - Cannot tell which member is the real one                          │
//! │                                                                         │
//! │  Key Image: I = x * Hp(P)                                               │
//! │    - Unique per output (prevents double-spend)                         │
//! │    - Does not reveal which output is spent                             │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### RingCT (Confidential Transactions)
//!
//! ```text
//! Pedersen Commitment: C = mask * G + amount * H
//!
//! Transaction Balance:
//!   Sum(input_commitments) = Sum(output_commitments) + fee * H
//!
//! This proves inputs = outputs + fee without revealing amounts
//! ```

pub mod cryptonote;
pub mod ringct;
pub mod stealth;

// Re-exports
pub use cryptonote::*;
pub use ringct::*;
pub use stealth::*;

/// Block time target in seconds (2 minutes)
pub const BLOCK_TIME_TARGET: u64 = 120;

/// Ring size (number of decoys + 1 real input)
pub const RING_SIZE: usize = 16;

/// Minimum mixin count
pub const MIN_MIXIN: usize = 15;

/// Atomic units per XMR
pub const ATOMIC_UNITS: u64 = 1_000_000_000_000; // 10^12 piconero

/// Base block reward (initially 2^64 - 1 piconero)
pub const GENESIS_BLOCK_REWARD: u64 = u64::MAX;

/// Tail emission per block (0.6 XMR)
pub const TAIL_EMISSION_REWARD: u64 = 600_000_000_000;

/// Difficulty adjustment window
pub const DIFFICULTY_WINDOW: usize = 720;

/// RandomX hash output size
pub const RANDOMX_HASH_SIZE: usize = 32;
