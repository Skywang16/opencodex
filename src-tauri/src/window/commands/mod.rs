// Window command handlers exposed to Tauri

pub mod state;

pub use state::*;

use crate::window::WindowStateResult;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime, State};
use tokio::sync::Mutex;

// Top-level window state container (managed by Tauri as State<AppWindowState>)
pub struct AppWindowState {
    pub cache: crate::storage::cache::UnifiedCache,
    inner: Mutex<WindowStateInner>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub platform: String,
    pub arch: String,
    pub os_version: String,
    pub is_mac: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateSnapshot {
    pub always_on_top: bool,
    pub current_directory: String,
    pub home_directory: String,
    pub platform_info: PlatformInfo,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateUpdate {
    pub always_on_top: Option<bool>,
    pub refresh_directories: Option<bool>,
}

#[derive(Debug)]
pub struct WindowConfigManager {
    platform_info: Option<PlatformInfo>,
    default_window_id: String,
}

#[derive(Debug)]
pub struct WindowStateManager {
    always_on_top: AtomicBool,
}

impl Default for WindowStateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct WindowStateInner {
    config: WindowConfigManager,
    state: WindowStateManager,
}

impl WindowStateManager {
    pub fn new() -> Self {
        Self {
            always_on_top: AtomicBool::new(false),
        }
    }

    pub fn set_always_on_top(&mut self, value: bool) {
        self.always_on_top.store(value, Ordering::Release); // Atomic write
    }

    pub fn is_always_on_top(&self) -> bool {
        self.always_on_top.load(Ordering::Acquire) // Atomic read
    }

    pub fn toggle_always_on_top(&mut self) -> bool {
        let new_value = !self.always_on_top.load(Ordering::Acquire);
        self.always_on_top.store(new_value, Ordering::Release);
        new_value
    }

    pub fn clear_cache(&mut self) {
        // No-op: cache fields removed
    }

    pub fn reset(&mut self) {
        self.always_on_top.store(false, Ordering::Release);
    }
}

impl WindowConfigManager {
    pub fn new() -> Self {
        Self {
            platform_info: None,
            default_window_id: "main".to_string(),
        }
    }

    pub fn set_platform_info(&mut self, info: PlatformInfo) {
        self.platform_info = Some(info);
    }

    pub fn platform_info(&self) -> Option<&PlatformInfo> {
        self.platform_info.as_ref()
    }

    pub fn default_window_id(&self) -> &str {
        &self.default_window_id
    }
}

impl Default for WindowConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AppWindowState {
    pub fn new() -> WindowStateResult<Self> {
        Ok(Self {
            cache: crate::storage::cache::UnifiedCache::new(),
            inner: Mutex::new(WindowStateInner {
                config: WindowConfigManager::new(),
                state: WindowStateManager::new(),
            }),
        })
    }

    pub async fn with_config_manager<F, R>(&self, f: F) -> WindowStateResult<R>
    where
        F: FnOnce(&WindowConfigManager) -> WindowStateResult<R>,
    {
        let inner = self.inner.lock().await;
        f(&inner.config)
    }

    pub async fn with_config_manager_mut<F, R>(&self, f: F) -> WindowStateResult<R>
    where
        F: FnOnce(&mut WindowConfigManager) -> WindowStateResult<R>,
    {
        let mut inner = self.inner.lock().await;
        f(&mut inner.config)
    }

    pub async fn with_state_manager<F, R>(&self, f: F) -> WindowStateResult<R>
    where
        F: FnOnce(&WindowStateManager) -> WindowStateResult<R>,
    {
        let inner = self.inner.lock().await;
        f(&inner.state)
    }

    pub async fn with_state_manager_mut<F, R>(&self, f: F) -> WindowStateResult<R>
    where
        F: FnOnce(&mut WindowStateManager) -> WindowStateResult<R>,
    {
        let mut inner = self.inner.lock().await;
        f(&mut inner.state)
    }
}
