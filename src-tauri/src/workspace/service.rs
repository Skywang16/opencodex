use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;
use sqlx::{self, Row};
use tokio::task;

use crate::agent::persistence::models::build_agent_node;
use crate::agent::persistence::AgentNode;
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
    pub parent_id: Option<i64>,
    pub title: Option<String>,
    pub message_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionNodeRecord {
    pub id: i64,
    pub backing_session_id: Option<i64>,
    pub role: String,
    pub profile: String,
    pub title: String,
    pub status: String,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub children: Vec<ExecutionNodeRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionViewRecord {
    pub session: SessionRecord,
    pub timeline: Vec<SessionTimelineItemRecord>,
    pub execution_tree: Vec<ExecutionNodeRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTimelineItemRecord {
    pub id: String,
    pub message_id: i64,
    pub title: String,
    pub created_at: i64,
    pub status: Option<String>,
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
                    WorkspaceError::invalid_path(format!("Canonicalize failed: {e}"))
                })?
            } else {
                candidate
            };
            path_to_string(&canonical)
        })
        .await
        .map_err(|e| WorkspaceError::internal(format!("Task join error: {e}")))?
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

        rows.into_iter().map(build_workspace).collect()
    }

    pub async fn list_sessions(&self, workspace_path: &str) -> WorkspaceResult<Vec<SessionRecord>> {
        let normalized = self.normalize_path(workspace_path).await?;
        let rows = sqlx::query(
            "SELECT s.id, s.workspace_path, s.parent_id, s.title, s.created_at, s.updated_at,
                    (SELECT COUNT(*) FROM messages WHERE session_id = s.id AND role = 'user') as message_count
             FROM sessions s
             WHERE s.workspace_path = ?
             ORDER BY s.updated_at DESC, s.id DESC",
        )
        .bind(&normalized)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(build_session).collect()
    }

    pub async fn list_session_views(
        &self,
        workspace_path: &str,
    ) -> WorkspaceResult<Vec<SessionViewRecord>> {
        let sessions = self.list_sessions(workspace_path).await?;
        let mut views = Vec::with_capacity(sessions.len());

        for session in sessions {
            views.push(self.build_session_view(session).await?);
        }

        Ok(views)
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
            let has_title = match session.title.as_ref() {
                Some(title) => !title.trim().is_empty(),
                None => false,
            };
            if has_title {
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
            .map_err(|e| WorkspaceError::internal(format!("List session messages failed: {e}")))?;

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
            .map_err(|e| WorkspaceError::internal(format!("Trim session messages failed: {e}")))?;

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

    pub async fn get_session_messages(
        &self,
        session_id: i64,
        limit: i64,
        before_id: Option<i64>,
    ) -> WorkspaceResult<Vec<Message>> {
        self.agent_persistence
            .messages()
            .list_by_session_paginated(session_id, limit, before_id)
            .await
            .map_err(|e| WorkspaceError::internal(format!("Load session messages failed: {e}")))
    }

    pub async fn list_session_timeline(
        &self,
        session_id: i64,
    ) -> WorkspaceResult<Vec<SessionTimelineItemRecord>> {
        let message_rows = sqlx::query(
            "SELECT id, blocks, created_at
             FROM messages
             WHERE session_id = ? AND role = 'user' AND is_summary = 0
             ORDER BY created_at ASC, id ASC",
        )
        .bind(session_id)
        .fetch_all(self.pool())
        .await?;

        let run_rows = sqlx::query(
            "SELECT id, trigger_message_id, status, created_at
             FROM runs
             WHERE session_id = ? AND trigger_message_id IS NOT NULL
             ORDER BY created_at DESC, id DESC",
        )
        .bind(session_id)
        .fetch_all(self.pool())
        .await?;

        let mut latest_run_by_message = std::collections::HashMap::<i64, (i64, String)>::new();
        for row in run_rows {
            let message_id: i64 = row.try_get("trigger_message_id")?;
            latest_run_by_message
                .entry(message_id)
                .or_insert((row.try_get("id")?, row.try_get::<String, _>("status")?));
        }

        let mut items = Vec::with_capacity(message_rows.len());
        for row in message_rows {
            let message_id: i64 = row.try_get("id")?;
            let blocks_json: String = row.try_get("blocks")?;
            let title = extract_user_text_from_blocks(&blocks_json)?
                .map(|text| normalize_timeline_title(&text))
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| "Untitled message".to_string());

            let status = latest_run_by_message
                .get(&message_id)
                .map(|(_, status)| Some(status.clone()))
                .unwrap_or(None);

            items.push(SessionTimelineItemRecord {
                id: format!("message-{message_id}"),
                message_id,
                title,
                created_at: row.try_get("created_at")?,
                status,
            });
        }

        Ok(items)
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

        let total_workspaces = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM workspaces")
            .fetch_one(self.pool())
            .await?;
        let excess = total_workspaces.saturating_sub(max_entries);

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

        row.map(build_workspace).transpose()
    }

    pub async fn get_session(&self, id: i64) -> WorkspaceResult<Option<SessionRecord>> {
        let row = sqlx::query(
            "SELECT s.id, s.workspace_path, s.parent_id, s.title, s.created_at, s.updated_at,
                    (SELECT COUNT(*) FROM messages WHERE session_id = s.id AND role = 'user') as message_count
             FROM sessions s WHERE s.id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        row.map(build_session).transpose()
    }

    async fn build_session_view(
        &self,
        session: SessionRecord,
    ) -> WorkspaceResult<SessionViewRecord> {
        Ok(SessionViewRecord {
            timeline: self.list_session_timeline(session.id).await?,
            execution_tree: self.list_execution_tree(session.id).await?,
            session,
        })
    }

    async fn list_execution_tree(
        &self,
        session_id: i64,
    ) -> WorkspaceResult<Vec<ExecutionNodeRecord>> {
        let node_rows = sqlx::query(
            "SELECT n.*
             FROM agent_nodes n
             INNER JOIN runs r ON r.id = n.run_id
             WHERE r.session_id = ?
             ORDER BY n.created_at ASC, n.id ASC",
        )
        .bind(session_id)
        .fetch_all(self.pool())
        .await?;

        let nodes = node_rows
            .into_iter()
            .map(|row| build_agent_node(&row))
            .collect::<Result<Vec<AgentNode>, _>>()
            .map_err(|e| WorkspaceError::internal(format!("Build agent nodes failed: {e}")))?;

        Ok(build_execution_tree(nodes))
    }
}

fn build_execution_tree(nodes: Vec<AgentNode>) -> Vec<ExecutionNodeRecord> {
    use std::collections::HashMap;

    let mut by_parent: HashMap<Option<i64>, Vec<AgentNode>> = HashMap::new();
    for node in nodes {
        by_parent.entry(node.parent_node_id).or_default().push(node);
    }

    fn build_children(
        by_parent: &mut std::collections::HashMap<Option<i64>, Vec<AgentNode>>,
        parent_id: Option<i64>,
    ) -> Vec<ExecutionNodeRecord> {
        let mut children = by_parent.remove(&parent_id).unwrap_or_default();
        children.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });

        children
            .into_iter()
            .map(|node| ExecutionNodeRecord {
                id: node.id,
                backing_session_id: node.backing_session_id,
                role: node.role.as_str().to_string(),
                profile: node.profile,
                title: node.title,
                status: node.status.as_str().to_string(),
                started_at: node.started_at.map(|v| v.timestamp()),
                finished_at: node.finished_at.map(|v| v.timestamp()),
                children: build_children(by_parent, Some(node.id)),
            })
            .collect()
    }

    build_children(&mut by_parent, None)
}

