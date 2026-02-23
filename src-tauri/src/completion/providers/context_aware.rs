//! Context-aware completion provider
//!
//! Provides intelligent completion suggestions based on command execution history and output results

use crate::completion::error::{CompletionProviderError, CompletionProviderResult};
use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Command output record
#[derive(Debug, Clone)]
pub struct CommandOutputRecord {
    /// Command text
    pub command: String,
    /// Command output
    pub output: String,
    /// Execution timestamp
    pub timestamp: u64,
    /// Working directory
    pub working_directory: String,
    /// Extracted entities (e.g., PID, port, etc.)
    pub extracted_entities: HashMap<String, Vec<String>>,
}

/// Context-aware completion provider
pub struct ContextAwareProvider {
    /// Command output history
    command_history: Arc<RwLock<Vec<CommandOutputRecord>>>,
    /// Maximum history records
    max_history: usize,
}

impl ContextAwareProvider {
    /// Create new context-aware provider
    pub fn new() -> Self {
        Self {
            command_history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        }
    }

    /// Get recent PID list (public method for external use)
    pub fn get_recent_pids(&self) -> Vec<String> {
        let history = match self.command_history.read() {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let mut pids = Vec::new();
        for record in history.iter().rev().take(20) {
            if let Some(record_pids) = record.extracted_entities.get("pid") {
                pids.extend(record_pids.clone());
            }
        }

        // Deduplicate and limit count
        pids.sort();
        pids.dedup();
        pids.truncate(10);
        pids
    }

    /// Get recent port list (public method for external use)
    pub fn get_recent_ports(&self) -> Vec<String> {
        let history = match self.command_history.read() {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let mut ports = Vec::new();
        for record in history.iter().rev().take(20) {
            if let Some(record_ports) = record.extracted_entities.get("port") {
                ports.extend(record_ports.clone());
            }
        }

        ports.sort();
        ports.dedup();
        ports.truncate(10);
        ports
    }

    /// Get recently accessed path list (public method for external use)
    pub fn get_recent_paths(&self) -> Vec<String> {
        let history = match self.command_history.read() {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let mut paths = Vec::new();
        for record in history.iter().rev().take(20) {
            if let Some(file_paths) = record.extracted_entities.get("file_path") {
                paths.extend(file_paths.clone());
            }
            if let Some(dir_paths) = record.extracted_entities.get("directory_path") {
                paths.extend(dir_paths.clone());
            }
        }

        paths.sort();
        paths.dedup();
        paths.truncate(10);
        paths
    }

    /// Get last command and its output (for predictor use)
    pub fn get_last_command(&self) -> Option<(String, String)> {
        let history = match self.command_history.read() {
            Ok(h) => h,
            Err(_) => return None,
        };

        history
            .last()
            .map(|record| (record.command.clone(), record.output.clone()))
    }

    /// Record command output (caller has already provided extracted entities)
    pub fn record_command_output_with_entities(
        &self,
        command: String,
        output: String,
        working_directory: String,
        extracted_entities: HashMap<String, Vec<String>>,
        timestamp: u64,
    ) -> CompletionProviderResult<()> {
        let record = CommandOutputRecord {
            command: command.clone(),
            output,
            timestamp,
            working_directory,
            extracted_entities,
        };

        let mut history =
            self.command_history
                .write()
                .map_err(|_| CompletionProviderError::MutexPoisoned {
                    resource: "command_history",
                })?;

        history.push(record);

        // Limit history record count
        if history.len() > self.max_history {
            history.remove(0);
        }

        Ok(())
    }

    /// Get relevant completion suggestions
    fn get_contextual_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // Analyze current input to determine what type of completion is needed
        let current_command = context.current_word.clone();

        let history =
            self.command_history
                .read()
                .map_err(|_| CompletionProviderError::MutexPoisoned {
                    resource: "command_history",
                })?;

        // Provide corresponding completions based on current command type
        match &*current_command {
            "kill" | "killall" => {
                // Provide PID completions for kill commands
                items.extend(self.get_pid_completions(&history)?);
            }
            "nc" | "telnet" | "ssh" => {
                // Provide port and IP completions for network commands
                items.extend(self.get_network_completions(&history)?);
            }
            "cd" | "ls" | "cat" | "vim" | "nano" => {
                // Provide path completions for file operation commands (can combine with filesystem provider)
                items.extend(self.get_path_completions(&history)?);
            }
            _ => {
                // General completions: find relevant entities
                items.extend(self.get_general_completions(&history, &current_command)?);
            }
        }

        Ok(items)
    }

    /// Get PID completion suggestions
    fn get_pid_completions(
        &self,
        history: &[CommandOutputRecord],
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // Find recent process-related command outputs
        for record in history.iter().rev().take(10) {
            if let Some(pids) = record.extracted_entities.get("pid") {
                for pid in pids {
                    let description = if let Some(process_names) =
                        record.extracted_entities.get("process_name")
                    {
                        process_names.first().map(|name| format!("Process: {name}"))
                    } else {
                        Some("Process ID".to_string())
                    };

                    let item = CompletionItem::new(pid.clone(), CompletionType::Value)
                        .with_score(80.0) // High score, as it's context-related
                        .with_description(description.unwrap_or_default())
                        .with_source("context".to_string())
                        .with_metadata("type".to_string(), "pid".to_string())
                        .with_metadata("from_command".to_string(), record.command.clone());

                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    /// Get network-related completion suggestions
    fn get_network_completions(
        &self,
        history: &[CommandOutputRecord],
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        for record in history.iter().rev().take(10) {
            // Add port completions
            if let Some(ports) = record.extracted_entities.get("port") {
                for port in ports {
                    let item = CompletionItem::new(port.clone(), CompletionType::Value)
                        .with_score(75.0)
                        .with_description("Port number".to_string())
                        .with_source("context".to_string())
                        .with_metadata("type".to_string(), "port".to_string());

                    items.push(item);
                }
            }

            // Add IP address completions
            if let Some(ips) = record.extracted_entities.get("ip") {
                for ip in ips {
                    let item = CompletionItem::new(ip.clone(), CompletionType::Value)
                        .with_score(75.0)
                        .with_description("IP address".to_string())
                        .with_source("context".to_string())
                        .with_metadata("type".to_string(), "ip".to_string());

                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    /// Get path-related completion suggestions
    fn get_path_completions(
        &self,
        history: &[CommandOutputRecord],
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        for record in history.iter().rev().take(5) {
            if let Some(paths) = record.extracted_entities.get("path") {
                for path in paths {
                    let item = CompletionItem::new(path.clone(), CompletionType::File)
                        .with_score(70.0)
                        .with_description("File path".to_string())
                        .with_source("context".to_string())
                        .with_metadata("type".to_string(), "path".to_string());

                    items.push(item);
                }
            }
        }

        Ok(items)
    }

    /// Get general completion suggestions
    fn get_general_completions(
        &self,
        history: &[CommandOutputRecord],
        command: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        // Provide basic completions based on command type
        match command {
            "cd" => {
                // Provide directory completions for cd command
                if let Some(record) = history.last() {
                    if let Some(dirs) = record.extracted_entities.get("directory") {
                        for dir in dirs {
                            let item = CompletionItem::new(dir.clone(), CompletionType::Directory)
                                .with_score(60.0)
                                .with_description("Directory".to_string())
                                .with_source("context".to_string());
                            items.push(item);
                        }
                    }
                }
            }
            "cat" | "less" | "more" | "head" | "tail" => {
                // Provide file completions for file viewing commands
                if let Some(record) = history.last() {
                    if let Some(files) = record.extracted_entities.get("file") {
                        for file in files {
                            let item = CompletionItem::new(file.clone(), CompletionType::File)
                                .with_score(60.0)
                                .with_description("File".to_string())
                                .with_source("context".to_string());
                            items.push(item);
                        }
                    }
                }
            }
            _ => {
                // Other commands don't provide special completions for now
            }
        }

        Ok(items)
    }
}

#[async_trait]
impl CompletionProvider for ContextAwareProvider {
    fn name(&self) -> &'static str {
        "context_aware"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        // Always try to provide context-aware completions
        !context.current_word.is_empty()
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        self.get_contextual_completions(context)
    }

    fn priority(&self) -> i32 {
        20 // Highest priority, as it's context-related
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for ContextAwareProvider {
    fn default() -> Self {
        Self::new()
    }
}
