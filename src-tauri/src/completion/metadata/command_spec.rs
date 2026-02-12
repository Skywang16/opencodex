//! Command specification definitions
//!
//! Define command metadata to replace hardcoded command lists

use serde::{Deserialize, Serialize};

/// Command specification
///
/// Describes a command's behavioral characteristics for intelligent completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    /// Command name
    pub name: String,

    /// Whether files are accepted as arguments
    pub accepts_files: bool,

    /// Whether directories are accepted as arguments
    pub accepts_directories: bool,

    /// List of options that accept files (e.g., --file, -f)
    pub file_options: Vec<String>,

    /// List of options that accept directories (e.g., --dir, -d)
    pub directory_options: Vec<String>,

    /// Command description
    pub description: Option<String>,
}

impl CommandSpec {
    /// Create a new command specification
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            accepts_files: false,
            accepts_directories: false,
            file_options: Vec::new(),
            directory_options: Vec::new(),
            description: None,
        }
    }

    /// Set accepts files
    pub fn with_files(mut self) -> Self {
        self.accepts_files = true;
        self
    }

    /// Set accepts directories
    pub fn with_directories(mut self) -> Self {
        self.accepts_directories = true;
        self
    }

    /// Add file option
    pub fn with_file_option(mut self, option: impl Into<String>) -> Self {
        self.file_options.push(option.into());
        self
    }

    /// Add directory option
    pub fn with_directory_option(mut self, option: impl Into<String>) -> Self {
        self.directory_options.push(option.into());
        self
    }

    /// Add multiple file options
    pub fn with_file_options(mut self, options: Vec<String>) -> Self {
        self.file_options.extend(options);
        self
    }

    /// Add multiple directory options
    pub fn with_directory_options(mut self, options: Vec<String>) -> Self {
        self.directory_options.extend(options);
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Check if option accepts files
    pub fn is_file_option(&self, option: &str) -> bool {
        self.file_options.iter().any(|opt| opt == option)
    }

    /// Check if option accepts directories
    pub fn is_directory_option(&self, option: &str) -> bool {
        self.directory_options.iter().any(|opt| opt == option)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_spec_builder() {
        let spec = CommandSpec::new("test")
            .with_files()
            .with_file_option("--file")
            .with_file_option("-f")
            .with_directory_option("--dir")
            .with_description("Test command");

        assert_eq!(spec.name, "test");
        assert!(spec.accepts_files);
        assert!(!spec.accepts_directories);
        assert_eq!(spec.file_options.len(), 2);
        assert_eq!(spec.directory_options.len(), 1);
        assert!(spec.description.is_some());
    }

    #[test]
    fn test_is_file_option() {
        let spec = CommandSpec::new("test")
            .with_file_options(vec!["--file".to_string(), "-f".to_string()]);

        assert!(spec.is_file_option("--file"));
        assert!(spec.is_file_option("-f"));
        assert!(!spec.is_file_option("--dir"));
    }

    #[test]
    fn test_is_directory_option() {
        let spec = CommandSpec::new("test")
            .with_directory_options(vec!["--dir".to_string(), "-d".to_string()]);

        assert!(spec.is_directory_option("--dir"));
        assert!(spec.is_directory_option("-d"));
        assert!(!spec.is_directory_option("--file"));
    }
}
