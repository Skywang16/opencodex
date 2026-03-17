use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::agent::core::context::{TaskContext, TaskExecutionRequest};
use crate::agent::error::{ToolExecutorError, ToolExecutorResult};
use crate::agent::tools::metadata::{ExecutionMode, ToolCategory, ToolMetadata, ToolPriority};
use crate::agent::tools::{
    RunnableTool, ToolDescriptionContext, ToolResult, ToolResultContent, ToolResultStatus,
};
use crate::agent::types::{Block, SubtaskBlock, SubtaskStatus};

#[derive(Default)]
pub struct TaskTool;

impl TaskTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RunnableTool for TaskTool {
    fn name(&self) -> &str {
        "task"
    }

    fn description(&self) -> &str {
        "Run a delegated task with an authorized agent profile. Each delegated task runs as a child agent session."
    }

    fn description_with_context(&self, context: &ToolDescriptionContext) -> Option<String> {
        if context.allowed_task_profiles.is_empty() {
            return Some(
                "Task workflow fan-out is unavailable because no helper profiles are authorized for this agent."
                    .to_string(),
            );
        }

        Some(format!(
            "Run a delegated task using an authorized child-agent profile. Allowed profiles: {}.",
            context.allowed_task_profiles.join(", ")
        ))
    }

    fn parameters_schema(&self) -> Value {
        json!({
          "type": "object",
          "properties": {
            "description": { "type": "string", "description": "Short label (3-5 words)" },
            "prompt": { "type": "string", "description": "Full instructions for the task workflow" },
            "profile": { "type": "string", "description": "Execution profile (explore/general/research/bulk_edit)" },
            "model": { "type": "string", "description": "Optional model id override for this workflow. Use a faster/cheaper model for simple tasks." },
            "use_worktree": { "type": "boolean", "description": "Create an isolated git worktree when this workflow should materialize a separate execution node. Defaults to false." },
            "session_id": { "type": "number", "description": "Optional existing backing session id for the workflow" },
            "call_id": { "type": "string", "description": "Tool call id (injected by runtime)" }
          },
          "required": ["description", "prompt", "profile"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Delegation, ToolPriority::Expensive)
            .with_execution_mode(ExecutionMode::Parallel)
            .with_timeout(Duration::from_secs(30 * 60))
            .with_summary_key_arg("description")
            .with_tags(vec!["ui:hidden".into(), "orchestration".into()])
    }

    async fn run(&self, context: &TaskContext, args: Value) -> ToolExecutorResult<ToolResult> {
        let description = required_string_arg(&args, "description")?;
        let prompt = required_string_arg(&args, "prompt")?;
        let profile = required_string_arg(&args, "profile")?;
        let model_id = args
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let use_worktree = args
            .get("use_worktree")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let session_id = args.get("session_id").and_then(|v| v.as_i64());
        let call_id = args
            .get("call_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let persistence = context.agent_persistence();
        let parent_session = persistence
            .sessions()
            .get(context.session_id)
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: "task".to_string(),
                error: e.to_string(),
            })?
            .ok_or_else(|| ToolExecutorError::ExecutionFailed {
                tool_name: "task".to_string(),
                error: format!("parent session {} not found", context.session_id),
            })?;

        let child_session_id = match session_id {
            Some(id) => id,
            None => {
                let created = persistence
                    .sessions()
                    .create(
                        &parent_session.workspace_path,
                        Some(&description),
                        &profile,
                        Some(parent_session.id),
                        Some(&call_id),
                        parent_session.model_id.as_deref(),
                        parent_session.provider_id.as_deref(),
                    )
                    .await
                    .map_err(|e| ToolExecutorError::ExecutionFailed {
                        tool_name: "task".to_string(),
                        error: e.to_string(),
                    })?;
                created.id
            }
        };

        context
            .assistant_append_block(Block::Subtask(SubtaskBlock {
                id: call_id.clone(),
                child_session_id,
                agent_type: profile.clone(),
                description: description.clone(),
                status: SubtaskStatus::Running,
                summary: None,
            }))
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: "task".to_string(),
                error: e.to_string(),
            })?;