fn path_to_string(path: &Path) -> WorkspaceResult<String> {
    let display = path
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/");
    Ok(display)
}

fn build_workspace(row: sqlx::sqlite::SqliteRow) -> WorkspaceResult<WorkspaceRecord> {
    Ok(WorkspaceRecord {
        path: row.try_get("path")?,
        display_name: row.try_get("display_name")?,
        active_session_id: row.try_get("active_session_id")?,
        selected_run_action_id: row.try_get("selected_run_action_id")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        last_accessed_at: row.try_get("last_accessed_at")?,
    })
}

fn build_session(row: sqlx::sqlite::SqliteRow) -> WorkspaceResult<SessionRecord> {
    Ok(SessionRecord {
        id: row.try_get("id")?,
        workspace_path: row.try_get("workspace_path")?,
        parent_id: row.try_get("parent_id")?,
        title: row.try_get("title")?,
        message_count: row.try_get("message_count")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
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
            .map(extract_user_text_from_blocks)
            .transpose()?
            .flatten()
            .map(|text| normalize_timeline_title(&text))
            .filter(|text| !text.is_empty());

        let last_timestamp: Option<i64> =
            sqlx::query_scalar("SELECT MAX(created_at) FROM messages WHERE session_id = ?")
                .bind(session_id)
                .fetch_one(self.pool())
                .await?;

        let updated_at = match last_timestamp {
            Some(timestamp) => timestamp,
            None => Self::now_timestamp(),
        };

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
            .map_err(|e| WorkspaceError::internal(format!("List child sessions failed: {e}")))?;
        for child in children {
            stack.push((child.id, false));
        }
    }

    for id in delete_order {
        persistence
            .sessions()
            .delete(id)
            .await
            .map_err(|e| WorkspaceError::internal(format!("Delete session failed: {e}")))?;
    }
    Ok(())
}

fn extract_user_text_from_blocks(blocks_json: &str) -> WorkspaceResult<Option<String>> {
    let blocks: Vec<Block> = serde_json::from_str(blocks_json).map_err(|err| {
        WorkspaceError::internal(format!("Failed to parse user blocks JSON: {err}"))
    })?;
    Ok(blocks.into_iter().find_map(|block| match block {
        Block::UserText(t) => Some(t.content),
        _ => None,
    }))
}

fn normalize_timeline_title(input: &str) -> String {
    fn strip_leading_image_placeholders(mut text: &str) -> &str {
        loop {
            let trimmed = text.trim_start();
            if !trimmed.starts_with("[Image #") {
                return trimmed;
            }
            let Some(end_idx) = trimmed.find(']') else {
                return trimmed;
            };
            text = &trimmed[end_idx + 1..];
        }
    }

    fn strip_leading_comment_marker(text: &str) -> &str {
        let trimmed = text.trim_start();
        if !trimmed.starts_with("<!--") {
            return trimmed;
        }
        if let Some(end_idx) = trimmed.find("-->") {
            return &trimmed[end_idx + 3..];
        }
        if let Some(newline_idx) = trimmed.find('\n') {
            return &trimmed[newline_idx + 1..];
        }
        ""
    }

    fn strip_leading_slash_command(text: &str) -> &str {
        let trimmed = text.trim_start();
        for prefix in [
            "/code-review",
            "/skill-creator",
            "/skill-installer",
            "/plan-mode",
            "/orchestrate-mode",
        ] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                return rest;
            }
        }
        trimmed
    }

    fn strip_leading_xml_mode_tag(text: &str) -> &str {
        let trimmed = text.trim_start();
        let Some(after_lt) = trimmed.strip_prefix('<') else {
            return trimmed;
        };
        let Some(close_idx) = after_lt.find('>') else {
            return trimmed;
        };
        let tag_body = &after_lt[..close_idx];
        let tag_name = tag_body
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('/');
        if !tag_name.ends_with("-mode") {
            return trimmed;
        }
        &after_lt[close_idx + 1..]
    }

    fn strip_trailing_xml_mode_tag(text: &str) -> &str {
        let trimmed = text.trim_end();
        let Some(before_gt) = trimmed.strip_suffix('>') else {
            return trimmed;
        };
        let Some(open_idx) = before_gt.rfind("</") else {
            return trimmed;
        };
        let tag_name = before_gt[open_idx + 2..].trim();
        if !tag_name.ends_with("-mode") {
            return trimmed;
        }
        &before_gt[..open_idx]
    }

    let mut cleaned = input.trim();

    loop {
        let prev = cleaned;
        cleaned = strip_leading_image_placeholders(cleaned).trim_start();
        cleaned = strip_leading_comment_marker(cleaned).trim_start();
        cleaned = strip_leading_slash_command(cleaned).trim_start();
        cleaned = strip_leading_xml_mode_tag(cleaned).trim_start();
        if cleaned == prev {
            break;
        }
    }

    cleaned = strip_trailing_xml_mode_tag(cleaned).trim();

    cleaned
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ===== Run Actions =====

