//! Agent Shell execution module
//!
//! Provides Agent-specific Shell command execution functionality, supporting:
//! - Synchronous/asynchronous execution
//! - Background execution
//! - Timeout control
//! - Process management

mod buffer;
mod config;
mod error;
mod executor;
mod types;

pub use buffer::OutputRingBuffer;
pub use config::ShellExecutorConfig;
pub use error::ShellError;
pub use executor::AgentShellExecutor;
pub use types::*;
