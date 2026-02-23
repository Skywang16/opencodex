//! Filesystem completion provider

use crate::completion::error::{CompletionProviderError, CompletionProviderResult};
use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use async_trait::async_trait;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

/// Filesystem completion provider
pub struct FilesystemProvider {
    /// Fuzzy matcher
    matcher: SkimMatcherV2,
    /// Maximum search depth
    max_depth: usize,
    /// Whether to show hidden files
    show_hidden: bool,
}

impl FilesystemProvider {
    /// Create a new filesystem provider
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            max_depth: 3,
            show_hidden: false,
        }
    }

    /// Resolve path, handling relative and absolute paths
    fn resolve_path(&self, input: &str, working_dir: &Path) -> PathBuf {
        let path = Path::new(input);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            working_dir.join(path)
        }
    }

    /// Get files and subdirectories in a directory
    async fn get_directory_entries(
        &self,
        dir_path: &Path,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        if !dir_path.exists() || !dir_path.is_dir() {
            return Ok(items);
        }

        let mut entries = fs::read_dir(dir_path).await.map_err(|e| {
            CompletionProviderError::io("read directory", format!("({})", dir_path.display()), e)
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            CompletionProviderError::io(
                "read directory entry",
                format!("({})", dir_path.display()),
                e,
            )
        })? {
            let path = entry.path();
            let file_name = match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            // Skip hidden files (unless explicitly requested to show)
            if !self.show_hidden && file_name.starts_with('.') {
                continue;
            }

            let metadata = entry.metadata().await.map_err(|e| {
                CompletionProviderError::io("read metadata", format!("({})", path.display()), e)
            })?;

            let completion_type = if metadata.is_dir() {
                CompletionType::Directory
            } else {
                CompletionType::File
            };

            let mut text = file_name.clone();
            if metadata.is_dir() {
                text = format!("{text}/");
            }

            let mut item = CompletionItem::new(text, completion_type)
                .with_source("filesystem".to_string())
                .with_score(if metadata.is_dir() { 60.0 } else { 55.0 });

            // Add trailing slash for directories
            if metadata.is_dir() {
                item = item.with_display_text(format!("{file_name}/"));
            }

            // Add file size information
            if metadata.is_file() {
                let size = metadata.len();
                let size_str = format_file_size(size);
                item = item.with_metadata("size".to_string(), size_str);
            }

            items.push(item);
        }

        Ok(items)
    }

    /// Recursively search files (for deep search)
    fn search_files_recursive(
        &self,
        search_dir: &Path,
        pattern: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let mut items = Vec::new();

        for entry in WalkDir::new(search_dir)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let file_name = match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            // Skip hidden files
            if !self.show_hidden && file_name.starts_with('.') {
                continue;
            }

            // Fuzzy matching
            if let Some(score) = self.matcher.fuzzy_match(&file_name, pattern) {
                let completion_type = if path.is_dir() {
                    CompletionType::Directory
                } else {
                    CompletionType::File
                };

                let relative_path = path
                    .strip_prefix(search_dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();

                let mut text = relative_path.clone();
                if path.is_dir() {
                    text = format!("{text}/");
                }

                let mut item = CompletionItem::new(text, completion_type)
                    .with_score(((score as f64) / 100.0 * 60.0 + 40.0).min(100.0))
                    .with_source("filesystem".to_string());

                if path.is_dir() {
                    item = item.with_display_text(format!("{relative_path}/"));
                }

                items.push(item);
            }
        }

        // Sort by score (using CompletionItem's Ord implementation)
        items.sort_unstable();

        Ok(items)
    }
}

#[async_trait]
impl CompletionProvider for FilesystemProvider {
    fn name(&self) -> &'static str {
        "filesystem"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        // Provide filesystem completion only in the following cases:
        // 1. Current word contains path separators (explicit path input)
        if context.current_word.contains('/') || context.current_word.contains('\\') {
            return true;
        }

        // 2. Current word starts with . (relative path)
        if context.current_word.starts_with('.') {
            return true;
        }

        // 3. Current word starts with ~ (user home directory)
        if context.current_word.starts_with('~') {
            return true;
        }

        // 4. Current word starts with / (absolute path)
        if context.current_word.starts_with('/') {
            return true;
        }

        // No longer provide file completion for empty words to avoid interfering with command option completion
        false
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let current_word = &context.current_word;

        if current_word.is_empty() {
            return self.get_directory_entries(&context.working_directory).await;
        }

        // Resolve path
        let full_path = self.resolve_path(current_word, &context.working_directory);

        let result = if full_path.is_dir() {
            self.get_directory_entries(&full_path).await
        } else {
            let parent_dir = full_path.parent().unwrap_or(&context.working_directory);

            let file_name = full_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if parent_dir.exists() && parent_dir.is_dir() {
                let mut items = self.get_directory_entries(parent_dir).await?;

                // Filter matching items
                if !file_name.is_empty() {
                    items = items
                        .into_iter()
                        .filter_map(|mut item| {
                            let haystack = item.text.trim_end_matches('/');
                            if let Some(score) = self.matcher.fuzzy_match(haystack, &file_name) {
                                // Update score
                                item.score = ((score as f64) / 100.0 * 60.0 + 40.0).min(100.0);
                                Some(item)
                            } else {
                                None
                            }
                        })
                        .collect();

                    // Sort by score (using CompletionItem's Ord implementation)
                    items.sort_unstable();
                }

                Ok(items)
            } else {
                self.search_files_recursive(&context.working_directory, &file_name)
            }
        };

        result
    }

    fn priority(&self) -> i32 {
        10 // Filesystem completion has higher priority
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for FilesystemProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Format file size
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
