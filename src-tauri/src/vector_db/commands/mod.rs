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
            Err(poisoned) => {
                tracing::error!("vector search engine RwLock poisoned, recovering");
                poisoned.into_inner().clone()
            }
        }
    }

    pub fn replace_search_engine(&self, search_engine: Arc<SemanticSearchEngine>) {
        match self.search_engine.write() {
            Ok(mut guard) => *guard = Some(search_engine),
            Err(poisoned) => {
                tracing::error!("vector search engine RwLock poisoned, recovering");
                *poisoned.into_inner() = Some(search_engine);
            }
        }
    }
}
