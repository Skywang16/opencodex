//! OpenAI Responses API Input Format Converter
//!
//! Converts Anthropic message types to OpenAI Responses API input format.
//! Reference: opencode-dev convert-to-openai-responses-input.ts
//!
//! ## Key differences from Chat Completions (openai.rs):
//!
//! | Anthropic Type | Chat Completions | Responses API |
//! |----------------|------------------|---------------|
//! | User text      | `{"role":"user","content":"..."}` | `{"role":"user","content":[{"type":"input_text","text":"..."}]}` |
//! | Assistant text | `{"role":"assistant","content":"..."}` | `{"role":"assistant","content":[{"type":"output_text","text":"..."}]}` |
//! | Tool call      | `{"role":"assistant","tool_calls":[...]}` | `{"type":"function_call","call_id":"...","name":"...","arguments":"..."}` |
//! | Tool result    | `{"role":"tool","tool_call_id":"...","content":"..."}` | `{"type":"function_call_output","call_id":"...","output":"..."}` |
//! | Reasoning ref  | (not supported) | `{"type":"item_reference","id":"..."}` |

use crate::llm::anthropic_types::*;
use serde_json::{json, Value as JsonValue};

/// Convert Anthropic messages to OpenAI Responses API input format.
pub fn convert_to_openai_responses_input<'a, I>(anthropic_messages: I) -> Vec<JsonValue>
where
    I: IntoIterator<Item = &'a MessageParam>,
{
    let mut input: Vec<JsonValue> = Vec::new();

    for msg in anthropic_messages {
        match &msg.content {
            MessageContent::Text(text) => match msg.role {
                MessageRole::User => {
                    input.push(json!({
                        "role": "user",
                        "content": [{"type": "input_text", "text": text}]
                    }));
                }
                MessageRole::Assistant => {
                    input.push(json!({
                        "role": "assistant",
                        "content": [{"type": "output_text", "text": text}]
                    }));
                }
            },
            MessageContent::Blocks(blocks) => match msg.role {
                MessageRole::User => handle_user_blocks(blocks, &mut input),
                MessageRole::Assistant => handle_assistant_blocks(blocks, &mut input),
            },
        }
    }

    input
}

// ============================================================
// User Message Handling (Responses API)
// ============================================================

/// User blocks → Responses API items.
///
/// - Text/Image → `{"role": "user", "content": [...]}`
/// - ToolResult → `{"type": "function_call_output", "call_id": ..., "output": ...}`
fn handle_user_blocks(blocks: &[ContentBlock], output: &mut Vec<JsonValue>) {
    let mut user_content: Vec<JsonValue> = Vec::new();

    for block in blocks {
        match block {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error: _,
            } => {
                // Flush pending user content before emitting function_call_output
                if !user_content.is_empty() {
                    output.push(json!({"role": "user", "content": user_content}));
                    user_content = Vec::new();
                }

                let content_str = match content {
                    Some(ToolResultContent::Text(text)) => text.clone(),
                    Some(ToolResultContent::Blocks(blocks)) => blocks
                        .iter()
                        .map(|b| match b {
                            ToolResultBlock::Text { text } => text.as_str(),
                            ToolResultBlock::Image { .. } => {
                                "(see image in following user message)"
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    None => String::new(),
                };

                output.push(json!({
                    "type": "function_call_output",
                    "call_id": tool_use_id,
                    "output": content_str
                }));
            }
            ContentBlock::Text { text, .. } => {
                user_content.push(json!({"type": "input_text", "text": text}));
            }
            ContentBlock::Image { source, .. } => {
                user_content.push(convert_image_to_responses(source));
            }
            ContentBlock::ToolUse { .. } | ContentBlock::Thinking { .. } => {}
        }
    }

    if !user_content.is_empty() {
        output.push(json!({"role": "user", "content": user_content}));
    }
}

// ============================================================
// Assistant Message Handling (Responses API)
// ============================================================

/// Assistant blocks → Responses API items.
///
/// - Text → `{"role": "assistant", "content": [{"type": "output_text", ...}]}`
/// - ToolUse → `{"type": "function_call", "call_id": ..., "name": ..., "arguments": ...}`
/// - Thinking with item_id → `{"type": "item_reference", "id": ...}`
fn handle_assistant_blocks(blocks: &[ContentBlock], output: &mut Vec<JsonValue>) {
    for block in blocks {
        match block {
            ContentBlock::Text { text, .. } => {
                if !text.is_empty() {
                    output.push(json!({
                        "role": "assistant",
                        "content": [{"type": "output_text", "text": text}]
                    }));
                }
            }
            ContentBlock::ToolUse { id, name, input } => {
                output.push(json!({
                    "type": "function_call",
                    "call_id": id,
                    "name": name,
                    "arguments": input.to_string()
                }));
            }
            ContentBlock::Thinking {
                reasoning_metadata, ..
            } => {
                // Emit item_reference when OpenAI item_id is available
                if let Some(item_id) = reasoning_metadata
                    .as_ref()
                    .and_then(|m| m.item_id.as_deref())
                {
                    output.push(json!({
                        "type": "item_reference",
                        "id": item_id
                    }));
                }
            }
            ContentBlock::Image { .. } | ContentBlock::ToolResult { .. } => {}
        }
    }
}

// ============================================================
// Helpers
// ============================================================

fn convert_image_to_responses(source: &ImageSource) -> JsonValue {
    match source {
        ImageSource::Base64 { media_type, data } => {
            json!({
                "type": "input_image",
                "image_url": format!("data:{};base64,{}", media_type, data)
            })
        }
        ImageSource::Url { url } => {
            json!({"type": "input_image", "image_url": url})
        }
        ImageSource::FileId { file_id } => {
            json!({"type": "input_image", "file_id": file_id})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_text() {
        let messages = vec![
            MessageParam::user("Hello!"),
            MessageParam::assistant("Hi!"),
        ];
        let result = convert_to_openai_responses_input(&messages);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["role"], "user");
        assert_eq!(result[0]["content"][0]["type"], "input_text");
        assert_eq!(result[1]["role"], "assistant");
        assert_eq!(result[1]["content"][0]["type"], "output_text");
    }

    #[test]
    fn test_tool_call_cycle() {
        let messages = vec![
            MessageParam::user("What's the weather?"),
            MessageParam::assistant_blocks(vec![ContentBlock::ToolUse {
                id: "call_123".to_string(),
                name: "get_weather".to_string(),
                input: json!({"location": "SF"}),
            }]),
            MessageParam::user_blocks(vec![ContentBlock::tool_result("call_123", "72°F")]),
        ];
        let result = convert_to_openai_responses_input(&messages);
        assert_eq!(result.len(), 3);
        // User
        assert_eq!(result[0]["role"], "user");
        // function_call (not tool_calls!)
        assert_eq!(result[1]["type"], "function_call");
        assert_eq!(result[1]["call_id"], "call_123");
        assert_eq!(result[1]["name"], "get_weather");
        // function_call_output (not role:tool!)
        assert_eq!(result[2]["type"], "function_call_output");
        assert_eq!(result[2]["call_id"], "call_123");
        assert_eq!(result[2]["output"], "72°F");
    }
}
