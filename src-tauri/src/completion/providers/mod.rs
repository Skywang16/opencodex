//! Completion providers module
//!
//! Defines providers for various completion data sources

pub mod context_aware;
pub mod filesystem;
pub mod git;
pub mod history;
pub mod npm;
pub mod system_commands;

pub use context_aware::*;
pub use filesystem::*;
pub use git::*;
pub use history::*;
pub use npm::*;
pub use system_commands::*;

use crate::completion::error::CompletionProviderResult;
use crate::completion::types::{CompletionContext, CompletionItem};
use async_trait::async_trait;

/// Completion provider trait
#[async_trait]
pub trait CompletionProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &'static str;

    /// Check if completions should be provided for the given context
    fn should_provide(&self, context: &CompletionContext) -> bool;

    /// Provide completion suggestions
    async fn provide_completions(
        &self,
        context: &CompletionContext,
    ) -> CompletionProviderResult<Vec<CompletionItem>>;

    /// Get provider priority (higher number = higher priority)
    fn priority(&self) -> i32 {
        0
    }

    /// Method to support downcast
    fn as_any(&self) -> &dyn std::any::Any;
}
