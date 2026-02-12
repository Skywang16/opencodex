/*!
 * Terminal configuration related Tauri commands
 *
 * Provides terminal configuration retrieval, update, validation, and other functionality.
 * Uses JSON ConfigManager as the underlying implementation.
 */

use crate::config::{
    defaults::create_default_terminal_config, manager::ConfigManager, types::TerminalConfig,
};
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tracing::warn;

/// Terminal configuration validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalConfigValidationResult {
    /// Whether the configuration is valid
    pub is_valid: bool,
    /// List of error messages
    pub errors: Vec<String>,
    /// List of warning messages
    pub warnings: Vec<String>,
}

// Tauri command interface

/// Get terminal configuration
#[tauri::command]
pub async fn terminal_config_get(
    state: State<'_, Arc<ConfigManager>>,
) -> TauriApiResult<TerminalConfig> {
    match state.config_get().await {
        Ok(config) => {
            let terminal_config = config.terminal.clone();
            Ok(api_success!(terminal_config))
        }
        Err(_) => Ok(api_error!("config.get_failed")),
    }
}

/// Set terminal configuration (full replacement)
#[tauri::command]
pub async fn terminal_config_set(
    terminal_config: TerminalConfig,
    state: State<'_, Arc<ConfigManager>>,
) -> TauriApiResult<EmptyData> {
    let result = state
        .config_update(|config| {
            config.terminal = terminal_config.clone();
            Ok(())
        })
        .await;

    match result {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.update_failed")),
    }
}

/// Validate terminal configuration
#[tauri::command]
pub async fn terminal_config_validate(
    state: State<'_, Arc<ConfigManager>>,
) -> TauriApiResult<TerminalConfigValidationResult> {
    let config = match state.config_get().await {
        Ok(c) => c,
        Err(_) => return Ok(api_error!("config.get_failed")),
    };
    let terminal_config = &config.terminal;

    let mut errors = Vec::new();
    let warnings = Vec::new();

    // Validate scrollback buffer
    if !(100..=100000).contains(&terminal_config.scrollback) {
        errors.push(format!(
            "Scrollback buffer lines must be between 100-100000, current value: {}",
            terminal_config.scrollback
        ));
    }

    // Validate shell configuration
    if terminal_config.shell.default_shell.is_empty() {
        errors.push("Default shell cannot be empty".to_string());
    }

    // Validate cursor configuration
    if !(0.1..=5.0).contains(&terminal_config.cursor.thickness) {
        errors.push(format!(
            "Cursor thickness must be between 0.1-5.0, current value: {}",
            terminal_config.cursor.thickness
        ));
    }

    // Validate color format
    if !terminal_config.cursor.color.starts_with('#') || terminal_config.cursor.color.len() != 7 {
        errors.push(format!(
            "Invalid cursor color format: {}",
            terminal_config.cursor.color
        ));
    }

    let is_valid = errors.is_empty();

    if !is_valid {
        warn!("Terminal configuration validation failed: {:?}", errors);
    }

    Ok(api_success!(TerminalConfigValidationResult {
        is_valid,
        errors,
        warnings,
    }))
}

/// Reset terminal configuration to defaults
#[tauri::command]
pub async fn terminal_config_reset_to_defaults(
    state: State<'_, Arc<ConfigManager>>,
) -> TauriApiResult<EmptyData> {
    let default_terminal_config = create_default_terminal_config();

    // Update configuration
    let result = state
        .config_update(|config| {
            config.terminal = default_terminal_config.clone();
            Ok(())
        })
        .await;

    match result {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("config.reset_failed")),
    }
}
