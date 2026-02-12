//! Completion functionality related type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Completion item type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionType {
    /// File path
    File,
    /// Directory path
    Directory,
    /// Executable command
    Command,
    /// Command history
    History,
    /// Environment variable
    Environment,
    /// Alias
    Alias,
    /// Function
    Function,
    /// Command option
    Option,
    /// Subcommand
    Subcommand,
    /// Option value
    Value,
}

/// Completion item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    /// Completion text
    pub text: String,

    /// Display text (may contain additional information)
    pub display_text: Option<String>,

    /// Completion type (frontend expects field name as kind)
    #[serde(rename = "kind")]
    pub completion_type: String,

    /// Description information
    pub description: Option<String>,

    /// Match score (for sorting)
    pub score: f64,

    /// Completion source (field needed by frontend)
    pub source: Option<String>,

    /// Whether it's an exact match (not used by frontend, skip serialization)
    #[serde(skip)]
    pub exact_match: bool,

    /// Additional metadata (not used by frontend, skip serialization)
    #[serde(skip)]
    pub metadata: HashMap<String, String>,
}

impl CompletionItem {
    /// Create a new completion item
    pub fn new(text: impl Into<String>, completion_type: CompletionType) -> Self {
        Self {
            text: text.into(),
            display_text: None,
            completion_type: completion_type.to_string(),
            description: None,
            score: 0.0,
            source: None,
            exact_match: false,
            metadata: HashMap::new(),
        }
    }

    /// Set display text
    pub fn with_display_text(mut self, display_text: impl Into<String>) -> Self {
        self.display_text = Some(display_text.into());
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set score
    pub fn with_score(mut self, score: f64) -> Self {
        self.score = score;
        self
    }

    /// Set source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set as exact match
    pub fn with_exact_match(mut self, exact: bool) -> Self {
        self.exact_match = exact;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl PartialOrd for CompletionItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for CompletionItem {}

impl Ord for CompletionItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by score descending, if scores are equal then by text alphabetically
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| self.text.cmp(&other.text))
    }
}

impl fmt::Display for CompletionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::File => "file",
                Self::Directory => "directory",
                Self::Command => "command",
                Self::History => "history",
                Self::Environment => "environment",
                Self::Alias => "alias",
                Self::Function => "function",
                Self::Option => "option",
                Self::Subcommand => "subcommand",
                Self::Value => "value",
            }
        )
    }
}

/// Completion context
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// Complete command line currently input
    pub input: String,

    /// Cursor position
    pub cursor_position: usize,

    /// Current working directory
    pub working_directory: PathBuf,

    /// Word currently being completed
    pub current_word: String,

    /// Start position of current word
    pub word_start: usize,

    /// Command line parsing result
    pub parsed_command: Option<ParsedCommand>,
}

impl CompletionContext {
    /// Create a new completion context
    pub fn new(input: String, cursor_position: usize, working_directory: PathBuf) -> Self {
        let (current_word, word_start) = Self::extract_current_word(&input, cursor_position);

        Self {
            input,
            cursor_position,
            working_directory,
            current_word,
            word_start,
            parsed_command: None,
        }
    }

    /// Extract the word currently being edited
    fn extract_current_word(input: &str, cursor_position: usize) -> (String, usize) {
        let chars: Vec<char> = input.chars().collect();
        let cursor_pos = cursor_position.min(chars.len());

        // Search forward for word start
        let mut start = cursor_pos;
        while start > 0 {
            let ch = chars[start - 1];
            if ch.is_whitespace() || ch == '|' || ch == '&' || ch == ';' {
                break;
            }
            start -= 1;
        }

        // Search backward for word end
        let mut end = cursor_pos;
        while end < chars.len() {
            let ch = chars[end];
            if ch.is_whitespace() || ch == '|' || ch == '&' || ch == ';' {
                break;
            }
            end += 1;
        }

        let word: String = chars[start..end].iter().collect();
        (word, start)
    }
}

