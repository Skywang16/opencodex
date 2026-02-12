/*!
 * Task status query and management
 */

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;

use crate::agent::core::context::TaskContext;
use crate::agent::core::executor::{FileContextStatus, TaskExecutor, TaskSummary};
use crate::agent::error::{TaskExecutorError, TaskExecutorResult};

impl TaskExecutor {
    /// Get task summary information
    pub async fn get_task_summary(&self, task_id: &str) -> TaskExecutorResult<TaskSummary> {
        let ctx = self
            .active_tasks()
            .get(task_id)
            .map(|entry| Arc::clone(entry.value()))
            .ok_or_else(|| TaskExecutorError::TaskNotFound(task_id.to_string()))?;

        let (status, current_iteration, error_count, created_at, updated_at) = ctx
            .batch_read_state(|exec| {
                (
                    exec.runtime_status,
                    exec.current_iteration as i64,
                    exec.error_count as i64,
                    Utc::now(),
                    Utc::now(),
                )
            })
            .await;

        Ok(TaskSummary {
            task_id: task_id.to_string(),
            session_id: ctx.session_id,
            status: format!("{status:?}").to_lowercase(),
            current_iteration: current_iteration as i32,
            error_count: error_count as i32,
            created_at: created_at.to_rfc3339(),
            updated_at: updated_at.to_rfc3339(),
        })
    }

    /// List all active tasks
    pub async fn list_active_tasks(&self) -> Vec<TaskSummary> {
        let mut summaries = Vec::new();

        for entry in self.active_tasks().iter() {
            let task_id = entry.key();
            if let Ok(summary) = self.get_task_summary(task_id).await {
                summaries.push(summary);
            }
        }

        summaries
    }

    /// Get session context (search from active tasks)
    pub async fn get_session_context(&self, session_id: i64) -> Option<Arc<TaskContext>> {
        // Iterate through active tasks to find matching session_id
        for entry in self.active_tasks().iter() {
            if entry.value().session_id == session_id {
                return Some(Arc::clone(entry.value()));
            }
        }
        None
    }

    /// Get file context status
    pub async fn get_file_context_status(
        &self,
        session_id: i64,
    ) -> TaskExecutorResult<FileContextStatus> {
        let ctx = self.get_session_context(session_id).await.ok_or_else(|| {
            TaskExecutorError::InternalError(format!(
                "No active context found for session {session_id}"
            ))
        })?;

        let workspace_path = ctx.cwd.to_string();
        let files: Vec<String> = ctx
            .file_tracker()
            .get_active_files()
            .await
            .map_err(|e| TaskExecutorError::InternalError(e.to_string()))?
            .into_iter()
            .map(|entry| {
                let absolute =
                    Self::workspace_relative_to_absolute(&workspace_path, &entry.relative_path);
                absolute.to_string_lossy().replace('\\', "/")
            })
            .collect();

        Ok(FileContextStatus {
            workspace_path,
            file_count: files.len(),
            files,
        })
    }

    /// Clean up completed tasks (free memory)
    pub async fn cleanup_completed_tasks(&self) -> usize {
        let mut removed = 0;

        // Collect task_ids that need cleanup
        let to_remove: Vec<String> = self
            .active_tasks()
            .iter()
            .filter_map(|entry| {
                let status = entry
                    .value()
                    .states
                    .execution
                    .try_read()
                    .ok()
                    .map(|exec| exec.runtime_status);

                if let Some(status) = status {
                    use crate::agent::core::status::AgentTaskStatus;
                    if matches!(
                        status,
                        AgentTaskStatus::Completed
                            | AgentTaskStatus::Cancelled
                            | AgentTaskStatus::Error
                    ) {
                        return Some(entry.key().clone());
                    }
                }
                None
            })
            .collect();

        // Remove
        for task_id in to_remove {
            self.active_tasks().remove(&task_id);
            removed += 1;
        }

        removed
    }

    /// Get total task count statistics
    pub fn get_stats(&self) -> TaskExecutorStats {
        TaskExecutorStats {
            active_tasks: self.active_tasks().len(),
        }
    }

    /// Task list: new design only exposes active tasks in memory (does not persist task records).
    pub async fn list_tasks(
        &self,
        session_id: Option<i64>,
        status_filter: Option<String>,
    ) -> TaskExecutorResult<Vec<TaskSummary>> {
        let mut summaries = Vec::new();

        for entry in self.active_tasks().iter() {
            let ctx = entry.value();
            if let Some(target_session) = session_id {
                if ctx.session_id != target_session {
                    continue;
                }
            }

            let summary = self.get_task_summary(entry.key()).await?;
            if let Some(filter) = &status_filter {
                if summary.status != *filter {
                    continue;
                }
            }
            summaries.push(summary);
        }

        Ok(summaries)
    }

    pub(crate) fn workspace_relative_to_absolute(
        workspace_path: &str,
        stored_path: &str,
    ) -> PathBuf {
        let stored = Path::new(stored_path);
        if stored.is_absolute() {
            return stored.to_path_buf();
        }
        PathBuf::from(workspace_path).join(stored)
    }
}

/// Task executor statistics
#[derive(Debug, Clone)]
pub struct TaskExecutorStats {
    pub active_tasks: usize,
}
