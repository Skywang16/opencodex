//! Agent Shell executor

use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

use super::config::ShellExecutorConfig;
use super::error::ShellError;
use super::types::*;

/// Agent Shell executor
pub struct AgentShellExecutor {
    /// Configuration
    config: ShellExecutorConfig,
    /// Command ID generator
    next_command_id: AtomicU64,
}

impl AgentShellExecutor {
    /// Create new executor
    pub fn new() -> Self {
        Self::with_config(ShellExecutorConfig::default())
    }

    /// Create executor with specified configuration
    pub fn with_config(config: ShellExecutorConfig) -> Self {
        Self {
            config,
            next_command_id: AtomicU64::new(1),
        }
    }

    /// Generate next command ID
    fn next_id(&self) -> CommandId {
        self.next_command_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Validate command
    fn validate_command(&self, command: &str) -> Result<(), ShellError> {
        if command.trim().is_empty() {
            return Err(ShellError::ValidationFailed(
                "Command cannot be empty".into(),
            ));
        }

        if command.len() > self.config.max_command_length {
            return Err(ShellError::ValidationFailed(format!(
                "Command too long (max {} bytes)",
                self.config.max_command_length
            )));
        }

        Ok(())
    }

    /// Synchronously execute command (wait for completion or timeout)
    pub async fn execute(
        &self,
        command: &str,
        cwd: &str,
        timeout_duration: Option<Duration>,
    ) -> Result<ShellExecutionResult, ShellError> {
        self.validate_command(command)?;

        let timeout_duration = timeout_duration
            .unwrap_or(self.config.default_timeout)
            .min(self.config.max_timeout);

        let id = self.next_id();
        let mut running_cmd = RunningCommand::new(
            id,
            command.to_string(),
            cwd.to_string(),
            false,
            self.config.output_buffer_size,
        );

        running_cmd.status = CommandStatus::Running { pid: None };

        // Build command
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(command);
            c
        } else {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            let mut c = Command::new(shell);
            c.arg("-lc").arg(command);
            c
        };

        if !cwd.trim().is_empty() {
            cmd.current_dir(cwd);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Execute command
        let result = timeout(timeout_duration, async {
            let mut child = cmd.spawn()?;

            // Get PID
            let pid = child.id();
            running_cmd.pid = pid;
            running_cmd.status = CommandStatus::Running { pid };

            // Read output
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let mut output = String::new();

            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                    running_cmd.output_buffer.write_str(&line);
                    running_cmd.output_buffer.write_str("\n");
                }
            }

            if let Some(stderr) = stderr {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                    running_cmd.output_buffer.write_str(&line);
                    running_cmd.output_buffer.write_str("\n");
                }
            }

            let status = child.wait().await?;
            let exit_code = status.code().unwrap_or(-1);

            Ok::<(String, i32), std::io::Error>((output, exit_code))
        })
        .await;

        let duration_ms = running_cmd.elapsed_ms();

        match result {
            Ok(Ok((output, exit_code))) => {
                running_cmd.status = CommandStatus::Completed {
                    exit_code,
                    duration_ms,
                };

                Ok(ShellExecutionResult {
                    command_id: id,
                    status: running_cmd.status.clone(),
                    output,
                    exit_code: Some(exit_code),
                    duration_ms,
                    cwd: cwd.to_string(),
                    output_truncated: running_cmd.output_buffer.is_overflowed(),
                })
            }
            Ok(Err(e)) => {
                running_cmd.status = CommandStatus::Failed {
                    error: e.to_string(),
                };
                Err(ShellError::IoError(e))
            }
            Err(_) => {
                running_cmd.status = CommandStatus::TimedOut { duration_ms };
                Err(ShellError::Timeout(duration_ms))
            }
        }
    }
}

impl Default for AgentShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}
