//! Concrete scorer implementations
//!
//! Provides concrete implementations of various scoring strategies, eliminating hardcoded magic numbers

use super::calculator::ScoreCalculator;
use super::context::ScoringContext;
use super::*;

/// Base scorer
///
/// Calculates base score based on prefix matching and match ratio
pub struct BaseScorer;

impl ScoreCalculator for BaseScorer {
    fn calculate(&self, context: &ScoringContext) -> f64 {
        let mut score = 0.0;

        // Prefix matching is the most basic requirement
        if context.is_prefix_match {
            score += BASE_SCORE;

            // Match ratio bonus: the closer the input is to the complete completion, the higher the score
            let match_ratio = context.match_ratio();
            score += match_ratio * MATCH_RATIO_WEIGHT;
        }

        clamp_score(score)
    }

    fn name(&self) -> &'static str {
        "base"
    }
}

/// History scorer
///
/// Calculates score based on frequency and position in history records
pub struct HistoryScorer;

impl ScoreCalculator for HistoryScorer {
    fn calculate(&self, context: &ScoringContext) -> f64 {
        let mut score = 0.0;

        // History weight: reflects usage frequency
        score += context.history_weight * HISTORY_WEIGHT;

        // Position bonus: newer commands get higher scores
        if let Some(position) = context.history_position {
            let position_factor = (1000 - position.min(1000)) as f64 / 1000.0;
            score += position_factor * POSITION_WEIGHT;
        }

        clamp_score(score)
    }

    fn name(&self) -> &'static str {
        "history"
    }
}

/// Smart provider scorer
///
/// Adds bonus score for completions provided by smart providers
pub struct SmartProviderScorer;

impl ScoreCalculator for SmartProviderScorer {
    fn calculate(&self, context: &ScoringContext) -> f64 {
        let mut score = 0.0;

        // Default bonus for smart completions
        if context.source.as_deref() == Some("smart") {
            score += SMART_BOOST;
        }

        // Additional bonus for prefix matching
        if context.is_prefix_match {
            score += PREFIX_MATCH_BONUS;
        }

        clamp_score(score)
    }

    fn name(&self) -> &'static str {
        "smart"
    }
}

/// Frecency scorer
///
/// Scoring algorithm combining Frequency and Recency
///
/// References Mozilla Firefox's Frecency algorithm
pub struct FrecencyScorer;

impl FrecencyScorer {
    /// Calculate time decay factor
    ///
    /// Uses exponential decay: more distant times have lower weights
    fn time_decay_factor(seconds_ago: u64) -> f64 {
        const HOUR: u64 = 3600;
        const DAY: u64 = 86400;
        const WEEK: u64 = 604800;
        const MONTH: u64 = 2592000;

        // Exponential decay: most recent has highest weight
        match seconds_ago {
            0..HOUR => 1.0,     // Within 1 hour: 100%
            HOUR..DAY => 0.9,   // Within 1 day: 90%
            DAY..WEEK => 0.7,   // Within 1 week: 70%
            WEEK..MONTH => 0.5, // Within 1 month: 50%
            _ => 0.3,           // Earlier: 30%
        }
    }

    /// Calculate frequency factor
    ///
    /// Higher frequency yields higher score, but growth rate decreases (logarithmic growth)
    fn frequency_factor(frequency: usize) -> f64 {
        // Use logarithmic function to avoid excessive scores for high-frequency commands
        if frequency == 0 {
            0.0
        } else {
            (frequency as f64).ln() * 10.0 // ln(e) = 1 -> 10 points, ln(100) ~= 4.6 -> 46 points
        }
    }
}

impl ScoreCalculator for FrecencyScorer {
    fn calculate(&self, context: &ScoringContext) -> f64 {
        let mut score = 0.0;

        // Frequency scoring
        if let Some(frequency) = context.frequency {
            score += Self::frequency_factor(frequency);
        }

        // Recency scoring
        if let Some(last_used) = context.last_used_timestamp {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let seconds_ago = now.saturating_sub(last_used);
            let decay = Self::time_decay_factor(seconds_ago);

            // Recency bonus: recently used commands get higher scores
            score += decay * HISTORY_WEIGHT;
        }

        clamp_score(score)
    }

    fn name(&self) -> &'static str {
        "frecency"
    }
}

/// Composite scorer
///
/// Combines results from multiple scorers, supporting composable scoring strategies
pub struct CompositeScorer {
    scorers: Vec<Box<dyn ScoreCalculator>>,
}

impl CompositeScorer {
    /// Create a new composite scorer
    pub fn new(scorers: Vec<Box<dyn ScoreCalculator>>) -> Self {
        Self { scorers }
    }

    /// Create default composite scorer (for general use cases)
    pub fn default_composite() -> Self {
        Self::new(vec![
            Box::new(BaseScorer),
            Box::new(HistoryScorer),
            Box::new(SmartProviderScorer),
        ])
    }
}

impl ScoreCalculator for CompositeScorer {
    fn calculate(&self, context: &ScoringContext) -> f64 {
        let total: f64 = self
            .scorers
            .iter()
            .map(|scorer| scorer.calculate(context))
            .sum();

        clamp_score(total)
    }

