pub mod build;
pub mod index;

pub use build::*;
pub use index::*;

use crate::vector_db::SemanticSearchEngine;
use std::sync::Arc;
use std::sync::RwLock;

/// Vector database global state (managed by Tauri)
pub struct VectorDbState {
    search_engine: RwLock<Option<Arc<SemanticSearchEngine>>>,
}

impl VectorDbState {
    pub fn new(search_engine: Arc<SemanticSearchEngine>) -> Self {
        Self {
            search_engine: RwLock::new(Some(search_engine)),
        }
    }

    /// Create a state with no search engine (embedding model not configured).
    pub fn empty() -> Self {
        Self {
            search_engine: RwLock::new(None),
        }
    }

    pub fn current_search_engine(&self) -> Option<Arc<SemanticSearchEngine>> {
        match self.search_engine.read() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                tracing::warn!("Vector search engine lock poisoned during read; recovering");
                err.into_inner().clone()
            }
        }
    }

    pub fn replace_search_engine(&self, search_engine: Arc<SemanticSearchEngine>) {
        let mut guard = match self.search_engine.write() {
            Ok(guard) => guard,
            Err(err) => {
                tracing::warn!("Vector search engine lock poisoned during write; recovering");
                err.into_inner()
            }
        };
        *guard = Some(search_engine);
    }
}
