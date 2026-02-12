/*!
 * New shortcut system
 *
 * Configuration-driven design, supports:
 * - Unified shortcut management
 * - Dynamic function mapping
 * - Conflict detection and validation
 * - Runtime configuration updates
 */

pub mod actions;
pub mod commands;
pub mod core;
pub mod types;

// Re-export core modules
pub use actions::*;
pub use commands::*;
pub use core::*;
pub use types::*;
