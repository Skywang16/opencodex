//! Node.js Version Management Module
//!
//! Provides Node.js project detection, version manager identification, version switching, and more

pub mod commands;
pub mod detector;
pub mod types;

pub use commands::*;
pub use types::*;
