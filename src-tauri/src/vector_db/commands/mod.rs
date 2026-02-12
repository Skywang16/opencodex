pub mod build;
pub mod index;

pub use build::*;
pub use index::*;

use crate::vector_db::SemanticSearchEngine;
use std::sync::Arc;

/// Vector database global state (managed by Tauri)
pub struct VectorDbState {
    pub search_engine: Arc<SemanticSearchEngine>,
}

impl VectorDbState {
    pub fn new(search_engine: Arc<SemanticSearchEngine>) -> Self {
        Self { search_engine }
    }
}
