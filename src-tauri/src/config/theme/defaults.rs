/*!
 * Theme system default configuration
 *
 * Provides default values for theme-related configuration items.
 */

use super::types::ThemeConfig;

/// Create default theme configuration
pub fn create_default_theme_config() -> ThemeConfig {
    ThemeConfig {
        terminal_theme: "default".to_string(),
        light_theme: "light".to_string(),
        dark_theme: "dark".to_string(),
        follow_system: true,
    }
}
