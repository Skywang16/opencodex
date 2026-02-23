//! Anthropic Messages API type definitions
//!
//! ## Correspondence with TypeScript SDK
//!
//! | Rust Type | TypeScript (@anthropic-ai/sdk) |
//! |-----------|--------------------------------|
//! | `MessageParam` | `Anthropic.Messages.MessageParam` |
//! | `ContentBlock` | `Anthropic.TextBlockParam \| Anthropic.ImageBlockParam \| ...` |
//! | `ToolUseBlock` | `Anthropic.ToolUseBlock` |
//! | `ToolResultBlockParam` | `Anthropic.ToolResultBlockParam` |

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ============================================================
// Message Structure (Messages)
// ============================================================

/// Message parameter - corresponds to `Anthropic.Messages.MessageParam`
///
/// Used to build message history sent to the API
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageParam {
    /// Message role: user or assistant
    pub role: MessageRole,
    /// Message content: can be plain text or structured content block array
    pub content: MessageContent,
}

/// Message role
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Message content - can be string or content block array
///
/// Corresponds to TypeScript: `string | ContentBlock[]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    /// Plain text content
    Text(String),
    /// Structured content block array (supports text, images, tool calls, etc.)
    Blocks(Vec<ContentBlock>),
}

impl MessageContent {
    /// Create text content
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }
}

// ============================================================
// Content Blocks
// ============================================================

/// Content block - corresponds to various Anthropic BlockParam types
///
/// Uses `#[serde(tag = "type")]` to implement tagged union
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text block - corresponds to `TextBlockParam`
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// Image block - corresponds to `ImageBlockParam`
    Image {
        source: ImageSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// Tool call (sent by assistant) - corresponds to `ToolUseBlock`
    ToolUse {
        /// Unique ID for tool call (format: toolu_xxx)
        id: String,
        /// Tool name
        name: String,
        /// Tool input parameters (JSON object)
        input: JsonValue,
    },

    /// Tool result (returned by user) - corresponds to `ToolResultBlockParam`
    ToolResult {
        /// Corresponding tool call ID
        tool_use_id: String,
        /// Tool execution result (can be string or block array containing images)
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ToolResultContent>,
        /// Whether this is an error result
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },

    /// Thinking block (Extended Thinking feature)
    ///
    /// Returned when model uses Extended Thinking.
    /// `signature` is used by Anthropic for verification.
    /// `reasoning_metadata` carries OpenAI `item_id` / `encrypted_content` for
    /// Responses API `item_reference` round-tripping.
    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
        /// Full provider metadata — preserved across turns so that reasoning
        /// traces can be forwarded back as `item_reference` (OpenAI) or
        /// `thinking` with `signature` (Anthropic).
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_metadata: Option<ReasoningBlockMetadata>,
    },
}

/// Image source - supports three methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64 encoded image
    Base64 {
        /// MIME type (image/jpeg, image/png, image/gif, image/webp)
        media_type: String,
        /// Base64 encoded image data
        data: String,
    },
    /// URL linked image
    Url {
        /// Image URL
        url: String,
    },
    /// File uploaded via Files API
    #[serde(rename = "file")]
    FileId {
        /// File ID (obtained via Files API)
        file_id: String,
    },
}

/// Tool result content - can be string or block array containing images
///
/// Corresponds to TypeScript: `string | ToolResultBlock[]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Plain text result
    Text(String),
    /// Rich content result (can contain text and images)
    Blocks(Vec<ToolResultBlock>),
}

/// Tool result block - only supports text and images
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultBlock {
    /// Text block
    Text { text: String },
    /// Image block
    Image { source: ImageSource },
}

/// Prompt Cache control
///
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheControl {
    /// Cache type (currently only supports "ephemeral")
    #[serde(rename = "type")]
    pub cache_type: String,
    /// Cache TTL (optional: 5m or 1h)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
}

impl CacheControl {
    /// Create ephemeral cache control (default 5 minutes)
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: None,
        }
    }
}

// ============================================================
// Tool Definitions (Tools)
// ============================================================

/// Tool definition - corresponds to `Anthropic.Tool`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for input parameters
    pub input_schema: JsonValue,
}

impl Tool {
    /// Create new tool definition
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: JsonValue,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

// ============================================================
// API Request/Response
// ============================================================

/// CreateMessageRequest - Anthropic message creation request
///
/// Corresponds to API: `POST /v1/messages`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    /// Model ID (e.g., "claude-3-5-sonnet-20241022")
    pub model: String,

