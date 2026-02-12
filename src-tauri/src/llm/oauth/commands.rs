use super::manager::OAuthManager;
use super::types::OAuthFlowInfo;
use crate::storage::repositories::ai_models::OAuthConfig;
use std::sync::Arc;
use tauri::State;

/// Start OAuth flow
#[tauri::command]
pub async fn start_oauth_flow(
    provider: String,
    manager: State<'_, Arc<OAuthManager>>,
) -> Result<OAuthFlowInfo, String> {
    manager
        .start_oauth_flow(&provider)
        .await
        .map_err(|e| e.to_string())
}

/// Wait for OAuth callback
#[tauri::command]
pub async fn wait_oauth_callback(
    flow_id: String,
    provider: String,
    manager: State<'_, Arc<OAuthManager>>,
) -> Result<OAuthConfig, String> {
    manager
        .wait_for_callback(&flow_id, &provider)
        .await
        .map_err(|e| e.to_string())
}

/// Cancel OAuth flow
#[tauri::command]
pub async fn cancel_oauth_flow(
    flow_id: String,
    manager: State<'_, Arc<OAuthManager>>,
) -> Result<(), String> {
    manager
        .cancel_flow(&flow_id)
        .await
        .map_err(|e| e.to_string())
}

/// Refresh OAuth token
#[tauri::command]
pub async fn refresh_oauth_token(
    mut oauth_config: OAuthConfig,
    manager: State<'_, Arc<OAuthManager>>,
) -> Result<OAuthConfig, String> {
    manager
        .refresh_token(&mut oauth_config)
        .await
        .map_err(|e| e.to_string())?;

    Ok(oauth_config)
}

/// Check OAuth status
#[tauri::command]
pub async fn check_oauth_status(
    oauth_config: OAuthConfig,
    manager: State<'_, Arc<OAuthManager>>,
) -> Result<String, String> {
    // Check if access_token exists
    if oauth_config.access_token.is_none() {
        return Ok("not_authorized".to_string());
    }

    // Check if token needs refresh
    if manager.should_refresh_token(&oauth_config) {
        Ok("token_expired".to_string())
    } else {
        Ok("authorized".to_string())
    }
}
