//! Command registry
//!
//! Manages metadata for all known commands

use super::command_spec::CommandSpec;
use crate::completion::CompletionRuntime;
use std::collections::HashMap;

/// Command registry
pub struct CommandRegistry {
    commands: HashMap<String, CommandSpec>,
}

impl CommandRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Get global registry instance
    pub fn global() -> &'static CommandRegistry {
        CompletionRuntime::global().registry()
    }

    /// Register command
    pub fn register(&mut self, spec: CommandSpec) {
        self.commands.insert(spec.name.clone(), spec);
    }

    /// Register multiple commands
    pub fn register_all(&mut self, specs: Vec<CommandSpec>) {
        for spec in specs {
            self.register(spec);
        }
    }

    /// Lookup command specification
    pub fn lookup(&self, command: &str) -> Option<&CommandSpec> {
        self.commands.get(command)
    }

    /// Check if command accepts files
    pub fn accepts_files(&self, command: &str) -> bool {
        self.lookup(command)
            .map(|spec| spec.accepts_files)
            .unwrap_or(false)
    }

    /// Check if command accepts directories
    pub fn accepts_directories(&self, command: &str) -> bool {
        self.lookup(command)
            .map(|spec| spec.accepts_directories)
            .unwrap_or(false)
    }

    /// Check if option accepts files
    pub fn is_file_option(&self, command: &str, option: &str) -> bool {
        self.lookup(command)
            .map(|spec| spec.is_file_option(option))
            .unwrap_or_else(|| Self::is_common_file_option(option))
    }

    /// Check if option accepts directories
    pub fn is_directory_option(&self, command: &str, option: &str) -> bool {
        self.lookup(command)
            .map(|spec| spec.is_directory_option(option))
            .unwrap_or_else(|| Self::is_common_directory_option(option))
    }

    /// Common file options (fallback check)
    fn is_common_file_option(option: &str) -> bool {
        matches!(
            option,
            "--file" | "--input" | "--output" | "--config" | "--script" | "-f" | "-i" | "-o" | "-c"
        )
    }

    /// Common directory options (fallback check)
    fn is_common_directory_option(option: &str) -> bool {
        matches!(
            option,
            "--directory" | "--dir" | "--path" | "--workdir" | "-d" | "-p"
        )
    }

    /// Load builtin commands
    pub(crate) fn load_builtin_commands(&mut self) {
        let builtin_commands = super::builtin::load_builtin_commands();
        self.register_all(builtin_commands);
    }

    /// Get count of all registered commands
    pub fn count(&self) -> usize {
        self.commands.len()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_basic() {
        let mut registry = CommandRegistry::new();

        let spec = CommandSpec::new("test")
            .with_files()
            .with_file_option("--file");

        registry.register(spec);

        assert!(registry.lookup("test").is_some());
        assert!(registry.lookup("nonexistent").is_none());
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_accepts_files() {
        let mut registry = CommandRegistry::new();
        registry.register(CommandSpec::new("cat").with_files());

        assert!(registry.accepts_files("cat"));
        assert!(!registry.accepts_files("ls"));
    }

    #[test]
    fn test_is_file_option() {
        let mut registry = CommandRegistry::new();
        registry.register(CommandSpec::new("test").with_file_option("--file"));

        assert!(registry.is_file_option("test", "--file"));
        assert!(!registry.is_file_option("test", "--dir"));

        // Test common fallback options
        assert!(registry.is_file_option("unknown", "--input"));
    }

    #[test]
    fn test_global_registry() {
        let registry = CommandRegistry::global();

        // Should have loaded builtin commands
        assert!(registry.count() > 0);

        // Test some common commands
        assert!(registry.accepts_files("cat"));
        assert!(registry.accepts_files("vim"));
    }
}
