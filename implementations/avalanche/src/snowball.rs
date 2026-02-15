//! Snowball Consensus Implementation
//!
//! ## Consensus Comparison
//!
//! | Aspect | Nakamoto (PoW) | BFT (Tendermint) | Snowball (Avalanche) |
//! |--------|---------------|------------------|----------------------|
//! | Message Complexity | O(n) | O(n²) | O(k) per round |
//! | Leader | Yes (miner) | Yes (proposer) | No |
//! | Finality | Probabilistic | Deterministic | Probabilistic |
//! | Latency | ~60 min | ~1-7 sec | ~1-2 sec |
//! | Byzantine Tolerance | <50% hashrate | <1/3 stake | <50% stake |
//!
//! ## Protocol Layers
//!
//! ### Slush (Simplest)
//! ```text
//! Query k validators → majority preference → update my preference
//! (No memory, just follows latest poll)
//! ```
//!
//! ### Snowflake (Adds Confidence)
//! ```text
//! Query k validators → if ≥ α votes for same choice:
//!   - If matches my preference: confidence++
//!   - Else: switch preference, reset confidence
//! If confidence ≥ β: FINALIZED
//! ```
//!
//! ### Snowball (Adds Long-term Memory)
//! ```text
//! Query k validators → if ≥ α votes for same choice:
//!   - Add to preferenceStrength[choice]
//!   - If preferenceStrength[choice] > current winner: switch
//! Combines with Snowflake's confidence tracking
//! ```

use crate::constants::*;
use std::collections::HashMap;

/// Choice identifier (e.g., block hash)
pub type ChoiceId = [u8; 32];

/// Consensus decision state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Still undecided
    Processing,
    /// Finalized with this choice
    Accepted(ChoiceId),
    /// Rejected
    Rejected,
}

// =============================================================================
// Slush - Simplest Protocol
// =============================================================================

/// Slush protocol - simple majority following
///
/// The simplest protocol: just track the most recently successful preference.
#[derive(Debug, Clone)]
pub struct Slush {
    /// Current preference
    preference: Option<ChoiceId>,
}

impl Slush {
    pub fn new() -> Self {
        Self { preference: None }
    }

    /// Initialize with a preference
    pub fn with_preference(choice: ChoiceId) -> Self {
        Self {
            preference: Some(choice),
        }
    }

    /// Get current preference
    pub fn preference(&self) -> Option<ChoiceId> {
        self.preference
    }

    /// Record a poll result
    ///
    /// If count >= alpha, update preference to the majority choice.
    pub fn record_poll(&mut self, count: usize, choice: ChoiceId) {
        if count >= ALPHA_PREFERENCE {
            self.preference = Some(choice);
        }
    }
}

impl Default for Slush {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Snowflake - Adds Confidence Counter
// =============================================================================

/// Snowflake protocol - adds confidence tracking
///
/// Tracks consecutive successful polls for the same preference.
/// Finalizes when confidence reaches β.
#[derive(Debug, Clone)]
pub struct Snowflake {
    /// Current preference
    preference: Option<ChoiceId>,
    /// Consecutive successful polls for current preference
    confidence: usize,
    /// Has this reached finality?
    finalized: bool,
}

impl Snowflake {
    pub fn new() -> Self {
        Self {
            preference: None,
            confidence: 0,
            finalized: false,
        }
    }

    /// Initialize with a preference
    pub fn with_preference(choice: ChoiceId) -> Self {
        Self {
            preference: Some(choice),
            confidence: 0,
            finalized: false,
        }
    }

    /// Get current preference
    pub fn preference(&self) -> Option<ChoiceId> {
        self.preference
    }

    /// Get current confidence level
    pub fn confidence(&self) -> usize {
        self.confidence
    }

    /// Check if finalized
    pub fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Record a successful poll
    ///
    /// If count >= alpha and matches preference: increase confidence
    /// If count >= alpha but different choice: switch and reset
    /// If count < alpha: unsuccessful poll, reset confidence
    pub fn record_poll(&mut self, count: usize, choice: ChoiceId) {
        if self.finalized {
            return;
        }

        if count >= ALPHA_CONFIDENCE {
            if self.preference == Some(choice) {
                // Same preference, increase confidence
                self.confidence += 1;

                if self.confidence >= BETA {
                    self.finalized = true;
                }
            } else {
                // Different preference, switch and reset
                self.preference = Some(choice);
                self.confidence = 1;
            }
        } else {
            // Unsuccessful poll, reset confidence
            self.record_unsuccessful_poll();
        }
    }

    /// Record an unsuccessful poll (count < alpha)
    pub fn record_unsuccessful_poll(&mut self) {
        self.confidence = 0;
    }

