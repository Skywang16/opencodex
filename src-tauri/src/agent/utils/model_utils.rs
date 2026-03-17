use crate::storage::repositories::AIModels;
use crate::storage::DatabaseManager;
use std::sync::Arc;
use tracing::warn;

/// Get the context window size of a model
pub async fn get_model_context_window(
    database: &Arc<DatabaseManager>,
    model_id: &str,
) -> Option<u32> {
    let model = match AIModels::new(database).find_by_id(model_id).await {
        Ok(model) => model?,
        Err(err) => {
            warn!(
                model_id = model_id,
                error = %err,
                "Failed to load model when resolving context window"
            );
            return None;
        }
    };

    model
        .options
        .and_then(|opts| opts.get("maxContextTokens")?.as_u64())
        .map(|v| v as u32)
}
