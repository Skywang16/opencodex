/*!
 * Task status query and management
 */

use std::sync::Arc;

use chrono::Utc;

use crate::agent::core::executor::{TaskExecutor, TaskSummary};
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
}

/// Task executor statistics
#[derive(Debug, Clone)]
pub struct TaskExecutorStats {
    pub active_tasks: usize,
}
