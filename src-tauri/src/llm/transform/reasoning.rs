//! Reasoning/Thinking transform functions for multi-provider support.
//!
//! This module handles the conversion of reasoning content between different
//! provider formats for multi-turn conversations.
//!
//! ## Provider-specific handling:
//!
//! - **Anthropic**: Uses `thinking` blocks with `signature` for verification
//! - **OpenAI**: Uses `item_reference` for stored content, or full reasoning objects
//! - **DeepSeek**: Uses `reasoning_content` field in messages

use crate::agent::types::ReasoningPart;
use crate::llm::anthropic_types::{ContentBlock, MessageParam};
use serde_json::{json, Value};

/// Configuration for reasoning transform
#[derive(Debug, Clone)]
pub struct ReasoningTransformConfig {
    /// Provider identifier
    pub provider: String,
    /// Whether to use item_reference for OpenAI (store=true mode)
    pub use_item_reference: bool,
}

impl Default for ReasoningTransformConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            use_item_reference: true,
        }
    }
}

/// Transform reasoning parts for inclusion in API requests.
///
/// For OpenAI with store=true: converts to item_reference
/// For OpenAI with store=false: converts to full reasoning object
/// For Anthropic: keeps as thinking block with signature
pub fn transform_reasoning_for_request(
    parts: &[ReasoningPart],
    config: &ReasoningTransformConfig,
) -> Vec<Value> {
    match config.provider.as_str() {
        "openai" => transform_for_openai(parts, config.use_item_reference),
        "anthropic" => transform_for_anthropic(parts),
        _ => transform_for_anthropic(parts), // Default to Anthropic format
    }
}

/// Transform reasoning for OpenAI Responses API.
///
/// When use_item_reference=true (store mode):
///   Returns item_reference objects that reference stored reasoning
///
/// When use_item_reference=false:
///   Returns full reasoning objects with encrypted_content and summary
fn transform_for_openai(parts: &[ReasoningPart], use_item_reference: bool) -> Vec<Value> {
    if use_item_reference {
        // Use item_reference to avoid re-sending content
        parts
            .iter()
            .filter_map(|part| {
                part.openai_item_id().map(|id| {
                    json!({
                        "type": "item_reference",
                        "id": id
                    })
                })
            })
            .collect()
    } else {
        // Send full reasoning objects
        parts
            .iter()
            .map(|part| {
                let mut reasoning = json!({
                    "type": "reasoning",
                    "summary": [{
                        "type": "summary_text",
                        "text": part.text
                    }]
                });

                if let Some(id) = part.openai_item_id() {
                    reasoning["id"] = json!(id);
                }

                if let Some(ec) = part
                    .metadata
                    .as_ref()
                    .and_then(|m| m.encrypted_content.as_ref())
                {
                    reasoning["encrypted_content"] = json!(ec);
                }

                reasoning
            })
            .collect()
    }
}

/// Transform reasoning for Anthropic Extended Thinking.
///
/// Returns thinking blocks with signature for verification.
fn transform_for_anthropic(parts: &[ReasoningPart]) -> Vec<Value> {
    parts
        .iter()
        .filter(|p| !p.text.is_empty()) // Anthropic rejects empty thinking
        .map(|part| {
            let mut thinking = json!({
                "type": "thinking",
                "thinking": part.text
            });

            // Include signature if available
            if let Some(sig) = part.anthropic_signature() {
                thinking["signature"] = json!(sig);
            }

            thinking
        })
        .collect()
}

/// Extract reasoning parts from assistant message content blocks.
pub fn extract_reasoning_from_content(content: &[ContentBlock]) -> Vec<ReasoningPart> {
    content
        .iter()
        .filter_map(|block| {
            if let ContentBlock::Thinking {
                thinking,
                signature,
                ..
            } = block
            {
                let mut part = ReasoningPart::new(
                    uuid::Uuid::new_v4().to_string(),
                    0,             // session_id will be set by caller
                    String::new(), // message_id will be set by caller
                );
                part.text = thinking.clone();
                if signature.is_some() {
                    part.set_anthropic_metadata(signature.clone());
                }
                Some(part)
            } else {
                None
            }
        })
        .collect()
}

/// Check if a message contains reasoning content.
pub fn has_reasoning_content(message: &MessageParam) -> bool {
    match &message.content {
        crate::llm::anthropic_types::MessageContent::Text(_) => false,
        crate::llm::anthropic_types::MessageContent::Blocks(blocks) => blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::Thinking { .. })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_for_openai_item_reference() {
        let mut part = ReasoningPart::new("r1".to_string(), 1, "m1".to_string());
        part.text = "Thinking about the problem...".to_string();
        part.set_openai_metadata("item_123".to_string(), Some("encrypted_xyz".to_string()));

        let config = ReasoningTransformConfig {
            provider: "openai".to_string(),
            use_item_reference: true,
        };

        let result = transform_reasoning_for_request(&[part], &config);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "item_reference");
        assert_eq!(result[0]["id"], "item_123");
    }

    #[test]
    fn test_transform_for_openai_full() {
        let mut part = ReasoningPart::new("r1".to_string(), 1, "m1".to_string());
        part.text = "Thinking about the problem...".to_string();
        part.set_openai_metadata("item_123".to_string(), Some("encrypted_xyz".to_string()));

        let config = ReasoningTransformConfig {
            provider: "openai".to_string(),
            use_item_reference: false,
        };

        let result = transform_reasoning_for_request(&[part], &config);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "reasoning");
        assert_eq!(result[0]["encrypted_content"], "encrypted_xyz");
    }

    #[test]
    fn test_transform_for_anthropic() {
        let mut part = ReasoningPart::new("r1".to_string(), 1, "m1".to_string());
        part.text = "Thinking deeply...".to_string();
        part.set_anthropic_metadata(Some("sig_abc".to_string()));

        let config = ReasoningTransformConfig {
            provider: "anthropic".to_string(),
            use_item_reference: false,
        };

        let result = transform_reasoning_for_request(&[part], &config);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "thinking");
        assert_eq!(result[0]["thinking"], "Thinking deeply...");
        assert_eq!(result[0]["signature"], "sig_abc");
    }

    #[test]
    fn test_empty_reasoning_filtered_for_anthropic() {
        let part = ReasoningPart::new("r1".to_string(), 1, "m1".to_string());
        // Empty text

        let config = ReasoningTransformConfig {
            provider: "anthropic".to_string(),
            use_item_reference: false,
        };

        let result = transform_reasoning_for_request(&[part], &config);
        assert!(result.is_empty()); // Empty reasoning should be filtered
    }
}
