use crate::lsp::types::LspServerId;
use std::path::{Path, PathBuf};

pub fn language_id_for_path(path: &Path) -> &'static str {
    match extension_str(path) {
        "ts" => "typescript",
        "tsx" => "typescriptreact",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "javascriptreact",
        "mts" | "cts" => "typescript",
        "vue" => "vue",
        "rs" => "rust",
        "py" => "python",
        "go" => "go",
        _ => "plaintext",
    }
}

pub fn candidate_servers_for_path(path: &Path, workspace_root: &Path) -> Vec<LspServerId> {
    match extension_str(path) {
        "vue" => vec![LspServerId::Vue, LspServerId::Typescript],
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "mts" | "cts" => {
            if has_deno_manifest(path, workspace_root) {
                vec![LspServerId::Deno, LspServerId::Typescript]
            } else {
                vec![LspServerId::Typescript]
            }
        }
        "rs" => vec![LspServerId::RustAnalyzer],
        "py" => vec![LspServerId::Pyright],
        "go" => vec![LspServerId::Gopls],
        _ => Vec::new(),
    }
}

pub fn nearest_root(start: &Path, workspace_root: &Path, markers: &[&str]) -> Option<PathBuf> {
    ancestors_in_workspace(start, workspace_root)
        .into_iter()
        .find(|dir| markers.iter().any(|marker| dir.join(marker).exists()))
}

pub fn has_deno_manifest(start: &Path, workspace_root: &Path) -> bool {
    nearest_root(start, workspace_root, &["deno.json", "deno.jsonc"]).is_some()
}

fn ancestors_in_workspace<'a>(start: &'a Path, workspace_root: &'a Path) -> Vec<PathBuf> {
    let start_dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        match start.parent() {
            Some(parent) => parent.to_path_buf(),
            None => workspace_root.to_path_buf(),
        }
    };
    let workspace_root = match workspace_root.canonicalize() {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(
                "failed to canonicalize workspace root {}: {}",
                workspace_root.display(),
                err
            );
            workspace_root.to_path_buf()
        }
    };
    let mut dirs = Vec::new();
    let mut current = match start_dir.canonicalize() {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(
                "failed to canonicalize start directory {}: {}",
                start_dir.display(),
                err
            );
            start_dir
        }
    };
    loop {
        if !current.starts_with(&workspace_root) {
            break;
        }
        dirs.push(current.clone());
        if current == workspace_root {
            break;
        }
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }
    dirs
}

fn extension_str(path: &Path) -> &str {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
}