/// Parsed command
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    /// Command name
    pub command: String,

    /// Argument list
    pub args: Vec<String>,

    /// Position of argument currently being completed
    pub current_arg_index: usize,

    /// Whether completing command name
    pub completing_command: bool,
}

/// Completion request
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionRequest {
    /// Input text
    pub input: String,

    /// Cursor position
    pub cursor_position: usize,

    /// Working directory
    pub working_directory: String,

    /// Maximum number of results to return
    pub max_results: Option<usize>,
}

/// Completion response
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResponse {
    /// List of completion items
    pub items: Vec<CompletionItem>,

    /// Start position for replacement
    pub replace_start: usize,

    /// End position for replacement
    pub replace_end: usize,

    /// Whether there are more results
    pub has_more: bool,
}

/// Command execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecutionContext {
    /// Command text
    pub command: String,

    /// Command arguments
    pub args: Vec<String>,

    /// Execution timestamp
    pub timestamp: u64,

    /// Working directory
    pub working_directory: String,

    /// Command output
    pub output: Option<CommandOutput>,

    /// Exit code
    pub exit_code: Option<i32>,

    /// Execution duration (milliseconds)
    pub duration: Option<u64>,

    /// Environment variables (only save critical ones)
    pub environment: HashMap<String, String>,
}

/// Command output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandOutput {
    /// Standard output
    pub stdout: String,

    /// Standard error output
    pub stderr: String,

    /// Parsed structured data
    pub parsed_data: Option<ParsedOutputData>,
}

/// Parsed output data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedOutputData {
    /// Data type
    pub data_type: OutputDataType,

    /// Extracted entities
    pub entities: Vec<OutputEntity>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Output data type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum OutputDataType {
    /// Process list (e.g., ps, lsof output)
    ProcessList,

    /// File list (e.g., ls output)
    FileList,

    /// Network information (e.g., netstat output)
    NetworkInfo,

    /// System information (e.g., top, htop output)
    SystemInfo,

    /// Git information
    GitInfo,

    /// Package manager information
    PackageInfo,

    /// Service information
    ServiceInfo,

    /// Unknown type
    Unknown,
}

/// Output entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEntity {
    /// Entity type
    pub entity_type: EntityType,

    /// Entity value
    pub value: String,

    /// Entity description
    pub description: Option<String>,

    /// Related attributes
    pub attributes: HashMap<String, String>,

    /// Confidence score
    pub confidence: f64,
}

/// Entity type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    /// Process ID
    ProcessId,

    /// Port number
    Port,

    /// File path
    FilePath,

    /// Directory path
    DirectoryPath,

    /// IP address
    IpAddress,

    /// Username
    Username,

    /// Service name
    ServiceName,

    /// Package name
    PackageName,

    /// Git branch
    GitBranch,

    /// Git commit hash
    GitCommit,

    /// Environment variable
    EnvironmentVariable,

    /// Other
    Other,
}

/// Context session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSession {
    /// Session ID
    pub session_id: String,

    /// Session start time
    pub start_time: u64,

    /// Last activity time
    pub last_activity: u64,

    /// Command execution history
    pub command_history: Vec<CommandExecutionContext>,

    /// Session state
    pub state: SessionState,

    /// Working directory history
    pub directory_history: Vec<String>,
}

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    /// Active state
    Active,

    /// Idle state
    Idle,

    /// Ended
    Ended,
}

impl CommandExecutionContext {
    /// Create a new command execution context
    pub fn new(command: String, args: Vec<String>, working_directory: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            command,
            args,
            timestamp,
            working_directory,
            output: None,
            exit_code: None,
            duration: None,
            environment: HashMap::new(),
        }
    }

    /// Set command output
    pub fn with_output(mut self, output: CommandOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Set exit code
    pub fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = Some(exit_code);
        self
    }

    /// Set execution duration
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Add environment variable
    pub fn with_environment(mut self, key: String, value: String) -> Self {
        self.environment.insert(key, value);
        self
    }

    /// Get full command line
    pub fn get_full_command(&self) -> String {
        if self.args.is_empty() {
            self.command.clone()
        } else {
            format!("{} {}", self.command, self.args.join(" "))
        }
    }

    /// Check if command executed successfully
    pub fn is_successful(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// Get related entities
    pub fn get_entities(&self) -> Vec<&OutputEntity> {
        self.output
            .as_ref()
            .and_then(|output| output.parsed_data.as_ref())
            .map(|data| data.entities.iter().collect())
            .unwrap_or_default()
    }

    /// Get entities by type
    pub fn get_entities_by_type(&self, entity_type: &EntityType) -> Vec<&OutputEntity> {
        self.get_entities()
            .into_iter()
            .filter(|entity| &entity.entity_type == entity_type)
            .collect()
    }
}

