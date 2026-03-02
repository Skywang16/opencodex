use crate::storage::repositories::AIModels;
use crate::storage::DatabaseManager;
use std::sync::Arc;

pub const DEFAULT_CONTEXT_WINDOW: u32 = 128_000;

/// Get the context window size of a model from DB metadata.
/// Returns `DEFAULT_CONTEXT_WINDOW` (128k) when the model is not found or
/// has no explicit context_window set.
pub async fn get_model_context_window(
    database: &Arc<DatabaseManager>,
    model_id: &str,
) -> u32 {
    match AIModels::new(database).find_by_id(model_id).await {
        Ok(Some(model)) => model.context_window.unwrap_or(DEFAULT_CONTEXT_WINDOW),
        _ => DEFAULT_CONTEXT_WINDOW,
    }
}

/// Get the max output tokens for a model from DB metadata.
pub async fn get_model_max_output(database: &Arc<DatabaseManager>, model_id: &str) -> Option<u32> {
    let model = AIModels::new(database).find_by_id(model_id).await.ok()??;
    Some(model.max_output.unwrap_or(8_192))
}
