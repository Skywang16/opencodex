//! History command completion provider
//!
//! Provides completion suggestions based on user command history

use crate::completion::command_line::extract_command_key;
use crate::completion::error::{CompletionProviderError, CompletionProviderResult};
use crate::completion::providers::CompletionProvider;
use crate::completion::scoring::{
    BaseScorer, CompositeScorer, FrecencyScorer, HistoryScorer, ScoreCalculator, ScoringContext,
};
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::storage::cache::UnifiedCache;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use tokio::fs;

const HISTORY_TTL_DAYS: u64 = 30;
const KEY_MODE_SCAN_LIMIT: usize = 2000;
const KEY_MODE_MAX_RESULTS: usize = 50;
const FULL_MODE_SCAN_LIMIT: usize = 400;
const PSEUDO_RECENCY_STEP_SECS: u64 = 90;

/// Shell type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Unknown,
}

impl ShellType {
    /// Infer shell type from file path
    fn from_path(path: &Path) -> Self {
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            match file_name {
                ".bash_history" => Self::Bash,
                ".zsh_history" => Self::Zsh,
                ".fish_history" => Self::Fish,
                _ => Self::Unknown,
            }
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HistoryEntry {
    command: String,
    last_used_ts: Option<u64>,
}

#[derive(Debug, Clone)]
struct CommandKeyStats {
    count: u64,
    first_index: usize,
    last_used_ts: Option<u64>,
}

/// History command completion provider
pub struct HistoryProvider {
    /// History file path
    history_file: Option<PathBuf>,
    /// Maximum history entries
    max_entries: usize,
    /// Unified cache
    cache: Arc<UnifiedCache>,
    /// Current shell type
    shell_type: ShellType,
}

impl HistoryProvider {
    /// Create new history provider
    pub fn new(cache: Arc<UnifiedCache>) -> Self {
        let mut provider = Self {
            history_file: None,
            max_entries: 1000,
            cache,
            shell_type: ShellType::Unknown,
        };

        // Try to set default history file path
        if let Some(home_dir) = dirs::home_dir() {
            let bash_history = home_dir.join(".bash_history");
            let zsh_history = home_dir.join(".zsh_history");
            let fish_history = home_dir.join(".local/share/fish/fish_history");

            if bash_history.exists() {
                provider = provider.with_history_file(bash_history);
            } else if zsh_history.exists() {
                provider = provider.with_history_file(zsh_history);
            } else if fish_history.exists() {
                provider = provider.with_history_file(fish_history);
            }
        }

        provider
    }

    /// Set history file path
    pub fn with_history_file(mut self, path: PathBuf) -> Self {
        self.shell_type = ShellType::from_path(&path);
        self.history_file = Some(path);
        self
    }

    /// Read history from file
    async fn read_history(&self) -> CompletionProviderResult<Vec<HistoryEntry>> {
        let cache_key = "completion:history:commands";

        // Try to get from cache
        if let Some(cached_value) = self.cache.get(cache_key).await {
            if let Ok(entries) = serde_json::from_value::<Vec<HistoryEntry>>(cached_value) {
                return Ok(entries);
            }
        }

        // Cache miss, read from file
        if let Some(history_file) = &self.history_file {
            if history_file.exists() {
                let content = fs::read_to_string(history_file).await.map_err(|e| {
                    CompletionProviderError::io(
                        "read history file",
                        format!("({})", history_file.display()),
                        e,
                    )
                })?;
                let entries = self.parse_history_content(&content);

                // Store in cache
                if let Ok(entries_value) = serde_json::to_value(&entries) {
                    let _ = self.cache.set(cache_key, entries_value).await;
                }

                return Ok(entries);
            }
        }

        Ok(Vec::new())
    }

    /// Parse history file content, supports different shell formats
    fn parse_history_content(&self, content: &str) -> Vec<HistoryEntry> {
        let mut entries = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse different formats based on shell type
            let entry = match self.shell_type {
                ShellType::Zsh => {
                    if line.starts_with(": ") && line.contains(';') {
                        let timestamp = line
                            .split(';')
                            .next()
                            .and_then(|head| head.split(':').nth(1))
                            .and_then(|ts| ts.trim().parse::<u64>().ok());

                        line.find(';')
                            .map(|pos| line[pos + 1..].trim())
                            .filter(|cmd| !cmd.is_empty())
                            .map(|cmd| HistoryEntry {
                                command: cmd.to_string(),
                                last_used_ts: timestamp,
                            })
                    } else {
                        Some(HistoryEntry {
                            command: line.to_string(),
                            last_used_ts: None,
                        })
                    }
                }
                ShellType::Fish => {
                    // Fish history format is usually YAML, simplified handling here
                    if line.starts_with("- cmd: ") {
                        Some(HistoryEntry {
                            command: line[8..].trim().to_string(),
                            last_used_ts: None,
                        })
                    } else {
                        None
                    }
                }
                _ => {
                    // Bash and other formats: direct commands
                    Some(HistoryEntry {
                        command: line.to_string(),
                        last_used_ts: None,
                    })
                }
            };

            if let Some(entry) = entry {
                entries.push(entry);
            }
        }

        // Deduplicate and keep most recent commands
        self.deduplicate_entries(entries)
    }

    /// Deduplicate commands, keep newest (return order: new -> old)
    fn deduplicate_entries(&self, entries: Vec<HistoryEntry>) -> Vec<HistoryEntry> {
        let mut unique_entries = Vec::new();
        let mut seen = HashSet::new();

        // Traverse from back to front (file tail is usually "newer"), keep newest one; don't reverse, maintain new->old order
        for entry in entries.into_iter().rev().take(self.max_entries) {
            if seen.insert(entry.command.clone()) {
                unique_entries.push(entry);
            }
        }

        unique_entries
    }

    /// Get matching history commands
    async fn get_matching_commands(
        &self,
        pattern: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let entries = self.read_history().await?;
        let mut matches = Vec::new();

        let now_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let cutoff_ts = now_ts.saturating_sub(HISTORY_TTL_DAYS * 24 * 60 * 60);

        if pattern.contains(' ') {
            // User has typed to "argument level", return full commands, but only scan recent section to avoid old junk flooding.
            for (index, entry) in entries.iter().take(FULL_MODE_SCAN_LIMIT).enumerate() {
                if entry.last_used_ts.is_some_and(|ts| ts < cutoff_ts) {
                    continue;
                }

                if self.is_command_match(&entry.command, pattern) {
                    let score = self.calculate_command_score(
                        &entry.command,
                        pattern,
                        index,
                        entry.last_used_ts,
                    );
                    matches.push(
                        CompletionItem::new(entry.command.clone(), CompletionType::History)
                            .with_score(score)
                            .with_description(format!(
                                "History command ({})",
                                self.shell_type_name()
                            ))
                            .with_source("history".to_string()),
                    );
                }
            }
        } else {
            // Only typed root (e.g., "git"): don't return full line commands with hash/path.
            // Here do key aggregation (git status / git show / docker ps ...) with frequency + recency.
            let mut stats: HashMap<String, CommandKeyStats> = HashMap::new();
            for (index, entry) in entries.iter().take(KEY_MODE_SCAN_LIMIT).enumerate() {
                if entry.last_used_ts.is_some_and(|ts| ts < cutoff_ts) {
                    continue;
                }

                let Some(key) = extract_command_key(&entry.command) else {
                    continue;
                };

                if !key.key.starts_with(pattern) || key.key == pattern {
                    continue;
                }

                stats
                    .entry(key.key)
                    .and_modify(|s| {
                        s.count = s.count.saturating_add(1);
                        s.first_index = s.first_index.min(index);
                        if s.last_used_ts.is_none() {
                            s.last_used_ts = entry.last_used_ts;
                        }
                    })
                    .or_insert(CommandKeyStats {
                        count: 1,
                        first_index: index,
                        last_used_ts: entry.last_used_ts,
                    });
            }

            for (key, s) in stats {
                let pseudo_ts =
                    now_ts.saturating_sub((s.first_index as u64) * PSEUDO_RECENCY_STEP_SECS);
                let ts = s.last_used_ts.unwrap_or(pseudo_ts);
                let score =
                    self.calculate_command_key_score(pattern, &key, s.first_index, s.count, ts);
                matches.push(
                    CompletionItem::new(key, CompletionType::History)
                        .with_score(score)
                        .with_description(format!("History command ({})", self.shell_type_name()))
                        .with_source("history".to_string()),
                );
            }
        }

        // Sort by score (using CompletionItem's Ord implementation)
        matches.sort_unstable();
        matches.truncate(KEY_MODE_MAX_RESULTS);

        Ok(matches)
    }

    /// Check if command matches pattern
    fn is_command_match(&self, command: &str, pattern: &str) -> bool {
        command.starts_with(pattern) && command != pattern
    }

    /// Calculate command score (using unified scoring system)
    fn calculate_command_score(
        &self,
        command: &str,
        pattern: &str,
        index: usize,
        last_used_ts: Option<u64>,
    ) -> f64 {
        let is_prefix_match = command.starts_with(pattern);
        let history_weight = Self::calculate_history_weight(index);

        let mut context = ScoringContext::new(pattern, command)
            .with_prefix_match(is_prefix_match)
            .with_history_weight(history_weight)
            .with_history_position(index)
            .with_source("history");

        if let Some(ts) = last_used_ts {
            context = context.with_last_used_timestamp(ts);
        }

        let scorer = CompositeScorer::new(vec![
            Box::new(BaseScorer),
            Box::new(HistoryScorer),
            Box::new(FrecencyScorer),
        ]);

        scorer.calculate(&context)
    }

    fn calculate_command_key_score(
        &self,
        pattern: &str,
        key: &str,
        index: usize,
        frequency: u64,
        last_used_ts: u64,
    ) -> f64 {
        let is_prefix_match = key.starts_with(pattern);
        let history_weight = Self::calculate_history_weight(index);

        let context = ScoringContext::new(pattern, key)
            .with_prefix_match(is_prefix_match)
            .with_history_weight(history_weight)
            .with_history_position(index)
            .with_frequency(frequency as usize)
            .with_last_used_timestamp(last_used_ts)
            .with_source("history");

        let scorer = CompositeScorer::new(vec![
            Box::new(BaseScorer),
            Box::new(HistoryScorer),
            Box::new(FrecencyScorer),
        ]);

        scorer.calculate(&context)
    }

    /// Calculate history weight (based on position)
    ///
    /// Newer commands have higher weight, using exponential decay
    fn calculate_history_weight(index: usize) -> f64 {
        // First 100 commands weight decays from 1.0 to 0.1
        // After 100, weight fixed at 0.1
        if index < 100 {
            1.0 - (index as f64 / 100.0) * 0.9
        } else {
            0.1
        }
    }

    /// Get shell type name
    fn shell_type_name(&self) -> &'static str {
        match self.shell_type {
            ShellType::Bash => "bash",
            ShellType::Zsh => "zsh",
            ShellType::Fish => "fish",
            ShellType::Unknown => "shell",
        }
    }
}

