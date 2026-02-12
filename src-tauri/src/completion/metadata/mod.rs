//! Command metadata module
//!
//! Provides a configuration-driven command metadata system, replacing hardcoded command lists
//!

pub mod builtin;
pub mod command_spec;
pub mod registry;

pub use command_spec::CommandSpec;
pub use registry::CommandRegistry;
