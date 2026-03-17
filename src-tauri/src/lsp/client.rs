use crate::lsp::language::language_id_for_path;
use crate::lsp::types::{
    LspDocumentSymbol, LspFileDiagnostics, LspHoverResult, LspLocation, LspServerStatus,
    ResolvedServerConfig,
};
use dashmap::DashMap;
use lsp_types::{
    request::{
        DocumentSymbolRequest, GotoDefinition, HoverRequest, References, Request,
        WorkspaceSymbolRequest,
    },
    ClientCapabilities, Diagnostic, DidChangeConfigurationParams, DidChangeTextDocumentParams,
    DidOpenTextDocumentParams, DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams,
    HoverParams, InitializeParams, PartialResultParams, Position, PublishDiagnosticsParams,
    ReferenceContext, ReferenceParams, TextDocumentContentChangeEvent, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams, Uri, VersionedTextDocumentIdentifier,
    WorkDoneProgressParams, WorkspaceFolder, WorkspaceSymbolParams,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{oneshot, Mutex};

#[derive(Debug, thiserror::Error)]
pub enum LspClientError {
    #[error("failed to spawn LSP server `{command}`: {source}")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("LSP server process missing {0} stream")]
    MissingStream(&'static str),
    #[error("LSP transport is not connected")]
    NotConnected,
    #[error("LSP request timed out")]
    Timeout,
    #[error("LSP request failed: {0}")]
    Protocol(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("path is not valid UTF-8 URI: {0}")]
    InvalidPath(String),
}

type Result<T> = std::result::Result<T, LspClientError>;

pub struct LspClient {
    config: ResolvedServerConfig,
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<tokio::process::ChildStdin>>,
    pending: Arc<DashMap<i64, oneshot::Sender<Result<Value>>>>,
    next_id: AtomicI64,
    connected: Arc<AtomicBool>,
    initialized: Arc<AtomicBool>,
    diagnostics: Arc<DashMap<String, Vec<Diagnostic>>>,
    open_documents: Arc<Mutex<HashMap<String, i32>>>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl LspClient {
    pub async fn spawn(config: ResolvedServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .current_dir(&config.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|source| LspClientError::Spawn {
            command: config.command.clone(),
            source,
        })?;
        let stdin = child
            .stdin
            .take()
            .ok_or(LspClientError::MissingStream("stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or(LspClientError::MissingStream("stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or(LspClientError::MissingStream("stderr"))?;

        let client = Self {
            config,
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(Some(stdin)),
            pending: Arc::new(DashMap::new()),
            next_id: AtomicI64::new(1),
            connected: Arc::new(AtomicBool::new(true)),
            initialized: Arc::new(AtomicBool::new(false)),
            diagnostics: Arc::new(DashMap::new()),
            open_documents: Arc::new(Mutex::new(HashMap::new())),
            last_error: Arc::new(Mutex::new(None)),
        };

        client.start_stdout_loop(stdout);
        client.start_stderr_loop(stderr);
        client.initialize().await?;
        Ok(client)
    }

    pub fn status(&self) -> LspServerStatus {
        let open_documents = match self.open_documents.try_lock() {
            Ok(docs) => docs.len(),
            Err(err) => {
                tracing::warn!("failed to inspect LSP open document count: {}", err);
                0
            }
        };
        let last_error = match self.last_error.try_lock() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                tracing::warn!("failed to inspect LSP last_error: {}", err);
                None
            }
        };
        LspServerStatus {
            server_id: self.config.server_id.as_str().to_string(),
            root: self.config.root.to_string_lossy().to_string(),
            command: format!("{} {}", self.config.command, self.config.args.join(" "))
                .trim()
                .to_string(),
            initialized: self.initialized.load(Ordering::SeqCst),
            connected: self.connected.load(Ordering::SeqCst),
            open_documents,
            diagnostics_files: self.diagnostics.len(),
            last_error,
        }
    }

    pub async fn document_symbols(&self, path: &std::path::Path) -> Result<Vec<LspDocumentSymbol>> {
        let uri = self.ensure_document(path).await?;
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let response: Option<DocumentSymbolResponse> =
            self.request(DocumentSymbolRequest::METHOD, params).await?;
        let Some(response) = response else {
            return Ok(Vec::new());
        };
        Ok(match response {
            DocumentSymbolResponse::Nested(items) => items.into_iter().map(Into::into).collect(),
            DocumentSymbolResponse::Flat(items) => items
                .into_iter()
                .map(|item| {
                    Ok(LspDocumentSymbol {
                        name: item.name,
                        kind: symbol_kind_to_u32(item.kind)?,
                        detail: None,
                        range: item.location.range.into(),
                        selection_range: item.location.range.into(),
                        children: Vec::new(),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        })
    }

    pub async fn workspace_symbols(
        &self,
        query: String,
    ) -> Result<Vec<crate::lsp::types::LspWorkspaceSymbol>> {
        let params = WorkspaceSymbolParams {
            query,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let response: Option<
            Vec<lsp_types::OneOf<lsp_types::SymbolInformation, lsp_types::WorkspaceSymbol>>,
        > = self.request(WorkspaceSymbolRequest::METHOD, params).await?;
        let symbols = response.unwrap_or_default();
        symbols
            .into_iter()
            .map(|item| match item {
                lsp_types::OneOf::Left(info) => Ok(info.into()),
                lsp_types::OneOf::Right(symbol) => Ok(crate::lsp::types::LspWorkspaceSymbol {
                    name: symbol.name,
                    kind: symbol_kind_to_u32(symbol.kind)?,
                    container_name: symbol.container_name,
                    location: match symbol.location {
                        lsp_types::OneOf::Left(location) => Some(location.into()),
                        lsp_types::OneOf::Right(_) => None,
                    },
                }),
            })
            .collect::<Result<Vec<_>>>()
    }

    pub async fn hover(
        &self,
        path: &std::path::Path,
        position: Position,
    ) -> Result<Option<LspHoverResult>> {
        let uri = self.ensure_document(path).await?;
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let response: Option<lsp_types::Hover> = self.request(HoverRequest::METHOD, params).await?;
        Ok(response.map(Into::into))
    }

    pub async fn definition(
        &self,
        path: &std::path::Path,
        position: Position,
    ) -> Result<Vec<LspLocation>> {
        let uri = self.ensure_document(path).await?;
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let response: Option<lsp_types::GotoDefinitionResponse> =
            self.request(GotoDefinition::METHOD, params).await?;
        Ok(match response {
            Some(lsp_types::GotoDefinitionResponse::Scalar(location)) => vec![location.into()],
            Some(lsp_types::GotoDefinitionResponse::Array(items)) => {
                items.into_iter().map(Into::into).collect()
            }
            Some(lsp_types::GotoDefinitionResponse::Link(items)) => items
                .into_iter()
                .map(|item| LspLocation {
                    path: uri_to_path_string(item.target_uri.as_str()),
                    range: item.target_selection_range.into(),
                })
                .collect(),
            None => Vec::new(),
        })
    }

    pub async fn references(
        &self,
        path: &std::path::Path,
        position: Position,
    ) -> Result<Vec<LspLocation>> {
        let uri = self.ensure_document(path).await?;
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };
        let response: Option<Vec<lsp_types::Location>> =
            self.request(References::METHOD, params).await?;
        let locations = response.unwrap_or_default();
        Ok(locations.into_iter().map(Into::into).collect())
    }

    pub fn diagnostics_for_path(&self, path: &std::path::Path) -> Vec<Diagnostic> {
        let key = normalize_path_string(path);
        self.diagnostics
            .get(&key)
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }

    pub fn all_diagnostics(&self) -> Vec<LspFileDiagnostics> {
        self.diagnostics
            .iter()
            .map(|entry| LspFileDiagnostics {
                path: entry.key().clone(),
                diagnostics: entry.value().clone(),
            })
            .collect()
    }

    pub async fn shutdown(&self) {
        if let Err(err) = self.notify("shutdown", json!(null)).await {
            tracing::warn!("failed to notify LSP shutdown: {err}");
        }
        if let Err(err) = self.notify("exit", json!(null)).await {
            tracing::warn!("failed to notify LSP exit: {err}");
        }
        self.connected.store(false, Ordering::SeqCst);
        match self.child.try_lock() {
            Ok(mut child) => {
                if let Some(mut child) = child.take() {
                    if let Err(err) = child.kill().await {
                        tracing::warn!("failed to kill LSP child process: {err}");
                    }
                }
            }
            Err(err) => {
                tracing::warn!("failed to acquire LSP child lock during shutdown: {}", err);
            }
        }
    }

    async fn initialize(&self) -> Result<()> {
        let root_uri = file_uri(&self.config.root)?;
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            initialization_options: self.config.initialization_options.clone(),
            capabilities: ClientCapabilities::default(),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: root_uri,
                name: self.config.root.to_string_lossy().to_string(),
            }]),
            ..Default::default()
        };

        let _: Value = self.request("initialize", params).await?;
        self.initialized.store(true, Ordering::SeqCst);
        self.notify("initialized", json!({})).await?;
        if let Some(settings) = &self.config.initialization_options {
            self.notify(
                "workspace/didChangeConfiguration",
                serde_json::to_value(DidChangeConfigurationParams {
                    settings: settings.clone(),
                })?,
            )
            .await?;
        }
        Ok(())
    }

    async fn ensure_document(&self, path: &std::path::Path) -> Result<Uri> {
        let path = normalize_path(path);
        let path_string = normalize_path_string(&path);
        let text = tokio::fs::read_to_string(&path).await?;
        let uri = file_uri(&path)?;
        let language_id = language_id_for_path(&path).to_string();

        let mut docs = self.open_documents.lock().await;
        if let Some(version) = docs.get_mut(&path_string) {
            *version += 1;
            self.notify(
                "textDocument/didChange",
                serde_json::to_value(DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier {
                        uri: uri.clone(),
                        version: *version,
                    },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text,
                    }],
                })?,
            )
            .await?;
        } else {
            docs.insert(path_string, 0);
            self.notify(
                "textDocument/didOpen",
                serde_json::to_value(DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: uri.clone(),
                        language_id,
                        version: 0,
                        text,
                    },
                })?,
            )
            .await?;
        }
        Ok(uri)
    }

    async fn request<T: serde::Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R> {
        if !self.connected.load(Ordering::SeqCst) {
            return Err(LspClientError::NotConnected);
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_message(&message).await?;

        let value = tokio::time::timeout(std::time::Duration::from_secs(45), rx)
            .await
            .map_err(|_| LspClientError::Timeout)?
            .map_err(|_| LspClientError::NotConnected)??;
        Ok(serde_json::from_value(value)?)
    }

    async fn notify(&self, method: &str, params: Value) -> Result<()> {
        if !self.connected.load(Ordering::SeqCst) {
            return Err(LspClientError::NotConnected);
        }
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
        .await
    }

    async fn write_message(&self, value: &Value) -> Result<()> {
        let bytes = serde_json::to_vec(value)?;
        let mut stdin = self.stdin.lock().await;
        let Some(stdin) = stdin.as_mut() else {
            return Err(LspClientError::NotConnected);
        };
        let header = format!("Content-Length: {}\r\n\r\n", bytes.len());
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(&bytes).await?;
        stdin.flush().await?;
        Ok(())
    }

    fn start_stdout_loop(&self, stdout: tokio::process::ChildStdout) {
        let pending = Arc::clone(&self.pending);
        let connected = Arc::clone(&self.connected);
        let diagnostics = Arc::clone(&self.diagnostics);
        let last_error = Arc::clone(&self.last_error);
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_message(&mut reader).await {
                    Ok(Some(value)) => {
                        if let Some(id) = value.get("id").and_then(|id| id.as_i64()) {
                            if let Some((_, tx)) = pending.remove(&id) {
                                if let Some(error) = value.get("error") {
                                    if tx
                                        .send(Err(LspClientError::Protocol(error.to_string())))
                                        .is_err()
                                    {
                                        tracing::warn!(
                                            target: "lsp",
                                            "Failed to deliver LSP error response for request {}",
                                            id
                                        );
                                    }
                                } else {
                                    let result = match value.get("result") {
                                        Some(result) => result.clone(),
                                        None => Value::Null,
                                    };
                                    if tx.send(Ok(result)).is_err() {
                                        tracing::warn!(
                                            target: "lsp",
                                            "Failed to deliver LSP response for request {}",
                                            id
                                        );
                                    }
                                }
                            }
                            continue;
                        }

                        if value.get("method").and_then(Value::as_str)
                            == Some("textDocument/publishDiagnostics")
                        {
                            if let Some(params) = value.get("params") {
                                match serde_json::from_value::<PublishDiagnosticsParams>(
                                    params.clone(),
                                ) {
                                    Ok(payload) => {
                                        let key = uri_to_path_string(payload.uri.as_str());
                                        diagnostics.insert(key, payload.diagnostics);
                                    }
                                    Err(err) => {
                                        *last_error.lock().await = Some(err.to_string());
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(err) => {
                        *last_error.lock().await = Some(err.to_string());
                        break;
                    }
                }
            }

            connected.store(false, Ordering::SeqCst);
            for entry in pending.iter() {
                let id = *entry.key();
                if let Some((_, tx)) = pending.remove(&id) {
                    if tx.send(Err(LspClientError::NotConnected)).is_err() {
                        tracing::warn!(
                            target: "lsp",
                            "Failed to notify pending LSP request {} about disconnect",
                            id
                        );
                    }
                }
            }
        });
    }

    fn start_stderr_loop(&self, stderr: tokio::process::ChildStderr) {
        let last_error = Arc::clone(&self.last_error);
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            loop {
                match read_message_line(&mut reader).await {
                    Ok(Some(line)) => {
                        tracing::warn!(target: "lsp", "lsp stderr: {}", line);
                        *last_error.lock().await = Some(line);
                    }
                    Ok(None) => break,
                    Err(err) => {
                        *last_error.lock().await = Some(err.to_string());
                        break;
                    }
                }
            }
        });
    }
}

