use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{interval, MissedTickBehavior};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::llm::{
    anthropic_types::{CreateMessageRequest, Message, MessageContent, MessageParam, StreamEvent},
    error::{LlmError, LlmProviderResult, LlmResult},
    provider_registry::ProviderRegistry,
    retry::{error_retry_reason, is_retryable_error, retry_async, RetryConfig},
    types::{EmbeddingRequest, EmbeddingResponse, LLMProviderConfig, OAuthRuntimeConfig},
};
use crate::storage::repositories::{AIModels, AuthType};
use crate::storage::DatabaseManager;

pub struct LLMService {
    database: Arc<DatabaseManager>,
}

impl LLMService {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    /// Get Provider config and model name: model_id → (config, model_name)
    async fn get_provider_config_and_model(
        &self,
        model_id: &str,
    ) -> LlmResult<(LLMProviderConfig, String)> {
        let model = AIModels::new(&self.database)
            .find_by_id(model_id)
            .await?
            .ok_or_else(|| LlmError::ModelNotFound {
                model_id: model_id.to_string(),
            })?;

        let provider_type = model.provider.as_str().to_string();

        if !ProviderRegistry::global().supports(&provider_type) {
            return Err(LlmError::UnsupportedProvider {
                provider: provider_type.clone(),
            });
        }

        let options = match model.options {
            Some(value) => Some(
                serde_json::from_value::<std::collections::HashMap<String, serde_json::Value>>(
                    value,
                )
                .map_err(|source| LlmError::OptionsParse { source })?,
            ),
            None => None,
        };

        // Build config based on authentication type
        let (api_key, oauth_config) = match model.auth_type {
            AuthType::OAuth => {
                // OAuth authentication: use access_token as Bearer token
                let oauth = model.oauth_config.ok_or_else(|| LlmError::Configuration {
                    message: "OAuth configuration is required for OAuth models".to_string(),
                })?;

                let access_token = oauth.access_token.ok_or_else(|| LlmError::Configuration {
                    message: "OAuth access token is missing. Please re-authorize.".to_string(),
                })?;

                // Check if token has expired
                if let Some(expires_at) = oauth.expires_at {
                    let now = chrono::Utc::now().timestamp();
                    if now >= expires_at {
                        return Err(LlmError::Configuration {
                            message:
                                "OAuth access token has expired. Please refresh or re-authorize."
                                    .to_string(),
                        });
                    }
                }

                let runtime_config = OAuthRuntimeConfig {
                    provider: oauth.provider.to_string(),
                    access_token: access_token.clone(),
                    refresh_token: oauth.refresh_token,
                    expires_at: oauth.expires_at,
                };

                (access_token, Some(runtime_config))
            }
            AuthType::ApiKey => {
                // API Key authentication
                let api_key = model.api_key.ok_or_else(|| LlmError::Configuration {
                    message: "API key is required".to_string(),
                })?;
                (api_key, None)
            }
        };

        let config = LLMProviderConfig {
            provider_type,
            api_key,
            api_url: model.api_url.filter(|url| !url.is_empty()),
            options,
            oauth_config,
        };

        Ok((config, model.model))
    }

    /// Non-streaming call with automatic retry
    pub async fn call(&self, request: CreateMessageRequest) -> LlmResult<Message> {
        self.validate_request(&request)?;

        let (config, model_name) = self.get_provider_config_and_model(&request.model).await?;

        let provider = ProviderRegistry::global()
            .create(config.clone())
            .map_err(LlmError::from)?;

        let mut actual_request = request;
        actual_request.model = model_name.clone();

        // Anthropic provider automatically applies prompt cache optimization
        if config.provider_type == "anthropic" {
            actual_request = crate::llm::providers::anthropic::apply_prompt_caching(actual_request);
        }

        let retry_config = RetryConfig::default();

        let result = retry_async(
            retry_config,
            || {
                let provider = provider.clone();
                let req = actual_request.clone();
                async move { provider.call(req).await }
            },
            |e| {
                let retryable = is_retryable_error(e);
                if retryable {
                    tracing::debug!(
                        "🔄 Retryable error detected ({}): {}",
                        error_retry_reason(e),
                        e
                    );
                }
                retryable
            },
        )
        .await;

        result.map_err(LlmError::from)
    }

