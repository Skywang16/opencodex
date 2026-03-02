//! AI model management commands — unified single-table design

use super::AIManagerState;
use crate::ai::error::AIServiceError;
use crate::storage::repositories::AIModelConfig;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use tauri::State;
use tracing::warn;

// ── Model commands ────────────────────────────────────────────────────────────

/// Get all models
#[tauri::command]
pub async fn ai_models_get(state: State<'_, AIManagerState>) -> TauriApiResult<Vec<AIModelConfig>> {
    match state.ai_service.get_models().await {
        Ok(models) => Ok(api_success!(models)),
        Err(e) => {
            warn!(error = %e, "Failed to load models");
            Ok(api_error!("ai.get_models_failed"))
        }
    }
}

/// Add a new model (complete config: auth + model + metadata)
#[tauri::command]
pub async fn ai_models_add(
    mut model: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<AIModelConfig> {
    // Auto-fill models.dev metadata if missing
    if model.context_window.is_none() {
        if let Some(md) = crate::llm::models_dev::get_model(&model.provider_id, &model.model).await
        {
            model.context_window = Some(md.context_window());
            model.max_output = Some(md.max_output());
            model.reasoning = md.reasoning;
            model.tool_call = md.tool_call;
            model.attachment = md.attachment;
            model.cost = md.cost.as_ref().and_then(|c| serde_json::to_value(c).ok());
            if model.display_name.is_empty() {
                model.display_name = md.name.clone();
            }
        }
    }
    match state.ai_service.add_model(model).await {
        Ok(m) => Ok(api_success!(m, "ai.add_model_success")),
        Err(e) => {
            warn!(error = %e, "Failed to add model");
            Ok(api_error!("ai.add_model_failed"))
        }
    }
}

/// Full update of a model (overwrites the entire row)
#[tauri::command]
pub async fn ai_models_update(
    mut model: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<AIModelConfig> {
    // Re-sync models.dev metadata on provider/model change
    if let Some(md) = crate::llm::models_dev::get_model(&model.provider_id, &model.model).await {
        model.context_window = Some(md.context_window());
        model.max_output = Some(md.max_output());
        model.reasoning = md.reasoning;
        model.tool_call = md.tool_call;
        model.attachment = md.attachment;
        model.cost = md.cost.as_ref().and_then(|c| serde_json::to_value(c).ok());
    }
    match state.ai_service.update_model(model).await {
        Ok(m) => Ok(api_success!(m, "ai.update_model_success")),
        Err(AIServiceError::ModelNotFound { .. }) => Ok(api_error!("ai.model_not_found")),
        Err(e) => {
            warn!(error = %e, "Failed to update model");
            Ok(api_error!("ai.update_model_failed"))
        }
    }
}

/// Remove a model
#[tauri::command]
pub async fn ai_models_remove(
    model_id: String,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    match state.ai_service.remove_model(&model_id).await {
        Ok(_) => Ok(api_success!(EmptyData, "ai.remove_model_success")),
        Err(e) => {
            warn!(error = %e, "Failed to remove model");
            Ok(api_error!("ai.remove_model_failed"))
        }
    }
}

/// Test model connection
#[tauri::command]
pub async fn ai_models_test(
    model: AIModelConfig,
    state: State<'_, AIManagerState>,
) -> TauriApiResult<EmptyData> {
    match state.ai_service.test_model(&model).await {
        Ok(_) => Ok(api_success!(EmptyData, "ai.test_connection_success")),
        Err(e) => Ok(api_error!("ai.test_connection_error", "error" => e.to_string())),
    }
}
