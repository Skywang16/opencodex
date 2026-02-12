//! Preset model data definitions
//!
//! This module contains preset model lists for all LLM providers.
//! Model capabilities (reasoning, tool_call, attachment) are defined here.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// Model capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    /// Supports reasoning/extended thinking
    #[serde(default)]
    pub reasoning: bool,
    /// Supports tool/function calling
    #[serde(default)]
    pub tool_call: bool,
    /// Supports image/file attachments
    #[serde(default)]
    pub attachment: bool,
}

impl ModelCapabilities {
    pub fn new(reasoning: bool, tool_call: bool, attachment: bool) -> Self {
        Self {
            reasoning,
            tool_call,
            attachment,
        }
    }

    /// Full capabilities (reasoning + tool_call + attachment)
    pub fn full() -> Self {
        Self::new(true, true, true)
    }

    /// Standard capabilities (tool_call + attachment, no reasoning)
    pub fn standard() -> Self {
        Self::new(false, true, true)
    }

    /// Basic capabilities (tool_call only)
    pub fn basic() -> Self {
        Self::new(false, true, false)
    }
}

/// Preset model information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetModel {
    /// Model ID (for API calls)
    pub id: String,
    /// Model display name
    pub name: String,
    /// Maximum output tokens (None means unlimited or dynamically determined by the model)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Context window size (tokens)
    pub context_window: u32,
    /// Model description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Model capabilities
    #[serde(default)]
    pub capabilities: ModelCapabilities,
}

impl PresetModel {
    fn new(
        id: &str,
        name: &str,
        max_tokens: Option<u32>,
        context_window: u32,
        description: Option<&str>,
        capabilities: ModelCapabilities,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            max_tokens,
            context_window,
            description: description.map(|s| s.to_string()),
            capabilities,
        }
    }

    /// Create a reasoning model (with extended thinking)
    fn reasoning(id: &str, name: &str, max_tokens: Option<u32>, context_window: u32) -> Self {
        Self::new(
            id,
            name,
            max_tokens,
            context_window,
            None,
            ModelCapabilities::full(),
        )
    }

    /// Create a standard model (no reasoning)
    fn standard(id: &str, name: &str, max_tokens: Option<u32>, context_window: u32) -> Self {
        Self::new(
            id,
            name,
            max_tokens,
            context_window,
            None,
            ModelCapabilities::standard(),
        )
    }
}

// =============================================================================
// Anthropic Models
// =============================================================================
// https://docs.anthropic.com/en/docs/about-claude/models
// Claude 4+ models support Extended Thinking (reasoning)
pub static ANTHROPIC_MODELS: Lazy<Vec<PresetModel>> = Lazy::new(|| {
    vec![
        // Claude 4.5 series - supports Extended Thinking
        PresetModel::reasoning(
            "claude-sonnet-4-5-20250929",
            "Claude Sonnet 4.5",
            Some(64000),
            200_000,
        ),
        // Claude 4 series - supports Extended Thinking
        PresetModel::reasoning(
            "claude-sonnet-4-20250514",
            "Claude Sonnet 4",
            Some(64000),
            200_000,
        ),
        PresetModel::reasoning(
            "claude-opus-4-1-20250805",
            "Claude Opus 4.1",
            Some(32000),
            200_000,
        ),
        PresetModel::reasoning(
            "claude-opus-4-20250514",
            "Claude Opus 4",
            Some(32000),
            200_000,
        ),
        // Claude 3.7 series - supports Extended Thinking
        PresetModel::reasoning(
            "claude-3-7-sonnet-20250219",
            "Claude 3.7 Sonnet",
            Some(64000),
            200_000,
        ),
        // Claude 3.5 series - standard (no Extended Thinking)
        PresetModel::standard(
            "claude-3-5-sonnet-20241022",
            "Claude 3.5 Sonnet",
            Some(8192),
            200_000,
        ),
        PresetModel::standard(
            "claude-3-5-haiku-20241022",
            "Claude 3.5 Haiku",
            Some(8192),
            200_000,
        ),
    ]
});

