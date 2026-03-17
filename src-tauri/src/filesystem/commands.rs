use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use ignore::gitignore::GitignoreBuilder;
use ignore::WalkBuilder;
use std::path::PathBuf;

/// Extended directory entry, includes gitignore status
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntryExt {
    pub name: String,
    pub is_directory: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub is_ignored: bool,
}

/// Read directory contents (full version, includes gitignore status)
#[tauri::command]
pub async fn fs_read_dir(path: String) -> TauriApiResult<Vec<DirEntryExt>> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Ok(api_error!("common.not_found"));
    }
    if !root.is_dir() {
        return Ok(api_error!("common.invalid_path"));
    }

    // Manually create gitignore checker
    let mut gi_builder = GitignoreBuilder::new(&root);

    // Try to manually add current directory's .gitignore file
    let gitignore_path = root.join(".gitignore");
    if gitignore_path.exists() && gi_builder.add(&gitignore_path).is_some() {
        tracing::warn!(
            "failed to add gitignore file while reading directory: {}",
            gitignore_path.display()
        );
    }

    // Search upward and add parent directories' .gitignore
    let mut parent = root.parent();
    while let Some(p) = parent {
        let parent_gitignore = p.join(".gitignore");
        if parent_gitignore.exists() && gi_builder.add(&parent_gitignore).is_some() {
            tracing::warn!(
                "failed to add parent gitignore file while reading directory: {}",
                parent_gitignore.display()
            );
        }
        // Check if reached git repository root or filesystem root
        if p.join(".git").exists() || p.parent().is_none() {
            break;
        }
        parent = p.parent();
    }

    let gitignore = match gi_builder.build() {
        Ok(gitignore) => gitignore,
        Err(err) => {
            tracing::warn!(
                "failed to build gitignore matcher for {}: {}",
                root.display(),
                err
            );
            return Ok(api_error!("common.operation_failed"));
        }
    };

    let mut entries = Vec::new();

    // Read directory contents
    let read_dir = match std::fs::read_dir(&root) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to read directory: {}", e);
            return Ok(api_error!("common.operation_failed"));
        }
    };

    for entry_result in read_dir {
        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read directory entry: {}", e);
                continue;
            }
        };

        let file_path = entry.path();
        let file_name = entry.file_name();

        let name = file_name.to_string_lossy().to_string();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                tracing::warn!(
                    "failed to inspect directory entry type for {}: {}",
                    file_path.display(),
                    err
                );
                continue;
            }
        };
        let is_dir = file_type.is_dir();
        let is_file = file_type.is_file();
        let is_symlink = file_type.is_symlink();

        // Check if ignored by gitignore (relative to root directory)
        let relative_path = match file_path.strip_prefix(&root) {
            Ok(relative_path) => relative_path,
            Err(err) => {
                tracing::warn!(
                    "failed to strip root prefix {} from {}: {}",
                    root.display(),
                    file_path.display(),
                    err
                );
                continue;
            }
        };
        let is_ignored = gitignore.matched(relative_path, is_dir).is_ignore();

        entries.push(DirEntryExt {
            name,
            is_directory: is_dir,
            is_file,
            is_symlink,
            is_ignored,
        });
    }

    Ok(api_success!(entries))
}

/// Built-in ignored directories - automatically skip these large directories during recursive traversal
/// Note: If user directly specifies these directories as root path, they can still be accessed
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

pub(crate) async fn fs_list_directory(
    path: String,
    recursive: bool,
) -> TauriApiResult<Vec<String>> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Ok(api_error!("common.not_found"));
    }
    if !root.is_dir() {
        return Ok(api_error!("common.invalid_path"));
    }

    let mut builder = WalkBuilder::new(&root);
    builder
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_exclude(true)
        .parents(true)
        .standard_filters(true)
        .sort_by_file_name(|a, b| a.cmp(b));

    if !recursive {
        builder.max_depth(Some(1));
    }

    // Filter built-in ignored directories when recursive (only filter when depth > 0, so user can still access if directly specified)
    if recursive {
        builder.filter_entry(|entry| {
            // depth 0 is the root directory itself, don't filter
            if entry.depth() == 0 {
                return true;
            }
            // Only filter directories
            let is_dir = match entry.file_type() {
                Some(file_type) => file_type.is_dir(),
                None => return true,
            };
            if is_dir {
                let name = entry.file_name().to_string_lossy();
                if BUILTIN_SKIP_DIRS.contains(&name.as_ref()) {
                    return false;
                }
            }
            true
        });
    }

    let mut entries: Vec<(String, bool)> = Vec::new();

    for result in builder.build() {
        let entry = match result {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read directory entry: {}", e);
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        let p = entry.path();
        let rel = match p.strip_prefix(&root) {
            Ok(relative_path) => relative_path,
            Err(err) => {
                tracing::warn!(
                    "failed to strip root prefix {} from {}: {}",
                    root.display(),
                    p.display(),
                    err
                );
                continue;
            }
        };
        let is_dir = match entry.file_type() {
            Some(file_type) => file_type.is_dir(),
            None => match std::fs::metadata(p) {
                Ok(metadata) => metadata.is_dir(),
                Err(err) => {
                    tracing::warn!("failed to inspect walked path {}: {}", p.display(), err);
                    continue;
                }
            },
        };
        let mut name = rel.to_string_lossy().to_string();
        if is_dir && !name.ends_with('/') {
            name.push('/');
        }
        entries.push((name, is_dir));
    }

    // Sort: directories first, then alphabetical order
    entries.sort_unstable_by(|a, b| match (a.1, b.1) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.0.cmp(&b.0),
    });

    let out: Vec<String> = entries.into_iter().map(|(s, _)| s).collect();
    Ok(api_success!(out))
}
