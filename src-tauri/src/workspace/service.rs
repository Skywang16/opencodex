use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;
use sqlx::{self, Row};
use tokio::task;

use crate::agent::persistence::AgentPersistence;
use crate::agent::types::{Block, Message};
use crate::storage::DatabaseManager;

use super::error::{WorkspaceError, WorkspaceResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub path: String,
    pub display_name: Option<String>,
    pub active_session_id: Option<i64>,
    pub selected_run_action_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_accessed_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunActionRecord {
    pub id: String,
    pub workspace_path: String,
    pub name: String,
    pub command: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub id: i64,
    pub workspace_path: String,
    pub title: Option<String>,
    pub message_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct WorkspaceService {
    database: Arc<DatabaseManager>,
    agent_persistence: Arc<AgentPersistence>,
}

impl WorkspaceService {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        let persistence = Arc::new(AgentPersistence::new(Arc::clone(&database)));
        Self {
            database,
            agent_persistence: persistence,
        }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.database.pool()
    }

    fn now_timestamp() -> i64 {
        Utc::now().timestamp()
    }

    async fn normalize_path(&self, path: &str) -> WorkspaceResult<String> {
        if path.is_empty() || path.trim().is_empty() {
            return Err(WorkspaceError::invalid_path("Path cannot be empty"));
        }
        let original = path.to_string();
        task::spawn_blocking(move || -> WorkspaceResult<String> {
            let candidate = PathBuf::from(&original);
            let canonical = if candidate.exists() {
                std::fs::canonicalize(&candidate).map_err(|e| {
                    WorkspaceError::invalid_path(format!("Canonicalize failed: {}", e))
                })?
            } else {
                candidate
            };
            path_to_string(&canonical)
        })
        .await
        .map_err(|e| WorkspaceError::internal(format!("Task join error: {}", e)))?
    }

    pub async fn get_or_create_workspace(&self, path: &str) -> WorkspaceResult<WorkspaceRecord> {
        let normalized = self.normalize_path(path).await?;
        let ts = Self::now_timestamp();
        sqlx::query(
            "INSERT INTO workspaces (path, display_name, active_session_id, created_at, updated_at, last_accessed_at)
             VALUES (?, NULL, NULL, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET
                updated_at = excluded.updated_at,
                last_accessed_at = excluded.last_accessed_at",
        )
        .bind(&normalized)
        .bind(ts)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        self.get_workspace(&normalized)
            .await?
            .ok_or_else(|| WorkspaceError::workspace_not_found(&normalized))
    }

    pub async fn list_recent_workspaces(
        &self,
        limit: i64,
    ) -> WorkspaceResult<Vec<WorkspaceRecord>> {
        let rows = sqlx::query(
            "SELECT path, display_name, active_session_id, selected_run_action_id, created_at, updated_at, last_accessed_at
             FROM workspaces
             ORDER BY last_accessed_at DESC LIMIT ?",
        )
        .bind(limit.max(1))
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(build_workspace).collect())
    }

    pub async fn list_sessions(&self, workspace_path: &str) -> WorkspaceResult<Vec<SessionRecord>> {
        let normalized = self.normalize_path(workspace_path).await?;
        let rows = sqlx::query(
            "SELECT s.id, s.workspace_path, s.title, s.created_at, s.updated_at,
                    (SELECT COUNT(*) FROM messages WHERE session_id = s.id AND role = 'user') as message_count
             FROM sessions s
             WHERE s.workspace_path = ? AND s.title IS NOT NULL AND s.title != ''
             ORDER BY s.updated_at DESC, s.id DESC",
        )
        .bind(&normalized)
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(build_session).collect())
    }

    pub async fn create_session(
        &self,
        workspace_path: &str,
        title: Option<&str>,
    ) -> WorkspaceResult<SessionRecord> {
        let workspace = self.get_or_create_workspace(workspace_path).await?;
        let ts = Self::now_timestamp();
        let result = sqlx::query(
            "INSERT INTO sessions (workspace_path, title, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&workspace.path)
        .bind(title)
        .bind(ts)
        .bind(ts)
        .execute(self.pool())
        .await?;

        let id = result.last_insert_rowid();
        self.get_session(id)
            .await?
            .ok_or_else(|| WorkspaceError::session_not_found(id))
    }

    pub async fn get_active_session(
        &self,
        workspace_path: &str,
    ) -> WorkspaceResult<Option<SessionRecord>> {
        let workspace = self.get_or_create_workspace(workspace_path).await?;
        match workspace.active_session_id {
            Some(session_id) => self.get_session(session_id).await,
            None => Ok(None),
        }
    }

    pub async fn ensure_active_session(
        &self,
        workspace_path: &str,
    ) -> WorkspaceResult<SessionRecord> {
        self.ensure_active_session_with_title(workspace_path, "")
            .await
    }

    pub async fn ensure_active_session_with_title(
        &self,
        workspace_path: &str,
        title: &str,
    ) -> WorkspaceResult<SessionRecord> {
        if let Some(session) = self.get_active_session(workspace_path).await? {
            // If there is already an active session with a title, return directly
            if session
                .title
                .as_ref()
                .map(|t| !t.trim().is_empty())
                .unwrap_or(false)
            {
                return Ok(session);
            }
            // If active session has no title, update it
            if !title.trim().is_empty() {
                self.update_session_title(session.id, title).await?;
                return self
                    .get_session(session.id)
                    .await?
                    .ok_or_else(|| WorkspaceError::session_not_found(session.id));
            }
            return Ok(session);
        }

        // Create new session
        let title_opt = if title.trim().is_empty() {
            None
        } else {
            Some(title)
        };
        let created = self.create_session(workspace_path, title_opt).await?;
        self.set_active_session(workspace_path, Some(created.id))
            .await?;
        Ok(created)
    }

    async fn update_session_title(&self, session_id: i64, title: &str) -> WorkspaceResult<()> {
        let ts = Self::now_timestamp();
        sqlx::query("UPDATE sessions SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(ts)
            .bind(session_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn trim_session_messages(
        &self,
        workspace_path: &str,
        session_id: i64,
        message_id: i64,
    ) -> WorkspaceResult<()> {
        let normalized = self.normalize_path(workspace_path).await?;
        let session = self
            .get_session(session_id)
            .await?
            .ok_or_else(|| WorkspaceError::session_not_found(session_id))?;

        if session.workspace_path != normalized {
            return Err(WorkspaceError::session_workspace_mismatch(
                session_id,
                workspace_path,
            ));
        }

        let messages_to_delete = self
            .agent_persistence
            .messages()
            .list_messages_from(session_id, message_id)
            .await
            .map_err(|e| {
                WorkspaceError::internal(format!("List session messages failed: {}", e))
            })?;

        let mut child_session_ids = Vec::new();
        for msg in &messages_to_delete {
            for block in &msg.blocks {
                if let Block::Subtask(subtask) = block {
                    child_session_ids.push(subtask.child_session_id);
                }
            }
        }

        child_session_ids.sort();
        child_session_ids.dedup();
        for child_session_id in child_session_ids {
            delete_session_cascade(&self.agent_persistence, child_session_id).await?;
        }

        self.agent_persistence
            .messages()
            .delete_messages_from(session_id, message_id)
            .await
            .map_err(|e| {
                WorkspaceError::internal(format!("Trim session messages failed: {}", e))
            })?;

        sqlx::query(
            "UPDATE messages
             SET is_summary = 0
             WHERE session_id = ? AND id <= ?",
        )
        .bind(session_id)
        .bind(message_id)
        .execute(self.pool())
        .await?;

        self.refresh_session_title(session_id).await?;
        Ok(())
    }

    pub async fn set_active_session(
        &self,
        workspace_path: &str,
        session_id: Option<i64>,
    ) -> WorkspaceResult<()> {
        let normalized = self.normalize_path(workspace_path).await?;
        let ts = Self::now_timestamp();
        sqlx::query(
            "UPDATE workspaces
             SET active_session_id = ?, updated_at = ?, last_accessed_at = ?
             WHERE path = ?",
        )
        .bind(session_id)
        .bind(ts)
        .bind(ts)
        .bind(&normalized)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn get_session_messages(&self, session_id: i64) -> WorkspaceResult<Vec<Message>> {
        self.agent_persistence
            .messages()
            .list_by_session(session_id)
            .await
            .map_err(|e| WorkspaceError::internal(format!("Load session messages failed: {}", e)))
    }

    pub async fn delete_session(&self, session_id: i64) -> WorkspaceResult<()> {
        delete_session_cascade(&self.agent_persistence, session_id).await
    }

    pub async fn delete_workspace(&self, path: &str) -> WorkspaceResult<()> {
        let normalized = self.normalize_path(path).await?;
        sqlx::query("DELETE FROM workspaces WHERE path = ?")
            .bind(&normalized)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn maintain(
        &self,
        max_age_days: i64,
        max_entries: i64,
    ) -> WorkspaceResult<(u64, u64)> {
        let cutoff = Self::now_timestamp() - max_age_days * 24 * 60 * 60;

        let deleted_expired = sqlx::query("DELETE FROM workspaces WHERE last_accessed_at < ?")
            .bind(cutoff)
            .execute(self.pool())
            .await?
            .rows_affected();

        let excess = sqlx::query_scalar::<_, Option<i64>>("SELECT COUNT(*) FROM workspaces")
            .fetch_one(self.pool())
            .await?
            .unwrap_or(0)
            .saturating_sub(max_entries);

        if excess > 0 {
            sqlx::query(
                "DELETE FROM workspaces WHERE path IN (
                    SELECT path FROM workspaces
                    ORDER BY last_accessed_at DESC
                    LIMIT -1 OFFSET ?
                )",
            )
            .bind(max_entries)
            .execute(self.pool())
            .await?;
        }

        Ok((deleted_expired, excess.max(0) as u64))
    }

    async fn get_workspace(&self, path: &str) -> WorkspaceResult<Option<WorkspaceRecord>> {
        let row = sqlx::query(
            "SELECT path, display_name, active_session_id, selected_run_action_id, created_at, updated_at, last_accessed_at
             FROM workspaces WHERE path = ?",
        )
        .bind(path)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(build_workspace))
    }

    pub async fn get_session(&self, id: i64) -> WorkspaceResult<Option<SessionRecord>> {
        let row = sqlx::query(
            "SELECT s.id, s.workspace_path, s.title, s.created_at, s.updated_at,
                    (SELECT COUNT(*) FROM messages WHERE session_id = s.id AND role = 'user') as message_count
             FROM sessions s WHERE s.id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(build_session))
    }
}

fn path_to_string(path: &Path) -> WorkspaceResult<String> {
    let display = path
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/");
    Ok(display)
}

fn build_workspace(row: sqlx::sqlite::SqliteRow) -> WorkspaceRecord {
    WorkspaceRecord {
        path: row.try_get("path").unwrap_or_default(),
        display_name: row.try_get("display_name").unwrap_or(None),
        active_session_id: row.try_get("active_session_id").unwrap_or(None),
        selected_run_action_id: row.try_get("selected_run_action_id").unwrap_or(None),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
        last_accessed_at: row.try_get("last_accessed_at").unwrap_or_default(),
    }
}

fn build_session(row: sqlx::sqlite::SqliteRow) -> SessionRecord {
    SessionRecord {
        id: row.try_get("id").unwrap_or_default(),
        workspace_path: row.try_get("workspace_path").unwrap_or_default(),
        title: row.try_get("title").unwrap_or(None),
        message_count: row.try_get("message_count").unwrap_or(0),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

impl WorkspaceService {
    pub async fn refresh_session_title(&self, session_id: i64) -> WorkspaceResult<()> {
        let latest_user_blocks_json: Option<String> = sqlx::query_scalar(
            "SELECT blocks FROM messages
             WHERE session_id = ? AND role = 'user'
             ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(session_id)
        .fetch_optional(self.pool())
        .await?
        .flatten();

        let latest_user_content = latest_user_blocks_json
            .as_deref()
            .and_then(extract_user_text_from_blocks);

        let last_timestamp: Option<i64> =
            sqlx::query_scalar("SELECT MAX(created_at) FROM messages WHERE session_id = ?")
                .bind(session_id)
                .fetch_one(self.pool())
                .await?;

        let updated_at = last_timestamp.unwrap_or_else(Self::now_timestamp);

        sqlx::query("UPDATE sessions SET title = ?, updated_at = ? WHERE id = ?")
            .bind(latest_user_content.as_deref())
            .bind(updated_at)
            .bind(session_id)
            .execute(self.pool())
            .await?;

        Ok(())
    }
}

async fn delete_session_cascade(
    persistence: &crate::agent::persistence::AgentPersistence,
    session_id: i64,
) -> WorkspaceResult<()> {
    let mut delete_order = Vec::new();
    let mut stack = vec![(session_id, false)];

    while let Some((id, visited)) = stack.pop() {
        if visited {
            delete_order.push(id);
            continue;
        }

        stack.push((id, true));
        let children = persistence
            .sessions()
            .list_children(id)
            .await
            .map_err(|e| WorkspaceError::internal(format!("List child sessions failed: {}", e)))?;
        for child in children {
            stack.push((child.id, false));
        }
    }

    for id in delete_order {
        persistence
            .sessions()
            .delete(id)
            .await
            .map_err(|e| WorkspaceError::internal(format!("Delete session failed: {}", e)))?;
    }
    Ok(())
}

fn extract_user_text_from_blocks(blocks_json: &str) -> Option<String> {
    let blocks: Vec<Block> = serde_json::from_str(blocks_json).ok()?;
    blocks.into_iter().find_map(|block| match block {
        Block::UserText(t) => Some(t.content),
        _ => None,
    })
}

// ===== Run Actions =====

fn build_run_action(row: sqlx::sqlite::SqliteRow) -> RunActionRecord {
    RunActionRecord {
        id: row.try_get("id").unwrap_or_default(),
        workspace_path: row.try_get("workspace_path").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        command: row.try_get("command").unwrap_or_default(),
        sort_order: row.try_get("sort_order").unwrap_or_default(),
    }
}

impl WorkspaceService {
    pub async fn list_run_actions(&self, workspace_path: &str) -> WorkspaceResult<Vec<RunActionRecord>> {
        let normalized = self.normalize_path(workspace_path).await?;
        let rows = sqlx::query(
            "SELECT id, workspace_path, name, command, sort_order
             FROM run_actions
             WHERE workspace_path = ?
             ORDER BY sort_order, id",
        )
        .bind(&normalized)
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(build_run_action).collect())
    }

    pub async fn create_run_action(
        &self,
        workspace_path: &str,
        name: &str,
        command: &str,
    ) -> WorkspaceResult<RunActionRecord> {
        let normalized = self.normalize_path(workspace_path).await?;
        let id = uuid::Uuid::new_v4().to_string();

        let max_sort: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(sort_order) FROM run_actions WHERE workspace_path = ?",
        )
        .bind(&normalized)
        .fetch_one(self.pool())
        .await?;
        let sort_order = max_sort.unwrap_or(-1) + 1;

        sqlx::query(
            "INSERT INTO run_actions (id, workspace_path, name, command, sort_order)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&normalized)
        .bind(name)
        .bind(command)
        .bind(sort_order)
        .execute(self.pool())
        .await?;

        Ok(RunActionRecord {
            id,
            workspace_path: normalized,
            name: name.to_string(),
            command: command.to_string(),
            sort_order,
        })
    }

    pub async fn update_run_action(
        &self,
        id: &str,
        name: &str,
        command: &str,
    ) -> WorkspaceResult<()> {
        let result = sqlx::query(
            "UPDATE run_actions SET name = ?, command = ? WHERE id = ?",
        )
        .bind(name)
        .bind(command)
        .bind(id)
        .execute(self.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(WorkspaceError::internal(format!("Run action not found: {}", id)));
        }
        Ok(())
    }

    pub async fn delete_run_action(&self, id: &str) -> WorkspaceResult<()> {
        sqlx::query("DELETE FROM run_actions WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn set_selected_run_action(
        &self,
        workspace_path: &str,
        action_id: Option<&str>,
    ) -> WorkspaceResult<()> {
        let normalized = self.normalize_path(workspace_path).await?;
        let ts = Self::now_timestamp();
        sqlx::query(
            "UPDATE workspaces SET selected_run_action_id = ?, updated_at = ? WHERE path = ?",
        )
        .bind(action_id)
        .bind(ts)
        .bind(&normalized)
        .execute(self.pool())
        .await?;
        Ok(())
    }
}
