use crate::lsp::language::{candidate_servers_for_path, nearest_root};
use crate::lsp::types::{LspServerId, ResolvedServerConfig};
use serde_json::json;
use std::env;
use std::path::{Path, PathBuf};

pub fn resolve_server_for_file(path: &Path, workspace_root: &Path) -> Option<ResolvedServerConfig> {
    for server_id in candidate_servers_for_path(path, workspace_root) {
        if let Some(config) = resolve_server(server_id, path, workspace_root) {
            return Some(config);
        }
    }
    None
}

fn resolved_root_or_workspace(
    path: &Path,
    workspace_root: &Path,
    markers: &[&str],
    server_name: &str,
) -> PathBuf {
    match nearest_root(path, workspace_root, markers) {
        Some(root) => root,
        None => {
            tracing::debug!(
                "No specific root markers found for {}, using workspace root '{}'",
                server_name,
                workspace_root.display()
            );
            workspace_root.to_path_buf()
        }
    }
}

fn resolve_server_command(root: &Path, binary_name: &str) -> Option<String> {
    if let Some(command) = resolve_node_binary(root, binary_name) {
        return Some(command);
    }

    if let Some(command) = which(binary_name) {
        return Some(command);
    }

    tracing::warn!(
        "Failed to resolve language server binary '{}' for root '{}'",
        binary_name,
        root.display()
    );
    None
}

fn resolve_server(
    server_id: LspServerId,
    path: &Path,
    workspace_root: &Path,
) -> Option<ResolvedServerConfig> {
    match server_id {
        LspServerId::Deno => {
            let root = nearest_root(path, workspace_root, &["deno.json", "deno.jsonc"])?;
            Some(ResolvedServerConfig {
                server_id,
                root,
                command: which("deno")?,
                args: vec!["lsp".to_string()],
                initialization_options: None,
            })
        }
        LspServerId::Typescript => {
            let root = resolved_root_or_workspace(
                path,
                workspace_root,
                &[
                    "package-lock.json",
                    "bun.lockb",
                    "bun.lock",
                    "pnpm-lock.yaml",
                    "yarn.lock",
                    "package.json",
                ],
                "typescript-language-server",
            );

            let command = resolve_server_command(&root, "typescript-language-server")?;
            let tsserver = resolve_typescript_tsserver(&root);

            Some(ResolvedServerConfig {
                server_id,
                root,
                command,
                args: vec!["--stdio".to_string()],
                initialization_options: tsserver.map(|path| {
                    json!({
                        "tsserver": {
                            "path": path.to_string_lossy().to_string()
                        }
                    })
                }),
            })
        }
        LspServerId::Vue => {
            let root = resolved_root_or_workspace(
                path,
                workspace_root,
                &[
                    "package-lock.json",
                    "bun.lockb",
                    "bun.lock",
                    "pnpm-lock.yaml",
                    "yarn.lock",
                    "package.json",
                ],
                "vue-language-server",
            );

            let command = resolve_server_command(&root, "vue-language-server")?;

            Some(ResolvedServerConfig {
                server_id,
                root,
                command,
                args: vec!["--stdio".to_string()],
                initialization_options: None,
            })
        }
        LspServerId::RustAnalyzer => Some(ResolvedServerConfig {
            server_id,
            root: resolved_root_or_workspace(
                path,
                workspace_root,
                &["Cargo.toml"],
                "rust-analyzer",
            ),
            command: which("rust-analyzer")?,
            args: Vec::new(),
            initialization_options: None,
        }),
        LspServerId::Pyright => Some(ResolvedServerConfig {
            server_id,
            root: resolved_root_or_workspace(
                path,
                workspace_root,
                &["pyproject.toml", "requirements.txt", ".git"],
                "pyright-langserver",
            ),
            command: resolve_server_command(workspace_root, "pyright-langserver")?,
            args: vec!["--stdio".to_string()],
            initialization_options: None,
        }),
        LspServerId::Gopls => Some(ResolvedServerConfig {
            server_id,
            root: resolved_root_or_workspace(path, workspace_root, &["go.work", "go.mod"], "gopls"),
            command: which("gopls")?,
            args: Vec::new(),
            initialization_options: None,
        }),
    }
}

fn resolve_typescript_tsserver(root: &Path) -> Option<PathBuf> {
    let markers = ["node_modules/typescript/lib/tsserver.js"];
    for dir in candidate_dirs(root) {
        for marker in markers {
            let path = dir.join(marker);
            if path.exists() {
                return Some(path);
            }
        }
    }
    None
}

fn resolve_node_binary(root: &Path, name: &str) -> Option<String> {
    let suffix = if cfg!(windows) { ".cmd" } else { "" };
    for dir in candidate_dirs(root) {
        let bin = dir
            .join("node_modules")
            .join(".bin")
            .join(format!("{name}{suffix}"));
        if bin.exists() {
            return Some(bin.to_string_lossy().to_string());
        }
    }
    None
}

fn candidate_dirs(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut current = match root.canonicalize() {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(
                "failed to canonicalize LSP root {}: {}",
                root.display(),
                err
            );
            root.to_path_buf()
        }
    };
    loop {
        out.push(current.clone());
        let Some(parent) = current.parent() else {
            break;
        };
        if parent == current {
            break;
        }
        current = parent.to_path_buf();
    }
    out
}

fn which(command: &str) -> Option<String> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(command);
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{command}.exe"));
            if exe.exists() {
                return Some(exe.to_string_lossy().to_string());
            }
        }
    }
    None
}
