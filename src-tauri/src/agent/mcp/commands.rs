use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::agent::mcp::registry::McpRegistry;
use crate::agent::mcp::types::{McpServerStatus, McpTestResult};
use crate::api_success;
use crate::settings::types::McpServerConfig;
use crate::settings::SettingsManager;
use crate::utils::TauriApiResult;

/// Get MCP server status list (requires workspace to be meaningful)
#[tauri::command]
pub async fn list_mcp_servers(
    workspace: Option<String>,
    registry: State<'_, Arc<McpRegistry>>,
) -> TauriApiResult<Vec<McpServerStatus>> {
    let Some(workspace) = workspace else {
        return Ok(api_success!(Vec::<McpServerStatus>::new()));
    };

    let workspace_root = PathBuf::from(workspace);
    let workspace_root = tokio::fs::canonicalize(&workspace_root)
        .await
        .unwrap_or(workspace_root);
    let workspace_key = workspace_root.to_string_lossy().to_string();
    Ok(api_success!(
        registry.get_servers_status(Some(workspace_key.as_str()))
    ))
}

/// Test MCP server connection (does not write to registry)
#[tauri::command]
pub async fn test_mcp_server(
    name: String,
    config: McpServerConfig,
    workspace: Option<String>,
) -> TauriApiResult<McpTestResult> {
    let workspace_root = workspace
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);

    let result =
        match crate::agent::mcp::client::McpClient::new(name, &config, &workspace_root).await {
            Ok(client) => McpTestResult {
                success: true,
                tools_count: client.tools().len(),
                error: None,
            },
            Err(e) => McpTestResult {
                success: false,
                tools_count: 0,
                error: Some(e.to_string()),
            },
        };

    Ok(api_success!(result))
}

/// Reload MCP servers (currently refreshes at workspace level)
#[tauri::command]
pub async fn reload_mcp_servers(
    workspace: Option<String>,
    registry: State<'_, Arc<McpRegistry>>,
    settings_mgr: State<'_, Arc<SettingsManager>>,
) -> TauriApiResult<Vec<McpServerStatus>> {
    let Some(workspace) = workspace else {
        return Ok(api_success!(Vec::<McpServerStatus>::new()));
    };

    let workspace_root = PathBuf::from(workspace);
    let workspace_root = tokio::fs::canonicalize(&workspace_root)
        .await
        .unwrap_or(workspace_root);
    let effective = match settings_mgr
        .get_effective_settings(Some(workspace_root.clone()))
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(target: "mcp", error = %e, "Failed to load effective settings for MCP reload");
            return Ok(api_success!(Vec::<McpServerStatus>::new()));
        }
    };

    let workspace_settings = settings_mgr
        .get_workspace_settings(&workspace_root)
        .await
        .ok()
        .flatten();

    let _ = registry
        .reload_workspace_servers(&workspace_root, &effective, workspace_settings.as_ref())
        .await;

    let workspace_key = workspace_root.to_string_lossy().to_string();
    Ok(api_success!(
        registry.get_servers_status(Some(workspace_key.as_str()))
    ))
}
