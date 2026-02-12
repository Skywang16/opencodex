//! Shell Integration - Complete Shell Integration System
//!
//! Supports integration with multiple shells, including command tracking, CWD synchronization, window title updates, and more

pub mod commands;
pub mod error;
pub mod integration;
pub mod osc_parser;
pub mod script_generator;

#[cfg(test)]
mod integration_test;

pub use commands::*;
pub use error::*;
pub use integration::*;
pub use osc_parser::*;
pub use script_generator::*;

// Export Shell events from unified events module
pub use crate::events::ShellEvent;
