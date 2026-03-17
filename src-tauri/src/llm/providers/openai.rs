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
const OPENAI_API_BASE_URL: &str = "https://api.openai.com/v1";

fn build_openai_chat_body(
    req: &crate::llm::anthropic_types::CreateMessageRequest,
    stream: bool,
) -> Value {
    let mut chat_messages: Vec<Value> = Vec::new();
    let system_sections = collect_openai_instruction_sections(req);
    if !system_sections.is_empty() {
        chat_messages.push(json!({
            "role": "system",
            "content": system_sections.join("\n\n"),
        }));
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
    let mut input_items = Vec::new();
    if let Some(dev_ctx) = &req.developer_context {
        for text in dev_ctx
            .iter()
            .map(|text| text.trim())
            .filter(|text| !text.is_empty())
        {
            input_items.push(json!({
                "type": "message",
                "role": "developer",
                "content": [{ "type": "input_text", "text": text }],
            }));
        }
    }
    input_items
        .extend(crate::llm::transform::openai::convert_to_responses_api_input(&req.messages));

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
    if let Some(instructions) = extract_openai_system_prompt(&req.system) {
        body["instructions"] = json!(instructions);
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
        body["reasoning"] = match reasoning_effort {
            Some(effort) => json!({ "effort": effort }),
            None => json!({}),
        };
    }

    body
}

fn extract_openai_system_prompt(
    system: &Option<crate::llm::anthropic_types::SystemPrompt>,
) -> Option<String> {
    use crate::llm::anthropic_types::SystemPrompt;

    system.as_ref().and_then(|system| {
        let text = match system {
            SystemPrompt::Text(text) => text.clone(),
            SystemPrompt::Blocks(blocks) => blocks
                .iter()
                .map(|block| block.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        };
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn collect_openai_instruction_sections(
    req: &crate::llm::anthropic_types::CreateMessageRequest,
) -> Vec<String> {
    let mut sections = Vec::new();
    if let Some(system) = extract_openai_system_prompt(&req.system) {
        sections.push(system);
    }
    if let Some(developer_context) = &req.developer_context {
        sections.extend(
            developer_context
                .iter()
                .map(|item| item.trim())
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned),
        );
    }
    sections
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

    fn base_url(&self) -> &str {
        match self.config.api_url.as_deref() {
            Some(api_url) => api_url,
            None => OPENAI_API_BASE_URL,
        }
    }

    /// Get Chat Completions endpoint
    fn get_chat_endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url())
    }

    /// Get Responses API endpoint
    fn get_responses_endpoint(&self) -> String {
        format!("{}/responses", self.base_url())
    }

    /// Get Embedding API endpoint
    fn get_embedding_endpoint(&self) -> String {
        format!("{}/embeddings", self.base_url())
    }

    /// Get request headers
    fn get_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.config.api_key),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("User-Agent".to_string(), "openai-codex/1.0.0".to_string());
        headers
    }

    /// Handle API error response
    fn handle_error_response(&self, status: StatusCode, body: &str) -> OpenAiError {
        match serde_json::from_str::<Value>(body) {
            Ok(error_json) => {
                if let Some(error_obj) = error_json.get("error").and_then(|v| v.as_object()) {
                    let error_type = error_obj
                        .get("type")
                        .or_else(|| error_obj.get("code"))
                        .and_then(|v| v.as_str());

                    let error_message = error_obj.get("message").and_then(|v| v.as_str());

                    if let Some(error_message) = error_message {
                        let message = match error_type {
                            Some("insufficient_quota") => {
                                format!("Quota exceeded: {error_message}")
                            }
                            Some("invalid_request_error") => {
                                format!("Request error: {error_message}")
                            }
                            Some("authentication_error") => {
                                format!("Authentication failed: {error_message}")
                            }
                            _ => error_message.to_string(),
                        };

                        return OpenAiError::Api { status, message };
                    }

                    return OpenAiError::Api {
                        status,
                        message: body.to_string(),
                    };
                }
            }
            Err(err) => {
                tracing::debug!(
                    status = %status,
                    error = %err,
                    "Failed to parse OpenAI error response as JSON"
                );
            }
        }
        OpenAiError::Api {
            status,
            message: format!("Unexpected response: {body}"),
        }
    }

    async fn read_error_body(response: reqwest::Response) -> String {
        match response.text().await {
            Ok(text) => text,
            Err(err) => {
                tracing::warn!("Failed to read OpenAI error response body: {}", err);
                format!("<failed to read error response body: {err}>")
            }
        }
    }

    fn require_stream_string<'a>(value: &'a Value, field: &'static str) -> OpenAiResult<&'a str> {
        value.as_str().ok_or(OpenAiError::MissingField { field })
    }

    fn parse_responses_stream_usage(
        response_json: &Value,
    ) -> OpenAiResult<crate::llm::anthropic_types::Usage> {
        use crate::llm::anthropic_types::Usage;

        let usage =
            response_json["response"]["usage"]
                .as_object()
                .ok_or(OpenAiError::MissingField {
                    field: "response.usage",
                })?;

        Ok(Usage {
            input_tokens: usage
                .get("input_tokens")
                .and_then(|value| value.as_u64())
                .ok_or(OpenAiError::MissingField {
                    field: "response.usage.input_tokens",
                })? as u32,
            output_tokens: usage
                .get("output_tokens")
                .and_then(|value| value.as_u64())
                .ok_or(OpenAiError::MissingField {
                    field: "response.usage.output_tokens",
                })? as u32,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        })
    }

    fn parse_chat_completion_message(
        response_json: &Value,
    ) -> OpenAiResult<(String, crate::llm::anthropic_types::Usage, String)> {
        use crate::llm::anthropic_types::Usage;

        let id = response_json["id"]
            .as_str()
            .ok_or(OpenAiError::MissingField { field: "id" })?
            .to_string();
        let choices = response_json["choices"]
            .as_array()
            .ok_or(OpenAiError::MissingField { field: "choices" })?;
        let first_choice = choices.first().ok_or(OpenAiError::MissingField {
            field: "choices[0]",
        })?;
        let content = first_choice["message"]["content"]
            .as_str()
            .ok_or(OpenAiError::MissingField {
                field: "choices[0].message.content",
            })?
            .to_string();
        let usage_obj = response_json["usage"]
            .as_object()
            .ok_or(OpenAiError::MissingField { field: "usage" })?;
        let prompt_tokens = usage_obj
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .ok_or(OpenAiError::MissingField {
                field: "usage.prompt_tokens",
            })?;
        let completion_tokens = usage_obj
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .ok_or(OpenAiError::MissingField {
                field: "usage.completion_tokens",
            })?;

        Ok((
            content,
            Usage {
                input_tokens: prompt_tokens as u32,
                output_tokens: completion_tokens as u32,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            },
            id,
        ))
    }

    /// Parse embedding response
    fn parse_embedding_response(&self, response_json: &Value) -> OpenAiResult<EmbeddingResponse> {
        let data_array = response_json["data"]
            .as_array()
            .ok_or(OpenAiError::EmbeddingField { field: "data" })?;

        let mut embedding_data = Vec::new();
        for item in data_array {
            let embedding_vec = item["embedding"]
                .as_array()
                .ok_or(OpenAiError::EmbeddingField {
                    field: "data[].embedding",
                })?
                .iter()
                .map(|value| {
                    value
                        .as_f64()
                        .ok_or(OpenAiError::EmbeddingField {
                            field: "data[].embedding[]",
                        })
                        .map(|number| number as f32)
                })
                .collect::<OpenAiResult<Vec<f32>>>()?;

            embedding_data.push(EmbeddingData {
                embedding: embedding_vec,
                index: item["index"]
                    .as_u64()
                    .ok_or(OpenAiError::EmbeddingField {
                        field: "data[].index",
                    })
                    .map(|value| value as usize)?,
                object: item["object"]
                    .as_str()
                    .ok_or(OpenAiError::EmbeddingField {
                        field: "data[].object",
                    })?
                    .to_string(),
            });
        }

        let model = response_json["model"]
            .as_str()
            .ok_or(OpenAiError::EmbeddingField { field: "model" })?
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
        let usage_obj = response_json["usage"].as_object()?;
        Some(LLMUsage {
            prompt_tokens: usage_obj.get("prompt_tokens")?.as_u64()? as u32,
            completion_tokens: usage_obj.get("completion_tokens")?.as_u64()? as u32,
            total_tokens: usage_obj.get("total_tokens")?.as_u64()? as u32,
        })
    }

    /// Extract text content from Responses API response
    fn extract_responses_text(response_json: &Value) -> OpenAiResult<String> {
        // Responses API returns output array with message items
        // Each message has content array with output_text items
        let output = response_json["output"]
            .as_array()
            .ok_or(OpenAiError::MissingField { field: "output" })?;
        let mut text_parts = Vec::new();
        for item in output {
            if item["type"].as_str() == Some("message") {
                let content = item["content"]
                    .as_array()
                    .ok_or(OpenAiError::MissingField {
                        field: "output[].content",
                    })?;
                for block in content {
                    if block["type"].as_str() == Some("output_text") {
                        let text = block["text"].as_str().ok_or(OpenAiError::MissingField {
                            field: "output[].content[].text",
                        })?;
                        text_parts.push(text.to_string());
                    }
                }
            }
        }
        Ok(text_parts.join(""))
    }

    /// Extract usage from Responses API response
    fn extract_responses_usage(
        response_json: &Value,
    ) -> OpenAiResult<crate::llm::anthropic_types::Usage> {
        use crate::llm::anthropic_types::Usage;
        let usage = response_json["usage"]
            .as_object()
            .ok_or(OpenAiError::MissingField { field: "usage" })?;
        Ok(Usage {
            input_tokens: usage.get("input_tokens").and_then(|v| v.as_u64()).ok_or(
                OpenAiError::MissingField {
                    field: "usage.input_tokens",
                },
            )? as u32,
            output_tokens: usage.get("output_tokens").and_then(|v| v.as_u64()).ok_or(
                OpenAiError::MissingField {
                    field: "usage.output_tokens",
                },
            )? as u32,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        })
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
                                            let response_id = match Self::require_stream_string(
                                                &value["response"]["id"],
                                                "response.id",
                                            ) {
                                                Ok(response_id) => response_id.to_string(),
                                                Err(err) => {
                                                    return Some((
                                                        Err(LlmProviderError::OpenAi(err)),
                                                        (stream, state),
                                                    ));
                                                }
                                            };
                                            state.message_started = true;
                                            state.response_id = Some(response_id.clone());
                                            state.pending_events.push_back(
                                                StreamEvent::MessageStart {
                                                    message: MessageStartData {
                                                        id: response_id,
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
                                            let response_id = match Self::require_stream_string(
                                                &value["response_id"],
                                                "response_id",
                                            ) {
                                                Ok(response_id) => response_id.to_string(),
                                                Err(err) => {
                                                    return Some((
                                                        Err(LlmProviderError::OpenAi(err)),
                                                        (stream, state),
                                                    ));
                                                }
                                            };
                                            state.message_started = true;
                                            state.response_id = Some(response_id.clone());
                                            state.pending_events.push_back(
                                                StreamEvent::MessageStart {
                                                    message: MessageStartData {
                                                        id: response_id,
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
                                        let Some(item_type) = item["type"].as_str() else {
                                            continue;
                                        };

                                        if item_type == "reasoning" {
                                            // Extract reasoning item metadata
                                            let Some(item_id) = item["id"].as_str() else {
                                                continue;
                                            };
                                            let encrypted_content = item["encrypted_content"]
                                                .as_str()
                                                .map(|s| s.to_string());

                                            state.active_reasoning = Some(ActiveReasoning {
                                                item_id: item_id.to_string(),
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
                                                                        item_id: Some(
                                                                            item_id.to_string(),
                                                                        ),
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
                                            let Some(item_id) = item["id"].as_str() else {
                                                continue;
                                            };
                                            let Some(name) = item["name"].as_str() else {
                                                continue;
                                            };

                                            let index = state.next_block_index;
                                            state.next_block_index =
                                                state.next_block_index.saturating_add(1);
                                            state
                                                .tool_block_index_by_item_id
                                                .insert(item_id.to_string(), index);
                                            state.active_tool_item_id = Some(item_id.to_string());
                                            state.open_tool_block_indices.insert(index);

                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStart {
                                                    index,
                                                    content_block: ContentBlockStart::ToolUse {
                                                        id: item_id.to_string(),
                                                        name: name.to_string(),
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
                                                let Some(index) =
                                                    state.active_reasoning_block_index
                                                else {
                                                    continue;
                                                };
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
                                        let Some(item_type) = item["type"].as_str() else {
                                            continue;
                                        };

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
                                            let Some(item_id) = item["id"].as_str() else {
                                                continue;
                                            };
                                            if let Some(index) = state
                                                .tool_block_index_by_item_id
                                                .get(item_id)
                                                .copied()
                                            {
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockStop { index },
                                                );
                                                state.open_tool_block_indices.remove(&index);
                                            }
                                            if state.active_tool_item_id.as_deref() == Some(item_id)
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

                                        let usage = match Self::parse_responses_stream_usage(&value)
                                        {
                                            Ok(usage) => usage,
                                            Err(err) => {
                                                return Some((
                                                    Err(LlmProviderError::OpenAi(err)),
                                                    (stream, state),
                                                ));
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
                                                let index_from_item = value
                                                    .get("item_id")
                                                    .and_then(|v| v.as_str())
                                                    .and_then(|id| {
                                                        state
                                                            .tool_block_index_by_item_id
                                                            .get(id)
                                                            .copied()
                                                    });
                                                let index = match index_from_item {
                                                    Some(index) => Some(index),
                                                    None => state
                                                        .active_tool_item_id
                                                        .as_deref()
                                                        .and_then(|id| {
                                                            state
                                                                .tool_block_index_by_item_id
                                                                .get(id)
                                                                .copied()
                                                        }),
                                                };
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
    fn provider_name(&self) -> &'static str {
        "openai"
    }

    /// Non-streaming call (Anthropic native interface)
    async fn call(
        &self,
        request: crate::llm::anthropic_types::CreateMessageRequest,
    ) -> LlmProviderResult<crate::llm::anthropic_types::Message> {
        use crate::llm::anthropic_types::{ContentBlock, Message, MessageRole};

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
            let txt = Self::read_error_body(resp).await;
            return Err(LlmProviderError::from(
                self.handle_error_response(status, &txt),
            ));
        }
        let json: Value = resp
            .json()
            .await
            .map_err(|source| LlmProviderError::OpenAi(OpenAiError::Http { source }))?;

        // Parse response based on API type
        let (content, usage, message_id) = if use_responses {
            // Parse Responses API format
            let text = Self::extract_responses_text(&json)?;
            let usage = Self::extract_responses_usage(&json)?;
            let id = json["id"]
                .as_str()
                .ok_or(OpenAiError::MissingField { field: "id" })?
                .to_string();
            (text, usage, id)
        } else {
            let (text, usage, id) = Self::parse_chat_completion_message(&json)?;
            (text, usage, id)
        };

        let message = Message {
            id: message_id,
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
            let txt = Self::read_error_body(resp).await;
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
                                    let message_id =
                                        match Self::require_stream_string(&value["id"], "id") {
                                            Ok(message_id) => message_id.to_string(),
                                            Err(err) => {
                                                return Some((
                                                    Err(LlmProviderError::OpenAi(err)),
                                                    (stream, state),
                                                ));
                                            }
                                        };
                                    state.message_started = true;
                                    state.pending_events.push_back(StreamEvent::MessageStart {
                                        message: MessageStartData {
                                            id: message_id,
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
                                        let Some(raw_index) =
                                            tc.get("index").and_then(|v| v.as_u64())
                                        else {
                                            continue;
                                        };
                                        let raw_index = raw_index as usize;
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
                                                let Some(id) =
                                                    tc.get("id").and_then(|v| v.as_str())
                                                else {
                                                    continue;
                                                };
                                                state.tool_use_started.insert(event_index);
                                                state.pending_events.push_back(
                                                    StreamEvent::ContentBlockStart {
                                                        index: event_index,
                                                        content_block: ContentBlockStart::ToolUse {
                                                            id: id.to_string(),
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
                                            state.tool_use_started.insert(event_index);
                                            state.pending_events.push_back(
                                                StreamEvent::ContentBlockStart {
                                                    index: event_index,
                                                    content_block: ContentBlockStart::ToolUse {
                                                        id: "legacy_function_call".to_string(),
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
            let error_text = Self::read_error_body(response).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::anthropic_types::{CreateMessageRequest, MessageParam};

    #[test]
    fn test_build_openai_responses_body_includes_developer_messages() {
        let request = CreateMessageRequest {
            model: "gpt-5".to_string(),
            messages: vec![MessageParam::user("hello")],
            max_tokens: 128,
            system: Some(crate::llm::anthropic_types::SystemPrompt::Text(
                "core instructions".to_string(),
            )),
            developer_context: Some(vec!["env block".to_string(), "rule block".to_string()]),
            tools: None,
            temperature: None,
            stop_sequences: None,
            stream: false,
            top_p: None,
            top_k: None,
            metadata: None,
            thinking: None,
        };

        let body = build_openai_responses_body(&request, false, false, None);
        let input = body["input"].as_array().unwrap();

        assert_eq!(body["instructions"], "core instructions");
        assert_eq!(input[0]["type"], "message");
        assert_eq!(input[0]["role"], "developer");
        assert_eq!(input[0]["content"][0]["text"], "env block");
        assert_eq!(input[1]["type"], "message");
        assert_eq!(input[1]["role"], "developer");
        assert_eq!(input[2]["type"], "message");
        assert_eq!(input[2]["role"], "user");
    }

    #[test]
    fn test_build_openai_chat_body_merges_instruction_layers() {
        let request = CreateMessageRequest {
            model: "gpt-4o".to_string(),
            messages: vec![MessageParam::user("hello")],
            max_tokens: 128,
            system: Some(crate::llm::anthropic_types::SystemPrompt::Text(
                "core instructions".to_string(),
            )),
            developer_context: Some(vec!["env block".to_string(), "rule block".to_string()]),
            tools: None,
            temperature: None,
            stop_sequences: None,
            stream: false,
            top_p: None,
            top_k: None,
            metadata: None,
            thinking: None,
        };

        let body = build_openai_chat_body(&request, false);
        let messages = body["messages"].as_array().unwrap();

        assert_eq!(messages[0]["role"], "system");
        assert_eq!(
            messages[0]["content"],
            "core instructions\n\nenv block\n\nrule block"
        );
    }
}
