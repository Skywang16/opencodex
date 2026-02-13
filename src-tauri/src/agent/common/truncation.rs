//! Intelligent truncation utilities that preserve both head and tail content.
//!
//! Unlike simple head-only truncation, these utilities keep both the beginning
//! and end of content, which is crucial for:
//! - Shell output: preserving command start AND exit status/final output
//! - Error logs: keeping context AND the actual error message
//! - Large files: showing structure AND recent content

use serde::{Deserialize, Serialize};

/// Approximate bytes per token for estimation (conservative)
const APPROX_BYTES_PER_TOKEN: usize = 4;

/// Truncation policy defining how content should be limited
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncationPolicy {
    /// Truncate by character count (UTF-8 aware)
    Chars(usize),
    /// Truncate by estimated token count (4 bytes ≈ 1 token)
    Tokens(usize),
    /// Truncate by line count, keeping head and tail lines
    Lines { head: usize, tail: usize },
}

impl TruncationPolicy {
    /// Shell/command output: preserve up to 2000 lines (head-only)
    pub fn shell_output() -> Self {
        TruncationPolicy::Lines {
            head: 2000,
            tail: 0,
        }
    }

    /// Compact shell output for summaries: 20 head + 50 tail lines
    pub fn shell_compact() -> Self {
        TruncationPolicy::Lines { head: 20, tail: 50 }
    }

    /// File content: ~8000 tokens (~32KB)
    pub fn file_content() -> Self {
        TruncationPolicy::Tokens(8000)
    }

    /// Tool output summary: ~2000 chars
    pub fn tool_summary() -> Self {
        TruncationPolicy::Chars(2000)
    }

    /// Web fetch content: ~4000 chars
    pub fn web_content() -> Self {
        TruncationPolicy::Chars(4000)
    }

    /// Get the byte budget for this policy
    pub fn byte_budget(&self) -> usize {
        match self {
            TruncationPolicy::Chars(c) => c.saturating_mul(4), // conservative: assume 4 bytes/char
            TruncationPolicy::Tokens(t) => t.saturating_mul(APPROX_BYTES_PER_TOKEN),
            TruncationPolicy::Lines { head, tail } => {
                // Estimate ~150 bytes per line average
                (head + tail).saturating_mul(150)
            }
        }
    }

    /// Get the character budget for this policy
    pub fn char_budget(&self) -> usize {
        match self {
            TruncationPolicy::Chars(c) => *c,
            TruncationPolicy::Tokens(t) => t.saturating_mul(APPROX_BYTES_PER_TOKEN),
            TruncationPolicy::Lines { head, tail } => (head + tail).saturating_mul(100),
        }
    }
}

/// Information about the original content before truncation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncationInfo {
    /// Original character count
    pub chars: usize,
    /// Original line count
    pub lines: usize,
    /// Estimated token count
    pub estimated_tokens: usize,
    /// Number of chars/lines/tokens removed
    pub removed_count: usize,
    /// Unit of removed_count ("chars", "lines", "tokens")
    pub removed_unit: String,
}

/// Result of a truncation operation
#[derive(Debug, Clone)]
pub struct TruncatedResult {
    /// The truncated text
    pub text: String,
    /// Whether truncation occurred
    pub was_truncated: bool,
    /// Information about the original content (if truncated)
    pub info: Option<TruncationInfo>,
}

impl TruncatedResult {
    /// Create a non-truncated result
    fn unchanged(text: String) -> Self {
        Self {
            text,
            was_truncated: false,
            info: None,
        }
    }

    /// Create a truncated result with info
    fn truncated(text: String, info: TruncationInfo) -> Self {
        Self {
            text,
            was_truncated: true,
            info: Some(info),
        }
    }
}

/// Truncate content using the specified policy, preserving both head and tail
pub fn truncate_middle(s: &str, policy: TruncationPolicy) -> TruncatedResult {
    if s.is_empty() {
        return TruncatedResult::unchanged(String::new());
    }

    match policy {
        TruncationPolicy::Chars(max) => truncate_middle_chars(s, max),
        TruncationPolicy::Tokens(max) => truncate_middle_tokens(s, max),
        TruncationPolicy::Lines { head, tail } => truncate_middle_lines(s, head, tail),
    }
}

