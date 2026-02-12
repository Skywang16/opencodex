use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::sinks::UTF8;
use grep_searcher::Searcher;
use ignore::WalkBuilder;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;

use super::file_utils::{ensure_absolute, normalize_path};
use crate::agent::context::FileOperationRecord;
use crate::agent::context::FileRecordSource;
use crate::agent::core::context::TaskContext;
use crate::agent::error::ToolExecutorResult;
use crate::agent::tools::{
    RunnableTool, ToolCategory, ToolMetadata, ToolPriority, ToolResult, ToolResultContent,
    ToolResultStatus,
};

const DEFAULT_MAX_RESULTS: usize = 20;
const MAX_RESULTS_LIMIT: usize = 200;
const SNIPPET_MAX_LEN: usize = 300;

/// Built-in ignored directories - automatically skip these large directories during search
/// Note: If user directly specifies these directories as search paths, they can still be searched
const BUILTIN_SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "dist",
    "build",
    "target",
    ".next",
    ".nuxt",
    ".output",
    ".cache",
    ".turbo",
    "__pycache__",
    ".pytest_cache",
    "venv",
    ".venv",
    "vendor",
    "coverage",
    ".nyc_output",
    "bower_components",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrepArgs {
    pattern: String,
    #[serde(alias = "max_results")]
    max_results: Option<usize>,
    path: Option<String>,
    /// Glob pattern to filter files (e.g. "*.js", "*.{ts,tsx}")
    include: Option<String>,
    /// Number of context lines before and after each match
    #[serde(alias = "context_lines")]
    context_lines: Option<usize>,
    /// Case insensitive search
    #[serde(alias = "case_insensitive", default)]
    ignore_case: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrepResultEntry {
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub snippet: String,
    pub language: String,
}

pub struct GrepTool;

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GrepTool {
    pub fn new() -> Self {
        Self
    }

    async fn grep_search(
        &self,
        path: &Path,
        pattern: &str,
        max_results: usize,
        include: Option<&str>,
        context_lines: usize,
        ignore_case: bool,
    ) -> Result<Vec<GrepResultEntry>, String> {
        let path = path.to_path_buf();
        let pattern = pattern.to_string();
        let include = include.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            Self::grep_search_sync(
                &path,
                &pattern,
                max_results,
                include.as_deref(),
                context_lines,
                ignore_case,
            )
        })
        .await
        .map_err(|e| format!("Search task failed: {e}"))?
    }

    fn grep_search_sync(
        path: &Path,
        pattern: &str,
        max_results: usize,
        include: Option<&str>,
        context_lines: usize,
        ignore_case: bool,
    ) -> Result<Vec<GrepResultEntry>, String> {
        use std::cell::RefCell;
        use std::collections::BTreeMap;

        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(ignore_case)
            .build(pattern)
            .map_err(|e| format!("Invalid regex pattern: {e}"))?;

        // When context_lines > 0, we collect per-file matches first, then build snippets with context.
        // When context_lines == 0, we collect individual line matches directly.
        let results = RefCell::new(Vec::with_capacity(max_results));

        // Build glob override for include filter
        let mut builder = WalkBuilder::new(path);
        builder
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true);

        // Apply include glob filter using types
        if let Some(glob_pattern) = include {
            let mut types_builder = ignore::types::TypesBuilder::new();
            types_builder.add("custom", glob_pattern).ok();
            types_builder.select("custom");
            if let Ok(types) = types_builder.build() {
                builder.types(types);
            }
        }

        builder.filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            if is_dir {
                let name = entry.file_name().to_string_lossy();
                if BUILTIN_SKIP_DIRS.contains(&name.as_ref()) {
                    return false;
                }
            }
            true
        });

        let walker = builder.build();

        'outer: for entry in walker.flatten() {
            if results.borrow().len() >= max_results {
                break;
            }

            let entry_path = entry.path();
            if entry_path.is_dir() {
                continue;
            }

            // Skip large files (> 1MB)
            if let Ok(meta) = entry_path.metadata() {
                if meta.len() > 1024 * 1024 {
                    continue;
                }
            }

            let file_path_str = entry_path.display().to_string();
            let language = language_from_path(entry_path);

            if context_lines > 0 {
                // Collect all matches in this file first, then build context snippets
                let file_matches: RefCell<BTreeMap<usize, String>> = RefCell::new(BTreeMap::new());

                let mut searcher = Searcher::new();
                let _ = searcher.search_path(
                    &matcher,
                    entry_path,
                    UTF8(|line_num, line| {
                        file_matches
                            .borrow_mut()
                            .insert(line_num as usize, line.to_string());
                        Ok(true)
                    }),
                );

                let matches = file_matches.into_inner();
                if matches.is_empty() {
                    continue;
                }

                // Read full file for context
                let file_lines: Vec<String> = match std::fs::read_to_string(entry_path) {
                    Ok(content) => content.lines().map(|l| l.to_string()).collect(),
                    Err(_) => continue,
                };

                let mut res = results.borrow_mut();
                for &match_line in matches.keys() {
                    if res.len() >= max_results {
                        break;
                    }
                    let start = match_line.saturating_sub(context_lines).max(1);
                    let end = (match_line + context_lines).min(file_lines.len());

                    let mut snippet_parts = Vec::new();
                    for ln in start..=end {
                        if ln <= file_lines.len() {
                            let prefix = if ln == match_line { ":" } else { "-" };
                            let line_content = &file_lines[ln - 1];
                            snippet_parts.push(format!(
                                "{}{}{}",
                                ln,
                                prefix,
                                truncate_snippet(line_content.trim_end())
                            ));
                        }
                    }

                    res.push(GrepResultEntry {
                        file_path: file_path_str.clone(),
                        start_line: start,
                        end_line: end,
                        snippet: snippet_parts.join("\n"),
                        language: language.clone(),
                    });
                }
            } else {
                // Simple mode: one entry per match line
                let mut searcher = Searcher::new();
                let _ = searcher.search_path(
                    &matcher,
                    entry_path,
                    UTF8(|line_num, line| {
                        let mut res = results.borrow_mut();
                        if res.len() >= max_results {
                            return Ok(false);
                        }

                        res.push(GrepResultEntry {
                            file_path: file_path_str.clone(),
                            start_line: line_num as usize,
                            end_line: line_num as usize,
                            snippet: truncate_snippet(line.trim()),
                            language: language.clone(),
                        });

                        Ok(res.len() < max_results)
                    }),
                );
            }

            if results.borrow().len() >= max_results {
                break 'outer;
            }
        }

        Ok(results.into_inner())
    }
}

