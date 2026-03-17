use std::path::PathBuf;
use std::sync::Arc;

use crate::agent::agents::AgentConfigLoader;
use crate::agent::common::truncate_chars;
use crate::agent::core::context::{
    TaskContext, TaskContextDeps, TaskExecutionRequest, TaskExecutionResponse,
};
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};
use crate::agent::permissions::PermissionDecision;
use crate::agent::persistence::{AgentNodeRole, CreateAgentNodeParams, RunStatus};
use crate::agent::tools::RunnableTool;
use crate::agent::types::{Block, MessageRole, SubtaskStatus};
use crate::git::service as git_service;
use crate::storage::repositories::AIModels;

use super::TaskExecutor;

const MAX_ACTIVE_SUBTASKS_GLOBAL: usize = 8;
const MAX_ACTIVE_SUBTASKS_PER_PARENT: usize = 3;

pub async fn run_subtask(
    executor: &TaskExecutor,
    parent: &TaskContext,
    request: TaskExecutionRequest,
) -> TaskExecutorResult<TaskExecutionResponse> {
    let workspace_root = PathBuf::from(parent.cwd.as_ref());

    let parent_session = executor
        .agent_persistence()
        .sessions()
        .get(parent.session_id)
        .await
        .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        .ok_or_else(|| {
            TaskExecutorError::StatePersistenceFailed(format!(
                "parent session {} not found",
                parent.session_id
            ))
        })?;

    let agent_configs = AgentConfigLoader::load_for_workspace(&workspace_root)
        .await
        .map_err(|e| {
            TaskExecutorError::ConfigurationError(format!("Failed to load agent configs: {e}"))
        })?;
    let Some(sub_cfg) = agent_configs.get(request.profile.as_str()) else {
        return Err(TaskExecutorError::ConfigurationError(format!(
            "Unknown task profile: {}",
            request.profile
        )));
    };

    let parent_cfg = agent_configs
        .get(parent.agent_type.as_ref())
        .ok_or_else(|| {
            TaskExecutorError::ConfigurationError(format!(
                "Parent agent config not found: {}",
                parent.agent_type
            ))
        })?;

    match parent_cfg.task_permission_for(&request.profile) {
        PermissionDecision::Deny => {
            return Err(TaskExecutorError::ConfigurationError(format!(
                "Agent '{}' is not allowed to delegate to task profile '{}'",
                parent.agent_type, request.profile
            )))
        }
        PermissionDecision::Ask | PermissionDecision::Allow => {}
    }

    if !matches!(
        sub_cfg.mode,
        crate::agent::agents::config::AgentMode::TaskProfile
    ) {
        return Err(TaskExecutorError::ConfigurationError(format!(
            "Agent {} is not a child-execution profile",
            sub_cfg.name
        )));
    }

    let active_subtasks_global = executor.active_child_executions_global();
    if active_subtasks_global >= MAX_ACTIVE_SUBTASKS_GLOBAL {
        return Err(TaskExecutorError::TooManyActiveSubtasksGlobal {
            current: active_subtasks_global,
            limit: MAX_ACTIVE_SUBTASKS_GLOBAL,
        });
    }

    let active_subtasks_for_parent =
        executor.active_child_executions_for_parent(parent.task_id.as_ref());
    if active_subtasks_for_parent >= MAX_ACTIVE_SUBTASKS_PER_PARENT {
        return Err(TaskExecutorError::TooManyActiveSubtasksPerParent {
            parent_task_id: parent.task_id.to_string(),
            current: active_subtasks_for_parent,
            limit: MAX_ACTIVE_SUBTASKS_PER_PARENT,
        });
    }

    let database = executor.database();
    let repo = AIModels::new(database.as_ref());

    let configured_model = sub_cfg.model_id.as_ref().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let config_file_model = repo
        .get_agent_model_binding(&request.profile, true)
        .await
        .map_err(|e| TaskExecutorError::ConfigurationError(e.to_string()))?;

    let initial_model = request
        .model_id
        .clone()
        .or(config_file_model.clone())
        .or(configured_model.clone())
        .or(parent_session.model_id.clone());

    let initial_provider = if let Some(model_id) = initial_model.as_deref() {
        repo.find_by_id(model_id)
            .await
            .map_err(|e| TaskExecutorError::ConfigurationError(e.to_string()))?
            .map(|model| model.provider)
    } else {
        parent_session.provider_id.clone()
    };

    let child_session_id = match request.session_id {
        Some(id) => id,
        None => {
            let spawned_by = request.call_id.as_deref();
            let created = executor
                .agent_persistence()
                .sessions()
                .create(
                    &parent_session.workspace_path,
                    Some(&request.description),
                    &request.profile,
                    Some(parent.session_id),
                    spawned_by,
                    initial_model.as_deref(),
                    initial_provider.as_deref(),
                )
                .await
                .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;
            created.id
        }
    };

    let child_session = executor
        .agent_persistence()
        .sessions()
        .get(child_session_id)
        .await
        .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?
        .ok_or_else(|| {
            TaskExecutorError::StatePersistenceFailed(format!(
                "child session {child_session_id} not found"
            ))
        })?;

    if child_session.parent_id != Some(parent.session_id) {
        return Err(TaskExecutorError::ConfigurationError(
            "Child session does not belong to parent".to_string(),
        ));
    }

    // Model priority: request override > child session > config file binding > profile frontmatter > parent session
    let model_id = request
        .model_id
        .clone()
        .or(child_session.model_id.clone())
        .or(config_file_model.clone())
        .or(configured_model.clone())
        .or(parent_session.model_id.clone())
        .ok_or_else(|| {
            TaskExecutorError::ConfigurationError(
                "No model_id set on session; cannot run subtask".to_string(),
            )
        })?;

    let model_provider = repo
        .find_by_id(&model_id)
        .await
        .map_err(|e| TaskExecutorError::ConfigurationError(e.to_string()))?
        .map(|model| model.provider);

    executor
        .agent_persistence()
        .sessions()
        .update_model_selection(child_session_id, &model_id, model_provider.as_deref())
        .await
        .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

    let effective = executor
        .settings_manager()
        .get_effective_settings(Some(workspace_root.clone()))
        .await
        .map_err(|e| TaskExecutorError::ConfigurationError(e.to_string()))?;

    let workspace_settings = executor
        .settings_manager()
        .get_workspace_settings(&workspace_root)
        .await
        .map_err(|e| TaskExecutorError::ConfigurationError(e.to_string()))?;
    if let Err(err) = executor
        .mcp_registry()
        .init_workspace_servers(&workspace_root, &effective, workspace_settings.as_ref())
        .await
    {
        tracing::warn!(
            workspace = %workspace_root.display(),
            "Failed to initialize MCP workspace servers for child execution: {}",
            err
        );
    }

    let mcp_tools = executor
        .mcp_registry()
        .get_tools_for_workspace(parent.cwd.as_ref())
        .into_iter()
        .map(|t| Arc::new(t) as Arc<dyn RunnableTool>)
        .collect::<Vec<_>>();

    // Tool access is controlled entirely by each agent's frontmatter configuration.
    // Level-1 (general) agents may spawn Level-2 functional agents via task permissions.
    // Level-2 functional agents have task excluded from their tool_filter already.
    let merged_tool_filter = sub_cfg.tool_filter.clone();

    // Initialize Skill system (shares the same workspace as parent)
    let skill_manager = {
        let manager = Arc::new(crate::agent::skill::SkillManager::new());
        let global_skills_dir = crate::config::paths::skills_dir();

        if let Err(e) = manager
            .discover_skills(Some(&global_skills_dir), Some(&workspace_root))
            .await
        {
            tracing::warn!("Failed to discover skills for child execution: {}", e);
        }

        Some(manager)
    };

    let tool_registry = crate::agent::tools::create_tool_registry(
        "agent",
        effective.permissions,
        Some(merged_tool_filter),
        executor.tool_confirmations(),
        mcp_tools,
        executor.vector_search_engine(),
        Some(executor.lsp_manager()),
        skill_manager,
    )
    .await;

    let task_id = format!("task_{}", uuid::Uuid::new_v4());

    // Worktree isolation: create a dedicated git worktree branch for this child execution.
    // This allows parallel execution branches to modify files without conflicts.
    let cwd = if request.use_worktree {
        let repo_root = match git_service::find_repo_root(parent.cwd.as_ref()).await {
            Some(repo_root) => repo_root,
            None => {
                tracing::warn!(
                    "Failed to find git repo root for child execution parent cwd '{}', using cwd directly",
                    parent.cwd
                );
                parent.cwd.to_string()
            }
        };
        let branch = format!("opencodex/task-{child_session_id}");
        let wt_dir = format!("{repo_root}/.git/opencodex-worktrees/{child_session_id}");
        match crate::git::service::GitService::worktree_add(&repo_root, &branch, &wt_dir).await {
            Ok(wt_path) => {
                if let Err(err) = executor
                    .agent_persistence()
                    .sessions()
                    .set_worktree_path(child_session_id, &wt_path)
                    .await
                {
                    tracing::warn!(
                        session_id = child_session_id,
                        worktree = %wt_path,
                        "Failed to persist child execution worktree path: {}",
                        err
                    );
                }
                tracing::info!(
                    session_id = child_session_id,
                    branch = %branch,
                    worktree = %wt_path,
                    "Created isolated worktree for child execution"
                );
                wt_path
            }
            Err(e) => {
                tracing::warn!(
                    session_id = child_session_id,
                    error = %e.message,
                    "Failed to create worktree, falling back to parent cwd"
                );
                parent.cwd.to_string()
            }
        }
    } else {
        parent.cwd.to_string()
    };

    let child_node = executor
        .agent_persistence()
        .agent_nodes()
        .create(CreateAgentNodeParams {
            run_id: parent.run_id,
            parent_node_id: Some(parent.node_id),
            backing_session_id: Some(child_session_id),
            trigger_tool_call_id: request.call_id.as_deref(),
            role: AgentNodeRole::Branch,
            profile: &request.profile,
            title: &request.description,
            status: RunStatus::Running,
            worktree_path: if request.use_worktree {
                Some(cwd.as_str())
            } else {
                child_session.worktree_path.as_deref()
            },
            model_id: Some(model_id.as_str()),
        })
        .await
        .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

    let progress_channel = parent.progress_channel().await;

    let ctx = TaskContext::new(crate::agent::core::context::TaskContextInit {
        task_id,
        session_id: child_session_id,
        run_id: parent.run_id,
        node_id: child_node.id,
        user_prompt: request.prompt.clone(),
        agent_type: request.profile.clone(),
        config: crate::agent::config::TaskExecutionConfig::default(),
        workspace_path: cwd,
        updates_run_status: false,
        emit_task_events: false,
        // Mixed-view design: stream child agent tool/message events on the same channel, but
        // persist them to the child session. The UI merges sessions into one timeline.
        progress_channel,
        deps: TaskContextDeps {
            tool_registry: Arc::clone(&tool_registry),
            repositories: executor.database(),
            agent_persistence: executor.agent_persistence(),
            checkpoint_service: executor.checkpoint_service(),
            workspace_changes: executor.workspace_changes(),
            task_execution_runner: Arc::new(executor.clone()),
        },
    })
    .await?;

    let ctx = Arc::new(ctx);

    if let Some((checkpoint_id, workspace_root)) = parent.active_checkpoint_handle().await {
        ctx.inherit_checkpoint(checkpoint_id, workspace_root).await;
    }

    let parent_cancel = parent.create_stream_cancel_token();
    let ctx_for_cancel = Arc::clone(&ctx);
    tokio::spawn(async move {
        parent_cancel.cancelled().await;
        ctx_for_cancel.abort();
    });

    executor
        .restore_session_history(&ctx, child_session_id, None)
        .await?;

    ctx.file_tracker().take_recent_agent_edits().await;
    ctx.initialize_message_track(&request.prompt, None, true)
        .await?;

    let prompts = executor
        .prompt_orchestrator()
        .build_task_prompts(
            ctx.session_id,
            ctx.task_id.to_string(),
            &request.prompt,
            ctx.agent_type.as_ref(),
            &ctx.cwd,
            &tool_registry,
            Some(&model_id),
        )
        .await?;
    ctx.set_system_prompt(prompts.instructions).await?;
    ctx.set_developer_context(prompts.developer_context).await?;
    ctx.add_user_message(prompts.user_prompt).await?;
    ctx.set_status(crate::agent::core::status::AgentTaskStatus::Running)
        .await?;

    // Child executions share the parent's event stream; they must also be reachable for tool confirmation
    // resolution (agent_tool_confirm looks up by task_id).
    executor
        .active_tasks()
        .insert(ctx.task_id.to_string(), Arc::clone(&ctx));
    executor.increment_active_child_executions_for_parent(parent.task_id.as_ref());

    let task_key = ctx.task_id.to_string();
    let run_result = executor.run_task_loop(Arc::clone(&ctx), model_id).await;
    executor.active_tasks().remove(&task_key);
    executor.decrement_active_child_executions_for_parent(parent.task_id.as_ref());

    let (status, runtime_error) = match run_result {
        Ok(()) => (SubtaskStatus::Completed, None),
        Err(TaskExecutorError::TaskInterrupted) | Err(TaskExecutorError::TaskCancelled(_)) => {
            (SubtaskStatus::Cancelled, None)
        }
        Err(e) => (SubtaskStatus::Error, Some(e)),
    };

    let messages = executor
        .agent_persistence()
        .messages()
        .list_by_session(child_session_id)
        .await
        .map_err(|e| TaskExecutorError::StatePersistenceFailed(e.to_string()))?;

    // Do NOT write partial output back into the parent context on cancellation.
    // Cancellation is an incomplete run; the parent task will backfill a real summary on next turn.
    let summary = match status {
        SubtaskStatus::Cancelled => None,
        SubtaskStatus::Error => match runtime_error {
            Some(error) => Some(error.to_string()),
            None => extract_last_assistant_text(&messages).map(|text| truncate_chars(&text, 1200)),
        },
        SubtaskStatus::Completed => {
            extract_last_assistant_text(&messages).map(|text| truncate_chars(&text, 2000))
        }
        SubtaskStatus::Pending | SubtaskStatus::Running => None,
    };

    Ok(TaskExecutionResponse {
        session_id: child_session_id,
        status,
        summary,
    })
}

fn extract_last_assistant_text(messages: &[crate::agent::types::Message]) -> Option<String> {
    for msg in messages.iter().rev() {
        if !matches!(msg.role, MessageRole::Assistant) {
            continue;
        }
        let mut parts = Vec::new();
        for block in &msg.blocks {
            match block {
                Block::Text(b) => {
                    let cleaned = clean_subtask_text(&b.content);
                    if !cleaned.trim().is_empty() {
                        parts.push(cleaned);
                    }
                }
                Block::Error(b) => {
                    let cleaned = clean_subtask_text(&b.message);
                    if !cleaned.trim().is_empty() {
                        parts.push(cleaned);
                    }
                }
                _ => {}
            }
        }
        let out = parts.join("\n").trim().to_string();
        if !out.is_empty() {
            return Some(out);
        }
    }
    None
}

fn clean_subtask_text(input: &str) -> String {
    input
        .lines()
        .filter(|line| {
            let t = line.trim();
            if t.is_empty() {
                return false;
            }
            // Drop pseudo tool tags that some models emit as plain text.
            if t.starts_with('<') && t.ends_with('>') {
                return false;
            }
            true
        })
        .collect::<Vec<_>>()
        .join("\n")
}
