//! Shell execution related type definitions

use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use super::OutputRingBuffer;

/// Command ID type
pub type CommandId = u64;

/// Command execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CommandStatus {
    /// Pending execution
    Pending,
    /// Running
    Running {
        #[serde(skip_serializing_if = "Option::is_none")]
        pid: Option<u32>,
    },
    /// Completed
    Completed { exit_code: i32, duration_ms: u64 },
    /// Timed out
    TimedOut { duration_ms: u64 },
    /// Aborted
    Aborted,
    /// Execution failed
    Failed { error: String },
}

impl CommandStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            CommandStatus::Completed { .. }
                | CommandStatus::TimedOut { .. }
                | CommandStatus::Aborted
                | CommandStatus::Failed { .. }
        )
    }
}

/// Running command information
pub struct RunningCommand {
    /// Command ID
    pub id: CommandId,
    /// Original command string
    pub command: String,
    /// Working directory
    pub cwd: String,
    /// Start time
    pub started_at: Instant,
    /// Execution status
    pub status: CommandStatus,
    /// Output buffer
    pub output_buffer: OutputRingBuffer,
    /// Whether running in background
    pub is_background: bool,
    /// Abort signal
    pub abort_signal: Arc<AtomicBool>,
    /// Process ID
    pub pid: Option<u32>,
}

impl RunningCommand {
    pub fn new(
        id: CommandId,
        command: String,
        cwd: String,
        is_background: bool,
        buffer_capacity: usize,
    ) -> Self {
        Self {
            id,
            command,
            cwd,
            started_at: Instant::now(),
            status: CommandStatus::Pending,
            output_buffer: OutputRingBuffer::new(buffer_capacity),
            is_background,
            abort_signal: Arc::new(AtomicBool::new(false)),
            pid: None,
        }
    }

    /// Get elapsed time (milliseconds)
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }
}

/// Brief information of running command (for querying)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningCommandInfo {
    pub id: CommandId,
    pub command: String,
    pub cwd: String,
    pub status: CommandStatus,
    pub is_background: bool,
    pub elapsed_ms: u64,
    pub pid: Option<u32>,
}

impl From<&RunningCommand> for RunningCommandInfo {
    fn from(cmd: &RunningCommand) -> Self {
        Self {
            id: cmd.id,
            command: cmd.command.clone(),
            cwd: cmd.cwd.clone(),
            status: cmd.status.clone(),
            is_background: cmd.is_background,
            elapsed_ms: cmd.elapsed_ms(),
            pid: cmd.pid,
        }
    }
}

/// Shell execution result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellExecutionResult {
    /// Command ID
    pub command_id: CommandId,
    /// Execution status
    pub status: CommandStatus,
    /// Command output
    pub output: String,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Working directory
    pub cwd: String,
    /// Whether output was truncated
    pub output_truncated: bool,
}
