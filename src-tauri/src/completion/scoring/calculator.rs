//! Score Calculator Interface
//!
//! Defines a unified scoring interface supporting different scoring strategies

use super::context::ScoringContext;

/// Score Calculator
///
/// All scorers implement this trait to maintain a consistent interface
pub trait ScoreCalculator: Send + Sync {
    /// Calculate score for the given context
    ///
    /// # Parameters
    /// - `context`: Scoring context containing all information needed for scoring
    ///
    /// # Returns
    /// Score value (typically between 0.0 and 100.0)
    fn calculate(&self, context: &ScoringContext) -> f64;

    /// Scorer name (for debugging and logging)
    fn name(&self) -> &'static str {
        "unknown"
    }
}

/// Implement ScoreCalculator for closures
///
/// Allows using simple functions as scorers
impl<F> ScoreCalculator for F
where
    F: Fn(&ScoringContext) -> f64 + Send + Sync,
{
    fn calculate(&self, context: &ScoringContext) -> f64 {
        self(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestScorer {
        fixed_score: f64,
    }

    impl ScoreCalculator for TestScorer {
        fn calculate(&self, _context: &ScoringContext) -> f64 {
            self.fixed_score
        }

        fn name(&self) -> &'static str {
            "test"
        }
    }

    #[test]
    fn test_scorer_trait() {
        let scorer = TestScorer { fixed_score: 75.0 };
        let ctx = ScoringContext::new("test", "test item");

        assert_eq!(scorer.calculate(&ctx), 75.0);
        assert_eq!(scorer.name(), "test");
    }

    #[test]
    fn test_closure_scorer() {
        let scorer = |ctx: &ScoringContext| -> f64 {
            if ctx.is_prefix_match {
                100.0
            } else {
                50.0
            }
        };

        let ctx1 = ScoringContext::new("test", "test").with_prefix_match(true);
        let ctx2 = ScoringContext::new("test", "other").with_prefix_match(false);

        assert_eq!(scorer.calculate(&ctx1), 100.0);
        assert_eq!(scorer.calculate(&ctx2), 50.0);
    }
}
