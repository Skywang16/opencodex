/*!
 * ReAct Handler Trait - defines interface between TaskExecutor and ReactOrchestrator
 *
 */

use serde_json::Value;
use std::sync::Arc;

use crate::agent::context::ContextBuilder;
use crate::agent::core::context::{AgentToolCallResult, TaskContext};
use crate::agent::error::TaskExecutorResult;
use crate::agent::tools::ToolRegistry;
use crate::llm::anthropic_types::CreateMessageRequest;

/// ReAct executor interface
///
#[async_trait::async_trait]
pub trait ReactHandler {
    /// Build LLM request
    ///
    /// Note: uses references to avoid cloning
    async fn build_llm_request(
        &self,
        context: &TaskContext,
        model_id: &str,
        tool_registry: &ToolRegistry,
        cwd: &str,
        messages: Option<Vec<crate::llm::anthropic_types::MessageParam>>,
    ) -> TaskExecutorResult<CreateMessageRequest>;

    /// Execute tool calls
    ///
    /// Note: returns results instead of modifying state, more functional
    async fn execute_tools(
        &self,
        context: &TaskContext,
        iteration: u32,
        tool_calls: Vec<(String, String, Value)>,
    ) -> TaskExecutorResult<Vec<AgentToolCallResult>>;

    /// Get ContextBuilder
    ///
    /// Note: returns Arc to avoid cloning the builder itself
    async fn get_context_builder(&self, context: &TaskContext) -> Arc<ContextBuilder>;
}