    /// Get decision state
    pub fn decision(&self) -> Decision {
        if self.finalized {
            if let Some(choice) = self.preference {
                Decision::Accepted(choice)
            } else {
                Decision::Processing
            }
        } else {
            Decision::Processing
        }
    }
}

impl Default for Snowflake {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Snowball - Adds Long-term Preference Tracking
// =============================================================================

/// Snowball protocol - full protocol with preference strength
///
/// Tracks cumulative votes for each choice across all polls.
/// This provides Byzantine resilience by not just following the latest poll.
#[derive(Debug, Clone)]
pub struct Snowball {
    /// Snowflake instance for confidence tracking
    snowflake: Snowflake,
    /// Cumulative preference strength for each choice
    preference_strength: HashMap<ChoiceId, usize>,
    /// The choice with highest preference strength
    strongest: Option<ChoiceId>,
}

impl Snowball {
    pub fn new() -> Self {
        Self {
            snowflake: Snowflake::new(),
            preference_strength: HashMap::new(),
            strongest: None,
        }
    }

    /// Initialize with choices
    pub fn with_choices(choices: &[ChoiceId]) -> Self {
        let mut sb = Self::new();
        for choice in choices {
            sb.preference_strength.insert(*choice, 0);
        }
        if let Some(&first) = choices.first() {
            sb.strongest = Some(first);
            sb.snowflake = Snowflake::with_preference(first);
        }
        sb
    }

    /// Get current preference (based on Snowflake)
    pub fn preference(&self) -> Option<ChoiceId> {
        self.snowflake.preference()
    }

    /// Get strongest choice (based on cumulative votes)
    pub fn strongest(&self) -> Option<ChoiceId> {
        self.strongest
    }

    /// Get confidence level
    pub fn confidence(&self) -> usize {
        self.snowflake.confidence()
    }

    /// Check if finalized
    pub fn is_finalized(&self) -> bool {
        self.snowflake.is_finalized()
    }

    /// Get preference strength for a choice
    pub fn preference_strength(&self, choice: &ChoiceId) -> usize {
        self.preference_strength.get(choice).copied().unwrap_or(0)
    }

    /// Record a poll result
    ///
    /// Updates both long-term preference strength and Snowflake confidence.
    pub fn record_poll(&mut self, count: usize, choice: ChoiceId) {
        if self.is_finalized() {
            return;
        }

        if count >= ALPHA_PREFERENCE {
            // Update preference strength
            let strength = self.preference_strength.entry(choice).or_insert(0);
            *strength += 1;
            let new_strength = *strength;

            // Update strongest if this choice now has more support
            let current_strongest_strength = self
                .strongest
                .and_then(|s| self.preference_strength.get(&s).copied())
                .unwrap_or(0);

            if new_strength > current_strongest_strength {
                self.strongest = Some(choice);
            }
        }

        // Forward to Snowflake for confidence tracking
        self.snowflake.record_poll(count, choice);
    }

    /// Record an unsuccessful poll
    pub fn record_unsuccessful_poll(&mut self) {
        self.snowflake.record_unsuccessful_poll();
    }

    /// Get decision state
    pub fn decision(&self) -> Decision {
        self.snowflake.decision()
    }
}

impl Default for Snowball {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Binary Snowball (for two-choice decisions)
// =============================================================================

/// Binary Snowball - optimized for two-choice decisions
///
/// Used when deciding between exactly two options (e.g., block vs no-block).
#[derive(Debug, Clone)]
pub struct BinarySnowball {
    /// Preference (true = choice A, false = choice B)
    preference: bool,
    /// Preference strength for choice A
    strength_a: usize,
    /// Preference strength for choice B
    strength_b: usize,
    /// Confidence counter
    confidence: usize,
    /// Finalized?
    finalized: bool,
}

impl BinarySnowball {
    /// Create new binary Snowball with initial preference
    pub fn new(initial_preference: bool) -> Self {
        Self {
            preference: initial_preference,
            strength_a: 0,
            strength_b: 0,
            confidence: 0,
            finalized: false,
        }
    }

    /// Get current preference
    pub fn preference(&self) -> bool {
        self.preference
    }

    /// Get confidence
    pub fn confidence(&self) -> usize {
        self.confidence
    }