    /// Single-attempt streaming call (with cancellation token).
    ///
    /// Does NOT retry internally — the caller (e.g. ReactOrchestrator) owns
    /// the retry loop so it can emit progress events between attempts.
    pub async fn call_stream(
        &self,
        request: CreateMessageRequest,
        token: CancellationToken,
    ) -> LlmResult<impl tokio_stream::Stream<Item = LlmProviderResult<StreamEvent>>> {
        self.validate_request(&request)?;

        let (config, model_name) = self.get_provider_config_and_model(&request.model).await?;

        tracing::info!("🚀 Starting LLM stream call: model={}", model_name);

        let provider = ProviderRegistry::global()
            .create(config.clone())
            .map_err(LlmError::from)?;

        let model_for_logs = model_name.clone();
        let mut actual_request = request;
        actual_request.model = model_name;

        if config.provider_type == "anthropic" {
            actual_request = crate::llm::providers::anthropic::apply_prompt_caching(actual_request);
        }

        let stream = provider
            .call_stream(actual_request)
            .await
            .map_err(LlmError::from)?;

        let stream_with_cancel = tokio_stream::wrappers::ReceiverStream::new({
            let (tx, rx) = tokio::sync::mpsc::channel(10);
            let mut stream = Box::pin(stream);
            let start = Instant::now();

            tokio::spawn(async move {
                let mut event_count: u64 = 0;
                let mut first_event_logged = false;
                let mut idle_ticks: u32 = 0;
                let mut idle_interval = interval(Duration::from_secs(10));
                idle_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

                loop {
                    tokio::select! {
                        _ = token.cancelled() => {
                            tracing::warn!(
                                "⏸️  LLM stream cancelled (events={}, elapsed_ms={}, model={})",
                                event_count,
                                start.elapsed().as_millis(),
                                model_for_logs
                            );
                            break;
                        }
                        _ = idle_interval.tick(), if !first_event_logged => {
                            idle_ticks = idle_ticks.saturating_add(1);
                            if idle_ticks == 1 || idle_ticks % 3 == 0 {
                                tracing::warn!(
                                    "⌛ LLM stream has no events after {}s (model={})",
                                    idle_ticks.saturating_mul(10),
                                    model_for_logs
                                );
                            }
                        }
                        item = stream.next() => {
                            if let Some(item) = item {
                                event_count = event_count.saturating_add(1);
                                if !first_event_logged {
                                    first_event_logged = true;
                                    tracing::info!(
                                        "📡 LLM stream first event after {}ms (model={})",
                                        start.elapsed().as_millis(),
                                        model_for_logs
                                    );
                                }
                                if event_count <= 3 || event_count % 100 == 0 {
                                    match &item {
                                        Ok(event) => {
                                            tracing::debug!(
                                                "📡 LLM stream event #{}: {}",
                                                event_count,
                                                stream_event_kind(event)
                                            );
                                        }
                                        Err(err) => {
                                            tracing::warn!(
                                                "⚠️  LLM stream event error #{}: {}",
                                                event_count,
                                                err
                                            );
                                        }
                                    }
                                }
                                if tx.send(item).await.is_err() {
                                    break;
                                }
                            } else {
                                tracing::info!(
                                    "✅ LLM stream completed (events={}, elapsed_ms={}, model={})",
                                    event_count,
                                    start.elapsed().as_millis(),
                                    model_for_logs
                                );
                                break;
                            }
                        }
                    }
                }
            });
            rx
        });

        Ok(stream_with_cancel)
    }