    /// Message history
    pub messages: Vec<MessageParam>,

    /// Maximum tokens to generate
    pub max_tokens: u32,

    /// System prompt (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,

    /// Tool definition list (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Temperature parameter (0.0-1.0)
    /// Note: Must be 1.0 when extended thinking is enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Custom stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Whether to stream response
    #[serde(default)]
    pub stream: bool,

    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Top-k sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// Metadata (for tracking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// Extended Thinking configuration (optional)
    /// When enabled, temperature must be 1.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
}

/// System Prompt - can be string or blocks with cache control
///
/// Corresponds to TypeScript: `string | SystemBlock[]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SystemPrompt {
    /// Plain text
    Text(String),
    /// Block array with cache support
    Blocks(Vec<SystemBlock>),
}

impl SystemPrompt {
    /// Create system prompt with cache control
    pub fn with_cache(text: impl Into<String>) -> Self {
        Self::Blocks(vec![SystemBlock {
            block_type: "text".to_string(),
            text: text.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }])
    }
}

/// System block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemBlock {
    #[serde(rename = "type")]
    pub block_type: String, // "text"
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

/// Metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    /// User ID (for tracking and statistics)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

// ============================================================
// Extended Thinking
// ============================================================

/// Extended Thinking configuration
///
/// Enables the model to perform deeper reasoning before responding.
/// See: https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThinkingConfig {
    /// Type (always "enabled")
    #[serde(rename = "type")]
    pub thinking_type: String,
    /// Token budget for thinking (1024 to model's max output tokens)
    pub budget_tokens: u32,
}

impl ThinkingConfig {
    /// Create enabled thinking config with specified budget
    pub fn enabled(budget_tokens: u32) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens: budget_tokens.max(1024), // minimum 1024
        }
    }

    /// Default thinking budget (10000 tokens)
    pub fn default_budget() -> Self {
        Self::enabled(10000)
    }
}

/// API response message
///
/// Corresponds to `Anthropic.Messages.Message`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID
    pub id: String,

    /// Type (fixed as "message")
    #[serde(rename = "type")]
    pub message_type: String,

    /// Role (fixed as "assistant")
    pub role: MessageRole,

    /// Content block array
    pub content: Vec<ContentBlock>,

    /// Model used
    pub model: String,

    /// Stop reason
    pub stop_reason: Option<StopReason>,

    /// Stop sequence (if triggered by custom stop_sequence)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,

    /// Token usage statistics
    pub usage: Usage,
}

/// Stop reason
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Model naturally ended
    EndTurn,
    /// Reached max_tokens limit
    MaxTokens,
    /// Encountered custom stop_sequence
    StopSequence,
    /// Model wants to use tools
    ToolUse,
}

/// Token usage statistics
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Usage {
    /// Input token count (optional in message_delta events)
    #[serde(default)]
    pub input_tokens: u32,
    /// Output token count (optional in message_delta events)
    #[serde(default)]
    pub output_tokens: u32,
    /// Cache creation input token count (Prompt Caching)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    /// Cache read input token count (Prompt Caching)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

impl Usage {
    /// Total token count
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }

    /// Cache saved token count
    pub fn cache_savings(&self) -> u32 {
        self.cache_read_input_tokens.unwrap_or(0)
    }
}

// ============================================================
// Streaming Events
// ============================================================

/// SSE streaming event
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Message start
    MessageStart { message: MessageStartData },

    /// Content block start
    ContentBlockStart {
        index: usize,
        content_block: ContentBlockStart,
    },

    /// Content block delta
    ContentBlockDelta { index: usize, delta: ContentDelta },

    /// Content block stop
    ContentBlockStop { index: usize },

    /// Message-level changes (e.g., stop_reason)
    MessageDelta {
        delta: MessageDeltaData,
        usage: Usage,
    },

    /// Message stop
    MessageStop,

    /// Connection keepalive
    Ping,

    /// Error
    Error { error: ErrorData },

    /// Unknown event type
    #[serde(other)]
    Unknown,
}

/// Message start data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStartData {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String, // "message"
    pub role: MessageRole,
    pub model: String,
    pub usage: Usage,
}

/// Content block start data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockStart {
    /// Text block start
    Text { text: String },
    /// Tool call start
    ToolUse { id: String, name: String },
    /// Thinking/Reasoning block start
    /// For Anthropic: Extended Thinking
    /// For OpenAI: Reasoning summary
    Thinking {
        thinking: String,
        /// Provider-specific metadata
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<ReasoningBlockMetadata>,
    },
    /// Unknown content block type
    #[serde(other)]
    Unknown,
}

