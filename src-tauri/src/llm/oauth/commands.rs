use super::manager::OAuthManager;
use super::types::{OAuthFlowInfo, OAuthTokenResult};
use crate::storage::repositories::AIModelConfig;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use std::sync::Arc;
use tauri::State;

/// Start OAuth flow
#[tauri::command]
pub async fn start_oauth_flow(
    provider: String,
    manager: State<'_, Arc<OAuthManager>>,
) -> TauriApiResult<OAuthFlowInfo> {
    match manager.start_oauth_flow(&provider).await {
        Ok(flow_info) => Ok(api_success!(flow_info)),
        Err(e) => {
            tracing::error!("OAuth start flow failed: {}", e);
            Ok(api_error!("common.operation_failed"))
        }
    }
}

/// Wait for OAuth callback — returns token bundle for the frontend
#[tauri::command]
pub async fn wait_oauth_callback(
    flow_id: String,
    provider: String,
    manager: State<'_, Arc<OAuthManager>>,
) -> TauriApiResult<OAuthTokenResult> {
    match manager.wait_for_callback(&flow_id, &provider).await {
        Ok(result) => Ok(api_success!(result)),
        Err(e) => {
            tracing::error!("OAuth callback failed: {}", e);
            Ok(api_error!("common.operation_failed"))
        }
    }
}

/// Cancel OAuth flow
#[tauri::command]
pub async fn cancel_oauth_flow(
    flow_id: String,
    manager: State<'_, Arc<OAuthManager>>,
) -> TauriApiResult<EmptyData> {
    match manager.cancel_flow(&flow_id).await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("OAuth cancel flow failed: {}", e);
            Ok(api_error!("common.operation_failed"))
        }
    }
}

/// Refresh OAuth token on a stored model config
#[tauri::command]
pub async fn refresh_oauth_token(
    mut model: AIModelConfig,
    manager: State<'_, Arc<OAuthManager>>,
) -> TauriApiResult<AIModelConfig> {
    match manager.refresh_token(&mut model).await {
        Ok(_) => Ok(api_success!(model)),
        Err(e) => {
            tracing::error!("OAuth token refresh failed: {}", e);
            Ok(api_error!("common.operation_failed"))
        }
    }
}

/// Check OAuth status for a model config
#[tauri::command]
pub async fn check_oauth_status(
    model: AIModelConfig,
    manager: State<'_, Arc<OAuthManager>>,
) -> TauriApiResult<String> {
    if model.oauth_access_token.is_none() {
        return Ok(api_success!("not_authorized".to_string()));
    }
    if manager.should_refresh_token(&model) {
        Ok(api_success!("token_expired".to_string()))
    } else {
        Ok(api_success!("authorized".to_string()))
    }
}
