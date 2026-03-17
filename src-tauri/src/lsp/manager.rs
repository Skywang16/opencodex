use crate::agent::tools::builtin::file_utils::ensure_absolute;
use crate::lsp::client::{LspClient, LspClientError};
use crate::lsp::language::candidate_servers_for_path;
use crate::lsp::server::resolve_server_for_file;
use crate::lsp::types::{
    LspDocumentSymbol, LspFileDiagnostics, LspHoverResult, LspLocation, LspServerId,
    LspServerStatus, LspWorkspaceSymbol,
};
use dashmap::DashMap;
use lsp_types::Position;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum LspManagerError {
    #[error("path is not under a supported LSP workspace: {0}")]
    UnsupportedFile(String),
    #[error("{0}")]
    MissingDependency(String),
    #[error(transparent)]
    Client(#[from] LspClientError),
    #[error("invalid path: {0}")]
    InvalidPath(String),
}

type Result<T> = std::result::Result<T, LspManagerError>;

#[derive(Default)]
pub struct LspManager {
    clients: DashMap<String, Arc<LspClient>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn status(&self) -> Vec<LspServerStatus> {
        self.clients
            .iter()
            .map(|entry| entry.value().status())
            .collect()
    }

    pub async fn document_symbols(
        &self,
        workspace: &str,
        path: &str,
    ) -> Result<Vec<LspDocumentSymbol>> {
        let path = resolve_path(workspace, path)?;
        let client = self.client_for_path(&path, Path::new(workspace)).await?;
        client.document_symbols(&path).await.map_err(Into::into)
    }

    pub async fn workspace_symbols(
        &self,
        workspace: &str,
        path_hint: Option<&str>,
        query: String,
    ) -> Result<Vec<LspWorkspaceSymbol>> {
        let workspace_root = PathBuf::from(workspace);
        if let Some(path_hint) = path_hint {
            let path = resolve_path(workspace, path_hint)?;
            let client = self.client_for_path(&path, &workspace_root).await?;
            return client.workspace_symbols(query).await.map_err(Into::into);
        }

        for client in self.clients.iter() {
            if client.value().status().root == workspace_root.to_string_lossy() {
                let result = client.value().workspace_symbols(query.clone()).await?;
                if !result.is_empty() {
                    return Ok(result);
                }
            }
        }

        Err(LspManagerError::UnsupportedFile(workspace.to_string()))
    }

    pub async fn hover(
        &self,
        workspace: &str,
        path: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<LspHoverResult>> {
        let path = resolve_path(workspace, path)?;
        let client = self.client_for_path(&path, Path::new(workspace)).await?;
        client
            .hover(&path, Position { line, character })
            .await
            .map_err(Into::into)
    }

    pub async fn definition(
        &self,
        workspace: &str,
        path: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<LspLocation>> {
        let path = resolve_path(workspace, path)?;
        let client = self.client_for_path(&path, Path::new(workspace)).await?;
        client
            .definition(&path, Position { line, character })
            .await
            .map_err(Into::into)
    }

    pub async fn references(
        &self,
        workspace: &str,
        path: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<LspLocation>> {
        let path = resolve_path(workspace, path)?;
        let client = self.client_for_path(&path, Path::new(workspace)).await?;
        client
            .references(&path, Position { line, character })
            .await
            .map_err(Into::into)
    }

    pub async fn diagnostics(
        &self,
        workspace: &str,
        path: Option<&str>,
    ) -> Result<Vec<LspFileDiagnostics>> {
        let workspace_root = PathBuf::from(workspace);
        if let Some(path) = path {
            let file = resolve_path(workspace, path)?;
            let client = self.client_for_path(&file, &workspace_root).await?;
            return Ok(vec![LspFileDiagnostics {
                path: file.to_string_lossy().to_string(),
                diagnostics: client.diagnostics_for_path(&file),
            }]);
        }

        let mut out = Vec::new();
        for entry in self.clients.iter() {
            if entry.value().status().root == workspace_root.to_string_lossy() {
                out.extend(entry.value().all_diagnostics());
            }
        }
        Ok(out)
    }

    async fn client_for_path(&self, path: &Path, workspace_root: &Path) -> Result<Arc<LspClient>> {
        let candidates = candidate_servers_for_path(path, workspace_root);
        if candidates.is_empty() {
            return Err(LspManagerError::UnsupportedFile(
                path.to_string_lossy().to_string(),
            ));
        }

        let config = resolve_server_for_file(path, workspace_root).ok_or_else(|| {
            LspManagerError::MissingDependency(missing_dependency_message(candidates[0]))
        })?;
        let key = format!(
            "{}::{}",
            config.server_id.as_str(),
            config.root.to_string_lossy()
        );
        if let Some(existing) = self.clients.get(&key) {
            return Ok(existing.clone());
        }
        let client = Arc::new(LspClient::spawn(config).await?);
        self.clients.insert(key, client.clone());
        Ok(client)
    }
}

fn resolve_path(workspace: &str, path: &str) -> Result<PathBuf> {
    ensure_absolute(path, workspace).map_err(|err| LspManagerError::InvalidPath(err.to_string()))
}

fn missing_dependency_message(server_id: LspServerId) -> String {
    match server_id {
        LspServerId::Typescript => {
            "LSP unavailable: missing `typescript-language-server`. Install it with `npm i -g typescript typescript-language-server`.".to_string()
        }
        LspServerId::RustAnalyzer => {
            "LSP unavailable: missing `rust-analyzer`. Install it with `rustup component add rust-analyzer`.".to_string()
        }
        LspServerId::Pyright => {
            "LSP unavailable: missing `pyright-langserver`. Install it with `npm i -g pyright`.".to_string()
        }
        LspServerId::Vue => {
            "LSP unavailable: missing `vue-language-server`. Install it with `npm i -g @vue/language-server typescript`.".to_string()
        }
        LspServerId::Gopls => {
            "LSP unavailable: missing `gopls`. Install it with `go install golang.org/x/tools/gopls@latest`.".to_string()
        }
        LspServerId::Deno => {
            "LSP unavailable: missing `deno`. Install Deno and make sure `deno` is available in PATH.".to_string()
        }
    }
}