// =============================================================================
// OpenAI Models
// =============================================================================
// https://openai.com/api/pricing/
// GPT-5 and o-series models support Reasoning (via Responses API)
pub static OPENAI_MODELS: Lazy<Vec<PresetModel>> = Lazy::new(|| {
    vec![
        // GPT-5 series - supports Reasoning
        PresetModel::reasoning("gpt-5", "GPT-5", Some(128000), 400_000),
        PresetModel::reasoning(
            "gpt-5-2025-08-07",
            "GPT-5 (2025-08-07)",
            Some(128000),
            400_000,
        ),
        PresetModel::reasoning("gpt-5-mini", "GPT-5 Mini", Some(128000), 400_000),
        PresetModel::reasoning(
            "gpt-5-mini-2025-08-07",
            "GPT-5 Mini (2025-08-07)",
            Some(128000),
            400_000,
        ),
        PresetModel::reasoning("gpt-5-nano", "GPT-5 Nano", Some(128000), 400_000),
        PresetModel::reasoning(
            "gpt-5-nano-2025-08-07",
            "GPT-5 Nano (2025-08-07)",
            Some(128000),
            400_000,
        ),
        // o-series - reasoning models
        PresetModel::reasoning("o3", "o3", Some(100_000), 200_000),
        PresetModel::reasoning("o3-mini", "o3 Mini", Some(100_000), 200_000),
        PresetModel::reasoning("o4-mini", "o4 Mini", Some(100_000), 200_000),
        PresetModel::reasoning("o1", "o1", Some(100_000), 200_000),
        PresetModel::reasoning("o1-mini", "o1 Mini", Some(65_536), 128_000),
        // GPT-4.1 series - standard
        PresetModel::standard("gpt-4.1", "GPT-4.1", Some(32_768), 1_047_576),
        PresetModel::standard("gpt-4.1-mini", "GPT-4.1 Mini", Some(32_768), 1_047_576),
        PresetModel::standard("gpt-4.1-nano", "GPT-4.1 Nano", Some(32_768), 1_000_000),
        // GPT-4o series - standard
        PresetModel::standard("gpt-4o", "GPT-4o", Some(16_384), 128_000),
        PresetModel::standard("gpt-4o-mini", "GPT-4o Mini", Some(16_384), 128_000),
    ]
});

// =============================================================================
// OpenAI Compatible Models (suggested list)
// =============================================================================
// These are common OpenAI Compatible models that users can choose or customize
pub static OPENAI_COMPATIBLE_SUGGESTIONS: Lazy<Vec<PresetModel>> = Lazy::new(|| {
    vec![
        // DeepSeek models - reasoning models
        PresetModel::new(
            "deepseek-reasoner",
            "DeepSeek Reasoner",
            Some(65536),
            128_000,
            Some("DeepSeek R1 - supports reasoning"),
            ModelCapabilities::full(),
        ),
        PresetModel::new(
            "deepseek-chat",
            "DeepSeek Chat",
            Some(8192),
            128_000,
            Some("DeepSeek V3 - API URL: https://api.deepseek.com"),
            ModelCapabilities::standard(),
        ),
        // Qwen models
        PresetModel::new(
            "qwen3-235b-a22b",
            "Qwen3 235B",
            Some(16384),
            131_072,
            Some("Qwen3 - supports reasoning"),
            ModelCapabilities::full(),
        ),
        PresetModel::new(
            "qwen-plus",
            "Qwen Plus",
            Some(32768),
            1_000_000,
            Some("Alibaba Qwen - API URL: https://dashscope.aliyuncs.com/compatible-mode/v1"),
            ModelCapabilities::standard(),
        ),
        // Ollama local models
        PresetModel::new(
            "llama3.3",
            "Llama 3.3 (Ollama)",
            Some(8192),
            128_000,
            Some("Ollama - URL: http://localhost:11434/v1"),
            ModelCapabilities::basic(),
        ),
        PresetModel::new(
            "qwen2.5-coder",
            "Qwen 2.5 Coder (Ollama)",
            Some(8192),
            128_000,
            Some("Ollama - for coding tasks"),
            ModelCapabilities::basic(),
        ),
        // OpenRouter models
        PresetModel::new(
            "openrouter/auto",
            "OpenRouter Auto",
            None,
            200_000,
            Some("OpenRouter - auto selects best model"),
            ModelCapabilities::standard(),
        ),
    ]
});

