//! Terminal Mux - Core Terminal Multiplexer
//!
//! Provides unified terminal session management, event notifications, and PTY I/O handling

pub mod config;
pub mod error;

pub mod io_handler;
pub mod pane;
pub mod shell_manager;
pub mod singleton;
// Note: tauri_integration module removed - event handling now unified in terminal::event_handler
pub mod terminal_mux;
pub mod types;

pub use config::*;

pub use error::{
    IoHandlerError, IoHandlerResult, MuxError, MuxResult, PaneError, PaneResult, TerminalMuxError,
    TerminalMuxResult,
};
pub use io_handler::*;
pub use pane::*;
pub use shell_manager::*;
pub use singleton::*;
pub use terminal_mux::*;
pub use types::*;

// Export Mux events from unified events module
pub use crate::events::MuxNotification;
