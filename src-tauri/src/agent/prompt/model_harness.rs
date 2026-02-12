//! Model-specific prompt harness.
//!
//! Maps a `model_id` string to supplementary system-prompt instructions.
//! Detection happens once via `ModelFamily::detect()`, then both `hints()` and
//! `name()` are zero-cost lookups on the resulting enum.

/// Recognized model families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFamily {
    OpenAICodex,
    OpenAIOSeries,
    OpenAIGPT,
    AnthropicClaude,
    GoogleGemini,
    DeepSeek,
    Generic,
}

impl ModelFamily {
    /// Classify a model id into a family.  Case-insensitive.
    pub fn detect(model_id: &str) -> Self {
        let id = model_id.to_lowercase();
        if id.contains("codex") {
            Self::OpenAICodex
        } else if id.starts_with("o1")
            || id.starts_with("o3")
            || id.starts_with("o4")
            || id.contains("-o1")
            || id.contains("-o3")
            || id.contains("-o4")
        {
            Self::OpenAIOSeries
        } else if id.contains("deepseek") {
            Self::DeepSeek
        } else if id.contains("gpt") {
            Self::OpenAIGPT
        } else if id.contains("claude") {
            Self::AnthropicClaude
        } else if id.contains("gemini") {
            Self::GoogleGemini
        } else {
            Self::Generic
        }
    }

    /// Human-readable family name for logs and cache keys.
    pub fn name(self) -> &'static str {
        match self {
            Self::OpenAICodex => "openai-codex",
            Self::OpenAIOSeries => "openai-o-series",
            Self::OpenAIGPT => "openai-gpt",
            Self::AnthropicClaude => "anthropic-claude",
            Self::GoogleGemini => "google-gemini",
            Self::DeepSeek => "deepseek",
            Self::Generic => "generic",
        }
    }

    /// Model-family-specific prompt hints, or `None` if no special treatment.
    pub fn hints(self) -> Option<&'static str> {
        match self {
            Self::OpenAICodex => Some(OPENAI_CODEX_HINTS),
            Self::OpenAIOSeries => Some(OPENAI_O_SERIES_HINTS),
            Self::DeepSeek => Some(DEEPSEEK_HINTS),
            _ => None,
        }
    }
}

// ── Convenience free functions (thin wrappers) ───────────────────────────

pub fn hints_for_model(model_id: &str) -> Option<&'static str> {
    ModelFamily::detect(model_id).hints()
}

pub fn model_family(model_id: &str) -> &'static str {
    ModelFamily::detect(model_id).name()
}

// ── Hint blocks ──────────────────────────────────────────────────────────

const OPENAI_CODEX_HINTS: &str = "\
## Model-Specific Notes (OpenAI Codex)

- If a tool exists for an action, **always** prefer the tool over shell commands (e.g. `read_file` over `cat`, `grep` over shell `rg`).
- Keep reasoning summaries to 1–2 sentences. Note new discoveries or tactic changes; avoid commenting on your own communication.
- Do not communicate mid-turn intentions. Focus on producing code and tool calls; save explanations for the final message.
- Unless the user explicitly asks for a plan, assume they want code changes. Go ahead and implement rather than proposing in a message.
- Reasoning traces are critical for your performance. They will be preserved and forwarded across turns automatically.";

const OPENAI_O_SERIES_HINTS: &str = "\
## Model-Specific Notes (OpenAI o-series)

- Your reasoning traces are preserved across turns. Use them to maintain continuity in long tasks.
- If a tool exists for an action, prefer the tool over shell commands.
- Be decisive: when the task is clear, implement directly instead of proposing.";

const DEEPSEEK_HINTS: &str = "\
## Model-Specific Notes (DeepSeek)

- Use `reasoning_content` for extended thinking. It will be preserved across turns.
- Prefer structured tool calls over shell commands for file operations.
- When editing files, use the edit tool rather than writing inline Python scripts.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_families() {
        assert_eq!(
            ModelFamily::detect("gpt-5.1-codex-max"),
            ModelFamily::OpenAICodex
        );
        assert_eq!(ModelFamily::detect("o3-mini"), ModelFamily::OpenAIOSeries);
        assert_eq!(
            ModelFamily::detect("claude-4-opus"),
            ModelFamily::AnthropicClaude
        );
        assert_eq!(
            ModelFamily::detect("gemini-2.0-flash"),
            ModelFamily::GoogleGemini
        );
        assert_eq!(ModelFamily::detect("deepseek-r1"), ModelFamily::DeepSeek);
        assert_eq!(
            ModelFamily::detect("some-random-model"),
            ModelFamily::Generic
        );
    }

    #[test]
    fn hints_only_for_special_families() {
        assert!(ModelFamily::OpenAICodex.hints().is_some());
        assert!(ModelFamily::OpenAIOSeries.hints().is_some());
        assert!(ModelFamily::DeepSeek.hints().is_some());
        assert!(ModelFamily::AnthropicClaude.hints().is_none());
        assert!(ModelFamily::GoogleGemini.hints().is_none());
        assert!(ModelFamily::Generic.hints().is_none());
    }
}
