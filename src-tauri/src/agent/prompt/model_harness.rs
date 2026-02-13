//! Model-specific prompt harness.
//!
//! Maps a `model_id` string to a stable model-family identifier and the
//! corresponding prompt-profile key under `prompts/models/*.md`.
//! Detection happens once via `ModelFamily::detect()`, then both `name()` and
//! `profile_key()` are zero-cost lookups on the resulting enum.

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

    /// Prompt profile key under `prompts/models/{profile_key}.md`.
    pub fn profile_key(self) -> &'static str {
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
}

// ── Convenience free functions (thin wrappers) ───────────────────────────

pub fn profile_for_model(model_id: &str) -> &'static str {
    ModelFamily::detect(model_id).profile_key()
}

pub fn model_family(model_id: &str) -> &'static str {
    ModelFamily::detect(model_id).name()
}

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
    fn all_families_have_profile_keys() {
        assert_eq!(ModelFamily::OpenAICodex.profile_key(), "openai-codex");
        assert_eq!(ModelFamily::OpenAIOSeries.profile_key(), "openai-o-series");
        assert_eq!(ModelFamily::OpenAIGPT.profile_key(), "openai-gpt");
        assert_eq!(ModelFamily::AnthropicClaude.profile_key(), "anthropic-claude");
        assert_eq!(ModelFamily::GoogleGemini.profile_key(), "google-gemini");
        assert_eq!(ModelFamily::DeepSeek.profile_key(), "deepseek");
        assert_eq!(ModelFamily::Generic.profile_key(), "generic");
    }
}