/// Truncate by character count, keeping head and tail
fn truncate_middle_chars(s: &str, max_chars: usize) -> TruncatedResult {
    let char_count = s.chars().count();

    if char_count <= max_chars {
        return TruncatedResult::unchanged(s.to_string());
    }

    if max_chars == 0 {
        return TruncatedResult::truncated(
            format!("[{char_count} chars truncated]"),
            TruncationInfo {
                chars: char_count,
                lines: s.lines().count(),
                estimated_tokens: char_count / APPROX_BYTES_PER_TOKEN,
                removed_count: char_count,
                removed_unit: "chars".to_string(),
            },
        );
    }

    // Split budget: 40% head, 60% tail (tail often more important)
    let head_budget = max_chars * 2 / 5;
    let tail_budget = max_chars - head_budget;

    let chars: Vec<char> = s.chars().collect();
    let head: String = chars.iter().take(head_budget).collect();
    let tail: String = chars
        .iter()
        .rev()
        .take(tail_budget)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let removed = char_count.saturating_sub(max_chars);
    let marker = format!("\n\n... [{removed} chars truncated] ...\n\n");

    TruncatedResult::truncated(
        format!("{head}{marker}{tail}"),
        TruncationInfo {
            chars: char_count,
            lines: s.lines().count(),
            estimated_tokens: char_count / APPROX_BYTES_PER_TOKEN,
            removed_count: removed,
            removed_unit: "chars".to_string(),
        },
    )
}

/// Truncate by token count (estimated), keeping head and tail
fn truncate_middle_tokens(s: &str, max_tokens: usize) -> TruncatedResult {
    // Convert to approximate char budget
    let max_chars = max_tokens.saturating_mul(APPROX_BYTES_PER_TOKEN);
    let result = truncate_middle_chars(s, max_chars);

    // Adjust the info to show tokens instead of chars
    if let Some(mut info) = result.info {
        info.removed_count /= APPROX_BYTES_PER_TOKEN;
        info.removed_unit = "tokens".to_string();
        TruncatedResult::truncated(
            result.text.replace("chars truncated", "tokens truncated"),
            info,
        )
    } else {
        result
    }
}

/// Truncate by line count, keeping head and tail lines
fn truncate_middle_lines(s: &str, head_lines: usize, tail_lines: usize) -> TruncatedResult {
    let lines: Vec<&str> = s.lines().collect();
    let total = lines.len();
    let char_count = s.chars().count();

    if total <= head_lines + tail_lines {
        return TruncatedResult::unchanged(s.to_string());
    }

    if head_lines == 0 && tail_lines == 0 {
        return TruncatedResult::truncated(
            format!("[{total} lines truncated]"),
            TruncationInfo {
                chars: char_count,
                lines: total,
                estimated_tokens: char_count / APPROX_BYTES_PER_TOKEN,
                removed_count: total,
                removed_unit: "lines".to_string(),
            },
        );
    }

    // Head-only truncation (tail_lines == 0)
    if tail_lines == 0 {
        let head_part = lines[..head_lines].join("\n");
        let removed = total.saturating_sub(head_lines);
        let marker = format!(
            "\n\n...{removed} lines truncated...\n\nOutput exceeded {head_lines} lines. Use Grep to search or Read with offset/limit to view specific sections."
        );
        return TruncatedResult::truncated(
            format!("{head_part}{marker}"),
            TruncationInfo {
                chars: char_count,
                lines: total,
                estimated_tokens: char_count / APPROX_BYTES_PER_TOKEN,
                removed_count: removed,
                removed_unit: "lines".to_string(),
            },
        );
    }

    // Head + tail truncation
    let head_part = lines[..head_lines].join("\n");
    let tail_part = lines[total.saturating_sub(tail_lines)..].join("\n");
    let removed = total.saturating_sub(head_lines).saturating_sub(tail_lines);

    let marker = format!("\n\n... [{removed} lines truncated, total {total} lines] ...\n\n");

    TruncatedResult::truncated(
        format!("{head_part}{marker}{tail_part}"),
        TruncationInfo {
            chars: char_count,
            lines: total,
            estimated_tokens: char_count / APPROX_BYTES_PER_TOKEN,
            removed_count: removed,
            removed_unit: "lines".to_string(),
        },
    )
}

/// Format shell/command execution output with metadata
#[derive(Debug, Clone)]
pub struct ExecOutputFormatter {
    pub exit_code: Option<i32>,
    pub duration_secs: Option<f32>,
    pub output: String,
    pub was_truncated: bool,
    pub original_lines: Option<usize>,
}

impl ExecOutputFormatter {
    /// Create from raw output with optional truncation
    pub fn new(output: &str, exit_code: Option<i32>, duration_secs: Option<f32>) -> Self {
        Self {
            exit_code,
            duration_secs,
            output: output.to_string(),
            was_truncated: false,
            original_lines: None,
        }
    }

