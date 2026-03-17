use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::agent::error::{AgentError, AgentResult};

use super::{opt_timestamp_to_datetime, timestamp_to_datetime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub path: String,
    pub display_name: Option<String>,
    pub active_session_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub workspace_path: String,

    pub parent_id: Option<i64>,
    pub agent_type: String,
    pub spawned_by_tool_call: Option<String>,

    pub worktree_path: Option<String>,

    pub title: Option<String>,
    pub model_id: Option<String>,
    pub provider_id: Option<String>,

    pub status: SessionStatus,
    pub is_archived: bool,

    pub total_messages: i64,
    pub total_tokens: i64,
    pub total_cost: f64,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_message_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    Idle,
    Running,
    Completed,
    Error,
    Cancelled,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for SessionStatus {
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idle" => Ok(Self::Idle),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(AgentError::Parse(format!(
                "Unknown session status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolExecutionStatus {
    Pending,
    Running,
    Completed,
    Error,
    Cancelled,
}

impl ToolExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for ToolExecutionStatus {
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(AgentError::Parse(format!(
                "Unknown tool execution status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub id: i64,
    pub message_id: i64,
    pub session_id: i64,
    pub call_id: String,
    pub tool_name: String,
    pub status: ToolExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunStatus {
    Queued,
    Running,
    Completed,
    Error,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for RunStatus {
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "error" => Ok(Self::Error),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(AgentError::Parse(format!("Unknown run status: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: i64,
    pub session_id: i64,
    pub trigger_message_id: Option<i64>,
    pub root_node_id: Option<i64>,
    pub status: RunStatus,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentNodeRole {
    Root,
    Fork,
    Branch,
}

impl AgentNodeRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Fork => "fork",
            Self::Branch => "branch",
        }
    }
}

impl FromStr for AgentNodeRole {
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "root" => Ok(Self::Root),
            "fork" => Ok(Self::Fork),
            "branch" => Ok(Self::Branch),
            other => Err(AgentError::Parse(format!(
                "Unknown agent node role: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    pub id: i64,
    pub run_id: i64,
    pub parent_node_id: Option<i64>,
    pub backing_session_id: Option<i64>,
    pub trigger_tool_call_id: Option<String>,
    pub role: AgentNodeRole,
    pub profile: String,
    pub title: String,
    pub status: RunStatus,
    pub worktree_path: Option<String>,
    pub model_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

pub(crate) fn build_workspace(row: &sqlx::sqlite::SqliteRow) -> AgentResult<Workspace> {
    Ok(Workspace {
        path: row.try_get("path")?,
        display_name: row.try_get("display_name")?,
        active_session_id: row.try_get("active_session_id")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
        updated_at: timestamp_to_datetime(row.try_get::<i64, _>("updated_at")?),
        last_accessed_at: timestamp_to_datetime(row.try_get::<i64, _>("last_accessed_at")?),
    })
}

pub(crate) fn build_session(row: &sqlx::sqlite::SqliteRow) -> AgentResult<Session> {
    Ok(Session {
        id: row.try_get("id")?,
        workspace_path: row.try_get("workspace_path")?,
        parent_id: row.try_get("parent_id")?,
        agent_type: row.try_get("agent_type")?,
        spawned_by_tool_call: row.try_get("spawned_by_tool_call")?,
        worktree_path: row.try_get("worktree_path")?,
        title: row.try_get("title")?,
        model_id: row.try_get("model_id")?,
        provider_id: row.try_get("provider_id")?,
        status: SessionStatus::from_str(row.try_get::<String, _>("status")?.as_str())?,
        is_archived: row.try_get::<i64, _>("is_archived")? != 0,
        total_messages: row.try_get("total_messages")?,
        total_tokens: row.try_get("total_tokens")?,
        total_cost: row.try_get("total_cost")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
        updated_at: timestamp_to_datetime(row.try_get::<i64, _>("updated_at")?),
        last_message_at: opt_timestamp_to_datetime(row.try_get("last_message_at")?),
    })
}

pub(crate) fn build_tool_execution(row: &sqlx::sqlite::SqliteRow) -> AgentResult<ToolExecution> {
    Ok(ToolExecution {
        id: row.try_get("id")?,
        message_id: row.try_get("message_id")?,
        session_id: row.try_get("session_id")?,
        call_id: row.try_get("call_id")?,
        tool_name: row.try_get("tool_name")?,
        status: ToolExecutionStatus::from_str(row.try_get::<String, _>("status")?.as_str())?,
        started_at: timestamp_to_datetime(row.try_get::<i64, _>("started_at")?),
        finished_at: opt_timestamp_to_datetime(row.try_get("finished_at")?),
        duration_ms: row.try_get("duration_ms")?,
    })
}

pub(crate) fn build_run(row: &sqlx::sqlite::SqliteRow) -> AgentResult<Run> {
    Ok(Run {
        id: row.try_get("id")?,
        session_id: row.try_get("session_id")?,
        trigger_message_id: row.try_get("trigger_message_id")?,
        root_node_id: row.try_get("root_node_id")?,
        status: RunStatus::from_str(row.try_get::<String, _>("status")?.as_str())?,
        summary: row.try_get("summary")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
        started_at: opt_timestamp_to_datetime(row.try_get("started_at")?),
        finished_at: opt_timestamp_to_datetime(row.try_get("finished_at")?),
    })
}

pub(crate) fn build_agent_node(row: &sqlx::sqlite::SqliteRow) -> AgentResult<AgentNode> {
    Ok(AgentNode {
        id: row.try_get("id")?,
        run_id: row.try_get("run_id")?,
        parent_node_id: row.try_get("parent_node_id")?,
        backing_session_id: row.try_get("backing_session_id")?,
        trigger_tool_call_id: row.try_get("trigger_tool_call_id")?,
        role: AgentNodeRole::from_str(row.try_get::<String, _>("role")?.as_str())?,
        profile: row.try_get("profile")?,
        title: row.try_get("title")?,
        status: RunStatus::from_str(row.try_get::<String, _>("status")?.as_str())?,
        worktree_path: row.try_get("worktree_path")?,
        model_id: row.try_get("model_id")?,
        created_at: timestamp_to_datetime(row.try_get::<i64, _>("created_at")?),
        started_at: opt_timestamp_to_datetime(row.try_get("started_at")?),
        finished_at: opt_timestamp_to_datetime(row.try_get("finished_at")?),
    })
}