    fn name(&self) -> &'static str {
        "composite"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_scorer() {
        let scorer = BaseScorer;

        // Prefix match, high match ratio
        let ctx1 = ScoringContext::new("git status", "git status").with_prefix_match(true);
        let score1 = scorer.calculate(&ctx1);
        assert!(
            score1 > BASE_SCORE,
            "Exact match should have additional bonus"
        );

        // Prefix match, low match ratio
        let ctx2 = ScoringContext::new("git", "git status").with_prefix_match(true);
        let score2 = scorer.calculate(&ctx2);
        assert!(
            score2 >= BASE_SCORE && score2 < score1,
            "Partial match should have lower score"
        );

        // Non-prefix match
        let ctx3 = ScoringContext::new("status", "git status").with_prefix_match(false);
        let score3 = scorer.calculate(&ctx3);
        assert_eq!(score3, 0.0, "Non-prefix match should score 0");
    }

    #[test]
    fn test_history_scorer() {
        let scorer = HistoryScorer;

        // High weight, latest position
        let ctx1 = ScoringContext::new("git", "git status")
            .with_history_weight(1.0)
            .with_history_position(0);
        let score1 = scorer.calculate(&ctx1);
        assert!(
            score1 > HISTORY_WEIGHT,
            "Latest and high frequency should have high score"
        );

        // Low weight, old position
        let ctx2 = ScoringContext::new("git", "git status")
            .with_history_weight(0.1)
            .with_history_position(999);
        let score2 = scorer.calculate(&ctx2);
        assert!(score2 < score1, "Old commands should have lower score");

        // No history data
        let ctx3 = ScoringContext::new("git", "git status");
        let score3 = scorer.calculate(&ctx3);
        assert_eq!(score3, 0.0, "No history data should score 0");
    }

    #[test]
    fn test_smart_provider_scorer() {
        let scorer = SmartProviderScorer;

        // Smart provider + prefix match
        let ctx1 = ScoringContext::new("git", "git status")
            .with_source("smart")
            .with_prefix_match(true);
        let score1 = scorer.calculate(&ctx1);
        assert!(
            score1 > SMART_BOOST,
            "Smart + prefix should have additional bonus"
        );

        // Non-smart provider
        let ctx2 = ScoringContext::new("git", "git status")
            .with_source("history")
            .with_prefix_match(true);
        let score2 = scorer.calculate(&ctx2);
        assert_eq!(score2, PREFIX_MATCH_BONUS, "Should only have prefix bonus");

        // Smart provider but non-prefix
        let ctx3 = ScoringContext::new("git", "git status")
            .with_source("smart")
            .with_prefix_match(false);
        let score3 = scorer.calculate(&ctx3);
        assert_eq!(score3, SMART_BOOST, "Should only have smart bonus");
    }

    #[test]
    fn test_composite_scorer() {
        let scorer = CompositeScorer::default_composite();

        // Perfect scenario: prefix match + high history weight + smart provider
        let ctx = ScoringContext::new("git", "git status")
            .with_prefix_match(true)
            .with_history_weight(1.0)
            .with_history_position(0)
            .with_source("smart");

        let score = scorer.calculate(&ctx);
        assert!(
            score > BASE_SCORE,
            "Composite score should be greater than base score"
        );
        assert!(score <= MAX_SCORE, "Score should not exceed maximum");
    }

    #[test]
    fn test_score_clamping() {
        let scorer = CompositeScorer::default_composite();

        // Create extreme scenario to ensure score doesn't overflow
        let ctx = ScoringContext::new("test", "test")
            .with_prefix_match(true)
            .with_history_weight(1.0)
            .with_history_position(0)
            .with_source("smart");

        let score = scorer.calculate(&ctx);
        assert!(
            (0.0..=MAX_SCORE).contains(&score),
            "Score should be within valid range"
        );
    }

    #[test]
    fn test_empty_composite_scorer() {
        let scorer = CompositeScorer::new(vec![]);
        let ctx = ScoringContext::new("test", "test");

        assert_eq!(
            scorer.calculate(&ctx),
            0.0,
            "Empty composite should return 0"
        );
    }

    #[test]
    fn test_custom_composite() {
        // Use only base and history scorers
        let scorer = CompositeScorer::new(vec![Box::new(BaseScorer), Box::new(HistoryScorer)]);

        let ctx = ScoringContext::new("git", "git status")
            .with_prefix_match(true)
            .with_history_weight(0.5);

        let score = scorer.calculate(&ctx);
        assert!(score > 0.0, "Custom composite should work correctly");
    }

    #[test]
    fn test_frecency_scorer_frequency() {
        let scorer = FrecencyScorer;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // High frequency command
        let ctx_high = ScoringContext::new("git", "git status")
            .with_frequency(100)
            .with_last_used_timestamp(now);

        // Low frequency command
        let ctx_low = ScoringContext::new("git", "git status")
            .with_frequency(2)
            .with_last_used_timestamp(now);

        let score_high = scorer.calculate(&ctx_high);
        let score_low = scorer.calculate(&ctx_low);

        assert!(
            score_high > score_low,
            "High frequency commands should score higher: {score_high} vs {score_low}"
        );
    }

    #[test]
    fn test_frecency_scorer_recency() {
        let scorer = FrecencyScorer;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Recently used command
        let ctx_recent = ScoringContext::new("git", "git status")
            .with_frequency(10)
            .with_last_used_timestamp(now);

        // Command used 1 day ago
        let ctx_old = ScoringContext::new("git", "git status")
            .with_frequency(10)
            .with_last_used_timestamp(now - 86400);

        let score_recent = scorer.calculate(&ctx_recent);
        let score_old = scorer.calculate(&ctx_old);

        assert!(
            score_recent > score_old,
            "Recently used commands should score higher: {score_recent} vs {score_old}"
        );
    }
}
