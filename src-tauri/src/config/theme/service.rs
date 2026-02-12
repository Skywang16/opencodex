/*!
 * Theme service
 *
 * Determines which theme should be used based on theme settings in the configuration file.
 * Supports two modes: following system theme and manual theme selection.
 */

use super::manager::ThemeManager;
use super::types::{Theme, ThemeConfig};
use crate::config::error::{ThemeConfigError, ThemeConfigResult};
use crate::config::paths::ConfigPaths;
use crate::config::theme::ThemeManagerOptions;
use crate::storage::cache::UnifiedCache;
use std::sync::Arc;
use tracing::warn;

/// Theme service
pub struct ThemeService {
    /// Theme manager
    theme_manager: Arc<ThemeManager>,
}

impl ThemeService {
    /// Create a new theme service instance
    pub async fn new(
        paths: ConfigPaths,
        options: ThemeManagerOptions,
        cache: Arc<UnifiedCache>,
    ) -> ThemeConfigResult<Self> {
        let theme_manager = Arc::new(ThemeManager::new(paths, options, cache.clone()).await?);
        Ok(Self { theme_manager })
    }

    /// Get theme manager reference
    pub fn theme_manager(&self) -> &Arc<ThemeManager> {
        &self.theme_manager
    }

    /// Get the theme name that should be used based on configuration
    ///
    /// # Arguments
    /// * `theme_config` - Theme configuration
    /// * `is_system_dark` - Whether system is in dark mode (optional, used when following system theme)
    ///
    /// # Returns
    /// Returns the theme name that should be used
    pub fn get_current_theme_name(
        &self,
        theme_config: &ThemeConfig,
        is_system_dark: Option<bool>,
    ) -> String {
        if theme_config.follow_system {
            // Follow system theme mode
            match is_system_dark {
                Some(true) => theme_config.dark_theme.clone(),
                Some(false) => theme_config.light_theme.clone(),
                None => theme_config.dark_theme.clone(),
            }
        } else {
            // Manual theme selection mode
            theme_config.terminal_theme.clone()
        }
    }

    /// Load current theme based on configuration
    ///
    /// # Arguments
    /// * `theme_config` - Theme configuration
    /// * `is_system_dark` - Whether system is in dark mode (optional)
    ///
    /// # Returns
    /// Returns theme data
    pub async fn load_current_theme(
        &self,
        theme_config: &ThemeConfig,
        is_system_dark: Option<bool>,
    ) -> ThemeConfigResult<Theme> {
        let theme_name = self.get_current_theme_name(theme_config, is_system_dark);

        match self.theme_manager.load_theme(&theme_name).await {
            Ok(theme) => Ok(theme),
            Err(e) => {
                warn!("Theme loading failed: {} - {}", theme_name, e);

                // Try loading fallback theme
                let fallback_theme = if theme_config.follow_system {
                    match is_system_dark {
                        Some(true) => &theme_config.light_theme,
                        _ => &theme_config.dark_theme,
                    }
                } else {
                    &theme_config.dark_theme
                };

                self.theme_manager
                    .load_theme(fallback_theme)
                    .await
                    .map_err(|fallback_err| {
                        ThemeConfigError::Internal(format!(
                            "Failed to load theme {theme_name} ({e}), fallback {fallback_theme} failed: {fallback_err}"
                        ))
                    })
            }
        }
    }

    /// Validate that themes referenced in theme configuration exist
    ///
    /// # Arguments
    /// * `theme_config` - Theme configuration
    ///
    /// # Returns
    /// Returns validation result and list of missing themes
    pub async fn validate_theme_config(
        &self,
        theme_config: &ThemeConfig,
    ) -> ThemeConfigResult<Vec<String>> {
        let mut missing_themes = Vec::new();

        if self
            .theme_manager
            .load_theme(&theme_config.terminal_theme)
            .await
            .is_err()
        {
            missing_themes.push(theme_config.terminal_theme.clone());
        }

        if self
            .theme_manager
            .load_theme(&theme_config.light_theme)
            .await
            .is_err()
        {
            missing_themes.push(theme_config.light_theme.clone());
        }

        if self
            .theme_manager
            .load_theme(&theme_config.dark_theme)
            .await
            .is_err()
        {
            missing_themes.push(theme_config.dark_theme.clone());
        }

        if !missing_themes.is_empty() {
            warn!("Found missing themes: {:?}", missing_themes);
        }

        Ok(missing_themes)
    }

    /// Get list of all available themes
    pub async fn list_available_themes(&self) -> ThemeConfigResult<Vec<String>> {
        let themes = self.theme_manager.list_themes().await?;
        let theme_names: Vec<String> = themes.into_iter().map(|t| t.name).collect();
        Ok(theme_names)
    }

    /// Check if specified theme exists
    pub async fn theme_exists(&self, theme_name: &str) -> bool {
        self.theme_manager.load_theme(theme_name).await.is_ok()
    }
}

