/*!
 * Theme system module
 *
 * Unified management of all theme-related functionality, including theme management, command interfaces, services, and type definitions.
 */

pub mod commands;
pub mod defaults;
pub mod manager;
pub mod service;
pub mod types;

// Re-export core types and functions
pub use commands::{
    handle_system_theme_change, theme_get_available, theme_get_config_status, theme_get_current,
    theme_set_follow_system, theme_set_terminal, ThemeConfigStatus,
};
pub use defaults::create_default_theme_config;
pub use manager::{
    ThemeIndexEntry, ThemeManager, ThemeManagerOptions, ThemeValidationResult, ThemeValidator,
};
pub use service::{SystemThemeDetector, ThemeService};
pub use types::{AnsiColors, SyntaxHighlight, Theme, ThemeConfig, ThemeType, UIColors};
