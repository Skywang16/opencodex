use crate::llm::types::LLMProviderConfig;
use serde::{Deserialize, Serialize};

/// Remote embedding model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEmbeddingConfig {
    /// LLM Provider configuration (API Key, URL, etc.)
    pub provider_config: LLMProviderConfig,

    /// Model name (e.g., "text-embedding-3-small")
    pub model_name: String,

    /// Vector dimension
    pub dimension: usize,

    /// Chunk size (number of tokens)
    pub chunk_size: usize,

    /// Chunk overlap (number of tokens)
    pub chunk_overlap: usize,
}

impl Default for RemoteEmbeddingConfig {
    fn default() -> Self {
        Self {
            provider_config: LLMProviderConfig {
                provider_type: String::new(),
                api_key: String::new(),
                api_url: None,
                options: None,
                oauth_config: None,
            },
            model_name: String::new(),
            dimension: 0,
            chunk_size: 512,
            chunk_overlap: 100,
        }
    }
}

/// Vector database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDbConfig {
    /// Remote embedding model configuration
    pub embedding: RemoteEmbeddingConfig,

    /// Maximum number of results to return when searching
    pub max_results: usize,

    /// Similarity threshold
    pub similarity_threshold: f32,

    /// File size limit (bytes)
    pub max_file_size: u64,

    /// Semantic search weight (0.0-1.0)
    pub semantic_weight: f32,

    /// Keyword search weight (0.0-1.0)
    pub keyword_weight: f32,
}

impl Default for VectorDbConfig {
    fn default() -> Self {
        Self {
            embedding: RemoteEmbeddingConfig::default(),
            max_results: 20,
            similarity_threshold: 0.3,
            max_file_size: 10 * 1024 * 1024,
            semantic_weight: 0.7,
            keyword_weight: 0.3,
        }
    }
}

impl VectorDbConfig {
    pub fn validate(&self) -> crate::vector_db::core::Result<()> {
        if self.embedding.model_name.is_empty() {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "Embedding model name is required".to_string(),
            ));
        }
        if self.embedding.provider_config.api_key.is_empty() {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "API key is required".to_string(),
            ));
        }
        if self.embedding.dimension == 0 {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "Dimension must be > 0".to_string(),
            ));
        }
        if self.embedding.chunk_size == 0 {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "Chunk size must be > 0".to_string(),
            ));
        }
        if self.embedding.chunk_overlap >= self.embedding.chunk_size {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "Chunk overlap must be < chunk size".to_string(),
            ));
        }
        if self.similarity_threshold < 0.0 || self.similarity_threshold > 1.0 {
            return Err(crate::vector_db::core::VectorDbError::Config(
                "Similarity threshold must be in [0, 1]".to_string(),
            ));
        }
        Ok(())
    }
}
