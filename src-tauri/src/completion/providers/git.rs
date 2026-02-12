//! Git command completion provider

use crate::completion::error::{CompletionProviderError, CompletionProviderResult};
use crate::completion::providers::CompletionProvider;
use crate::completion::types::{CompletionContext, CompletionItem, CompletionType};
use crate::storage::cache::UnifiedCache;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command as AsyncCommand;

/// Git completion provider
pub struct GitCompletionProvider {
    /// Uses unified cache
    cache: Arc<UnifiedCache>,
}

impl GitCompletionProvider {
    /// Create new Git completion provider
    pub fn new(cache: Arc<UnifiedCache>) -> Self {
        Self { cache }
    }

    /// Check if in a git repository
    async fn is_git_repository(&self, working_directory: &Path) -> CompletionProviderResult<bool> {
        let path_str = working_directory.to_string_lossy().to_string();
        let cache_key = format!("completion/git/repo:{path_str}");

        if let Some(cached_result) = self.cache.get(&cache_key).await {
            if let Some(is_repo) = cached_result.as_bool() {
                return Ok(is_repo);
            }
        }

        // Execute git command
        let output = AsyncCommand::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(working_directory)
            .output()
            .await
            .map_err(|e| {
                CompletionProviderError::io(
                    "git rev-parse",
                    format!("({})", working_directory.display()),
                    e,
                )
            })?;

        let is_repo = output.status.success();

        // Cache result
        let _ = self
            .cache
            .set_with_ttl(
                &cache_key,
                serde_json::Value::Bool(is_repo),
                Duration::from_secs(300),
            )
            .await;

        Ok(is_repo)
    }

    /// Parse git command
    fn parse_git_command(&self, context: &CompletionContext) -> Option<(String, Vec<String>)> {
        let parts: Vec<&str> = context.input.split_whitespace().collect();
        if parts.is_empty() || parts[0] != "git" {
            return None;
        }

        if parts.len() == 1 {
            // Only "git", complete subcommands
            return Some(("".to_string(), vec![]));
        }

        let subcommand = parts[1].to_string();
        let args = parts
            .get(2..)
            .unwrap_or(&[])
            .iter()
            .map(|s| s.to_string())
            .collect();
        Some((subcommand, args))
    }

    /// Get branch completions
    async fn get_branch_completions(
        &self,
        working_directory: &Path,
        query: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let output = AsyncCommand::new("git")
            .args(["branch", "--all", "--format=%(refname:short)"])
            .current_dir(working_directory)
            .output()
            .await
            .map_err(|e| {
                CompletionProviderError::io(
                    "git branch",
                    format!("({})", working_directory.display()),
                    e,
                )
            })?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let branches_output = String::from_utf8_lossy(&output.stdout);
        let mut completions = Vec::new();

        for line in branches_output.lines() {
            let branch = line.trim();
            if branch.is_empty() || branch.starts_with("origin/HEAD") {
                continue;
            }

            // Simple prefix matching
            if !query.is_empty() && !branch.to_lowercase().starts_with(&query.to_lowercase()) {
                continue;
            }

            let (completion_type, description, score) = if branch.starts_with("origin/") {
                (
                    CompletionType::Value,
                    format!("Remote branch: {branch}"),
                    8.0,
                )
            } else {
                (
                    CompletionType::Value,
                    format!("Local branch: {branch}"),
                    10.0,
                )
            };

            let mut item = CompletionItem::new(branch.to_string(), completion_type)
                .with_description(description)
                .with_score(score)
                .with_source("git".to_string());

            // Add metadata
            item = item.with_metadata("type".to_string(), "branch".to_string());
            if branch.starts_with("origin/") {
                item = item.with_metadata("remote".to_string(), "true".to_string());
            }

            completions.push(item);
        }

        Ok(completions)
    }

