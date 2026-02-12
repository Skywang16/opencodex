//! Terminal completion functionality module
//!
//! Provides intelligent terminal command completion features, including:
//! - File path completion
//! - Command history completion
//! - System command completion
//! - Environment variable completion

pub mod command_line;
pub mod commands;
pub mod context_analyzer;
pub mod engine;
pub mod error;
pub mod learning;
pub mod metadata;
pub mod output_analyzer;
pub mod prediction;
pub mod providers;
pub mod runtime;
pub mod scoring;
pub mod smart_extractor;
pub mod smart_provider;
pub mod types;

pub use commands::*;
pub use engine::*;
pub use error::*;
pub use providers::*;
pub use runtime::*;
pub use types::*;
