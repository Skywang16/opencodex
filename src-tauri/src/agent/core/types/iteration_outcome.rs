/*!
 * Iteration Outcome - Classification of ReAct loop iteration results
 *
 * This is the core of architectural refactoring: replace implicit judgment logic with explicit data structures.
 *
 * Design principles:
 * 1. Eliminate special cases: only three explicit states
 * 2. Data-driven decisions: outcome determines whether to continue the loop
 * 3. Single responsibility: only responsible for classification, not execution
 */

use serde::{Deserialize, Serialize};

/// Iteration result: explicit classification after LLM response
///
/// This enum eliminates all original implicit judgments:
/// - No longer checks `visible.is_empty()`
/// - No longer guesses "does having thinking count as completion"
/// - No longer relies on magic numbers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IterationOutcome {
    /// Need to execute tool calls, then continue next iteration
    ///
    /// This is the only case that requires continuing the loop.
    ContinueWithTools {
        /// List of tool calls to execute (id, name, input)
        tool_calls: Vec<(String, String, serde_json::Value)>,
    },

    /// Task complete: LLM provided response content
    ///
    /// **Key insight: as long as there is any content (thinking or output), it's complete.**
    /// No need to judge whether content is "sufficient" or "meaningful".
    Complete {
        /// Content within thinking tags
        ///
        /// Even if there's only thinking without output, it still counts as complete.
        thinking: Option<String>,

        /// Visible output content (text outside tags)
        output: Option<String>,
    },

    /// Truly empty response (abnormal case)
    ///
    /// LLM has neither tool calls nor any text output.
    /// This usually indicates:
    /// - LLM error
    /// - Network issues
    /// - Other abnormal conditions
    ///
    /// Should increment idle counter, trigger safety net termination after multiple consecutive occurrences.
    Empty,
}

impl IterationOutcome {
    /// Whether iteration should continue
    ///
    /// Only ContinueWithTools returns true, others should terminate.
    pub fn should_continue(&self) -> bool {
        matches!(self, Self::ContinueWithTools { .. })
    }

    /// Whether it's an empty response
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Get output content for persistence
    ///
    /// Prefer returning output, if not available then return thinking.
    pub fn get_output_for_persistence(&self) -> Option<String> {
        match self {
            Self::Complete { thinking, output } => output.clone().or_else(|| thinking.clone()),
            _ => None,
        }
    }

    /// Determine if there's substantial content
    ///
    /// Any thinking or output counts as having content, no need to judge length.
    pub fn has_content(&self) -> bool {
        match self {
            Self::Complete { thinking, output } => thinking.is_some() || output.is_some(),
            Self::ContinueWithTools { tool_calls } => !tool_calls.is_empty(),
            Self::Empty => false,
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::ContinueWithTools { tool_calls } => {
                if tool_calls.len() == 1 {
                    "Need to execute 1 tool"
                } else {
                    "Need to execute multiple tools"
                }
            }
            Self::Complete { thinking, output } => match (thinking.is_some(), output.is_some()) {
                (true, true) => "Complete (has thinking and output)",
                (true, false) => "Complete (only thinking)",
                (false, true) => "Complete (only output)",
                (false, false) => "Complete (no content)",
            },
            Self::Empty => "Empty response",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_continue() {
        let continue_outcome = IterationOutcome::ContinueWithTools { tool_calls: vec![] };
        assert!(continue_outcome.should_continue());

        let complete_outcome = IterationOutcome::Complete {
            thinking: Some("thinking".to_string()),
            output: None,
        };
        assert!(!complete_outcome.should_continue());

        let empty_outcome = IterationOutcome::Empty;
        assert!(!empty_outcome.should_continue());
    }

    #[test]
    fn test_has_content() {
        // Has thinking
        let outcome1 = IterationOutcome::Complete {
            thinking: Some("thinking content".to_string()),
            output: None,
        };
        assert!(outcome1.has_content());

        // Has output
        let outcome2 = IterationOutcome::Complete {
            thinking: None,
            output: Some("output content".to_string()),
        };
        assert!(outcome2.has_content());

        // Has both
        let outcome3 = IterationOutcome::Complete {
            thinking: Some("thinking".to_string()),
            output: Some("output".to_string()),
        };
        assert!(outcome3.has_content());

        // Has neither (abnormal case, but still Complete)
        let outcome4 = IterationOutcome::Complete {
            thinking: None,
            output: None,
        };
        assert!(!outcome4.has_content());

        // Empty
        let outcome5 = IterationOutcome::Empty;
        assert!(!outcome5.has_content());
    }

    #[test]
    fn test_get_output_for_persistence() {
        // Prefer output
        let outcome1 = IterationOutcome::Complete {
            thinking: Some("thinking".to_string()),
            output: Some("output".to_string()),
        };
        assert_eq!(
            outcome1.get_output_for_persistence(),
            Some("output".to_string())
        );

        // If no output, return thinking
        let outcome2 = IterationOutcome::Complete {
            thinking: Some("thinking".to_string()),
            output: None,
        };
        assert_eq!(
            outcome2.get_output_for_persistence(),
            Some("thinking".to_string())
        );

        // Has neither
        let outcome3 = IterationOutcome::Complete {
            thinking: None,
            output: None,
        };
        assert_eq!(outcome3.get_output_for_persistence(), None);
    }
}
