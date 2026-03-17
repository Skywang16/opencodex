use std::sync::Arc;

use crate::storage::database::DatabaseManager;

use super::repositories::{
    AgentNodeRepository, MessageRepository, RunRepository, SessionRepository,
    ToolExecutionRepository, WorkspaceRepository,
};

/// Facade that wires all persistence repositories together for the agent backend.
#[derive(Debug)]
pub struct AgentPersistence {
    database: Arc<DatabaseManager>,
    workspaces: WorkspaceRepository,
    sessions: SessionRepository,
    runs: RunRepository,
    agent_nodes: AgentNodeRepository,
    messages: MessageRepository,
    tool_executions: ToolExecutionRepository,
}

impl AgentPersistence {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self {
            tool_executions: ToolExecutionRepository::new(Arc::clone(&database)),
            workspaces: WorkspaceRepository::new(Arc::clone(&database)),
            sessions: SessionRepository::new(Arc::clone(&database)),
            runs: RunRepository::new(Arc::clone(&database)),
            agent_nodes: AgentNodeRepository::new(Arc::clone(&database)),
            messages: MessageRepository::new(Arc::clone(&database)),
            database,
        }
    }

    pub fn database(&self) -> &DatabaseManager {
        &self.database
    }

    pub fn workspaces(&self) -> &WorkspaceRepository {
        &self.workspaces
    }

    pub fn sessions(&self) -> &SessionRepository {
        &self.sessions
    }

    pub fn runs(&self) -> &RunRepository {
        &self.runs
    }

    pub fn agent_nodes(&self) -> &AgentNodeRepository {
        &self.agent_nodes
    }

    pub fn messages(&self) -> &MessageRepository {
        &self.messages
    }

    pub fn tool_executions(&self) -> &ToolExecutionRepository {
        &self.tool_executions
    }
}