    /// Check if finalized
    pub fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Record a poll for choice A (true) or B (false)
    pub fn record_poll(&mut self, count: usize, choice: bool) {
        if self.finalized {
            return;
        }

        if count >= ALPHA_PREFERENCE {
            // Update preference strength
            if choice {
                self.strength_a += 1;
            } else {
                self.strength_b += 1;
            }

            // Update preference based on cumulative strength
            let new_preference = self.strength_a >= self.strength_b;

            if new_preference == self.preference {
                // Same preference, increase confidence
                self.confidence += 1;
                if self.confidence >= BETA {
                    self.finalized = true;
                }
            } else {
                // Different preference, switch and reset
                self.preference = new_preference;
                self.confidence = 1;
            }
        } else {
            // Unsuccessful poll
            self.confidence = 0;
        }
    }
}

// =============================================================================
// Poll Result
// =============================================================================

/// Result of a poll round
#[derive(Debug, Clone)]
pub struct PollResult {
    /// Votes for each choice
    pub votes: HashMap<ChoiceId, usize>,
    /// Total responses received
    pub total_responses: usize,
}

impl PollResult {
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
            total_responses: 0,
        }
    }

    /// Add a vote
    pub fn add_vote(&mut self, choice: ChoiceId) {
        *self.votes.entry(choice).or_insert(0) += 1;
        self.total_responses += 1;
    }

    /// Get the majority choice (if any reaches alpha)
    pub fn majority(&self) -> Option<(ChoiceId, usize)> {
        self.votes
            .iter()
            .max_by_key(|(_, &count)| count)
            .filter(|(_, &count)| count >= ALPHA_PREFERENCE)
            .map(|(&choice, &count)| (choice, count))
    }
}

impl Default for PollResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_choice(id: u8) -> ChoiceId {
        let mut arr = [0u8; 32];
        arr[0] = id;
        arr
    }

    #[test]
    fn test_slush_preference() {
        let mut slush = Slush::new();
        let choice_a = make_choice(1);
        let choice_b = make_choice(2);

        // Initially no preference
        assert!(slush.preference().is_none());

        // Successful poll for A
        slush.record_poll(ALPHA_PREFERENCE, choice_a);
        assert_eq!(slush.preference(), Some(choice_a));

        // Successful poll for B switches preference
        slush.record_poll(ALPHA_PREFERENCE, choice_b);
        assert_eq!(slush.preference(), Some(choice_b));

        // Unsuccessful poll doesn't change preference
        slush.record_poll(ALPHA_PREFERENCE - 1, choice_a);
        assert_eq!(slush.preference(), Some(choice_b));
    }

    #[test]
    fn test_snowflake_confidence() {
        let choice_a = make_choice(1);
        let mut sf = Snowflake::with_preference(choice_a);

        // Build confidence
        for i in 0..BETA - 1 {
            sf.record_poll(ALPHA_CONFIDENCE, choice_a);
            assert_eq!(sf.confidence(), i + 1);
            assert!(!sf.is_finalized());
        }

        // Final poll reaches beta
        sf.record_poll(ALPHA_CONFIDENCE, choice_a);
        assert!(sf.is_finalized());
    }

    #[test]
    fn test_snowflake_reset_on_switch() {
        let choice_a = make_choice(1);
        let choice_b = make_choice(2);
        let mut sf = Snowflake::with_preference(choice_a);

        // Build some confidence
        for _ in 0..5 {
            sf.record_poll(ALPHA_CONFIDENCE, choice_a);
        }
        assert_eq!(sf.confidence(), 5);

        // Switching resets confidence
        sf.record_poll(ALPHA_CONFIDENCE, choice_b);
        assert_eq!(sf.preference(), Some(choice_b));
        assert_eq!(sf.confidence(), 1);
    }

    #[test]
    fn test_snowball_preference_strength() {
        let choice_a = make_choice(1);
        let choice_b = make_choice(2);
        let mut sb = Snowball::with_choices(&[choice_a, choice_b]);

        // Vote for A 3 times
        for _ in 0..3 {
            sb.record_poll(ALPHA_PREFERENCE, choice_a);
        }
        assert_eq!(sb.preference_strength(&choice_a), 3);
        assert_eq!(sb.preference_strength(&choice_b), 0);

        // Vote for B 2 times - A still stronger
        for _ in 0..2 {
            sb.record_poll(ALPHA_PREFERENCE, choice_b);
        }
        assert_eq!(sb.strongest(), Some(choice_a));

        // Vote for B 2 more times - B now stronger
        for _ in 0..2 {
            sb.record_poll(ALPHA_PREFERENCE, choice_b);
        }
        assert_eq!(sb.preference_strength(&choice_b), 4);
        assert_eq!(sb.strongest(), Some(choice_b));
    }

    #[test]
    fn test_binary_snowball_finalization() {
        let mut sb = BinarySnowball::new(true);

        // Consistent votes for true
        for _ in 0..BETA {
            sb.record_poll(ALPHA_PREFERENCE, true);
        }

        assert!(sb.is_finalized());
        assert!(sb.preference());
    }
}
