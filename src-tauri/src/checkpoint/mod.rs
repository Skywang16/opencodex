//! Checkpoint system module
//!
//! Provides Git-like file state snapshot functionality, supporting:
//! - Automatic checkpoint creation (when user sends a message)
//! - View checkpoint history
//! - Rollback to any historical state
//! - File diff comparison

mod blob_store;
pub mod commands;
mod config;
mod models;
mod service;
mod storage;

pub use blob_store::{BlobStore, BlobStoreStats};
pub use commands::CheckpointState;
pub use config::CheckpointConfig;
pub use models::{
    Checkpoint, CheckpointError, CheckpointResult, CheckpointSummary, FileChangeType, FileDiff,
    FileSnapshot, NewCheckpoint, NewFileSnapshot, RollbackResult,
};
pub use service::CheckpointService;
pub use storage::CheckpointStorage;
