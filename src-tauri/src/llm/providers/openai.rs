use async_trait::async_trait;
use eventsource_stream::Eventsource;
use once_cell::sync::Lazy;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::pin::Pin;
use std::time::Duration;
use tokio_stream::Stream;

use crate::llm::{
    error::{LlmProviderError, LlmProviderResult, OpenAiError},
    providers::base::LLMProvider,
    types::{EmbeddingData, EmbeddingRequest, EmbeddingResponse, LLMProviderConfig, LLMUsage},
};

/// Global shared HTTP client for optimized connection reuse
static SHARED_HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(120))
        .build()
        .expect("Failed to create shared HTTP client")
});

/// OpenAI Provider (messages unsupported in zero-abstraction mode)
/// Uses global shared HTTP client for optimized performance
#[derive(Clone)]
pub struct OpenAIProvider {
    config: LLMProviderConfig,
}

type OpenAiResult<T> = Result<T, OpenAiError>;

fn build_openai_chat_body(
    req: &crate::llm::anthropic_types::CreateMessageRequest,
    stream: bool,
) -> Value {
    use crate::llm::anthropic_types::SystemPrompt;
    let mut chat_messages: Vec<Value> = Vec::new();
    if let Some(system) = &req.system {
        let sys_text = match system {
            SystemPrompt::Text(t) => t.clone(),
            SystemPrompt::Blocks(blocks) => blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        };
        if !sys_text.is_empty() {
            chat_messages.push(json!({"role":"system","content":sys_text}));
        }
    }
    let converted = crate::llm::transform::openai::convert_to_openai_messages(&req.messages);
    chat_messages.extend(converted);

    let tools_val = req.tools.as_ref().map(|tools| {
        Value::Array(tools.iter().map(|t| json!({
            "type": "function",
            "function": {"name": t.name, "description": t.description, "parameters": t.input_schema}
        })).collect())
    });

    let mut body = json!({
        "model": req.model,
        "messages": chat_messages,
        "stream": stream
    });
    if let Some(temp) = req.temperature {
        body["temperature"] = json!(temp);
    }
    body["max_tokens"] = json!(req.max_tokens);
    if let Some(tv) = tools_val {
        body["tools"] = tv;
        body["tool_choice"] = json!("auto");
    }

    body
}

/// Build OpenAI Responses API request body
fn build_openai_responses_body(
    req: &crate::llm::anthropic_types::CreateMessageRequest,
    stream: bool,
    enable_deep_thinking: bool,
    reasoning_effort: Option<&str>,
) -> Value {
    use crate::llm::anthropic_types::SystemPrompt;

    // Responses API accepts the same message format as Chat Completions
    let input_items = crate::llm::transform::openai::convert_to_openai_messages(&req.messages);

    // Build tools for Responses API
    let tools_val = req.tools.as_ref().map(|tools| {
        Value::Array(
            tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema
                    })
                })
                .collect(),
        )
    });

    let mut body = json!({
        "model": req.model,
        "input": input_items,
        "stream": stream
    });

    // Add instructions (system prompt)
    if let Some(system) = &req.system {
        let sys_text = match system {
            SystemPrompt::Text(t) => t.clone(),
            SystemPrompt::Blocks(blocks) => blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        };
        if !sys_text.is_empty() {
            body["instructions"] = json!(sys_text);
        }
    }

    if let Some(temp) = req.temperature {
        body["temperature"] = json!(temp);
    }
    body["max_output_tokens"] = json!(req.max_tokens);

    if let Some(tv) = tools_val {
        body["tools"] = tv;
        body["tool_choice"] = json!("auto");
    }

    // Add reasoning configuration when deep thinking is enabled
    if enable_deep_thinking {
        let effort = reasoning_effort.unwrap_or("medium");
        body["reasoning"] = json!({"effort": effort});
    }

    body
}

impl OpenAIProvider {
    pub fn new(config: LLMProviderConfig) -> Self {
        Self { config }
    }

