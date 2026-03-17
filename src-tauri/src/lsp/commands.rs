use crate::lsp::manager::LspManager;
use crate::lsp::types::{
    LspDocumentSymbol, LspFileDiagnostics, LspHoverResult, LspLocation, LspServerStatus,
    LspWorkspaceSymbol,
};
use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspPathParams {
    pub workspace: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspSymbolSearchParams {
    pub workspace: String,
    pub query: String,
    #[serde(default)]
    pub path_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspPositionParams {
    pub workspace: String,
    pub path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspDiagnosticsParams {
    pub workspace: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[tauri::command]
pub async fn lsp_status(state: State<'_, Arc<LspManager>>) -> TauriApiResult<Vec<LspServerStatus>> {
    Ok(api_success!(state.status().await))
}

#[tauri::command]
pub async fn lsp_document_symbols(
    state: State<'_, Arc<LspManager>>,
    params: LspPathParams,
) -> TauriApiResult<Vec<LspDocumentSymbol>> {
    match state
        .document_symbols(&params.workspace, &params.path)
        .await
    {
        Ok(result) => Ok(api_success!(result)),
        Err(err) => Ok(api_error!(&err.to_string())),
    }
}

#[tauri::command]
pub async fn lsp_workspace_symbols(
    state: State<'_, Arc<LspManager>>,
    params: LspSymbolSearchParams,
) -> TauriApiResult<Vec<LspWorkspaceSymbol>> {
    match state
        .workspace_symbols(&params.workspace, params.path_hint.as_deref(), params.query)
        .await
    {
        Ok(result) => Ok(api_success!(result)),
        Err(err) => Ok(api_error!(&err.to_string())),
    }
}

#[tauri::command]
pub async fn lsp_hover(
    state: State<'_, Arc<LspManager>>,
    params: LspPositionParams,
) -> TauriApiResult<Option<LspHoverResult>> {
    match state
        .hover(
            &params.workspace,
            &params.path,
            params.line,
            params.character,
        )
        .await
    {
        Ok(result) => Ok(api_success!(result)),
        Err(err) => Ok(api_error!(&err.to_string())),
    }
}

#[tauri::command]
pub async fn lsp_definition(
    state: State<'_, Arc<LspManager>>,
    params: LspPositionParams,
) -> TauriApiResult<Vec<LspLocation>> {
    match state
        .definition(
            &params.workspace,
            &params.path,
            params.line,
            params.character,
        )
        .await
    {
        Ok(result) => Ok(api_success!(result)),
        Err(err) => Ok(api_error!(&err.to_string())),
    }
}

#[tauri::command]
pub async fn lsp_references(
    state: State<'_, Arc<LspManager>>,
    params: LspPositionParams,
) -> TauriApiResult<Vec<LspLocation>> {
    match state
        .references(
            &params.workspace,
            &params.path,
            params.line,
            params.character,
        )
        .await
    {
        Ok(result) => Ok(api_success!(result)),
        Err(err) => Ok(api_error!(&err.to_string())),
    }
}

#[tauri::command]
pub async fn lsp_diagnostics(
    state: State<'_, Arc<LspManager>>,
    params: LspDiagnosticsParams,
) -> TauriApiResult<Vec<LspFileDiagnostics>> {
    match state
        .diagnostics(&params.workspace, params.path.as_deref())
        .await
    {
        Ok(result) => Ok(api_success!(result)),
        Err(err) => Ok(api_error!(&err.to_string())),
    }
}
