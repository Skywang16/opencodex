//! Shell executor configuration

use std::time::Duration;

/// Shell executor configuration
#[derive(Debug, Clone)]
pub struct ShellExecutorConfig {
    /// Default timeout duration
    pub default_timeout: Duration,
    /// Output buffer size (bytes)
    pub output_buffer_size: usize,
    /// Maximum concurrent background commands
    pub max_background_commands: usize,
    /// Retention time for completed commands
    pub completed_retention: Duration,
    /// Maximum command length
    pub max_command_length: usize,
    /// Maximum timeout duration
    pub max_timeout: Duration,
}

impl Default for ShellExecutorConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(120),
            output_buffer_size: 1024 * 1024, // 1MB
            max_background_commands: 10,
            completed_retention: Duration::from_secs(300), // 5 minutes
            max_command_length: 10 * 1024,                 // 10KB
            max_timeout: Duration::from_secs(600),         // 10 minutes
        }
    }
}
