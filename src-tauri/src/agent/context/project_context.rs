/*!
 * Project Context Loader
 *
 * Read project documentation by priority and inject into Agent context
 * Priority order: CLAUDE.md > AGENTS.md > WARP.md > .cursorrules > README.md
 */

use std::path::PathBuf;
use tokio::fs;

/// Priority list of project context configuration files
const CONTEXT_FILES: &[&str] = &[
    "CLAUDE.md",
    "AGENTS.md",
    "WARP.md",
    ".cursorrules",
    "README.md",
];

/// Get list of all available rules files
pub fn get_available_rules_files<P: Into<PathBuf>>(project_root: P) -> Vec<String> {
    let root: PathBuf = project_root.into();
    CONTEXT_FILES
        .iter()
        .filter_map(|&filename| {
            let file_path = root.join(filename);
            if file_path.exists() {
                Some(filename.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Project context loader
pub struct ProjectContextLoader {
    project_root: PathBuf,
}

impl ProjectContextLoader {
    /// Create a new loader instance
    pub fn new<P: Into<PathBuf>>(project_root: P) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// Load the first existing project document by priority
    pub async fn load_context(&self) -> Option<ProjectContext> {
        self.load_with_preference(None).await
    }

    /// Load project document with specified preference, or use default priority if not specified or file doesn't exist
    pub async fn load_with_preference(
        &self,
        preferred_file: Option<&str>,
    ) -> Option<ProjectContext> {
        // If a preferred file is specified, try to load it first
        if let Some(pref) = preferred_file {
            if let Some(ctx) = self.try_load_file(pref).await {
                return Some(ctx);
            }
        }

        // Try loading by default priority
        for filename in CONTEXT_FILES {
            if let Some(ctx) = self.try_load_file(filename).await {
                return Some(ctx);
            }
        }

        None
    }

    /// Try to load a single file
    async fn try_load_file(&self, filename: &str) -> Option<ProjectContext> {
        let file_path = self.project_root.join(filename);

        if !file_path.exists() {
            return None;
        }

        match fs::read_to_string(&file_path).await {
            Ok(content) => {
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    return None;
                }

                Some(ProjectContext {
                    source_file: filename.to_string(),
                    content: trimmed.to_string(),
                })
            }
            Err(_) => None,
        }
    }
}

/// Project context data
#[derive(Debug, Clone)]
pub struct ProjectContext {
    /// Source filename (e.g., "CLAUDE.md")
    pub source_file: String,
    /// File content
    pub content: String,
}

impl ProjectContext {
    /// Format as reference to inject into System Prompt (content not included)
    pub fn format_for_prompt(&self) -> String {
        format!(
            "Project instructions available in `{}`. Read it for project-specific guidelines when needed.",
            self.source_file
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_highest_priority() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create multiple files, should only read highest priority CLAUDE.md
        std::fs::write(temp_path.join("CLAUDE.md"), "Claude instructions").unwrap();
        std::fs::write(temp_path.join("README.md"), "Readme content").unwrap();

        let loader = ProjectContextLoader::new(temp_path);
        let context = loader.load_context().await;

        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.source_file, "CLAUDE.md");
        assert_eq!(ctx.content, "Claude instructions");
    }

    #[tokio::test]
    async fn test_fallback_priority() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Only create WARP.md and README.md, should read WARP.md
        std::fs::write(temp_path.join("WARP.md"), "Warp config").unwrap();
        std::fs::write(temp_path.join("README.md"), "Readme content").unwrap();

        let loader = ProjectContextLoader::new(temp_path);
        let context = loader.load_context().await;

        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.source_file, "WARP.md");
        assert_eq!(ctx.content, "Warp config");
    }

    #[tokio::test]
    async fn test_no_context_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let loader = ProjectContextLoader::new(temp_path);
        let context = loader.load_context().await;

        assert!(context.is_none());
    }

    #[tokio::test]
    async fn test_skip_empty_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // CLAUDE.md is empty, should skip and read README.md
        std::fs::write(temp_path.join("CLAUDE.md"), "   \n  \n  ").unwrap();
        std::fs::write(temp_path.join("README.md"), "Readme content").unwrap();

        let loader = ProjectContextLoader::new(temp_path);
        let context = loader.load_context().await;

        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.source_file, "README.md");
    }
}
