use crate::vector_db::core::{ChunkId, ChunkType, Result, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Index manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexManifest {
    /// Version number
    pub version: String,

    /// Creation time (Unix timestamp)
    pub created_at: u64,

    /// Last update time (Unix timestamp)
    pub updated_at: u64,

    /// Embedding model
    pub embedding_model: String,

    /// Vector dimension
    pub vector_dimension: usize,

    /// File index mapping (file path -> file hash)
    pub files: HashMap<PathBuf, String>,

    /// Chunk index mapping (chunk ID -> chunk metadata)
    pub chunks: HashMap<ChunkId, ChunkMetadata>,
}

/// Chunk metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub file_path: PathBuf,
    pub span: Span,
    pub chunk_type: ChunkType,
    pub hash: String,
}

impl IndexManifest {
    /// Create a new index manifest
    pub fn new(embedding_model: String, vector_dimension: usize) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version: "1.0.0".to_string(),
            created_at: now,
            updated_at: now,
            embedding_model,
            vector_dimension,
            files: HashMap::new(),
            chunks: HashMap::new(),
        }
    }

    /// Load manifest from file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Save manifest to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Add file
    pub fn add_file(&mut self, file_path: PathBuf, file_hash: String) {
        self.files.insert(file_path, file_hash);
        self.update_timestamp();
    }

    /// Remove file
    pub fn remove_file(&mut self, file_path: &Path) {
        self.files.remove(file_path);
        // Remove all chunks for this file
        self.chunks
            .retain(|_, metadata| metadata.file_path != file_path);
        self.update_timestamp();
    }

    /// Add chunk
    pub fn add_chunk(&mut self, chunk_id: ChunkId, metadata: ChunkMetadata) {
        self.chunks.insert(chunk_id, metadata);
        self.update_timestamp();
    }

    /// Remove chunk
    pub fn remove_chunk(&mut self, chunk_id: &ChunkId) {
        self.chunks.remove(chunk_id);
        self.update_timestamp();
    }

    /// Update timestamp
    fn update_timestamp(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Get all chunks for a file
    pub fn get_file_chunks(&self, file_path: &Path) -> Vec<(ChunkId, &ChunkMetadata)> {
        self.chunks
            .iter()
            .filter(|(_, metadata)| metadata.file_path == file_path)
            .map(|(id, metadata)| (*id, metadata))
            .collect()
    }
}
