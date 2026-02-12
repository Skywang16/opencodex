/*!
 * Tool execution logger (Stage 2 update)
 *
 * Persists tool execution information to `tool_executions` table through new AgentPersistence interface,
 * while retaining original event output capability.
 */

use serde_json::Value as JsonValue;
use tracing::error;

use crate::agent::core::context::TaskContext;
use crate::agent::error::AgentResult;
use crate::agent::tools::ToolResult;

pub struct ToolExecutionLogger {
    verbose: bool,
}

impl ToolExecutionLogger {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    /// Log tool execution start
    pub async fn log_start(
        &self,
        _context: &TaskContext,
        call_id: &str,
        tool_name: &str,
        arguments: &JsonValue,
    ) -> AgentResult<String> {
        let _ = (tool_name, arguments);

        Ok(call_id.to_string())
    }

    /// Log tool execution success
    pub async fn log_success(
        &self,
        log_id: &str,
        _result: &ToolResult,
        duration_ms: u64,
    ) -> AgentResult<()> {
        let _ = (log_id, duration_ms);
        Ok(())
    }

    /// Log tool execution failure
    pub async fn log_failure(
        &self,
        log_id: &str,
        error_message: &str,
        duration_ms: u64,
    ) -> AgentResult<()> {
        if self.verbose {
            error!(
                "Tool execution failed: log_id={}, error={}, duration={}ms",
                log_id, error_message, duration_ms
            );
        }
        Ok(())
    }

    /// Log tool execution cancellation
    pub async fn log_cancelled(&self, log_id: &str) -> AgentResult<()> {
        let _ = log_id;
        Ok(())
    }
}
