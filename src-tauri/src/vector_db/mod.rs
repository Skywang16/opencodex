pub mod chunking;
pub mod commands;
pub mod core;
pub mod embedding;
pub mod search;
pub mod storage;
pub mod utils;

use std::sync::Arc;

use crate::llm::types::LLMProviderConfig;
use crate::storage::repositories::{AIModels, ModelType};

pub use chunking::*;
pub use commands::*;
pub use core::*;
pub use embedding::*;
pub use search::*;
pub use storage::*;

pub async fn build_search_engine_from_database(
    database: Arc<crate::storage::DatabaseManager>,
) -> crate::vector_db::core::Result<Arc<SemanticSearchEngine>> {
    let models = AIModels::new(&database)
        .find_all()
        .await
        .map_err(|e| crate::vector_db::core::VectorDbError::Config(e.to_string()))?;

    let model = models
        .into_iter()
        .find(|m| m.model_type == ModelType::Embedding)
        .ok_or_else(|| {
            crate::vector_db::core::VectorDbError::Config(
                "Embedding model configuration not found".to_string(),
            )
        })?;

    let dimension = model
        .options
        .as_ref()
        .and_then(|opts| opts.get("dimension"))
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(1024);

    let config = VectorDbConfig {
        embedding: RemoteEmbeddingConfig {
            provider_config: LLMProviderConfig {
                provider_type: model.provider.as_str().to_string(),
                api_key: model.api_key.unwrap_or_default(),
                api_url: model.api_url,
                options: model
                    .options
                    .as_ref()
                    .and_then(|v| v.as_object())
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                oauth_config: None,
            },
            model_name: model.model,
            dimension,
            chunk_size: 512,
            chunk_overlap: 100,
        },
        ..VectorDbConfig::default()
    };

    config.validate()?;

    let embedder = crate::vector_db::embedding::create_embedder(&config.embedding)?;
    Ok(Arc::new(SemanticSearchEngine::new(embedder, config)))
}