/// Metadata for reasoning/thinking blocks (provider-specific)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningBlockMetadata {
    /// OpenAI: item ID for item_reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,

    /// OpenAI: encrypted reasoning content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,

    /// Anthropic: thinking signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,

    /// Provider identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// Content delta
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    /// Text delta
    #[serde(rename = "text_delta")]
    Text { text: String },
    /// Tool input JSON delta (may be incomplete JSON fragment)
    #[serde(rename = "input_json_delta")]
    InputJson { partial_json: String },
    /// Thinking delta
    #[serde(rename = "thinking_delta")]
    Thinking { thinking: String },
    /// Signature delta (sent before content_block_stop for thinking blocks)
    /// Contains cryptographic signature for thinking content verification
    #[serde(rename = "signature_delta")]
    Signature { signature: String },
    /// Unknown delta type
    #[serde(other)]
    Unknown,
}

/// Message delta data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeltaData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
}

/// Error data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

// ============================================================
// Convenience Constructors
// ============================================================

impl MessageParam {
    /// Create user text message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create user message (supports multiple content blocks)
    pub fn user_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Blocks(blocks),
        }
    }

    /// Create assistant text message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create assistant message (supports multiple content blocks)
    pub fn assistant_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Blocks(blocks),
        }
    }
}

impl ContentBlock {
    /// Create text block
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            cache_control: None,
        }
    }

    /// Create text block with cache control
    pub fn text_with_cache(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }
    }

    /// Create URL image block
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Url { url: url.into() },
            cache_control: None,
        }
    }

    /// Create tool result block
    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(ToolResultContent::Text(content.into())),
            is_error: None,
        }
    }

    /// Create tool error result block
    pub fn tool_error(tool_use_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(ToolResultContent::Text(error.into())),
            is_error: Some(true),
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_message_param_serialization() {
        let msg = MessageParam::user("Hello, Claude!");
        let json = serde_json::to_value(&msg).unwrap();

        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Hello, Claude!");
    }

    #[test]
    fn test_content_blocks() {
        let blocks = vec![
            ContentBlock::text("What's in this image?"),
            ContentBlock::image_url("https://example.com/image.jpg"),
        ];

        let msg = MessageParam::user_blocks(blocks);
        let json = serde_json::to_value(&msg).unwrap();

        assert_eq!(json["role"], "user");
        assert_eq!(json["content"][0]["type"], "text");
        assert_eq!(json["content"][1]["type"], "image");
    }

    #[test]
    fn test_tool_use_serialization() {
        let tool_use = ContentBlock::ToolUse {
            id: "toolu_123".to_string(),
            name: "get_weather".to_string(),
            input: json!({"location": "San Francisco"}),
        };

        let json = serde_json::to_value(&tool_use).unwrap();
        assert_eq!(json["type"], "tool_use");
        assert_eq!(json["id"], "toolu_123");
        assert_eq!(json["name"], "get_weather");
    }

    #[test]
    fn test_tool_result_serialization() {
        let tool_result = ContentBlock::tool_result("toolu_123", "Temperature: 72°F");

        let json = serde_json::to_value(&tool_result).unwrap();
        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["tool_use_id"], "toolu_123");
        assert_eq!(json["content"], "Temperature: 72°F");
    }

    #[test]
    fn test_cache_control() {
        let block = ContentBlock::text_with_cache("Cached content");
        let json = serde_json::to_value(&block).unwrap();

        assert_eq!(json["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_system_prompt_with_cache() {
        let system = SystemPrompt::with_cache("You are a helpful assistant.");
        let json = serde_json::to_value(&system).unwrap();

        assert_eq!(json[0]["type"], "text");
        assert_eq!(json[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_usage_calculations() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: Some(20),
            cache_read_input_tokens: Some(80),
        };

        assert_eq!(usage.total_tokens(), 150);
        assert_eq!(usage.cache_savings(), 80);
    }

    #[test]
    fn test_deserialization_from_api() {
        // Simulate API returned JSON
        let json_str = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello!"
                }
            ],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20
            }
        }"#;

        let message: Message = serde_json::from_str(json_str).unwrap();
        assert_eq!(message.id, "msg_123");
        assert_eq!(message.role, MessageRole::Assistant);
        assert_eq!(message.usage.total_tokens(), 30);
    }
}
