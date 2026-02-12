//! Checkpoint configuration system

use std::time::Duration;

/// Checkpoint system configuration
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Maximum file size (bytes), files exceeding this size will not be snapshotted
    pub max_file_size: u64,

    /// Ignored file patterns (glob format)
    pub ignored_patterns: Vec<String>,

    /// Maximum checkpoint count (automatically clean old ones after exceeding)
    pub max_checkpoints: usize,

    /// Automatic garbage collection interval
    pub gc_interval: Duration,

    /// Stream processing buffer size
    pub stream_buffer_size: usize,

    /// Maximum number of files to process concurrently
    pub max_concurrent_files: usize,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            max_file_size: 50 * 1024 * 1024, // 50MB
            ignored_patterns: vec![
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                ".git/**".to_string(),
                "*.log".to_string(),
                "*.tmp".to_string(),
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
            ],
            max_checkpoints: 100,
            gc_interval: Duration::from_secs(300), // 5 minutes
            stream_buffer_size: 64 * 1024,         // 64KB
            max_concurrent_files: 10,
        }
    }
}

impl CheckpointConfig {
    /// Check if file should be ignored
    pub fn should_ignore_file(&self, file_path: &str) -> bool {
        for pattern in &self.ignored_patterns {
            if glob_match(pattern, file_path) {
                return true;
            }
        }
        false
    }

    /// Check if file size exceeds limit
    pub fn is_file_too_large(&self, size: u64) -> bool {
        size > self.max_file_size
    }
}

/// Simple glob matching implementation
fn glob_match(pattern: &str, text: &str) -> bool {
    // Simplified version, should actually use glob crate
    if let Some(prefix) = pattern.strip_suffix("/**") {
        text.starts_with(prefix)
    } else if pattern.contains('*') {
        // Simple wildcard matching
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            text.starts_with(parts[0]) && text.ends_with(parts[1])
        } else {
            false
        }
    } else {
        text == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore_file() {
        let config = CheckpointConfig::default();

        assert!(config.should_ignore_file("node_modules/react/index.js"));
        assert!(config.should_ignore_file("target/debug/main"));
        assert!(config.should_ignore_file("test.log"));
        assert!(!config.should_ignore_file("src/main.rs"));
    }

    #[test]
    fn test_file_size_limit() {
        let config = CheckpointConfig::default();

        assert!(!config.is_file_too_large(1024)); // 1KB
        assert!(config.is_file_too_large(100 * 1024 * 1024)); // 100MB
    }
}
