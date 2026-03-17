use super::*;
use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use tracing::warn;

#[tauri::command]
pub async fn window_state_get(
    refresh: Option<bool>,
    state: State<'_, AppWindowState>,
) -> TauriApiResult<WindowStateSnapshot> {
    let refresh = refresh.unwrap_or_default();

    if refresh {
        state.cache.remove("current_dir").await;
        state.cache.remove("home_dir").await;
    }

    let always_on_top = match state
        .with_state_manager(|manager| Ok(manager.is_always_on_top()))
        .await
    {
        Ok(value) => value,
        Err(_) => return Ok(api_error!("window.get_state_failed")),
    };

    let current_directory = match load_cached_directory(&state, "current_dir").await {
        Ok(Some(dir)) => dir,
        Ok(None) => {
            let dir = match env::current_dir() {
                Ok(path) => path.to_string_lossy().to_string(),
                Err(err) => {
                    warn!("Failed to get current directory: {}", err);
                    return Ok(api_error!("common.operation_failed"));
                }
            };
            if let Err(err) = state
                .cache
                .set("current_dir", serde_json::Value::String(dir.clone()))
                .await
            {
                warn!("Failed to update current directory cache: {}", err);
            }
            dir
        }
        Err(err) => {
            warn!("Invalid current directory cache value: {}", err);
            return Ok(api_error!("common.operation_failed"));
        }
    };

    let home_directory = match load_cached_directory(&state, "home_dir").await {
        Ok(Some(dir)) => dir,
        Ok(None) => {
            let dir = match env::var("HOME") {
                Ok(dir) => dir,
                Err(err) => {
                    warn!("Failed to get HOME: {}", err);
                    return Ok(api_error!("common.operation_failed"));
                }
            };
            if let Err(err) = state
                .cache
                .set("home_dir", serde_json::Value::String(dir.clone()))
                .await
            {
                warn!("Failed to update home directory cache: {}", err);
            }
            dir
        }
        Err(err) => {
            warn!("Invalid home directory cache value: {}", err);
            return Ok(api_error!("common.operation_failed"));
        }
    };

    let platform_info = match state
        .with_config_manager(|config| Ok(config.platform_info().cloned()))
        .await
    {
        Ok(Some(info)) => info,
        _ => {
            let info = PlatformInfo {
                platform: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                os_version: detect_os_version(),
                is_mac: cfg!(target_os = "macos"),
            };
            if let Err(err) = state
                .with_config_manager_mut(|config| {
                    config.set_platform_info(info.clone());
                    Ok(())
                })
                .await
            {
                warn!("Failed to persist platform info: {}", err);
            }
            info
        }
    };

    let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(err) => {
            warn!("System clock is before UNIX_EPOCH: {}", err);
            return Ok(api_error!("common.operation_failed"));
        }
    };

    Ok(api_success!(WindowStateSnapshot {
        always_on_top,
        current_directory,
        home_directory,
        platform_info,
        timestamp,
    }))
}

#[tauri::command]
pub async fn window_state_update<R: Runtime>(
    update: WindowStateUpdate,
    app: AppHandle<R>,
    state: State<'_, AppWindowState>,
) -> TauriApiResult<WindowStateSnapshot> {
    let needs_window = update.always_on_top.is_some();
    let window = if needs_window {
        let window_id = match state
            .with_config_manager(|config| Ok(config.default_window_id().to_string()))
            .await
        {
            Ok(id) => id,
            Err(_) => return Ok(api_error!("window.get_window_id_failed")),
        };

        match app.get_webview_window(&window_id) {
            Some(window) => Some(window),
            None => return Ok(api_error!("window.get_instance_failed")),
        }
    } else {
        None
    };

    if let Some(always_on_top) = update.always_on_top {
        if let Some(window) = &window {
            if window.set_always_on_top(always_on_top).is_err() {
                return Ok(api_error!("window.set_always_on_top_failed"));
            }
        }

        if state
            .with_state_manager_mut(|manager| {
                manager.set_always_on_top(always_on_top);
                Ok(())
            })
            .await
            .is_err()
        {
            return Ok(api_error!("window.set_always_on_top_failed"));
        }
    }

    if update.refresh_directories.unwrap_or_default() {
        state.cache.remove("current_dir").await;
        state.cache.remove("home_dir").await;
    }

    window_state_get(Some(false), state).await
}

async fn load_cached_directory(
    state: &AppWindowState,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(value) = state.cache.get(key).await else {
        return Ok(None);
    };
    let Some(dir) = value.as_str() else {
        return Err(format!("cache key '{key}' is not a string"));
    };
    if dir.trim().is_empty() {
        return Err(format!("cache key '{key}' is empty"));
    }
    Ok(Some(dir.to_string()))
}

fn detect_os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        match std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
        {
            Ok(output) => match String::from_utf8(output.stdout) {
                Ok(version) => return version.trim().to_string(),
                Err(err) => tracing::warn!("failed to decode macOS version output: {}", err),
            },
            Err(err) => tracing::warn!("failed to execute sw_vers: {}", err),
        }
        "macOS Unknown".to_string()
    }

    #[cfg(target_os = "linux")]
    {
        match std::fs::read_to_string("/etc/os-release") {
            Ok(contents) => {
                for line in contents.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        let version = line.trim_start_matches("PRETTY_NAME=").trim_matches('\"');
                        return version.to_string();
                    }
                }
            }
            Err(err) => tracing::warn!("failed to read /etc/os-release: {}", err),
        }
        "Linux Unknown".to_string()
    }

    #[cfg(target_os = "windows")]
    {
        match std::process::Command::new("cmd")
            .args(["/C", "ver"])
            .output()
        {
            Ok(output) => match String::from_utf8(output.stdout) {
                Ok(version) => return version.trim().to_string(),
                Err(err) => tracing::warn!("failed to decode Windows version output: {}", err),
            },
            Err(err) => tracing::warn!("failed to execute cmd /C ver: {}", err),
        }
        "Windows Unknown".to_string()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "Unknown OS".to_string()
    }
}
