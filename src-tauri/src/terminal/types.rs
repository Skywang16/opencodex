// Terminal context related type definitions

use crate::mux::PaneId;
use crate::terminal::error::{TerminalValidationError, TerminalValidationResult};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::SystemTime;

/// Terminal Channel message type
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum TerminalChannelMessage {
    Data { pane_id: u32, data: Vec<u8> },
    Error { pane_id: u32, error: String },
    Close { pane_id: u32 },
}

// TerminalContextEvent has been moved to crate::events::context module

/// Terminal context data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalContext {
    pub pane_id: PaneId,
    pub current_working_directory: Option<String>,
    pub shell_type: Option<ShellType>,
    pub shell_integration_enabled: bool,
    pub current_command: Option<CommandInfo>,
    pub command_history: Vec<CommandInfo>,
    pub window_title: Option<String>,
    pub last_activity: SystemTime,
    pub is_active: bool,
}

/// Shell type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum ShellType {
    #[default]
    Bash,
    Zsh,
    Fish,
    Other(String),
}

impl FromStr for ShellType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "bash" => ShellType::Bash,
            "zsh" => ShellType::Zsh,
            "fish" => ShellType::Fish,
            _ => ShellType::Other(s.to_string()),
        })
    }
}

impl ShellType {
    /// Get shell display name
    pub fn display_name(&self) -> &str {
        match self {
            ShellType::Bash => "Bash",
            ShellType::Zsh => "Zsh",
            ShellType::Fish => "Fish",
            ShellType::Other(name) => name,
        }
    }

    /// Check if shell integration is supported
    pub fn supports_integration(&self) -> bool {
        matches!(self, ShellType::Bash | ShellType::Zsh | ShellType::Fish)
    }

    /// Get shell default prompt
    pub fn default_prompt(&self) -> &str {
        match self {
            ShellType::Bash | ShellType::Zsh => "$ ",
            ShellType::Fish => "❯ ",
            ShellType::Other(_) => "$ ",
        }
    }
}

impl std::fmt::Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Command information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandInfo {
    pub command: String,
    pub args: Vec<String>,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub exit_code: Option<i32>,
    pub working_directory: Option<String>,
}

impl CommandInfo {
    /// Create new command information
    pub fn new(command: String, args: Vec<String>, working_directory: Option<String>) -> Self {
        Self {
            command,
            args,
            start_time: SystemTime::now(),
            end_time: None,
            exit_code: None,
            working_directory,
        }
    }

    /// Mark command as complete
    pub fn complete(&mut self, exit_code: i32) {
        self.end_time = Some(SystemTime::now());
        self.exit_code = Some(exit_code);
    }

    /// Validate command information validity
    pub fn validate(&self) -> Result<(), String> {
        if self.command.trim().is_empty() {
            return Err("Command cannot be empty".to_string());
        }

        // Validate time logic
        if let Some(end_time) = self.end_time {
            if end_time < self.start_time {
                return Err("End time cannot be earlier than start time".to_string());
            }
        }

        // Validate exit code logic
        if self.end_time.is_none() && self.exit_code.is_some() {
            return Err("Unfinished command should not have exit code".to_string());
        }

        Ok(())
    }

    /// Check if command is completed
    pub fn is_completed(&self) -> bool {
        self.end_time.is_some()
    }

    /// Check if command executed successfully
    pub fn is_successful(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// Get command execution duration
    pub fn duration(&self) -> Option<std::time::Duration> {
        if let Some(end_time) = self.end_time {
            end_time.duration_since(self.start_time).ok()
        } else {
            None
        }
    }
}

impl TerminalContext {
    /// Create new terminal context
    pub fn new(pane_id: PaneId) -> Self {
        Self {
            pane_id,
            current_working_directory: None,
            shell_type: None,
            shell_integration_enabled: false,
            current_command: None,
            command_history: Vec::new(),
            window_title: None,
            last_activity: SystemTime::now(),
            is_active: false,
        }
    }

    /// Create terminal context with default values (for error fallback)
    pub fn with_defaults(pane_id: PaneId) -> Self {
        Self {
            pane_id,
            current_working_directory: Some("~".to_string()),
            shell_type: Some(ShellType::Bash),
            shell_integration_enabled: false,
            current_command: None,
            command_history: Vec::new(),
            window_title: None,
            last_activity: SystemTime::now(),
            is_active: false,
        }
    }

    /// Validate terminal context integrity
    pub fn validate(&self) -> TerminalValidationResult<()> {
        // Validate if pane ID is valid
        if self.pane_id.as_u32() == 0 {
            return Err(TerminalValidationError::PaneId);
        }

        // Validate command history integrity
        for (index, command) in self.command_history.iter().enumerate() {
            if let Err(e) = command.validate() {
                return Err(TerminalValidationError::HistoryEntry { index, reason: e });
            }
        }

        // Validate current command integrity
        if let Some(ref command) = self.current_command {
            if let Err(e) = command.validate() {
                return Err(TerminalValidationError::CurrentCommand { reason: e });
            }
        }

        Ok(())
    }

    /// Check if context contains complete terminal state information
    pub fn is_complete(&self) -> bool {
        self.current_working_directory.is_some() && self.shell_type.is_some()
    }

    /// Get valid working directory, return default if none
    pub fn get_cwd_or_default(&self) -> String {
        self.current_working_directory
            .clone()
            .unwrap_or_else(|| "~".to_string())
    }