        let request = TaskExecutionRequest {
            description: description.clone(),
            prompt: prompt.clone(),
            profile: profile.clone(),
            session_id: Some(child_session_id),
            call_id: Some(call_id.clone()),
            model_id: model_id.clone(),
            use_worktree,
        };

        let response = match context
            .task_execution_runner()
            .run_task_execution(context, request)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if let Err(update_err) = context
                    .assistant_update_block(
                        &call_id,
                        Block::Subtask(SubtaskBlock {
                            id: call_id.clone(),
                            child_session_id,
                            agent_type: profile.clone(),
                            description: description.clone(),
                            status: SubtaskStatus::Error,
                            summary: Some(e.to_string()),
                        }),
                    )
                    .await
                {
                    tracing::warn!(
                        "Failed to update task block '{}' after task execution error: {}",
                        call_id,
                        update_err
                    );
                }

                return Err(ToolExecutorError::ExecutionFailed {
                    tool_name: "task".to_string(),
                    error: e.to_string(),
                });
            }
        };

        context
            .assistant_update_block(
                &call_id,
                Block::Subtask(SubtaskBlock {
                    id: call_id.clone(),
                    child_session_id: response.session_id,
                    agent_type: profile.clone(),
                    description: description.clone(),
                    status: response.status.clone(),
                    summary: response.summary.clone(),
                }),
            )
            .await
            .map_err(|e| ToolExecutorError::ExecutionFailed {
                tool_name: "task".to_string(),
                error: e.to_string(),
            })?;

        let response_status = response.status.clone();
        let response_status_label = format!("{response_status:?}");
        let summary = response
            .summary
            .clone()
            .filter(|summary| !summary.trim().is_empty());
        let (status, cancel_reason, content) = match (response_status, summary) {
            (SubtaskStatus::Completed, Some(summary)) => (
                ToolResultStatus::Success,
                None,
                ToolResultContent::Success(summary),
            ),
            (SubtaskStatus::Completed, None) => (
                ToolResultStatus::Error,
                None,
                ToolResultContent::Error(
                    "Task execution finished without returning the required summary.".to_string(),
                ),
            ),
            (SubtaskStatus::Cancelled, Some(summary)) => (
                ToolResultStatus::Cancelled,
                Some("cancelled".to_string()),
                ToolResultContent::Error(summary),
            ),
            (SubtaskStatus::Cancelled, None) => (
                ToolResultStatus::Cancelled,
                Some("cancelled".to_string()),
                ToolResultContent::Error("Task execution was cancelled.".to_string()),
            ),
            (SubtaskStatus::Pending, Some(summary))
            | (SubtaskStatus::Running, Some(summary))
            | (SubtaskStatus::Error, Some(summary)) => (
                ToolResultStatus::Error,
                None,
                ToolResultContent::Error(summary),
            ),
            (SubtaskStatus::Pending, None)
            | (SubtaskStatus::Running, None)
            | (SubtaskStatus::Error, None) => (
                ToolResultStatus::Error,
                None,
                ToolResultContent::Error(format!(
                    "Task execution returned status {response_status_label} without a summary."
                )),
            ),
        };

        Ok(ToolResult {
            content: vec![content],
            status,
            cancel_reason,
            execution_time_ms: None,
            ext_info: Some(json!({
                "child_session_id": response.session_id,
                "profile": profile,
            })),
        })
    }
}

fn required_string_arg(args: &Value, key: &str) -> ToolExecutorResult<String> {
    let value = args
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolExecutorError::InvalidArguments {
            tool_name: "task".to_string(),
            error: format!("task requires non-empty `{key}`"),
        })?;
    Ok(value.to_string())
}