    /// Get git subcommand completions
    fn get_subcommand_completions(&self, query: &str) -> Vec<CompletionItem> {
        let subcommands = vec![
            ("add", "Add files to staging area"),
            ("commit", "Commit changes"),
            ("push", "Push to remote repository"),
            ("pull", "Pull from remote repository"),
            ("checkout", "Switch branch or restore files"),
            ("branch", "Branch management"),
            ("merge", "Merge branches"),
            ("status", "View status"),
            ("log", "View commit history"),
            ("diff", "View differences"),
            ("reset", "Reset changes"),
            ("tag", "Tag management"),
            ("remote", "Remote repository management"),
            ("clone", "Clone repository"),
            ("init", "Initialize repository"),
        ];

        let mut completions = Vec::new();
        for (cmd, desc) in subcommands {
            if query.is_empty() || cmd.starts_with(query) {
                let score = if cmd.starts_with(query) { 10.0 } else { 5.0 };

                let item = CompletionItem::new(cmd.to_string(), CompletionType::Subcommand)
                    .with_description(desc.to_string())
                    .with_score(score)
                    .with_source("git".to_string())
                    .with_metadata("type".to_string(), "subcommand".to_string());

                completions.push(item);
            }
        }

        completions
    }

    /// Get file status completions (for git add)
    async fn get_file_status_completions(
        &self,
        working_directory: &Path,
        query: &str,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        let output = AsyncCommand::new("git")
            .args(["status", "--porcelain"])
            .current_dir(working_directory)
            .output()
            .await
            .map_err(|e| {
                CompletionProviderError::io(
                    "git status",
                    format!("({})", working_directory.display()),
                    e,
                )
            })?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let status_output = String::from_utf8_lossy(&output.stdout);
        let mut completions = Vec::new();

        for line in status_output.lines() {
            if line.len() >= 3 {
                // Safe slicing
                let status = line.get(0..2).unwrap_or("");
                let filename = line.get(3..).unwrap_or("");

                // Simple prefix matching
                if !query.is_empty() && !filename.to_lowercase().starts_with(&query.to_lowercase())
                {
                    continue;
                }

                let (description, score) = match status {
                    "??" => ("Untracked file", 10.0),
                    " M" => ("Modified file", 15.0),
                    "M " => ("Staged file", 5.0),
                    " D" => ("Deleted file", 12.0),
                    "A " => ("New file", 8.0),
                    _ => ("Other status file", 6.0),
                };

                let item = CompletionItem::new(filename.to_string(), CompletionType::File)
                    .with_description(format!("{description}: {filename}"))
                    .with_score(score)
                    .with_source("git".to_string())
                    .with_metadata("type".to_string(), "file".to_string())
                    .with_metadata("status".to_string(), status.to_string());

                completions.push(item);
            }
        }

        Ok(completions)
    }
}

#[async_trait]
impl CompletionProvider for GitCompletionProvider {
    fn name(&self) -> &'static str {
        "git"
    }

    fn should_provide(&self, context: &CompletionContext) -> bool {
        context.input.trim_start().starts_with("git ")
    }

    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>> {
        if !self.is_git_repository(&context.working_directory).await? {
            return Ok(vec![]);
        }

        // Parse git command
        let (subcommand, _args) = match self.parse_git_command(context) {
            Some(parsed) => parsed,
            None => return Ok(vec![]),
        };

        if subcommand.is_empty() {
            return Ok(self.get_subcommand_completions(&context.current_word));
        }

        // Provide corresponding completions based on subcommand
        match subcommand.as_str() {
            "checkout" | "co" | "merge" | "branch" => {
                self.get_branch_completions(&context.working_directory, &context.current_word)
                    .await
            }
            "add" => {
                self.get_file_status_completions(&context.working_directory, &context.current_word)
                    .await
            }
            _ => Ok(vec![]),
        }
    }

    fn priority(&self) -> i32 {
        15 // High priority, as this is specialized git completion
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Default for GitCompletionProvider {
    fn default() -> Self {
        Self::new(Arc::new(UnifiedCache::new()))
    }
}
