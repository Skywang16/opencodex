use async_trait::async_trait;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::llm::anthropic_types::{CreateMessageRequest, Message, StreamEvent};
use crate::llm::error::{LlmProviderError, LlmProviderResult};
use crate::llm::types::{EmbeddingRequest, EmbeddingResponse};

/// LLM Provider unified interface
///
/// "Never break userspace": From the caller's perspective, only Anthropic types are visible.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Non-streaming call
    ///
    /// Accepts Anthropic CreateMessageRequest, returns Anthropic Message.
    /// Other providers need internal conversion, but interface must be pure Anthropic types.
    async fn call(&self, request: CreateMessageRequest) -> LlmProviderResult<Message>;

    /// Streaming call
    ///
    /// Accepts Anthropic CreateMessageRequest, returns Anthropic StreamEvent stream.
    ///
    /// Streaming events include:
    /// - MessageStart: message start, includes initial usage
    /// - ContentBlockStart/Delta/Stop: content blocks (text/tool calls)
    /// - MessageDelta: message-level updates (stop_reason, etc.)
    /// - MessageStop: completion
    async fn call_stream(
        &self,
        request: CreateMessageRequest,
    ) -> LlmProviderResult<Pin<Box<dyn Stream<Item = LlmProviderResult<StreamEvent>> + Send>>>;

    /// Embedding call
    ///
    /// Generate vector representation of text for semantic search and similarity calculation.
    /// If provider doesn't support embedding, should return NotImplemented error.
    ///
    /// Note: Embedding doesn't use Anthropic types, as Anthropic doesn't provide embedding API.
    async fn create_embeddings(
        &self,
        _request: EmbeddingRequest,
    ) -> LlmProviderResult<EmbeddingResponse> {
        Err(LlmProviderError::UnsupportedOperation {
            provider: "unknown",
            operation: "embeddings",
        })
    }
}
