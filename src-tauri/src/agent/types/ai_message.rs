use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Message - a complete message from user or assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: i64,
    pub session_id: i64,
    pub role: MessageRole,
    pub agent_type: String,
    pub parent_message_id: Option<i64>,
    pub status: MessageStatus,
    pub blocks: Vec<Block>,
    pub is_summary: bool,
    pub is_internal: bool,
    pub model_id: Option<String>,
    pub provider_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub token_usage: Option<TokenUsage>,
    pub context_usage: Option<ContextUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Streaming,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
}

/// Context usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextUsage {
    /// Current number of tokens used
    pub tokens_used: u32,
    /// Total context window size
    pub context_window: u32,
}

/// Content block - building unit of a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    UserText(UserTextBlock),
    UserImage(UserImageBlock),
    Thinking(ThinkingBlock),
    Text(TextBlock),
    Tool(ToolBlock),
    AgentSwitch(AgentSwitchBlock),
    Subtask(SubtaskBlock),
    Error(ErrorBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTextBlock {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserImageBlock {
    pub data_url: String,
    pub mime_type: String,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingBlock {
    pub id: String,
    pub content: String,
    pub is_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextBlock {
    pub id: String,
    pub content: String,
    pub is_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolBlock {
    pub id: String,
    pub call_id: String,
    pub name: String,
    pub status: ToolStatus,
    pub input: Value,
    pub output: Option<ToolOutput>,
    pub compacted_at: Option<DateTime<Utc>>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    Pending,
    Running,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolOutput {
    pub content: Value,
    pub title: Option<String>,
    pub metadata: Option<Value>,
    pub cancel_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSwitchBlock {
    pub from_agent: String,
    pub to_agent: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtaskBlock {
    pub id: String,
    pub child_session_id: i64,
    pub agent_type: String,
    pub description: String,
    pub status: SubtaskStatus,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubtaskStatus {
    Pending,
    Running,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBlock {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

/// Task progress event (sole input from frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskEvent {
    #[serde(rename_all = "camelCase")]
    TaskCreated {
        task_id: String,
        session_id: i64,
        workspace_path: String,
    },

    #[serde(rename_all = "camelCase")]
    MessageCreated { task_id: String, message: Message },

    #[serde(rename_all = "camelCase")]
    BlockAppended {
        task_id: String,
        message_id: i64,
        block: Block,
    },

    #[serde(rename_all = "camelCase")]
    BlockUpdated {
        task_id: String,
        message_id: i64,
        block_id: String,
        block: Block,
    },

    #[serde(rename_all = "camelCase")]
    MessageFinished {
        task_id: String,
        message_id: i64,
        status: MessageStatus,
        finished_at: DateTime<Utc>,
        duration_ms: i64,
        token_usage: Option<TokenUsage>,
        context_usage: Option<ContextUsage>,
    },

    #[serde(rename_all = "camelCase")]
    TaskCompleted { task_id: String },

    #[serde(rename_all = "camelCase")]
    TaskError { task_id: String, error: ErrorBlock },

    #[serde(rename_all = "camelCase")]
    TaskCancelled { task_id: String },

    /// Tool execution confirmation request (frontend needs to show dialog and return decision)
    #[serde(rename_all = "camelCase")]
    ToolConfirmationRequested {
        task_id: String,
        request_id: String,
        workspace_path: String,
        tool_name: String,
        summary: String,
    },

    /// LLM request is being retried (connection/rate-limit/server error)
    #[serde(rename_all = "camelCase")]
    TaskRetrying {
        task_id: String,
        attempt: u32,
        max_attempts: u32,
        reason: String,
        error_message: String,
        retry_in_ms: u64,
    },
}
