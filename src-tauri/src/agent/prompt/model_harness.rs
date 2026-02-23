//! Model-specific prompt harness.
//!
//! Maps a `model_id` string to a stable model-family identifier and the
//! corresponding prompt-profile key under `prompts/models/*.md`.
//! Detection happens once via `ModelFamily::detect()`, then both `name()` and
//! `profile_key()` are zero-cost lookups on the resulting enum.

/// Recognized model families.
///
/// Covers Chinese models (Qwen, GLM, MiniMax, Doubao, Moonshot/Kimi) and
/// international models (Mistral/Devstral, Grok/xAI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFamily {
    OpenAICodex,
    OpenAIOSeries,
    OpenAIGPT,
    AnthropicClaude,
    GoogleGemini,
    DeepSeek,
    Qwen,
    GLM,
    MiniMax,
    Doubao,
    Moonshot,
    Mistral,
    Grok,
    Generic,
}

impl ModelFamily {
    /// Classify a model id into a family.  Case-insensitive.
    ///
    /// Order matters: more specific patterns must come before generic ones.
    /// E.g. "codex" before "gpt", "devstral" before "mistral".
    pub fn detect(model_id: &str) -> Self {
        let id = model_id.to_lowercase();

        // OpenAI Codex (must be before GPT — "gpt-5.1-codex" contains "gpt")
        if id.contains("codex") {
            return Self::OpenAICodex;
        }

        // OpenAI o-series reasoning models
        if id.starts_with("o1")
            || id.starts_with("o3")
            || id.starts_with("o4")
            || id.contains("-o1")
            || id.contains("-o3")
            || id.contains("-o4")
        {
            return Self::OpenAIOSeries;
        }

        // DeepSeek
        if id.contains("deepseek") {
            return Self::DeepSeek;
        }

        // Qwen / Alibaba (includes qwen-code, qwen-plus, qwen-max, etc.)
        if id.contains("qwen") {
            return Self::Qwen;
        }

        // GLM / Zhipu AI
        if id.contains("glm")
            || id.contains("z-ai/")
            || id.contains("zai-org/")
            || id.contains("chatglm")
        {
            return Self::GLM;
        }

        // MiniMax
        if id.contains("minimax") || id.contains("abab") {
            return Self::MiniMax;
        }

        // Doubao / ByteDance
        if id.contains("doubao") {
            return Self::Doubao;
        }

        // Moonshot / Kimi
        if id.contains("moonshot") || id.contains("kimi") {
            return Self::Moonshot;
        }

        // Grok / xAI (must be before generic)
        if id.contains("grok") {
            return Self::Grok;
        }

        // OpenAI GPT
        if id.contains("gpt") {
            return Self::OpenAIGPT;
        }

        // Anthropic Claude
        if id.contains("claude")
            || id.contains("sonnet")
            || id.contains("opus")
            || id.contains("haiku")
        {
            return Self::AnthropicClaude;
        }

        // Google Gemini
        if id.contains("gemini") {
            return Self::GoogleGemini;
        }

        // Mistral / Devstral
        if id.contains("mistral")
            || id.contains("devstral")
            || id.contains("codestral")
            || id.contains("pixtral")
        {
            return Self::Mistral;
        }

        Self::Generic
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
            Self::Qwen => "qwen",
            Self::GLM => "glm",
            Self::MiniMax => "minimax",
            Self::Doubao => "doubao",
            Self::Moonshot => "moonshot",
            Self::Mistral => "mistral",
            Self::Grok => "grok",
            Self::Generic => "generic",
        }
    }

    /// Prompt profile key under `prompts/models/{profile_key}.md`.
    ///
    /// Multiple families can share the same profile:
    /// - Chinese models (Qwen, GLM, MiniMax, Doubao, Moonshot) → "deepseek"
    ///   (concise style, similar behavioral characteristics)
    /// - Grok → "openai-gpt" (similar autonomous style)
    /// - Mistral → "generic" (neutral style)
    pub fn profile_key(self) -> &'static str {
        match self {
            Self::OpenAICodex => "openai-codex",
            Self::OpenAIOSeries => "openai-o-series",
            Self::OpenAIGPT => "openai-gpt",
            Self::AnthropicClaude => "anthropic-claude",
            Self::GoogleGemini => "google-gemini",
            Self::DeepSeek => "deepseek",
            Self::Qwen => "deepseek",
            Self::GLM => "deepseek",
            Self::MiniMax => "deepseek",
            Self::Doubao => "deepseek",
            Self::Moonshot => "deepseek",
            Self::Mistral => "generic",
            Self::Grok => "openai-gpt",
            Self::Generic => "generic",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_families() {
        // OpenAI
        assert_eq!(
            ModelFamily::detect("gpt-5.1-codex-max"),
            ModelFamily::OpenAICodex
        );
        assert_eq!(ModelFamily::detect("o3-mini"), ModelFamily::OpenAIOSeries);
        assert_eq!(
            ModelFamily::detect("o4-preview"),
            ModelFamily::OpenAIOSeries
        );
        assert_eq!(ModelFamily::detect("gpt-4o"), ModelFamily::OpenAIGPT);
        assert_eq!(ModelFamily::detect("gpt-5"), ModelFamily::OpenAIGPT);

        // Anthropic
        assert_eq!(
            ModelFamily::detect("claude-4-opus"),
            ModelFamily::AnthropicClaude
        );
        assert_eq!(
            ModelFamily::detect("claude-sonnet-4-6"),
            ModelFamily::AnthropicClaude
        );
        assert_eq!(ModelFamily::detect("sonnet"), ModelFamily::AnthropicClaude);
        assert_eq!(ModelFamily::detect("opus"), ModelFamily::AnthropicClaude);
        assert_eq!(ModelFamily::detect("haiku"), ModelFamily::AnthropicClaude);

        // Google
        assert_eq!(
            ModelFamily::detect("gemini-2.0-flash"),
            ModelFamily::GoogleGemini
        );
        assert_eq!(
            ModelFamily::detect("gemini-2.5-pro"),
            ModelFamily::GoogleGemini
        );

        // DeepSeek
        assert_eq!(ModelFamily::detect("deepseek-r1"), ModelFamily::DeepSeek);
        assert_eq!(ModelFamily::detect("deepseek-chat"), ModelFamily::DeepSeek);

        // Chinese models
        assert_eq!(ModelFamily::detect("qwen-max"), ModelFamily::Qwen);
        assert_eq!(ModelFamily::detect("qwen-code-plus"), ModelFamily::Qwen);
        assert_eq!(ModelFamily::detect("glm-4.7"), ModelFamily::GLM);
        assert_eq!(ModelFamily::detect("glm-5"), ModelFamily::GLM);
        assert_eq!(ModelFamily::detect("z-ai/glm-4"), ModelFamily::GLM);
        assert_eq!(ModelFamily::detect("chatglm-turbo"), ModelFamily::GLM);
        assert_eq!(ModelFamily::detect("minimax-m2"), ModelFamily::MiniMax);
        assert_eq!(ModelFamily::detect("abab-7"), ModelFamily::MiniMax);
        assert_eq!(ModelFamily::detect("doubao-pro"), ModelFamily::Doubao);
        assert_eq!(ModelFamily::detect("moonshot-v1"), ModelFamily::Moonshot);
        assert_eq!(ModelFamily::detect("kimi-k2"), ModelFamily::Moonshot);

        // International
        assert_eq!(ModelFamily::detect("mistral-large"), ModelFamily::Mistral);
        assert_eq!(ModelFamily::detect("devstral-small"), ModelFamily::Mistral);
        assert_eq!(
            ModelFamily::detect("codestral-latest"),
            ModelFamily::Mistral
        );
        assert_eq!(ModelFamily::detect("grok-4"), ModelFamily::Grok);

        // Fallback
        assert_eq!(
            ModelFamily::detect("some-random-model"),
            ModelFamily::Generic
        );
    }

    #[test]
    fn profile_key_mapping() {
        // Direct profiles
        assert_eq!(ModelFamily::OpenAICodex.profile_key(), "openai-codex");
        assert_eq!(ModelFamily::OpenAIOSeries.profile_key(), "openai-o-series");
        assert_eq!(ModelFamily::OpenAIGPT.profile_key(), "openai-gpt");
        assert_eq!(
            ModelFamily::AnthropicClaude.profile_key(),
            "anthropic-claude"
        );
        assert_eq!(ModelFamily::GoogleGemini.profile_key(), "google-gemini");
        assert_eq!(ModelFamily::DeepSeek.profile_key(), "deepseek");

        // Shared profiles — Chinese models → deepseek
        assert_eq!(ModelFamily::Qwen.profile_key(), "deepseek");
        assert_eq!(ModelFamily::GLM.profile_key(), "deepseek");
        assert_eq!(ModelFamily::MiniMax.profile_key(), "deepseek");
        assert_eq!(ModelFamily::Doubao.profile_key(), "deepseek");
        assert_eq!(ModelFamily::Moonshot.profile_key(), "deepseek");

        // Shared profiles — international
        assert_eq!(ModelFamily::Grok.profile_key(), "openai-gpt");
        assert_eq!(ModelFamily::Mistral.profile_key(), "generic");
        assert_eq!(ModelFamily::Generic.profile_key(), "generic");
    }
}