async fn read_message<R: AsyncRead + Unpin>(reader: &mut BufReader<R>) -> Result<Option<Value>> {
    let mut content_length = None::<usize>;
    loop {
        let line = match read_message_line(reader).await? {
            Some(line) => line,
            None => return Ok(None),
        };

        if line.is_empty() {
            break;
        }

        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = Some(value.trim().parse::<usize>().map_err(|err| {
                    LspClientError::Protocol(format!("invalid Content-Length: {err}"))
                })?);
            }
        }
    }

    let content_length =
        content_length.ok_or_else(|| LspClientError::Protocol("missing Content-Length".into()))?;
    let mut buf = vec![0u8; content_length];
    reader.read_exact(&mut buf).await?;
    Ok(Some(serde_json::from_slice(&buf)?))
}

async fn read_message_line<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
) -> Result<Option<String>> {
    let mut buf = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        let read = reader.read(&mut byte).await?;
        if read == 0 {
            if buf.is_empty() {
                return Ok(None);
            }
            break;
        }
        buf.push(byte[0]);
        if buf.ends_with(b"\r\n") {
            buf.truncate(buf.len().saturating_sub(2));
            break;
        }
        if buf.ends_with(b"\n") {
            buf.truncate(buf.len().saturating_sub(1));
            break;
        }
    }
    Ok(Some(String::from_utf8_lossy(&buf).to_string()))
}

fn file_uri(path: &std::path::Path) -> Result<Uri> {
    let url = url::Url::from_file_path(path)
        .map_err(|_| LspClientError::InvalidPath(path.to_string_lossy().to_string()))?;
    url.as_str()
        .parse::<Uri>()
        .map_err(|_| LspClientError::InvalidPath(path.to_string_lossy().to_string()))
}

fn normalize_path_string(path: &std::path::Path) -> String {
    normalize_path(path).to_string_lossy().to_string()
}

fn symbol_kind_to_u32(kind: lsp_types::SymbolKind) -> Result<u32> {
    serde_json::to_value(kind)?
        .as_u64()
        .map(|value| value as u32)
        .ok_or_else(|| LspClientError::Protocol("symbol kind is not an integer".into()))
}

fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
    match path.canonicalize() {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(
                "failed to canonicalize LSP path {}: {}",
                path.display(),
                err
            );
            path.to_path_buf()
        }
    }
}

fn uri_to_path_string(uri: &str) -> String {
    match url::Url::parse(uri) {
        Ok(url) => match url.to_file_path() {
            Ok(path) => normalize_path_string(&path),
            Err(_) => uri.to_string(),
        },
        Err(_) => uri.to_string(),
    }
}