#[async_trait]
impl RunnableTool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        r#"Fast regex content search powered by ripgrep. Use this instead of shell grep/rg.

- Searches file contents using regular expressions
- Supports full regex syntax (e.g., "log.*Error", "function\s+\w+")
- Automatically respects .gitignore and skips binary files
- Returns file paths, line numbers, and matching content snippets
- Filter files by glob pattern with the include parameter (e.g., "*.js", "*.{ts,tsx}")
- Use ignore_case for case-insensitive search
- Use context_lines to include surrounding lines for each match
- Use maxResults to limit output for broad patterns (default: 20, max: 200)

When to use grep vs other tools:
- Use grep for exact matches: symbol names, strings, error messages, imports
- Use semantic_search for conceptual questions: "how does X work", "where is Y handled"
- Use list_files for finding files by name/location
- Use glob for finding files by name pattern
- When doing an open-ended search that may require multiple rounds of grepping, use the Task tool with the explore agent instead

Examples:
- Find function definitions: {"pattern": "fn main", "path": "/project/src"}
- Find TODOs: {"pattern": "TODO|FIXME"}
- Find imports: {"pattern": "^import.*react", "ignore_case": true}
- Find with file filter: {"pattern": "Session", "include": "*.ts"}
- Find with context: {"pattern": "handleError", "context_lines": 3}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "maxResults": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 200,
                    "description": "Max results (default: 20, max: 200)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search (default: workspace root)"
                },
                "include": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. \"*.js\", \"*.{ts,tsx}\")"
                },
                "contextLines": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 10,
                    "description": "Number of context lines before and after each match (default: 0)"
                },
                "ignoreCase": {
                    "type": "boolean",
                    "description": "Case insensitive search (default: false)"
                }
            },
            "required": ["pattern"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::CodeAnalysis, ToolPriority::Standard).with_tags(vec![
            "search".into(),
            "grep".into(),
            "regex".into(),
        ])
    }

    async fn run(
        &self,
        context: &TaskContext,
        args: serde_json::Value,
    ) -> ToolExecutorResult<ToolResult> {
        let args: GrepArgs = serde_json::from_value(args)?;

        let pattern = args.pattern.trim();
        if pattern.is_empty() {
            return Ok(validation_error("Pattern cannot be empty"));
        }

        let max_results = args
            .max_results
            .unwrap_or(DEFAULT_MAX_RESULTS)
            .min(MAX_RESULTS_LIMIT);

        let context_lines = args.context_lines.unwrap_or(0).min(10);
        let ignore_case = args.ignore_case.unwrap_or(false);

        let search_path = match resolve_to_absolute(args.path.as_deref(), &context.cwd) {
            Ok(path) => path,
            Err(result) => return Ok(result),
        };

        if !search_path.exists() {
            return Ok(tool_error(format!(
                "Path does not exist: {}",
                search_path.display()
            )));
        }

        let started = Instant::now();
        let result = self
            .grep_search(
                &search_path,
                pattern,
                max_results,
                args.include.as_deref(),
                context_lines,
                ignore_case,
            )
            .await;
        let elapsed_ms = started.elapsed().as_millis() as u64;

        match result {
            Ok(entries) => {
                for entry in &entries {
                    if let Ok(p) = PathBuf::from(&entry.file_path).canonicalize() {
                        let _ = context
                            .file_tracker()
                            .track_file_operation(FileOperationRecord::new(
                                p.as_path(),
                                FileRecordSource::FileMentioned,
                            ))
                            .await;
                    }
                }

                if entries.is_empty() {
                    return Ok(ToolResult {
                        content: vec![ToolResultContent::Success(format!(
                            "No matches for pattern \"{pattern}\""
                        ))],
                        status: ToolResultStatus::Success,
                        cancel_reason: None,
                        execution_time_ms: Some(elapsed_ms),
                        ext_info: Some(json!({ "totalFound": 0, "pattern": pattern })),
                    });
                }

                let summary = format!("Found {} matches ({}ms)", entries.len(), elapsed_ms);
                let details = format_result_details(&entries);

                Ok(ToolResult {
                    content: vec![ToolResultContent::Success(format!(
                        "{summary}\n\n{details}"
                    ))],
                    status: ToolResultStatus::Success,
                    cancel_reason: None,
                    execution_time_ms: Some(elapsed_ms),
                    ext_info: Some(json!({
                        "results": entries,
                        "totalFound": entries.len(),
                        "pattern": pattern,
                    })),
                })
            }
            Err(err_msg) => Ok(tool_error(err_msg)),
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn language_from_path(path: &Path) -> String {
    use crate::vector_db::core::Language;
    match Language::from_path(path) {
        Some(Language::Rust) => "rust".into(),
        Some(Language::TypeScript) => "typescript".into(),
        Some(Language::JavaScript) => "javascript".into(),
        Some(Language::Python) => "python".into(),
        Some(Language::Go) => "go".into(),
        Some(Language::Java) => "java".into(),
        Some(Language::C) => "c".into(),
        Some(Language::Cpp) => "cpp".into(),
        Some(Language::CSharp) => "csharp".into(),
        Some(Language::Ruby) => "ruby".into(),
        Some(Language::Php) => "php".into(),
        Some(Language::Swift) => "swift".into(),
        Some(Language::Kotlin) => "kotlin".into(),
        None => "text".into(),
    }
}

fn truncate_snippet(snippet: &str) -> String {
    crate::agent::utils::truncate_with_ellipsis(snippet, SNIPPET_MAX_LEN)
}

fn resolve_to_absolute(path: Option<&str>, cwd: &str) -> Result<PathBuf, ToolResult> {
    if let Some(raw) = path {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(validation_error("Path cannot be empty"));
        }
        ensure_absolute(trimmed, cwd).map_err(|err| validation_error(err.to_string()))
    } else {
        let base = cwd.trim();
        if base.is_empty() {
            return Err(tool_error("Working directory is not available"));
        }
        let path = Path::new(base);
        if !path.is_absolute() {
            return Err(tool_error("Working directory must be an absolute path"));
        }
        Ok(normalize_path(path))
    }
}

fn validation_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}

fn tool_error(message: impl Into<String>) -> ToolResult {
    ToolResult {
        content: vec![ToolResultContent::Error(message.into())],
        status: ToolResultStatus::Error,
        cancel_reason: None,
        execution_time_ms: None,
        ext_info: None,
    }
}

fn format_result_details(results: &[GrepResultEntry]) -> String {
    results
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let snippet = entry.snippet.replace('\n', "\n   ");
            format!(
                "{}. {}:{}\n   {}",
                idx + 1,
                entry.file_path,
                entry.start_line,
                snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
