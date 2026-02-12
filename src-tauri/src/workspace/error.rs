use thiserror::Error;

pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("Workspace not found: {path}")]
    WorkspaceNotFound { path: String },

    #[error("Session not found: {id}")]
    SessionNotFound { id: i64 },

    #[error("Session {session_id} does not belong to workspace {workspace_path}")]
    SessionWorkspaceMismatch {
        session_id: i64,
        workspace_path: String,
    },

    #[error("Invalid workspace path: {reason}")]
    InvalidPath { reason: String },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Storage error: {0}")]
    Storage(#[from] crate::storage::error::RepositoryError),

    #[error("Workspace internal error: {0}")]
    Internal(String),
}

impl WorkspaceError {
    pub fn internal(message: impl Into<String>) -> Self {
        WorkspaceError::Internal(message.into())
    }

    pub fn workspace_not_found(path: impl Into<String>) -> Self {
        WorkspaceError::WorkspaceNotFound { path: path.into() }
    }

    pub fn session_not_found(id: i64) -> Self {
        WorkspaceError::SessionNotFound { id }
    }

    pub fn session_workspace_mismatch(session_id: i64, workspace_path: impl Into<String>) -> Self {
        WorkspaceError::SessionWorkspaceMismatch {
            session_id,
            workspace_path: workspace_path.into(),
        }
    }

    pub fn invalid_path(reason: impl Into<String>) -> Self {
        WorkspaceError::InvalidPath {
            reason: reason.into(),
        }
    }
}