    /// Get shell type, return default if none
    pub fn get_shell_type_or_default(&self) -> ShellType {
        self.shell_type.clone().unwrap_or(ShellType::Bash)
    }

    /// Update active status
    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
        if active {
            self.last_activity = SystemTime::now();
        }
    }

    /// Update working directory
    pub fn update_cwd(&mut self, cwd: String) {
        self.current_working_directory = Some(cwd);
        self.last_activity = SystemTime::now();
    }

    /// Update shell type
    pub fn update_shell_type(&mut self, shell_type: ShellType) {
        self.shell_type = Some(shell_type);
        self.last_activity = SystemTime::now();
    }

    /// Set shell integration status
    pub fn set_shell_integration(&mut self, enabled: bool) {
        self.shell_integration_enabled = enabled;
        self.last_activity = SystemTime::now();
    }

    /// Add command to history
    pub fn add_command(&mut self, command: CommandInfo) {
        self.command_history.push(command);
        self.last_activity = SystemTime::now();

        // Limit history record count
        if self.command_history.len() > 100 {
            self.command_history.remove(0);
        }
    }

    /// Set current command
    pub fn set_current_command(&mut self, command: Option<CommandInfo>) {
        self.current_command = command;
        self.last_activity = SystemTime::now();
    }

    /// Update window title
    pub fn update_window_title(&mut self, title: String) {
        self.window_title = Some(title);
        self.last_activity = SystemTime::now();
    }
}

/// Cached context information
/// Context query options
#[derive(Debug, Clone, Default)]
pub struct ContextQueryOptions {
    /// Whether to use cache
    pub use_cache: bool,
    /// Query timeout duration
    pub timeout: Option<std::time::Duration>,
    /// Whether to allow fallback to default values
    pub allow_fallback: bool,
    /// Whether to include command history
    pub include_history: bool,
    /// Maximum history record count
    pub max_history_count: Option<usize>,
}

impl ContextQueryOptions {
    /// Create fast query options (use cache, allow fallback)
    pub fn fast() -> Self {
        Self {
            use_cache: true,
            timeout: Some(std::time::Duration::from_millis(10)),
            allow_fallback: true,
            include_history: false,
            max_history_count: None,
        }
    }

    /// Create complete query options (include all information)
    pub fn complete() -> Self {
        Self {
            use_cache: false,
            timeout: Some(std::time::Duration::from_millis(100)),
            allow_fallback: true,
            include_history: true,
            max_history_count: Some(50),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mux::PaneId;

    #[test]
    fn test_terminal_context_creation() {
        let pane_id = PaneId::new(1);
        let context = TerminalContext::new(pane_id);

        assert_eq!(context.pane_id, pane_id);
        assert_eq!(context.current_working_directory, None);
        assert_eq!(context.shell_type, None);
        assert!(!context.shell_integration_enabled);
        assert!(!context.is_active);
    }

    #[test]
    fn test_terminal_context_with_defaults() {
        let pane_id = PaneId::new(1);
        let context = TerminalContext::with_defaults(pane_id);

        assert_eq!(context.pane_id, pane_id);
        assert_eq!(context.current_working_directory, Some("~".to_string()));
        assert_eq!(context.shell_type, Some(ShellType::Bash));
        assert!(!context.shell_integration_enabled);
        assert!(!context.is_active);
    }

    #[test]
    fn test_terminal_context_validation() {
        let pane_id = PaneId::new(1);
        let context = TerminalContext::new(pane_id);

        // Valid context should pass validation
        assert!(context.validate().is_ok());

        // Invalid pane ID should fail validation
        let invalid_context = TerminalContext::new(PaneId::new(0));
        assert!(invalid_context.validate().is_err());
    }

    #[test]
    fn test_command_info_validation() {
        let mut command = CommandInfo::new(
            "ls".to_string(),
            vec!["-la".to_string()],
            Some("/home/user".to_string()),
        );

        // Valid command should pass validation
        assert!(command.validate().is_ok());

        // Empty command should fail validation
        let empty_command = CommandInfo::new("".to_string(), vec![], None);
        assert!(empty_command.validate().is_err());

        // Complete the command
        command.complete(0);
        assert!(command.is_completed());
        assert!(command.is_successful());
    }

    #[test]
    fn test_shell_type_parsing() {
        assert_eq!(ShellType::from_str("bash").unwrap(), ShellType::Bash);
        assert_eq!(ShellType::from_str("zsh").unwrap(), ShellType::Zsh);
        assert_eq!(ShellType::from_str("fish").unwrap(), ShellType::Fish);
        assert_eq!(
            ShellType::from_str("powershell").unwrap(),
            ShellType::Other("powershell".to_string())
        );
        assert_eq!(
            ShellType::from_str("cmd").unwrap(),
            ShellType::Other("cmd".to_string())
        );
        assert_eq!(
            ShellType::from_str("unknown").unwrap(),
            ShellType::Other("unknown".to_string())
        );
    }

    #[test]
    fn test_context_query_options() {
        let fast_options = ContextQueryOptions::fast();
        assert!(fast_options.use_cache);
        assert!(fast_options.allow_fallback);
        assert!(!fast_options.include_history);

        let complete_options = ContextQueryOptions::complete();
        assert!(!complete_options.use_cache);
        assert!(complete_options.allow_fallback);
        assert!(complete_options.include_history);
    }
}