// =============================================================================
// Google Gemini Models
// =============================================================================
// https://ai.google.dev/gemini-api/docs/models/gemini
// Gemini 2.5+ and thinking models support reasoning
pub static GEMINI_MODELS: Lazy<Vec<PresetModel>> = Lazy::new(|| {
    vec![
        // Gemini 3 series - supports reasoning
        PresetModel::reasoning(
            "gemini-3-pro-preview",
            "Gemini 3 Pro Preview",
            Some(64_000),
            1_000_000,
        ),
        PresetModel::reasoning(
            "gemini-3-flash-preview",
            "Gemini 3 Flash Preview",
            Some(65_536),
            1_048_576,
        ),
        // Gemini 2.5 series - supports reasoning
        PresetModel::reasoning("gemini-2.5-pro", "Gemini 2.5 Pro", Some(65_536), 1_048_576),
        PresetModel::reasoning(
            "gemini-2.5-flash",
            "Gemini 2.5 Flash",
            Some(65_536),
            1_048_576,
        ),
        PresetModel::new(
            "gemini-2.5-flash-lite-preview",
            "Gemini 2.5 Flash Lite Preview",
            Some(65_536),
            1_000_000,
            Some("Preview - may not be available in all regions"),
            ModelCapabilities::standard(),
        ),
        // Gemini 2.0 series
        PresetModel::standard(
            "gemini-2.0-flash",
            "Gemini 2.0 Flash",
            Some(8192),
            1_048_576,
        ),
        PresetModel::new(
            "gemini-2.0-flash-thinking-exp",
            "Gemini 2.0 Flash Thinking (Exp)",
            Some(65_536),
            1_048_576,
            Some("Experimental thinking model"),
            ModelCapabilities::full(),
        ),
        // Gemini 1.5 series - standard
        PresetModel::standard(
            "gemini-1.5-flash",
            "Gemini 1.5 Flash",
            Some(8192),
            1_048_576,
        ),
        PresetModel::standard("gemini-1.5-pro", "Gemini 1.5 Pro", Some(8192), 2_097_152),
    ]
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_models_not_empty() {
        assert!(
            !ANTHROPIC_MODELS.is_empty(),
            "Anthropic models should not be empty"
        );
        assert!(
            ANTHROPIC_MODELS.iter().all(|m| !m.id.is_empty()),
            "All models should have valid IDs"
        );
    }

    #[test]
    fn test_anthropic_reasoning_models() {
        // Claude 4+ should support reasoning
        let claude_4 = ANTHROPIC_MODELS
            .iter()
            .find(|m| m.id.contains("claude-sonnet-4"))
            .expect("Should have Claude Sonnet 4");
        assert!(
            claude_4.capabilities.reasoning,
            "Claude 4 should support reasoning"
        );

        // Claude 3.5 should not support reasoning
        let claude_35 = ANTHROPIC_MODELS
            .iter()
            .find(|m| m.id.contains("claude-3-5-sonnet"))
            .expect("Should have Claude 3.5 Sonnet");
        assert!(
            !claude_35.capabilities.reasoning,
            "Claude 3.5 should not support reasoning"
        );
    }

    #[test]
    fn test_openai_models_not_empty() {
        assert!(
            !OPENAI_MODELS.is_empty(),
            "OpenAI models should not be empty"
        );
        assert!(
            OPENAI_MODELS.iter().all(|m| !m.id.is_empty()),
            "All models should have valid IDs"
        );
    }

    #[test]
    fn test_openai_reasoning_models() {
        // GPT-5 should support reasoning
        let gpt5 = OPENAI_MODELS
            .iter()
            .find(|m| m.id == "gpt-5")
            .expect("Should have GPT-5");
        assert!(
            gpt5.capabilities.reasoning,
            "GPT-5 should support reasoning"
        );

        // o3 should support reasoning
        let o3 = OPENAI_MODELS
            .iter()
            .find(|m| m.id == "o3")
            .expect("Should have o3");
        assert!(o3.capabilities.reasoning, "o3 should support reasoning");

        // GPT-4o should not support reasoning
        let gpt4o = OPENAI_MODELS
            .iter()
            .find(|m| m.id == "gpt-4o")
            .expect("Should have GPT-4o");
        assert!(
            !gpt4o.capabilities.reasoning,
            "GPT-4o should not support reasoning"
        );
    }

    #[test]
    fn test_gemini_models_not_empty() {
        assert!(
            !GEMINI_MODELS.is_empty(),
            "Gemini models should not be empty"
        );
        assert!(
            GEMINI_MODELS.iter().all(|m| !m.id.is_empty()),
            "All models should have valid IDs"
        );
    }

    #[test]
    fn test_openai_compatible_suggestions_not_empty() {
        assert!(
            !OPENAI_COMPATIBLE_SUGGESTIONS.is_empty(),
            "OpenAI Compatible suggestions should not be empty"
        );
        assert!(
            OPENAI_COMPATIBLE_SUGGESTIONS
                .iter()
                .all(|m| !m.id.is_empty()),
            "All models should have valid IDs"
        );
    }

    #[test]
    fn test_preset_model_serialization() {
        let model = PresetModel::reasoning("test-model", "Test Model", Some(4096), 128_000);
        let json = serde_json::to_string(&model).expect("Should serialize to JSON");
        assert!(json.contains("\"id\":\"test-model\""));
        assert!(json.contains("\"contextWindow\":128000")); // camelCase
        assert!(json.contains("\"maxTokens\":4096")); // camelCase
        assert!(json.contains("\"reasoning\":true")); // capabilities
    }

    #[test]
    fn test_model_capabilities() {
        let full = ModelCapabilities::full();
        assert!(full.reasoning);
        assert!(full.tool_call);
        assert!(full.attachment);

        let standard = ModelCapabilities::standard();
        assert!(!standard.reasoning);
        assert!(standard.tool_call);
        assert!(standard.attachment);

        let basic = ModelCapabilities::basic();
        assert!(!basic.reasoning);
        assert!(basic.tool_call);
        assert!(!basic.attachment);
    }
}