    /// Embedding call with automatic retry
    pub async fn create_embeddings(
        &self,
        request: EmbeddingRequest,
    ) -> LlmResult<EmbeddingResponse> {
        let (config, model_name) = self.get_provider_config_and_model(&request.model).await?;

        let provider = ProviderRegistry::global()
            .create(config)
            .map_err(LlmError::from)?;

        let mut actual_request = request;
        actual_request.model = model_name;

        let retry_config = RetryConfig::default();

        let result = retry_async(
            retry_config,
            || {
                let provider = provider.clone();
                let req = actual_request.clone();
                async move { provider.create_embeddings(req).await }
            },
            |e| {
                let retryable = is_retryable_error(e);
                if retryable {
                    tracing::debug!(
                        "🔄 Retryable embedding error detected ({}): {}",
                        error_retry_reason(e),
                        e
                    );
                }
                retryable
            },
        )
        .await;

        result.map_err(LlmError::from)
    }

    /// Get list of available models
    pub async fn get_available_models(&self) -> LlmResult<Vec<String>> {
        let ai_models = AIModels::new(&self.database);
        let models = ai_models.find_all().await?;

        Ok(models.into_iter().map(|m| m.id).collect())
    }

    /// Test model connection (construct minimal Anthropic CreateMessageRequest)
    pub async fn test_model_connection(&self, model_id: &str) -> LlmResult<bool> {
        let test_request = CreateMessageRequest {
            model: model_id.to_string(),
            messages: vec![MessageParam {
                role: crate::llm::anthropic_types::MessageRole::User,
                content: MessageContent::Text("Hello".to_string()),
            }],
            max_tokens: 10,
            system: None,
            tools: None,
            temperature: Some(0.1),
            stream: false,
            stop_sequences: None,
            top_p: None,
            top_k: None,
            metadata: None,
            thinking: None,
        };

        let result = self.call(test_request).await;
        match result {
            Ok(_) => Ok(true),
            Err(err) => {
                tracing::warn!("Model connection test failed for {}: {}", model_id, err);
                Ok(false)
            }
        }
    }

    /// Validate request parameters
    fn validate_request(&self, request: &CreateMessageRequest) -> LlmResult<()> {
        if request.model.is_empty() {
            return Err(LlmError::InvalidRequest {
                reason: "Model identifier cannot be empty".to_string(),
            });
        }

        if request.messages.is_empty() {
            return Err(LlmError::InvalidRequest {
                reason: "Message list cannot be empty".to_string(),
            });
        }

        if let Some(temp) = request.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(LlmError::InvalidRequest {
                    reason: "Temperature must be between 0.0 and 2.0".to_string(),
                });
            }
        }

        if request.max_tokens == 0 {
            return Err(LlmError::InvalidRequest {
                reason: "max_tokens must be greater than zero".to_string(),
            });
        }

        Ok(())
    }
}

fn stream_event_kind(event: &StreamEvent) -> &'static str {
    use crate::llm::anthropic_types::{ContentBlockStart, ContentDelta, StreamEvent};

    match event {
        StreamEvent::MessageStart { .. } => "message_start",
        StreamEvent::ContentBlockStart { content_block, .. } => match content_block {
            ContentBlockStart::Text { .. } => "content_block_start.text",
            ContentBlockStart::ToolUse { .. } => "content_block_start.tool_use",
            ContentBlockStart::Thinking { .. } => "content_block_start.thinking",
            ContentBlockStart::Unknown => "content_block_start.unknown",
        },
        StreamEvent::ContentBlockDelta { delta, .. } => match delta {
            ContentDelta::Text { .. } => "content_block_delta.text",
            ContentDelta::InputJson { .. } => "content_block_delta.input_json",
            ContentDelta::Thinking { .. } => "content_block_delta.thinking",
            ContentDelta::Signature { .. } => "content_block_delta.signature",
            ContentDelta::Unknown => "content_block_delta.unknown",
        },
        StreamEvent::ContentBlockStop { .. } => "content_block_stop",
        StreamEvent::MessageDelta { .. } => "message_delta",
        StreamEvent::MessageStop => "message_stop",
        StreamEvent::Ping => "ping",
        StreamEvent::Error { .. } => "error",
        StreamEvent::Unknown => "unknown",
    }
}