/// System theme detector
pub struct SystemThemeDetector;

impl SystemThemeDetector {
    /// Detect if system is in dark mode
    ///
    /// # Returns
    /// Returns system theme status, None means detection is not possible
    pub fn is_dark_mode() -> Option<bool> {
        #[cfg(target_os = "macos")]
        {
            // macOS system theme detection
            // Use osascript command to detect system theme
            use std::process::Command;

            let output = Command::new("osascript")
                .args(["-e", "tell application \"System Events\" to tell appearance preferences to get dark mode"])
                .output()
                .ok()?;

            if output.status.success() {
                let result = String::from_utf8_lossy(&output.stdout);
                let is_dark = result.trim().eq_ignore_ascii_case("true");
                Some(is_dark)
            } else {
                let output = Command::new("defaults")
                    .args(["read", "-g", "AppleInterfaceStyle"])
                    .output()
                    .ok()?;

                if output.status.success() {
                    let result = String::from_utf8_lossy(&output.stdout);
                    Some(result.trim().eq_ignore_ascii_case("dark"))
                } else {
                    None
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Windows system theme detection
            // Can be detected via registry, simplified here
            None
        }

        #[cfg(target_os = "linux")]
        {
            // Linux system theme detection
            if let Ok(theme) = std::env::var("GTK_THEME") {
                Some(theme.to_lowercase().contains("dark"))
            } else if let Ok(theme) = std::env::var("QT_STYLE_OVERRIDE") {
                Some(theme.to_lowercase().contains("dark"))
            } else {
                None
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            None
        }
    }

    /// Start system theme listener (macOS only)
    #[cfg(target_os = "macos")]
    pub fn start_system_theme_listener<F>(callback: F) -> Option<std::thread::JoinHandle<()>>
    where
        F: Fn(bool) + Send + 'static,
    {
        use std::thread;
        use std::time::Duration;

        Some(thread::spawn(move || {
            let mut last_dark_mode: Option<bool> = None;

            loop {
                let current_dark_mode = Self::is_dark_mode();

                if current_dark_mode != last_dark_mode {
                    if let Some(is_dark) = current_dark_mode {
                        callback(is_dark);
                    }
                    last_dark_mode = current_dark_mode;
                }

                // Check for theme changes every second
                thread::sleep(Duration::from_secs(1));
            }
        }))
    }

    /// Start system theme listener (empty implementation for non-macOS platforms)
    #[cfg(not(target_os = "macos"))]
    pub fn start_system_theme_listener<F>(_callback: F) -> Option<std::thread::JoinHandle<()>>
    where
        F: Fn(bool) + Send + 'static,
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::theme::ThemeConfig;
    use crate::storage::cache::UnifiedCache;

    fn create_test_theme_config() -> ThemeConfig {
        ThemeConfig {
            terminal_theme: "test-theme".to_string(),
            light_theme: "test-light".to_string(),
            dark_theme: "test-dark".to_string(),
            follow_system: false,
        }
    }

    #[tokio::test]
    async fn test_get_current_theme_name_manual_mode() {
        use crate::config::paths::ConfigPaths;
        use crate::config::theme::ThemeManagerOptions;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let paths = ConfigPaths::with_app_data_dir(temp_dir.path()).unwrap();
        let options = ThemeManagerOptions::default();
        let cache = Arc::new(UnifiedCache::new());
        let service = ThemeService::new(paths, options, cache).await.unwrap();
        let config = create_test_theme_config();

        let theme_name = service.get_current_theme_name(&config, Some(true));
        assert_eq!(theme_name, "test-theme");
    }

    #[tokio::test]
    async fn test_get_current_theme_name_follow_system_dark() {
        use crate::config::paths::ConfigPaths;
        use crate::config::theme::ThemeManagerOptions;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let paths = ConfigPaths::with_app_data_dir(temp_dir.path()).unwrap();
        let options = ThemeManagerOptions::default();
        let cache = Arc::new(UnifiedCache::new());
        let service = ThemeService::new(paths, options, cache).await.unwrap();
        let mut config = create_test_theme_config();
        config.follow_system = true;

        let theme_name = service.get_current_theme_name(&config, Some(true));
        assert_eq!(theme_name, "test-dark");
    }

    #[tokio::test]
    async fn test_get_current_theme_name_follow_system_light() {
        use crate::config::paths::ConfigPaths;
        use crate::config::theme::ThemeManagerOptions;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let paths = ConfigPaths::with_app_data_dir(temp_dir.path()).unwrap();
        let options = ThemeManagerOptions::default();
        let cache = Arc::new(UnifiedCache::new());
        let service = ThemeService::new(paths, options, cache).await.unwrap();
        let mut config = create_test_theme_config();
        config.follow_system = true;

        let theme_name = service.get_current_theme_name(&config, Some(false));
        assert_eq!(theme_name, "test-light");
    }
}
