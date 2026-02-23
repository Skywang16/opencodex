//! System command completion provider
//!
//! Provides completion for executable commands in system PATH

use crate::completion::error::CompletionProviderResult;
use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use async_trait::async_trait;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::collections::HashSet;
use std::env;
use std::path::Path;
use tokio::fs;
use tokio::sync::OnceCell;

/// System command completion provider
pub struct SystemCommandsProvider {
    /// Cached command list
    commands: OnceCell<HashSet<String>>,
    /// Fuzzy matcher
    matcher: SkimMatcherV2,
}

impl SystemCommandsProvider {
    /// Create new system command provider
    pub fn new() -> Self {
        Self {
            commands: OnceCell::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    async fn get_commands(&self) -> CompletionProviderResult<&HashSet<String>> {
        Ok(self
            .commands
            .get_or_init(|| async { Self::scan_commands().await })
            .await)
    }

    async fn scan_commands() -> HashSet<String> {
        let mut commands = HashSet::new();

        let path_var = env::var("PATH").unwrap_or_default();
        let paths = path_var.split(if cfg!(windows) { ';' } else { ':' });

        for path_str in paths {
            if path_str.is_empty() {
                continue;
            }

            let path = Path::new(path_str);
            if !path.exists() || !path.is_dir() {
                continue;
            }

            if let Ok(mut entries) = fs::read_dir(path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let entry_path = entry.path();

                    if let Some(file_name) = entry_path.file_name() {
                        if Self::is_executable(&entry_path).await {
                            commands.insert(file_name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        commands
    }

    /// Check if file is executable
    async fn is_executable(path: &Path) -> bool {
        if let Ok(metadata) = fs::metadata(path).await {
            if metadata.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = metadata.permissions();
                    return permissions.mode() & 0o111 != 0;
                }

                #[cfg(windows)]
                {
                    // On Windows, check file extension
                    if let Some(extension) = path.extension() {
                        let ext = extension.to_string_lossy().to_lowercase();
                        return matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com");
                    }
                }

                #[cfg(not(any(unix, windows)))]
                {
                    return true; // Other platforms default to executable
                }
            }
        }

        false
    }

    /// Get matching commands - using fuzzy matching
    async fn get_matching_commands(
        &self,
        pattern: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let commands = self.get_commands().await?;

        let mut matches = Vec::new();

        for command in commands.iter() {
            // Use fuzzy matcher for matching, but exclude exact same commands (no completion value)
            if command != pattern {
                if let Some(score) = self.matcher.fuzzy_match(command, pattern) {
                    let normalized = ((score as f64) / 100.0 * 60.0 + 40.0).min(100.0);
                    let item = CompletionItem::new(command.clone(), CompletionType::Command)
                        .with_score(normalized)
                        .with_description(format!("System command: {command}"))
                        .with_source("system_commands".to_string());

                    matches.push(item);
                }
            }
        }

        // Sort by match score (using CompletionItem's Ord implementation)
        matches.sort_unstable();

        Ok(matches)
    }
}

#[async_trait]
impl CompletionProvider for SystemCommandsProvider {
    fn name(&self) -> &'static str {
        "system_commands"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        let parts: Vec<&str> = context.input.split_whitespace().collect();

        if parts.is_empty() {
            return false;
        }

        let is_first_word =
            parts.len() == 1 || (parts.len() > 1 && context.cursor_position <= parts[0].len());

        is_first_word
            && !context.current_word.contains('/')
            && !context.current_word.contains('\\')
            && !context.current_word.starts_with('.')
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
        8 // System commands have higher priority
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for SystemCommandsProvider {
    fn default() -> Self {
        Self::new()
    }
}