#[async_trait]
impl CompletionProvider for HistoryProvider {
    fn name(&self) -> &'static str {
        "history"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        if self.history_file.is_none() {
            return false;
        }

        let word = &context.current_word;
        !word.is_empty()
            && !word.starts_with('/')
            && !word.starts_with('\\')
            && !word.starts_with('.')
            && !word.starts_with('-')  // Exclude option arguments
            && word.len() >= 2 // At least 2 characters before starting history completion
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        if context.current_word.is_empty() {
            return Ok(Vec::new());
        }

        self.get_matching_commands(&context.current_word).await
    }

    fn priority(&self) -> i32 {
        15 // History commands have highest priority - intelligently learns user habits
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::cache::UnifiedCache;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_shell_type_detection() {
        let bash_path = PathBuf::from("/home/user/.bash_history");
        let zsh_path = PathBuf::from("/home/user/.zsh_history");
        let fish_path = PathBuf::from("/home/user/.fish_history");
        let unknown_path = PathBuf::from("/home/user/.unknown_history");

        assert_eq!(ShellType::from_path(&bash_path), ShellType::Bash);
        assert_eq!(ShellType::from_path(&zsh_path), ShellType::Zsh);
        assert_eq!(ShellType::from_path(&fish_path), ShellType::Fish);
        assert_eq!(ShellType::from_path(&unknown_path), ShellType::Unknown);
    }

    #[tokio::test]
    async fn test_bash_history_parsing() {
        let cache = Arc::new(UnifiedCache::new());
        let provider = HistoryProvider::new(cache);

        let bash_content = r#"ls -la
cd /home/user
git status
npm install
ls -la
git commit -m "test"
"#;

        let entries = provider.parse_history_content(bash_content);
        let commands: Vec<String> = entries.into_iter().map(|e| e.command).collect();

        // Should deduplicate, keep newest commands
        assert!(commands.contains(&"ls -la".to_string()));
        assert!(commands.contains(&"cd /home/user".to_string()));
        assert!(commands.contains(&"git status".to_string()));
        assert!(commands.contains(&"npm install".to_string()));
        assert!(commands.contains(&"git commit -m \"test\"".to_string()));

        let ls_count = commands.iter().filter(|&cmd| cmd == "ls -la").count();
        assert_eq!(ls_count, 1);
    }

    #[tokio::test]
    async fn test_zsh_history_parsing() {
        let cache = Arc::new(UnifiedCache::new());
        let zsh_path = PathBuf::from(".zsh_history");
        let provider = HistoryProvider::new(cache).with_history_file(zsh_path);

        let zsh_content = r#": 1640995200:0;ls -la
: 1640995201:0;cd /home/user
: 1640995202:0;git status
# comment line
: 1640995203:0;npm install
"#;

        let entries = provider.parse_history_content(zsh_content);
        let commands: Vec<String> = entries.iter().map(|e| e.command.clone()).collect();

        assert!(commands.contains(&"ls -la".to_string()));
        assert!(commands.contains(&"cd /home/user".to_string()));
        assert!(commands.contains(&"git status".to_string()));
        assert!(commands.contains(&"npm install".to_string()));
        assert_eq!(commands.len(), 4);

        // zsh: should parse timestamp
        assert!(entries
            .iter()
            .any(|e| e.command == "ls -la" && e.last_used_ts.is_some()));
    }

    #[tokio::test]
    async fn test_command_matching() {
        let cache = Arc::new(UnifiedCache::new());
        let provider = HistoryProvider::new(cache);

        // Test matching logic
        assert!(provider.is_command_match("git status", "git"));
        assert!(provider.is_command_match("git commit", "git"));
        assert!(!provider.is_command_match("git", "git")); // Exact match doesn't match
        assert!(!provider.is_command_match("ls", "git")); // Doesn't match
    }

    #[tokio::test]
    async fn test_score_calculation() {
        let cache = Arc::new(UnifiedCache::new());
        let provider = HistoryProvider::new(cache);

        // Test same input, different positions
        let score1 = provider.calculate_command_score("git status", "git", 0, None);
        let score2 = provider.calculate_command_score("git status", "git", 10, None);
        let score3 = provider.calculate_command_score("git status", "git", 50, None);

        // Earlier positions have higher scores (new scoring system uses history weight + position bonus)
        assert!(
            score1 >= score2,
            "Position 0 should >= position 10: {score1} vs {score2}"
        );
        assert!(
            score2 >= score3,
            "Position 10 should >= position 50: {score2} vs {score3}"
        );

        // Test match quality: shorter input matching shorter command should score higher
        let score_short = provider.calculate_command_score("git", "g", 0, None);
        let score_long = provider.calculate_command_score("git status", "git", 0, None);

        // Both are prefix matches, scores should both be greater than 0
        assert!(
            score_short > 0.0,
            "Short match should have score: {score_short}"
        );
        assert!(
            score_long > 0.0,
            "Long match should have score: {score_long}"
        );
    }

    #[tokio::test]
    async fn test_should_provide_logic() {
        let cache = Arc::new(UnifiedCache::new());
        let bash_path = PathBuf::from(".bash_history");
        let provider = HistoryProvider::new(cache).with_history_file(bash_path);

        let context_valid =
            CompletionContext::new("git".to_string(), 3, PathBuf::from("/home/user"));

        let context_too_short =
            CompletionContext::new("g".to_string(), 1, PathBuf::from("/home/user"));

        let context_path =
            CompletionContext::new("/home".to_string(), 5, PathBuf::from("/home/user"));

        assert!(provider.should_provide(&context_valid));
        assert!(!provider.should_provide(&context_too_short));
        assert!(!provider.should_provide(&context_path));
    }
}