    /// Apply truncation policy
    pub fn with_truncation(mut self, policy: TruncationPolicy) -> Self {
        let result = truncate_middle(&self.output, policy);
        self.was_truncated = result.was_truncated;
        if let Some(info) = &result.info {
            self.original_lines = Some(info.lines);
        }
        self.output = result.text;
        self
    }

    /// Format for model consumption (human-readable)
    pub fn format_for_model(&self) -> String {
        let mut sections = Vec::new();

        if let Some(code) = self.exit_code {
            sections.push(format!("Exit code: {code}"));
        }

        if let Some(secs) = self.duration_secs {
            sections.push(format!("Wall time: {secs:.1} seconds"));
        }

        if self.was_truncated {
            if let Some(lines) = self.original_lines {
                sections.push(format!("Total output lines: {lines}"));
            }
        }

        if !self.output.is_empty() {
            sections.push("Output:".to_string());
            sections.push(self.output.clone());
        } else {
            sections.push("Output: (empty)".to_string());
        }

        sections.join("\n")
    }

    /// Format as structured JSON (for structured output modes)
    pub fn format_structured(&self) -> String {
        let metadata = serde_json::json!({
            "exit_code": self.exit_code,
            "duration_seconds": self.duration_secs,
            "truncated": self.was_truncated,
            "original_lines": self.original_lines,
        });

        serde_json::json!({
            "output": self.output,
            "metadata": metadata,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_chars_under_limit() {
        let result = truncate_middle("hello world", TruncationPolicy::Chars(100));
        assert!(!result.was_truncated);
        assert_eq!(result.text, "hello world");
    }

    #[test]
    fn test_truncate_chars_over_limit() {
        let text = "a".repeat(100);
        let result = truncate_middle(&text, TruncationPolicy::Chars(20));
        assert!(result.was_truncated);
        assert!(result.text.contains("chars truncated"));
        // Head + tail should be present
        assert!(result.text.starts_with("aaaaaaaa")); // head
        assert!(result.text.ends_with("aaaaaaaaaaaa")); // tail
    }

    #[test]
    fn test_truncate_lines_under_limit() {
        let text = "line1\nline2\nline3";
        let result = truncate_middle(text, TruncationPolicy::Lines { head: 5, tail: 5 });
        assert!(!result.was_truncated);
        assert_eq!(result.text, text);
    }

    #[test]
    fn test_truncate_lines_over_limit() {
        let lines: Vec<String> = (1..=100).map(|i| format!("line {i}")).collect();
        let text = lines.join("\n");
        let result = truncate_middle(&text, TruncationPolicy::Lines { head: 5, tail: 10 });

        assert!(result.was_truncated);
        assert!(result.text.contains("lines truncated"));
        assert!(result.text.contains("line 1")); // head
        assert!(result.text.contains("line 100")); // tail
        assert!(!result.text.contains("line 50")); // middle removed
    }

    #[test]
    fn test_truncate_utf8() {
        let text = "你好世界".repeat(100);
        let result = truncate_middle(&text, TruncationPolicy::Chars(20));
        assert!(result.was_truncated);
        // Should not panic on UTF-8 boundaries
        assert!(result.text.chars().count() < text.chars().count());
    }

    #[test]
    fn test_exec_formatter() {
        let output = "Building...\nCompiling...\nDone!";
        let formatter = ExecOutputFormatter::new(output, Some(0), Some(1.5));
        let formatted = formatter.format_for_model();

        assert!(formatted.contains("Exit code: 0"));
        assert!(formatted.contains("Wall time: 1.5 seconds"));
        assert!(formatted.contains("Done!"));
    }

    #[test]
    fn test_exec_formatter_with_truncation() {
        let lines: Vec<String> = (1..=1000).map(|i| format!("log line {i}")).collect();
        let output = lines.join("\n");

        let formatter = ExecOutputFormatter::new(&output, Some(0), Some(5.0))
            .with_truncation(TruncationPolicy::Lines { head: 10, tail: 20 });

        let formatted = formatter.format_for_model();
        assert!(formatted.contains("Total output lines: 1000"));
        assert!(formatted.contains("log line 1")); // head preserved
        assert!(formatted.contains("log line 1000")); // tail preserved
    }

    #[test]
    fn test_shell_output_policy() {
        let policy = TruncationPolicy::shell_output();
        match policy {
            TruncationPolicy::Lines { head, tail } => {
                assert_eq!(head, 2000);
                assert_eq!(tail, 0);
            }
            _ => panic!("Expected Lines policy"),
        }
    }
}
