//! Scoring context
//!
//! Encapsulates all data needed for scoring calculations, following the "separation of data and logic" principle

use std::path::PathBuf;

/// Scoring context
///
/// Contains all information needed by the scorer without mixing in scoring logic
#[derive(Debug, Clone)]
pub struct ScoringContext {
    /// Complete text entered by user
    pub input: String,

    /// Cursor position
    pub cursor_position: usize,

    /// Text of current completion item
    pub item_text: String,

    /// Whether it's a prefix match
    pub is_prefix_match: bool,

    /// History weight (0.0 to 1.0)
    pub history_weight: f64,

    /// History position index (smaller is newer)
    pub history_position: Option<usize>,

    /// Current working directory
    pub working_directory: Option<PathBuf>,

    /// Whether in a Git repository
    pub in_git_repo: bool,

    /// Completion source (e.g., "history", "filesystem", "smart")
    pub source: Option<String>,

    /// Command usage frequency (for Frecency algorithm)
    pub frequency: Option<usize>,

    /// Last used time (second-level timestamp)
    pub last_used_timestamp: Option<u64>,
}

impl ScoringContext {
    /// Create new scoring context
    pub fn new(input: impl Into<String>, item_text: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            cursor_position: 0,
            item_text: item_text.into(),
            is_prefix_match: false,
            history_weight: 0.0,
            history_position: None,
            working_directory: None,
            in_git_repo: false,
            source: None,
            frequency: None,
            last_used_timestamp: None,
        }
    }

    /// Set cursor position
    pub fn with_cursor_position(mut self, pos: usize) -> Self {
        self.cursor_position = pos;
        self
    }

    /// Set whether prefix match
    pub fn with_prefix_match(mut self, is_match: bool) -> Self {
        self.is_prefix_match = is_match;
        self
    }

    /// Set history weight
    pub fn with_history_weight(mut self, weight: f64) -> Self {
        self.history_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Set history position
    pub fn with_history_position(mut self, position: usize) -> Self {
        self.history_position = Some(position);
        self
    }

    /// Set completion source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set usage frequency
    pub fn with_frequency(mut self, frequency: usize) -> Self {
        self.frequency = Some(frequency);
        self
    }

    /// Set last used timestamp
    pub fn with_last_used_timestamp(mut self, timestamp: u64) -> Self {
        self.last_used_timestamp = Some(timestamp);
        self
    }

    /// Calculate match ratio (input length / completion text length)
    pub fn match_ratio(&self) -> f64 {
        if self.item_text.is_empty() {
            return 0.0;
        }
        self.input.len() as f64 / self.item_text.len() as f64
    }

    /// Check if it's an exact match
    pub fn is_exact_match(&self) -> bool {
        self.input == self.item_text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let ctx = ScoringContext::new("git st", "git status")
            .with_cursor_position(6)
            .with_prefix_match(true)
            .with_history_weight(0.8)
            .with_history_position(5)
            .with_source("history");

        assert_eq!(ctx.input, "git st");
        assert_eq!(ctx.item_text, "git status");
        assert_eq!(ctx.cursor_position, 6);
        assert!(ctx.is_prefix_match);
        assert_eq!(ctx.history_weight, 0.8);
        assert_eq!(ctx.history_position, Some(5));
        assert_eq!(ctx.source, Some("history".to_string()));
    }

    #[test]
    fn test_match_ratio() {
        let ctx = ScoringContext::new("git", "git status");
        assert!((ctx.match_ratio() - 3.0 / 10.0).abs() < 0.001);

        let ctx_exact = ScoringContext::new("git", "git");
        assert!((ctx_exact.match_ratio() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_is_exact_match() {
        let ctx = ScoringContext::new("git", "git");
        assert!(ctx.is_exact_match());

        let ctx2 = ScoringContext::new("git", "git status");
        assert!(!ctx2.is_exact_match());
    }

    #[test]
    fn test_history_weight_clamping() {
        let ctx = ScoringContext::new("test", "test").with_history_weight(1.5);
        assert_eq!(ctx.history_weight, 1.0);

        let ctx2 = ScoringContext::new("test", "test").with_history_weight(-0.5);
        assert_eq!(ctx2.history_weight, 0.0);
    }
}
