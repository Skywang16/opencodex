/*!
 * Theme-related Tauri commands
 *
 * Provides theme management interfaces for frontend calls, including getting current theme,
 * switching themes, getting theme lists, and other functionality.
 */

use super::service::{SystemThemeDetector, ThemeService};
use super::types::{Theme, ThemeConfig};
use crate::config::error::{ConfigCommandError, ConfigCommandResult};
use crate::config::ConfigManager;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

/// Theme configuration status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeConfigStatus {
    /// Currently used theme name
    pub current_theme_name: String,

    /// Theme configuration
    pub theme_config: ThemeConfig,

    /// Whether system is in dark mode
    pub is_system_dark: Option<bool>,
}

/// Get current theme configuration status
#[tauri::command]
pub async fn theme_get_config_status(
    config_manager: State<'_, Arc<ConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriApiResult<ThemeConfigStatus> {
    let config = match config_manager.config_get().await {
        Ok(config) => config,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };

    let theme_config = &config.appearance.theme_config;
    let is_system_dark = SystemThemeDetector::is_dark_mode();

    let current_theme_name = theme_service.get_current_theme_name(theme_config, is_system_dark);

    Ok(api_success!(ThemeConfigStatus {
        current_theme_name,
        theme_config: theme_config.clone(),
        is_system_dark,
    }))
}

/// Get current theme data
#[tauri::command]
pub async fn theme_get_current(
    config_manager: State<'_, Arc<ConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriApiResult<Theme> {
    let config = match config_manager.config_get().await {
        Ok(config) => config,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };

    let theme_config = &config.appearance.theme_config;
    let is_system_dark = SystemThemeDetector::is_dark_mode();

    match theme_service
        .load_current_theme(theme_config, is_system_dark)
        .await
    {
        Ok(theme) => Ok(api_success!(theme)),
        Err(_) => Ok(api_error!("config.get_failed")),
    }
}

/// Set terminal theme (manual mode)
#[tauri::command]
pub async fn theme_set_terminal<R: Runtime>(
    theme_name: String,
    app_handle: AppHandle<R>,
    config_manager: State<'_, Arc<ConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriApiResult<EmptyData> {
    // Validate theme exists
    if !theme_service.theme_exists(&theme_name).await {
        return Ok(api_error!("common.not_found"));
    }

    // Update configuration
    if config_manager
        .config_update(|config| {
            config.appearance.theme_config.terminal_theme = theme_name.clone();
            config.appearance.theme_config.follow_system = false; // Switch to manual mode
            Ok(())
        })
        .await
        .is_err()
    {
        return Ok(api_error!("config.update_failed"));
    }

    // Emit theme change event to ensure frontend responds immediately
    if app_handle.emit("theme-changed", &theme_name).is_err() {
        return Ok(api_error!("config.update_failed"));
    }

    Ok(api_success!())
}

/// Set follow system theme
#[tauri::command]
pub async fn theme_set_follow_system<R: Runtime>(
    follow_system: bool,
    light_theme: Option<String>,
    dark_theme: Option<String>,
    app_handle: AppHandle<R>,
    config_manager: State<'_, Arc<ConfigManager>>,
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriApiResult<EmptyData> {
    // Validate themes exist
    if let Some(ref light) = light_theme {
        if !theme_service.theme_exists(light).await {
            return Ok(api_error!("common.not_found"));
        }
    }

    if let Some(ref dark) = dark_theme {
        if !theme_service.theme_exists(dark).await {
            return Ok(api_error!("common.not_found"));
        }
    }

    // Update configuration
    if config_manager
        .config_update(|config| {
            config.appearance.theme_config.follow_system = follow_system;

            if let Some(light) = light_theme {
                config.appearance.theme_config.light_theme = light;
            }

            if let Some(dark) = dark_theme {
                config.appearance.theme_config.dark_theme = dark;
            }

            Ok(())
        })
        .await
        .is_err()
    {
        return Ok(api_error!("config.update_failed"));
    }

    if follow_system {
        let config = match config_manager.config_get().await {
            Ok(config) => config,
            Err(_) => return Ok(api_error!("config.get_failed")),
        };
        let is_system_dark = SystemThemeDetector::is_dark_mode();
        let current_theme_name =
            theme_service.get_current_theme_name(&config.appearance.theme_config, is_system_dark);

        // Emit theme change event
        if app_handle
            .emit("theme-changed", &current_theme_name)
            .is_err()
        {
            return Ok(api_error!("config.update_failed"));
        }
    }

    Ok(api_success!())
}

/// Get list of all available themes (returns complete theme data)
#[tauri::command]
pub async fn theme_get_available(
    theme_service: State<'_, Arc<ThemeService>>,
) -> TauriApiResult<Vec<Theme>> {
    let theme_list = match theme_service.theme_manager().list_themes().await {
        Ok(list) => list,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };

    let mut themes = Vec::new();
    for entry in theme_list {
        if let Ok(theme) = theme_service.theme_manager().load_theme(&entry.name).await {
            themes.push(theme);
        }
    }

    Ok(api_success!(themes))
}

/// Handle system theme change
pub async fn handle_system_theme_change<R: tauri::Runtime>(
    app_handle: &AppHandle<R>,
    is_dark: bool,
) -> ConfigCommandResult<()> {
    let config_manager = app_handle.state::<Arc<ConfigManager>>();
    let theme_service = app_handle.state::<Arc<ThemeService>>();

    let config = config_manager
        .config_get()
        .await
        .map_err(|err| ConfigCommandError::Internal(err.to_string()))?;

    // Only process when following system theme
    if config.appearance.theme_config.follow_system {
        let current_theme_name =
            theme_service.get_current_theme_name(&config.appearance.theme_config, Some(is_dark));

        // Notify frontend that theme has changed
        app_handle
            .emit("theme-changed", &current_theme_name)
            .map_err(|err| ConfigCommandError::Internal(err.to_string()))?;
    }

    Ok(())
}