impl CommandOutput {
    /// Create a new command output
    pub fn new(stdout: String, stderr: String) -> Self {
        Self {
            stdout,
            stderr,
            parsed_data: None,
        }
    }

    /// Set parsed data
    pub fn with_parsed_data(mut self, parsed_data: ParsedOutputData) -> Self {
        self.parsed_data = Some(parsed_data);
        self
    }

    /// Check if there is output
    pub fn has_output(&self) -> bool {
        !self.stdout.is_empty() || !self.stderr.is_empty()
    }

    /// Get all output text
    pub fn get_all_output(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
            .trim()
            .to_string()
    }
}

impl ParsedOutputData {
    /// Create a new parsed output data
    pub fn new(data_type: OutputDataType) -> Self {
        Self {
            data_type,
            entities: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add entity
    pub fn add_entity(mut self, entity: OutputEntity) -> Self {
        self.entities.push(entity);
        self
    }

    /// Add metadata
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get entities by type
    pub fn get_entities_by_type(&self, entity_type: &EntityType) -> Vec<&OutputEntity> {
        self.entities
            .iter()
            .filter(|entity| &entity.entity_type == entity_type)
            .collect()
    }

    /// Get high confidence entities
    pub fn get_high_confidence_entities(&self, min_confidence: f64) -> Vec<&OutputEntity> {
        self.entities
            .iter()
            .filter(|entity| entity.confidence >= min_confidence)
            .collect()
    }
}

impl OutputEntity {
    /// Create a new output entity
    pub fn new(entity_type: EntityType, value: String, confidence: f64) -> Self {
        Self {
            entity_type,
            value,
            description: None,
            attributes: HashMap::new(),
            confidence,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Check if it's a high confidence entity
    pub fn is_high_confidence(&self, threshold: f64) -> bool {
        self.confidence >= threshold
    }
}

impl ContextSession {
    /// Create a new context session
    pub fn new(session_id: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            session_id,
            start_time: now,
            last_activity: now,
            command_history: Vec::new(),
            state: SessionState::Active,
            directory_history: Vec::new(),
        }
    }

    /// Add command execution context
    pub fn add_command_context(&mut self, context: CommandExecutionContext) {
        self.command_history.push(context);
        self.update_last_activity();

        // Limit history record count
        if self.command_history.len() > 1000 {
            self.command_history.remove(0);
        }
    }

    /// Update last activity time
    pub fn update_last_activity(&mut self) {
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Get recent commands
    pub fn get_recent_commands(&self, count: usize) -> Vec<&CommandExecutionContext> {
        self.command_history.iter().rev().take(count).collect()
    }

    /// Search history by command name
    pub fn search_by_command(&self, command: &str) -> Vec<&CommandExecutionContext> {
        self.command_history
            .iter()
            .filter(|ctx| ctx.command == command)
            .collect()
    }

    /// Get related entities
    pub fn get_related_entities(
        &self,
        entity_type: &EntityType,
        limit: usize,
    ) -> Vec<&OutputEntity> {
        let mut entities = Vec::new();

        for context in self.command_history.iter().rev() {
            for entity in context.get_entities_by_type(entity_type) {
                entities.push(entity);
                if entities.len() >= limit {
                    break;
                }
            }
            if entities.len() >= limit {
                break;
            }
        }

        entities
    }

    /// Check if session is active
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// End session
    pub fn end_session(&mut self) {
        self.state = SessionState::Ended;
        self.update_last_activity();
    }
}
