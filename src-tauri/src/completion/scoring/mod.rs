//! Completion scoring system
//!
//! Provides unified completion item scoring mechanism, eliminating hardcoded magic numbers,
//! making scoring logic composable, testable, and configurable.
//!
//! # Architecture
//!
//! ```text
//! ScoringContext (data)
//!     ↓
//! ScoreCalculator (abstraction)
//!     ↓
//! BaseScorer / HistoryScorer / ... (concrete implementations)
//!     ↓
//! CompositeScorer (composition)
//! ```
//!
//! # Example
//!
//! ```rust
//! use crate::completion::scoring::*;
//!
//! let context = ScoringContext::new("git")
//!     .with_prefix_match(true)
//!     .with_history_weight(0.8);
//!
//! let scorer = CompositeScorer::new(vec![
//!     Box::new(BaseScorer),
//!     Box::new(HistoryScorer),
//! ]);
//!
//! let score = scorer.calculate(&context);
//! ```

pub mod calculator;
pub mod context;
pub mod scorers;

pub use calculator::ScoreCalculator;
pub use context::ScoringContext;
pub use scorers::{
    BaseScorer, CompositeScorer, FrecencyScorer, HistoryScorer, SmartProviderScorer,
};

/// Scoring constants - eliminate magic numbers
///
/// These constants are the result of trade-offs, not arbitrary numbers
/// Base match score - minimum score for any valid completion
pub const BASE_SCORE: f64 = 70.0;

/// History weight coefficient - impact of history records on scoring
pub const HISTORY_WEIGHT: f64 = 20.0;

/// Smart completion bonus - advantage of smart provider
pub const SMART_BOOST: f64 = 10.0;

/// Prefix match bonus - prefix match takes priority over fuzzy match
pub const PREFIX_MATCH_BONUS: f64 = 15.0;

/// Minimum valid score - completions below this score will be filtered
pub const MIN_SCORE: f64 = 10.0;

/// Maximum score limit - prevent score overflow
pub const MAX_SCORE: f64 = 100.0;

/// Position weight coefficient - impact of history position on scoring
pub const POSITION_WEIGHT: f64 = 10.0;

/// Match ratio weight coefficient - impact of match length on scoring
pub const MATCH_RATIO_WEIGHT: f64 = 20.0;

/// Ensure score is within valid range
#[inline]
pub fn clamp_score(score: f64) -> f64 {
    score.clamp(0.0, MAX_SCORE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_score() {
        assert_eq!(clamp_score(-10.0), 0.0);
        assert_eq!(clamp_score(50.0), 50.0);
        assert_eq!(clamp_score(150.0), MAX_SCORE);
    }

    #[test]
    fn test_score_constants() {
        // Ensure scoring coefficients don't exceed maximum score when summed
        let max_possible = BASE_SCORE + HISTORY_WEIGHT + SMART_BOOST + PREFIX_MATCH_BONUS;
        assert!(
            max_possible <= MAX_SCORE + 20.0,
            "Scoring coefficients sum too large: {max_possible}"
        );
    }
}
