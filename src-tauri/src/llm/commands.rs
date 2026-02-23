use std::sync::Arc;
use tauri::{ipc::Channel, State};
use tokio_stream::StreamExt;

use super::{provider_registry::ProviderRegistry, service::LLMService};
use crate::llm::anthropic_types::{CreateMessageRequest, Message, StreamEvent};
use crate::storage::DatabaseManager;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

pub struct LLMManagerState {
    pub service: Arc<LLMService>,
}

impl LLMManagerState {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        let service = Arc::new(LLMService::new(database.clone()));
        Self { service }
    }
}

#[tauri::command]
pub async fn llm_call(
    state: State<'_, LLMManagerState>,
    request: CreateMessageRequest,
) -> TauriApiResult<Message> {
    match state.service.call(request).await {
        Ok(response) => Ok(api_success!(response)),
        Err(e) => {
            tracing::error!("LLM call failed: {}", e);
            Ok(api_error!("llm.call_failed"))
        }
    }
}

#[tauri::command]
pub async fn llm_call_stream(
    state: State<'_, LLMManagerState>,
    request: CreateMessageRequest,
    on_chunk: Channel<StreamEvent>,
) -> TauriApiResult<EmptyData> {
    use tokio_util::sync::CancellationToken;
    let token = CancellationToken::new();
    let mut stream = match state.service.call_stream(request, token).await {
        Ok(stream) => stream,
        Err(e) => {
            tracing::error!("LLM stream call failed: {}", e);
            return Ok(api_error!("llm.stream_failed"));
        }
    };

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Err(e) = on_chunk.send(chunk) {
                    tracing::error!("Failed to send chunk: {}", e);
                    break;
                }
            }
            Err(e) => {
                tracing::error!("Stream error: {}", e);
                let error_chunk = StreamEvent::Error {
                    error: crate::llm::anthropic_types::ErrorData {
                        error_type: "stream_error".to_string(),
                        message: e.to_string(),
                    },
                };
                if let Err(e) = on_chunk.send(error_chunk) {
                    tracing::error!("Failed to send error chunk: {}", e);
                }
                break;
            }
        }
    }

    Ok(api_success!())
}

/// Get list of available models
#[tauri::command]
pub async fn llm_get_available_models(
    state: State<'_, LLMManagerState>,
) -> TauriApiResult<Vec<String>> {
    match state.service.get_available_models().await {
        Ok(models) => Ok(api_success!(models)),
        Err(e) => {
            tracing::error!("Failed to get available models: {}", e);
            Ok(api_error!("llm.get_models_failed"))
        }
    }
}

/// Test model connection
#[tauri::command]
pub async fn llm_test_model_connection(
    state: State<'_, LLMManagerState>,
    model_id: String,
) -> TauriApiResult<bool> {
    match state.service.test_model_connection(&model_id).await {
        Ok(result) => Ok(api_success!(result)),
        Err(e) => {
            tracing::error!("Failed to test model connection: {}", e);
            Ok(api_error!("llm.test_connection_failed"))
        }
    }
}

/// Get all provider information (legacy - from hardcoded presets)
#[tauri::command]
pub async fn llm_get_providers(
    _state: State<'_, LLMManagerState>,
) -> TauriApiResult<Vec<super::provider_registry::ProviderMetadata>> {
    let providers = ProviderRegistry::global()
        .get_all_providers_metadata()
        .to_vec();
    Ok(api_success!(providers))
}

/// Get providers from models.dev API
#[tauri::command]
pub async fn llm_get_models_dev_providers(
    _state: State<'_, LLMManagerState>,
) -> TauriApiResult<Vec<super::models_dev::ProviderInfo>> {
    let providers = super::models_dev::get_provider_infos().await;
    Ok(api_success!(providers))
}

/// Refresh models from models.dev API
#[tauri::command]
pub async fn llm_refresh_models_dev(
    _state: State<'_, LLMManagerState>,
) -> TauriApiResult<EmptyData> {
    match super::models_dev::refresh().await {
        Ok(_) => Ok(api_success!()),
        Err(e) => {
            tracing::error!("Failed to refresh models.dev: {}", e);
            Ok(api_error!("llm.refresh_models_failed"))
        }
    }
}

/// Get model info by provider and model ID
#[tauri::command]
pub async fn llm_get_model_info(
    _state: State<'_, LLMManagerState>,
    provider_id: String,
    model_id: String,
) -> TauriApiResult<Option<super::models_dev::ModelInfo>> {
    let model = super::models_dev::get_model(&provider_id, &model_id).await;
    Ok(api_success!(
        model.map(|m| super::models_dev::ModelInfo::from(&m))
    ))
}
