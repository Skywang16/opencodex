use std::sync::{Arc, LazyLock};

use tiktoken_rs::{cl100k_base, CoreBPE};
use tracing::warn;

use crate::llm::anthropic_types::{ContentBlock, MessageContent, MessageParam};

static TOKEN_ENCODER: LazyLock<Option<Arc<CoreBPE>>> = LazyLock::new(|| match cl100k_base() {
    Ok(encoder) => Some(encoder.into()),
    Err(err) => {
        warn!("Tokenizer initialization failed, falling back to heuristic token estimation: {err}");
        None
    }
});

#[inline]
fn estimate_tokens(text: &str) -> usize {
    // A pragmatic fallback when tokenizer initialization fails.
    // Rough heuristic: ~4 chars/token for English-like text. This is not exact, but avoids panics.
    (text.chars().count().saturating_add(3)) / 4
}

pub fn count_text_tokens(text: &str) -> usize {
    match TOKEN_ENCODER.as_ref() {
        Some(encoder) => encoder.encode_with_special_tokens(text).len(),
        None => estimate_tokens(text),
    }
}

/// Count tokens in Anthropic native messages
pub fn count_message_param_tokens(message: &MessageParam) -> usize {
    match &message.content {
        MessageContent::Text(text) => count_text_tokens(text),
        MessageContent::Blocks(blocks) => blocks.iter().map(count_block_tokens).sum(),
    }
}

fn count_block_tokens(block: &ContentBlock) -> usize {
    match block {
        ContentBlock::Text { text, .. } => count_text_tokens(text),
        ContentBlock::Image { source, .. } => {
            // Roughly estimate image block overhead based on metadata
            // Don't expand binary data, only estimate based on description field length
            let serialized = serde_json::json!({ "type": "image", "source": source });
            count_text_tokens(&serialized.to_string())
        }
        ContentBlock::ToolUse { id, name, input } => {
            let payload = serde_json::json!({
                "type": "tool_use",
                "id": id,
                "name": name,
                "input": input,
            });
            count_text_tokens(&payload.to_string())
        }
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => {
            let payload = serde_json::json!({
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": content,
                "is_error": is_error,
            });
            count_text_tokens(&payload.to_string())
        }
        ContentBlock::Thinking { thinking, .. } => count_text_tokens(thinking),
    }
}
