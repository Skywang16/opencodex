use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::llm::{
    error::{LlmProviderError, LlmProviderResult},
    preset_models::PresetModel,
    providers::{AnthropicProvider, GeminiProvider, OpenAIProvider, Provider},
    types::LLMProviderConfig,
};

/// Provider metadata - compile-time constants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMetadata {
    pub provider_type: &'static str,
    pub display_name: &'static str,
    pub default_api_url: &'static str,
    pub preset_models: Vec<PresetModel>,
}

/// Global Provider metadata - lazy initialization, allocated only once
static PROVIDER_METADATA: Lazy<Vec<ProviderMetadata>> = Lazy::new(|| {
    use crate::llm::preset_models::*;

    vec![
        ProviderMetadata {
            provider_type: "anthropic",
            display_name: "Anthropic",
            default_api_url: "https://api.anthropic.com/v1",
            preset_models: ANTHROPIC_MODELS.clone(),
        },
        ProviderMetadata {
            provider_type: "openai",
            display_name: "OpenAI",
            default_api_url: "https://api.openai.com/v1",
            preset_models: OPENAI_MODELS.clone(),
        },
        ProviderMetadata {
            provider_type: "openai_compatible",
            display_name: "OpenAI Compatible",
            default_api_url: "",
            preset_models: vec![],
        },
        ProviderMetadata {
            provider_type: "gemini",
            display_name: "Gemini",
            default_api_url: "https://generativelanguage.googleapis.com/v1beta",
            preset_models: GEMINI_MODELS.clone(),
        },
    ]
});

/// Provider registry
///
/// Eliminates all runtime overhead:
/// - Zero hash lookups (compile-time match)
/// - Zero heap allocations (direct enum construction)
/// - Zero function pointers (static dispatch)
pub struct ProviderRegistry;

impl ProviderRegistry {
    pub fn global() -> &'static ProviderRegistry {
        &ProviderRegistry
    }

    /// Create Provider - compile-time match, zero runtime overhead
    #[inline]
    pub fn create(&self, config: LLMProviderConfig) -> LlmProviderResult<Provider> {
        let provider_type = config.provider_type.as_str();

        match provider_type {
            "openai" | "openai_compatible" => Ok(Provider::OpenAI(OpenAIProvider::new(config))),
            "anthropic" => Ok(Provider::Anthropic(AnthropicProvider::new(config))),
            "gemini" => Ok(Provider::Gemini(GeminiProvider::new(config))),
            _ => Err(LlmProviderError::UnsupportedProvider {
                provider: provider_type.to_string(),
            }),
        }
    }

    /// Get all Provider metadata
    pub fn get_all_providers_metadata(&self) -> &[ProviderMetadata] {
        &PROVIDER_METADATA
    }

    /// Check if specified provider is supported - compile-time match
    #[inline]
    pub fn supports(&self, provider_type: &str) -> bool {
        matches!(
            provider_type,
            "openai" | "openai_compatible" | "anthropic" | "gemini"
        )
    }
}
