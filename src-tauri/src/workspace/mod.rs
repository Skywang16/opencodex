/*!
 * Workspace Module
 *
 * Workspace management module
 * Responsible for: recent workspace history, project rules management, workspace context
 */

pub mod commands;
pub mod error;
mod rules;
mod service;
mod types;

// Export commonly used types and functions
pub use commands::*;
pub use rules::get_available_rules_files;
pub use service::*;
pub use types::RULES_FILES;