fn build_run_action(row: sqlx::sqlite::SqliteRow) -> WorkspaceResult<RunActionRecord> {
    Ok(RunActionRecord {
        id: row.try_get("id")?,
        workspace_path: row.try_get("workspace_path")?,
        name: row.try_get("name")?,
        command: row.try_get("command")?,
        sort_order: row.try_get("sort_order")?,
    })
}

impl WorkspaceService {
    pub async fn list_run_actions(
        &self,
        workspace_path: &str,
    ) -> WorkspaceResult<Vec<RunActionRecord>> {
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

        rows.into_iter().map(build_run_action).collect()
    }

    pub async fn create_run_action(
        &self,
        workspace_path: &str,
        name: &str,
        command: &str,
    ) -> WorkspaceResult<RunActionRecord> {
        let normalized = self.normalize_path(workspace_path).await?;
        let id = uuid::Uuid::new_v4().to_string();

        let max_sort: Option<i64> =
            sqlx::query_scalar("SELECT MAX(sort_order) FROM run_actions WHERE workspace_path = ?")
                .bind(&normalized)
                .fetch_one(self.pool())
                .await?;
        let sort_order = match max_sort {
            Some(sort_order) => sort_order + 1,
            None => 0,
        };

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
        let result = sqlx::query("UPDATE run_actions SET name = ?, command = ? WHERE id = ?")
            .bind(name)
            .bind(command)
            .bind(id)
            .execute(self.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(WorkspaceError::internal(format!(
                "Run action not found: {id}"
            )));
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