    /// Get shared HTTP client
    fn client(&self) -> &'static Client {
        &SHARED_HTTP_CLIENT
    }

    /// Check if Responses API should be used
    fn use_responses_api(&self) -> bool {
        // Deep thinking implies Responses API for OpenAI
        self.enable_deep_thinking()
    }

    /// Check if deep thinking is enabled (for o1/o3/gpt-5 models with reasoning)
    fn enable_deep_thinking(&self) -> bool {
        self.config
            .options
            .as_ref()
            .and_then(|opts| opts.get("enableDeepThinking"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn reasoning_effort(&self) -> Option<String> {
        let effort = self
            .config
            .options
            .as_ref()
            .and_then(|opts| opts.get("reasoningEffort"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_ascii_lowercase())?;
        match effort.as_str() {
            "minimal" | "low" | "medium" | "high" | "xhigh" => Some(effort),
            _ => None,
        }
    }

    /// Get Chat Completions endpoint
    fn get_chat_endpoint(&self) -> String {
        let base = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        format!("{base}/chat/completions")
    }

    /// Get Responses API endpoint
    fn get_responses_endpoint(&self) -> String {
        let base = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        format!("{base}/responses")
    }

    /// Get Embedding API endpoint
    fn get_embedding_endpoint(&self) -> String {
        let base = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        format!("{base}/embeddings")
    }

    /// Get request headers
    fn get_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.config.api_key),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers
    }

    /// Handle API error response
    fn handle_error_response(&self, status: StatusCode, body: &str) -> OpenAiError {
        if let Ok(error_json) = serde_json::from_str::<Value>(body) {
            if let Some(error_obj) = error_json.get("error").and_then(|v| v.as_object()) {
                let error_type = error_obj
                    .get("type")
                    .or_else(|| error_obj.get("code"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let error_message = error_obj
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");

                let message = match error_type {
                    "insufficient_quota" => format!("Quota exceeded: {error_message}"),
                    "invalid_request_error" => format!("Request error: {error_message}"),
                    "authentication_error" => format!("Authentication failed: {error_message}"),
                    _ => error_message.to_string(),
                };

                return OpenAiError::Api { status, message };
            }
        }
        OpenAiError::Api {
            status,
            message: format!("Unexpected response: {body}"),
        }
    }

    /// Parse embedding response
    fn parse_embedding_response(&self, response_json: &Value) -> OpenAiResult<EmbeddingResponse> {
        let data_array = response_json["data"]
            .as_array()
            .ok_or(OpenAiError::EmbeddingField { field: "data" })?;

        let mut embedding_data = Vec::new();
        for (i, item) in data_array.iter().enumerate() {
            let embedding_vec = item["embedding"]
                .as_array()
                .ok_or(OpenAiError::EmbeddingField {
                    field: "data[].embedding",
                })?
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect::<Vec<f32>>();

            embedding_data.push(EmbeddingData {
                embedding: embedding_vec,
                index: item["index"].as_u64().unwrap_or(i as u64) as usize,
                object: item["object"].as_str().unwrap_or("embedding").to_string(),
            });
        }

        let model = response_json["model"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let usage = Self::extract_usage_static(response_json);

        Ok(EmbeddingResponse {
            data: embedding_data,
            model,
            usage,
        })
    }

    // Static version of extract_usage
    fn extract_usage_static(response_json: &Value) -> Option<LLMUsage> {
        response_json["usage"]
            .as_object()
            .map(|usage_obj| LLMUsage {
                prompt_tokens: usage_obj
                    .get("prompt_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: usage_obj
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: usage_obj
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
            })
    }

    /// Extract text content from Responses API response
    fn extract_responses_text(response_json: &Value) -> String {
        // Responses API returns output array with message items
        // Each message has content array with output_text items
        if let Some(output) = response_json["output"].as_array() {
            let mut text_parts = Vec::new();
            for item in output {
                if item["type"].as_str() == Some("message") {
                    if let Some(content) = item["content"].as_array() {
                        for block in content {
                            if block["type"].as_str() == Some("output_text") {
                                if let Some(text) = block["text"].as_str() {
                                    text_parts.push(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
            return text_parts.join("");
        }
        String::new()
    }

    /// Extract usage from Responses API response
    fn extract_responses_usage(response_json: &Value) -> crate::llm::anthropic_types::Usage {
        use crate::llm::anthropic_types::Usage;
        if let Some(usage) = response_json["usage"].as_object() {
            Usage {
                input_tokens: usage
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                output_tokens: usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            }
        } else {
            Usage {
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            }
        }
    }

    /// Handle Responses API streaming
    fn handle_responses_stream(
        resp: reqwest::Response,
        model: String,
    ) -> LlmProviderResult<
        Pin<
            Box<
                dyn Stream<Item = LlmProviderResult<crate::llm::anthropic_types::StreamEvent>>
                    + Send,
            >,
        >,
    > {
        use crate::llm::anthropic_types::{
            ContentBlockStart, ContentDelta, MessageDeltaData, MessageRole, MessageStartData,
            ReasoningBlockMetadata, StopReason, StreamEvent, Usage,
        };
        use futures::stream;
        use futures::StreamExt as FuturesStreamExt;

        /// Track active reasoning item for OpenAI Responses API
        #[derive(Debug, Clone, Default)]
        struct ActiveReasoning {
            item_id: String,
            encrypted_content: Option<String>,
        }

        struct ResponsesStreamState {
            message_started: bool,
            content_block_started: bool,
            reasoning_block_started: bool,
            /// Next available non-text content block index.
            ///
            /// Index 0 is reserved for text; all other blocks (thinking/tool_use) use 1+.
            next_block_index: usize,
            /// The active reasoning block index (if a reasoning/thinking block is open).
            active_reasoning_block_index: Option<usize>,
            /// Mapping from Responses output item id -> tool use block index.
            tool_block_index_by_item_id: HashMap<String, usize>,
            /// Currently active tool call item id (best-effort; Responses API streams are typically sequential).
            active_tool_item_id: Option<String>,
            /// Set of tool block indices that have started but not yet stopped (to close on response.done).
            open_tool_block_indices: HashSet<usize>,
            pending_events: VecDeque<StreamEvent>,
            response_id: Option<String>,
            /// Active reasoning item metadata (OpenAI Responses API)
            active_reasoning: Option<ActiveReasoning>,
        }

        let raw_stream = resp.bytes_stream().eventsource();

        let event_stream = stream::unfold(
            (
                raw_stream,
                ResponsesStreamState {
                    message_started: false,
                    content_block_started: false,
                    reasoning_block_started: false,
                    next_block_index: 1, // Index 0 is for text, 1+ for thinking/tool_use
                    active_reasoning_block_index: None,
                    tool_block_index_by_item_id: HashMap::new(),
                    active_tool_item_id: None,
                    open_tool_block_indices: HashSet::new(),
                    pending_events: VecDeque::new(),
                    response_id: None,
                    active_reasoning: None,
                },
            ),
            move |(mut stream, mut state)| {
                let model = model.clone();
                async move {
                    loop {
                        if let Some(evt) = state.pending_events.pop_front() {
                            return Some((Ok(evt), (stream, state)));
                        }

                        match FuturesStreamExt::next(&mut stream).await {
                            Some(Ok(event)) => {
                                // Parse SSE event
                                let value: Value = match serde_json::from_str(&event.data) {
                                    Ok(v) => v,
                                    Err(_) => continue,
                                };

                                let event_type = event.event.as_str();

                                match event_type {
                                    // Response created - send MessageStart
                                    "response.created" => {
                                        if !state.message_started {
                                            state.message_started = true;
                                            state.response_id = value["response"]["id"]
                                                .as_str()
                                                .map(|s| s.to_string());
                                            state.pending_events.push_back(
                                                StreamEvent::MessageStart {
                                                    message: MessageStartData {
                                                        id: state
                                                            .response_id
                                                            .clone()
                                                            .unwrap_or_else(|| {
                                                                format!(
                                                                    "resp_{}",
                                                                    uuid::Uuid::new_v4()
                                                                )
                                                            }),
                                                        message_type: "message".to_string(),
                                                        role: MessageRole::Assistant,
                                                        model: model.clone(),
                                                        usage: Usage {
                                                            input_tokens: 0,
                                                            output_tokens: 0,
                                                            cache_creation_input_tokens: None,
                                                            cache_read_input_tokens: None,
                                                        },
                                                    },
                                                },
                                            );
                                        }
                                    }

                                    // Content part added - send ContentBlockStart
                                    "response.content_part.added" => {
                                        if !state.content_block_started {
                                            state.content_block_started = true;
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStart {
                                                    index: 0,
                                                    content_block: ContentBlockStart::Text {
                                                        text: String::new(),
                                                    },
                                                },
                                            );
                                        }
                                    }

                                    // Text delta - send ContentBlockDelta
                                    "response.output_text.delta" => {
                                        // Ensure message and content block have started
                                        if !state.message_started {
                                            state.message_started = true;
                                            state.pending_events.push_back(
                                                StreamEvent::MessageStart {
                                                    message: MessageStartData {
                                                        id: format!(
                                                            "resp_{}",
                                                            uuid::Uuid::new_v4()
                                                        ),
                                                        message_type: "message".to_string(),
                                                        role: MessageRole::Assistant,
                                                        model: model.clone(),
                                                        usage: Usage {
                                                            input_tokens: 0,
                                                            output_tokens: 0,
                                                            cache_creation_input_tokens: None,
                                                            cache_read_input_tokens: None,
                                                        },
                                                    },
                                                },
                                            );
                                        }
                                        if !state.content_block_started {
                                            state.content_block_started = true;
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStart {
                                                    index: 0,
                                                    content_block: ContentBlockStart::Text {
                                                        text: String::new(),
                                                    },
                                                },
                                            );
                                        }

                                        if let Some(delta) = value["delta"].as_str() {
                                            if !delta.is_empty() {
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockDelta {
                                                        index: 0,
                                                        delta: ContentDelta::Text {
                                                            text: delta.to_string(),
                                                        },
                                                    },
                                                );
                                            }
                                        }
                                    }

                                    // Output item added - check for reasoning type
                                    "response.output_item.added" => {
                                        let item = &value["item"];
                                        let item_type = item["type"].as_str().unwrap_or("");

                                        if item_type == "reasoning" {
                                            // Extract reasoning item metadata
                                            let item_id =
                                                item["id"].as_str().unwrap_or("").to_string();
                                            let encrypted_content = item["encrypted_content"]
                                                .as_str()
                                                .map(|s| s.to_string());

                                            state.active_reasoning = Some(ActiveReasoning {
                                                item_id: item_id.clone(),
                                                encrypted_content: encrypted_content.clone(),
                                            });

                                            // Start thinking block with metadata
                                            if !state.reasoning_block_started {
                                                state.reasoning_block_started = true;
                                                let index = state.next_block_index;
                                                state.next_block_index =
                                                    state.next_block_index.saturating_add(1);
                                                state.active_reasoning_block_index = Some(index);
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockStart {
                                                        index,
                                                        content_block:
                                                            ContentBlockStart::Thinking {
                                                                thinking: String::new(),
                                                                metadata: Some(
                                                                    ReasoningBlockMetadata {
                                                                        item_id: Some(item_id),
                                                                        encrypted_content,
                                                                        signature: None,
                                                                        provider: Some(
                                                                            "openai".to_string(),
                                                                        ),
                                                                    },
                                                                ),
                                                            },
                                                    },
                                                );
                                            }
                                        } else if item_type == "function_call" {
                                            // Responses API tool call output item
                                            //
                                            // Map function_call output items to Anthropic-style ToolUse blocks so the
                                            // ReAct orchestrator can execute tools even when the assistant emits no text.
                                            let item_id =
                                                item["id"].as_str().unwrap_or("").to_string();
                                            let name =
                                                item["name"].as_str().unwrap_or("").to_string();

                                            // Some gateways may omit id/name; be defensive to avoid panics.
                                            if !item_id.is_empty() && !name.is_empty() {
                                                let index = state.next_block_index;
                                                state.next_block_index =
                                                    state.next_block_index.saturating_add(1);
                                                state
                                                    .tool_block_index_by_item_id
                                                    .insert(item_id.clone(), index);
                                                state.active_tool_item_id = Some(item_id.clone());
                                                state.open_tool_block_indices.insert(index);

                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockStart {
                                                        index,
                                                        content_block: ContentBlockStart::ToolUse {
                                                            id: item_id.clone(),
                                                            name: name.clone(),
                                                        },
                                                    },
                                                );

                                                // If the item already contains arguments, emit them as a delta.
                                                if let Some(args) = item["arguments"].as_str() {
                                                    if !args.is_empty() {
                                                        state.pending_events.push_back(
                                                            StreamEvent::ContentBlockDelta {
                                                                index,
                                                                delta: ContentDelta::InputJson {
                                                                    partial_json: args.to_string(),
                                                                },
                                                            },
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Reasoning summary part added - start thinking block
                                    "response.reasoning_summary_part.added" => {
                                        if !state.reasoning_block_started {
                                            state.reasoning_block_started = true;
                                            let index = state.next_block_index;
                                            state.next_block_index =
                                                state.next_block_index.saturating_add(1);
                                            state.active_reasoning_block_index = Some(index);
                                            let metadata =
                                                state.active_reasoning.as_ref().map(|r| {
                                                    ReasoningBlockMetadata {
                                                        item_id: Some(r.item_id.clone()),
                                                        encrypted_content: r
                                                            .encrypted_content
                                                            .clone(),
                                                        signature: None,
                                                        provider: Some("openai".to_string()),
                                                    }
                                                });
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStart {
                                                    index,
                                                    content_block: ContentBlockStart::Thinking {
                                                        thinking: String::new(),
                                                        metadata,
                                                    },
                                                },
                                            );
                                        }
                                    }

                                    // Reasoning summary delta - send thinking content
                                    "response.reasoning_summary_part.delta"
                                    | "response.reasoning_summary_text.delta" => {
                                        // Ensure reasoning block has started
                                        if !state.reasoning_block_started {
                                            state.reasoning_block_started = true;
                                            let index = state.next_block_index;
                                            state.next_block_index =
                                                state.next_block_index.saturating_add(1);
                                            state.active_reasoning_block_index = Some(index);
                                            let metadata =
                                                state.active_reasoning.as_ref().map(|r| {
                                                    ReasoningBlockMetadata {
                                                        item_id: Some(r.item_id.clone()),
                                                        encrypted_content: r
                                                            .encrypted_content
                                                            .clone(),
                                                        signature: None,
                                                        provider: Some("openai".to_string()),
                                                    }
                                                });
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStart {
                                                    index,
                                                    content_block: ContentBlockStart::Thinking {
                                                        thinking: String::new(),
                                                        metadata,
                                                    },
                                                },
                                            );
                                        }

                                        if let Some(delta) = value["delta"].as_str() {
                                            if !delta.is_empty() {
                                                let index =
                                                    state.active_reasoning_block_index.unwrap_or(1);
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockDelta {
                                                        index,
                                                        delta: ContentDelta::Thinking {
                                                            thinking: delta.to_string(),
                                                        },
                                                    },
                                                );
                                            }
                                        }
                                    }

                                    // Output item done - finalize reasoning if applicable
                                    "response.output_item.done" => {
                                        let item = &value["item"];
                                        let item_type = item["type"].as_str().unwrap_or("");

                                        if item_type == "reasoning" {
                                            // Update encrypted_content from final item
                                            if let Some(ref mut reasoning) = state.active_reasoning
                                            {
                                                if let Some(ec) = item["encrypted_content"].as_str()
                                                {
                                                    reasoning.encrypted_content =
                                                        Some(ec.to_string());
                                                }
                                            }
                                        } else if item_type == "function_call" {
                                            // Close tool use block once the tool output item is complete.
                                            let item_id =
                                                item["id"].as_str().unwrap_or("").to_string();
                                            if let Some(index) = state
                                                .tool_block_index_by_item_id
                                                .get(&item_id)
                                                .copied()
                                            {
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockStop { index },
                                                );
                                                state.open_tool_block_indices.remove(&index);
                                            }
                                            if state.active_tool_item_id.as_deref()
                                                == Some(item_id.as_str())
                                            {
                                                state.active_tool_item_id = None;
                                            }
                                        }
                                    }

                                    // Reasoning summary done - close thinking block
                                    "response.reasoning_summary_part.done"
                                    | "response.reasoning_summary_text.done" => {
                                        if state.reasoning_block_started {
                                            state.reasoning_block_started = false;
                                            if let Some(index) =
                                                state.active_reasoning_block_index.take()
                                            {
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockStop { index },
                                                );
                                            }
                                            // Clear active reasoning after block is done
                                            state.active_reasoning = None;
                                        }
                                    }

                                    // Response done - send MessageDelta and MessageStop
                                    "response.done" | "response.completed" => {
                                        // Close any open reasoning block first
                                        if state.reasoning_block_started {
                                            state.reasoning_block_started = false;
                                            if let Some(index) =
                                                state.active_reasoning_block_index.take()
                                            {
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockStop { index },
                                                );
                                            }
                                        }

                                        if state.content_block_started {
                                            state.content_block_started = false;
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStop { index: 0 },
                                            );
                                        }

                                        // Close any open tool blocks (defensive; normally closed by response.output_item.done).
                                        if !state.open_tool_block_indices.is_empty() {
                                            let indices: Vec<usize> = state
                                                .open_tool_block_indices
                                                .iter()
                                                .copied()
                                                .collect();
                                            for index in indices {
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockStop { index },
                                                );
                                            }
                                            state.open_tool_block_indices.clear();
                                            state.active_tool_item_id = None;
                                        }

                                        // Extract usage if available
                                        let usage = if let Some(usage_obj) =
                                            value["response"]["usage"].as_object()
                                        {
                                            Usage {
                                                input_tokens: usage_obj
                                                    .get("input_tokens")
                                                    .and_then(|v| v.as_u64())
                                                    .unwrap_or(0)
                                                    as u32,
                                                output_tokens: usage_obj
                                                    .get("output_tokens")
                                                    .and_then(|v| v.as_u64())
                                                    .unwrap_or(0)
                                                    as u32,
                                                cache_creation_input_tokens: None,
                                                cache_read_input_tokens: None,
                                            }
                                        } else {
                                            Usage {
                                                input_tokens: 0,
                                                output_tokens: 0,
                                                cache_creation_input_tokens: None,
                                                cache_read_input_tokens: None,
                                            }
                                        };

                                        state.pending_events.push_back(StreamEvent::MessageDelta {
                                            delta: MessageDeltaData {
                                                stop_reason: Some(StopReason::EndTurn),
                                                stop_sequence: None,
                                            },
                                            usage,
                                        });
                                        state.pending_events.push_back(StreamEvent::MessageStop);
                                    }

                                    // Handle function calls
                                    "response.function_call_arguments.delta" => {
                                        if let Some(delta) = value["delta"].as_str() {
                                            if !delta.is_empty() {
                                                let index = value
                                                    .get("item_id")
                                                    .and_then(|v| v.as_str())
                                                    .and_then(|id| {
                                                        state
                                                            .tool_block_index_by_item_id
                                                            .get(id)
                                                            .copied()
                                                    })
                                                    .or_else(|| {
                                                        state
                                                            .active_tool_item_id
                                                            .as_deref()
                                                            .and_then(|id| {
                                                                state
                                                                    .tool_block_index_by_item_id
                                                                    .get(id)
                                                                    .copied()
                                                            })
                                                    });
                                                let Some(index) = index else {
                                                    // If we can't associate this delta with a tool output item,
                                                    // do not emit it into an arbitrary block index.
                                                    continue;
                                                };
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockDelta {
                                                        index,
                                                        delta: ContentDelta::InputJson {
                                                            partial_json: delta.to_string(),
                                                        },
                                                    },
                                                );
                                            }
                                        }
                                    }

                                    _ => {
                                        // Ignore other event types
                                    }
                                }

                                continue;
                            }
                            Some(Err(e)) => {
                                tracing::error!("Responses API SSE stream error: {:?}", e);
                                return Some((
                                    Err(LlmProviderError::OpenAi(OpenAiError::Stream {
                                        message: format!("Network error: {e}"),
                                    })),
                                    (stream, state),
                                ));
                            }
                            None => return None, // Stream ended
                        }
                    }
                }
            },
        );

        Ok(Box::pin(event_stream))
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    /// Non-streaming call (Anthropic native interface)
    async fn call(
        &self,
        request: crate::llm::anthropic_types::CreateMessageRequest,
    ) -> LlmProviderResult<crate::llm::anthropic_types::Message> {
        use crate::llm::anthropic_types::{ContentBlock, Message, MessageRole, Usage};

        let use_responses = self.use_responses_api();
        let enable_deep_thinking = self.enable_deep_thinking();
        let url = if use_responses {
            self.get_responses_endpoint()
        } else {
            self.get_chat_endpoint()
        };
        let headers = self.get_headers();
        let reasoning_effort = self.reasoning_effort();
        let body = if use_responses {
            build_openai_responses_body(
                &request,
                false,
                enable_deep_thinking,
                reasoning_effort.as_deref(),
            )
        } else {
            build_openai_chat_body(&request, false)
        };

        let mut req = self.client().post(&url).json(&body);
        for (k, v) in headers {
            req = req.header(&k, &v);
        }

        let resp = req
            .send()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;
        let status = resp.status();
        if !status.is_success() {
            let txt = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmProviderError::from(
                self.handle_error_response(status, &txt),
            ));
        }
        let json: Value = resp
            .json()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;

        // Parse response based on API type
        let (content, usage) = if use_responses {
            // Parse Responses API format
            let text = Self::extract_responses_text(&json);
            let usage = Self::extract_responses_usage(&json);
            (text, usage)
        } else {
            // Parse Chat Completions format
            let text = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let usage_obj = json.get("usage");
            let usage = usage_obj
                .map(|u| Usage {
                    input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
                        as u32,
                    output_tokens: u
                        .get("completion_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                })
                .unwrap_or(Usage {
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                });
            (text, usage)
        };

        let message = Message {
            id: json["id"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("msg_{}", uuid::Uuid::new_v4())),
            message_type: "message".to_string(),
            role: MessageRole::Assistant,
            content: vec![ContentBlock::Text {
                text: content,
                cache_control: None,
            }],
            model: request.model.clone(),
            stop_reason: None,
            stop_sequence: None,
            usage,
        };
        Ok(message)
    }

    /// Streaming call (Anthropic native interface)
    async fn call_stream(
        &self,
        request: crate::llm::anthropic_types::CreateMessageRequest,
    ) -> LlmProviderResult<
        Pin<
            Box<
                dyn Stream<Item = LlmProviderResult<crate::llm::anthropic_types::StreamEvent>>
                    + Send,
            >,
        >,
    > {
        use crate::llm::anthropic_types::{
            ContentBlockStart, ContentDelta, MessageDeltaData, MessageRole, MessageStartData,
            StopReason, StreamEvent, Usage,
        };

        let use_responses = self.use_responses_api();
        let enable_deep_thinking = self.enable_deep_thinking();
        let url = if use_responses {
            self.get_responses_endpoint()
        } else {
            self.get_chat_endpoint()
        };
        let headers = self.get_headers();
        let reasoning_effort = self.reasoning_effort();
        let body = if use_responses {
            build_openai_responses_body(
                &request,
                true,
                enable_deep_thinking,
                reasoning_effort.as_deref(),
            )
        } else {
            build_openai_chat_body(&request, true)
        };

        let mut req = self.client().post(&url).json(&body);
        for (k, v) in headers {
            req = req.header(&k, &v);
        }

        let resp = req
            .send()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;

        let status = resp.status();
        if !status.is_success() {
            let txt = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmProviderError::from(
                self.handle_error_response(status, &txt),
            ));
        }

        use futures::stream;
        use futures::StreamExt as FuturesStreamExt;

        // Handle Responses API streaming
        if use_responses {
            return Self::handle_responses_stream(resp, request.model.clone());
        }

        // State machine: track whether key events have been sent
        struct StreamState {
            message_started: bool,
            content_block_started: bool,
            pending_events: VecDeque<StreamEvent>,
            tool_use_started: HashSet<usize>,
        }

        let model = request.model.clone();
        let raw_stream = resp.bytes_stream().eventsource();

        // Use unfold to maintain state
        let event_stream = stream::unfold(
            (
                raw_stream,
                StreamState {
                    message_started: false,
                    content_block_started: false,
                    pending_events: VecDeque::new(),
                    tool_use_started: HashSet::new(),
                },
            ),
            move |(mut stream, mut state)| {
                let model = model.clone();
                async move {
                    loop {
                        // Prioritize outputting queued events
                        if let Some(evt) = state.pending_events.pop_front() {
                            return Some((Ok(evt), (stream, state)));
                        }
                        match FuturesStreamExt::next(&mut stream).await {
                            Some(Ok(event)) => {
                                // OpenAI stream end marker
                                if event.data == "[DONE]" {
                                    // Close unclosed blocks before ending
                                    if state.content_block_started {
                                        state.content_block_started = false;
                                        state
                                            .pending_events
                                            .push_back(StreamEvent::ContentBlockStop { index: 0 });
                                    }
                                    if !state.tool_use_started.is_empty() {
                                        let indices: Vec<usize> =
                                            state.tool_use_started.iter().copied().collect();
                                        for idx in indices {
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStop { index: idx },
                                            );
                                        }
                                        state.tool_use_started.clear();
                                    }
                                    state.pending_events.push_back(StreamEvent::MessageStop);
                                    if let Some(evt) = state.pending_events.pop_front() {
                                        return Some((Ok(evt), (stream, state)));
                                    } else {
                                        continue;
                                    }
                                }

                                // Parse OpenAI streaming response
                                let value: Value = match serde_json::from_str(&event.data) {
                                    Ok(v) => v,
                                    Err(_) => continue, // Skip invalid data
                                };

                                // Extract choices[0]
                                let choice =
                                    match value["choices"].as_array().and_then(|arr| arr.first()) {
                                        Some(c) => c,
                                        None => continue,
                                    };

                                let delta = &choice["delta"];

                                // First event: MessageStart
                                if !state.message_started {
                                    state.message_started = true;
                                    state.pending_events.push_back(StreamEvent::MessageStart {
                                        message: MessageStartData {
                                            id: format!("msg_{}", uuid::Uuid::new_v4()),
                                            message_type: "message".to_string(),
                                            role: MessageRole::Assistant,
                                            model: model.clone(),
                                            usage: Usage {
                                                input_tokens: 0,
                                                output_tokens: 0,
                                                cache_creation_input_tokens: None,
                                                cache_read_input_tokens: None,
                                            },
                                        },
                                    });
                                }

                                // Second event: ContentBlockStart (when content is first encountered)
                                if !state.content_block_started && delta.get("content").is_some() {
                                    state.content_block_started = true;
                                    state.pending_events.push_back(
                                        StreamEvent::ContentBlockStart {
                                            index: 0,
                                            content_block: ContentBlockStart::Text {
                                                text: String::new(),
                                            },
                                        },
                                    );
                                }

                                // ContentBlockDelta (content increment)
                                if let Some(content) = delta["content"].as_str() {
                                    if !content.is_empty() {
                                        state.pending_events.push_back(
                                            StreamEvent::ContentBlockDelta {
                                                index: 0,
                                                delta: ContentDelta::Text {
                                                    text: content.to_string(),
                                                },
                                            },
                                        );
                                    }
                                }

                                // Handle tool call increment delta.tool_calls
                                if let Some(tc_arr) =
                                    delta.get("tool_calls").and_then(|v| v.as_array())
                                {
                                    for tc in tc_arr {
                                        let raw_index =
                                            tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0)
                                                as usize;
                                        let event_index = raw_index + 1; // Offset tool block index from text block (0)

                                        let func = tc.get("function");
                                        let name_opt = func
                                            .and_then(|f| f.get("name"))
                                            .and_then(|v| v.as_str());
                                        let args_opt = func
                                            .and_then(|f| f.get("arguments"))
                                            .and_then(|v| v.as_str());

                                        if !state.tool_use_started.contains(&event_index) {
                                            if let Some(name) = name_opt {
                                                let id = tc
                                                    .get("id")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string())
                                                    .unwrap_or_else(|| {
                                                        format!("call_{}", uuid::Uuid::new_v4())
                                                    });
                                                state.tool_use_started.insert(event_index);
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockStart {
                                                        index: event_index,
                                                        content_block: ContentBlockStart::ToolUse {
                                                            id,
                                                            name: name.to_string(),
                                                        },
                                                    },
                                                );
                                            }
                                        }

                                        if let Some(arguments) = args_opt {
                                            if !arguments.is_empty()
                                                && state.tool_use_started.contains(&event_index)
                                            {
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockDelta {
                                                        index: event_index,
                                                        delta: ContentDelta::InputJson {
                                                            partial_json: arguments.to_string(),
                                                        },
                                                    },
                                                );
                                            }
                                        }
                                    }
                                }

                                // Compatible with legacy function_call streaming field (some OpenAI-compatible models still send it)
                                if let Some(func) = delta.get("function_call") {
                                    let name_opt = func.get("name").and_then(|v| v.as_str());
                                    let args_opt = func.get("arguments").and_then(|v| v.as_str());
                                    let event_index = 1;

                                    if !state.tool_use_started.contains(&event_index) {
                                        if let Some(name) = name_opt {
                                            let id = format!("call_{}", uuid::Uuid::new_v4());
                                            state.tool_use_started.insert(event_index);
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStart {
                                                    index: event_index,
                                                    content_block: ContentBlockStart::ToolUse {
                                                        id,
                                                        name: name.to_string(),
                                                    },
                                                },
                                            );
                                        }
                                    }

                                    if let Some(arguments) = args_opt {
                                        if !arguments.is_empty()
                                            && state.tool_use_started.contains(&event_index)
                                        {
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockDelta {
                                                    index: event_index,
                                                    delta: ContentDelta::InputJson {
                                                        partial_json: arguments.to_string(),
                                                    },
                                                },
                                            );
                                        }
                                    }
                                }

                                // finish_reason (stream end reason)
                                if let Some(reason) = choice["finish_reason"].as_str() {
                                    // First send ContentBlockStop (text)
                                    if state.content_block_started {
                                        state.content_block_started = false;
                                        state
                                            .pending_events
                                            .push_back(StreamEvent::ContentBlockStop { index: 0 });
                                    }
                                    // If tool call ends, also close all opened tool blocks
                                    if (reason == "tool_calls" || reason == "function_call")
                                        && !state.tool_use_started.is_empty()
                                    {
                                        let indices: Vec<usize> =
                                            state.tool_use_started.iter().copied().collect();
                                        for idx in indices {
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStop { index: idx },
                                            );
                                        }
                                        state.tool_use_started.clear();
                                    }

                                    // Then send MessageDelta with stop_reason
                                    let stop_reason = match reason {
                                        "stop" => Some(StopReason::EndTurn),
                                        "length" => Some(StopReason::MaxTokens),
                                        "tool_calls" | "function_call" => Some(StopReason::ToolUse),
                                        "content_filter" => Some(StopReason::EndTurn),
                                        _ => None,
                                    };

                                    state.pending_events.push_back(StreamEvent::MessageDelta {
                                        delta: MessageDeltaData {
                                            stop_reason,
                                            stop_sequence: None,
                                        },
                                        usage: Usage {
                                            input_tokens: 0,
                                            output_tokens: 0,
                                            cache_creation_input_tokens: None,
                                            cache_read_input_tokens: None,
                                        },
                                    });

                                    if let Some(evt) = state.pending_events.pop_front() {
                                        return Some((Ok(evt), (stream, state)));
                                    } else {
                                        continue;
                                    }
                                }

                                // Skip other deltas (e.g., role: "assistant")
                                continue;
                            }
                            Some(Err(e)) => {
                                tracing::error!("OpenAI SSE stream error: {:?}", e);
                                return Some((
                                    Err(LlmProviderError::OpenAi(OpenAiError::Stream {
                                        message: format!("Network error: {e}"),
                                    })),
                                    (stream, state),
                                ));
                            }
                            None => return None, // Stream ended
                        }
                    }
                }
            },
        );

        Ok(Box::pin(event_stream))
    }

    /// Embedding call implementation
    async fn create_embeddings(
        &self,
        request: EmbeddingRequest,
    ) -> LlmProviderResult<EmbeddingResponse> {
        let url = self.get_embedding_endpoint();
        let headers = self.get_headers();

        // Build embedding request body
        let mut body = json!({
            "model": request.model,
            "input": request.input
        });

        if let Some(encoding_format) = &request.encoding_format {
            body["encoding_format"] = json!(encoding_format);
        }

        if let Some(dimensions) = request.dimensions {
            body["dimensions"] = json!(dimensions);
        }

        let mut req_builder = self.client().post(&url).json(&body);
        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }

        let response = req_builder
            .send()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let error = self.handle_error_response(status, &error_text);
            return Err(LlmProviderError::from(error));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;
        self.parse_embedding_response(&response_json)
            .map_err(LlmProviderError::from)
    }
}
