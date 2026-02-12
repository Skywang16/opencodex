//! Shell detection and management

use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::warn;

use crate::mux::ConfigManager;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellInfo {
    pub name: String,
    pub path: String,
    pub display_name: String,
}

impl ShellInfo {
    pub fn new(name: &str, path: &str, display_name: &str) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            display_name: display_name.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellManagerStats {
    pub available_shells: usize,
    pub default_shell: Option<ShellInfo>,
    pub last_detection_time: Option<u64>,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

#[derive(Debug, Clone)]
struct ShellCacheEntry {
    shells: Vec<ShellInfo>,
    default_shell: ShellInfo,
    timestamp: SystemTime,
    access_count: u64,
}

impl ShellCacheEntry {
    fn new(shells: Vec<ShellInfo>, default_shell: ShellInfo) -> Self {
        Self {
            shells,
            default_shell,
            timestamp: SystemTime::now(),
            access_count: 0,
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed().unwrap_or(Duration::MAX) > ttl
    }

    fn access(&mut self) {
        self.access_count += 1;
    }
}

static SHELL_CACHE: OnceLock<Mutex<Option<ShellCacheEntry>>> = OnceLock::new();

#[derive(Debug)]
pub struct ShellManager {
    stats: ShellManagerStats,
}

impl ShellManager {
    pub fn new() -> Self {
        let mut manager = Self {
            stats: ShellManagerStats::default(),
        };
        manager.update_stats();
        manager
    }

    fn update_stats(&mut self) {
        let cache = Self::get_cache();
        let cache_guard = Self::lock_cache(cache);

        if let Some(entry) = cache_guard.as_ref() {
            self.stats.available_shells = entry.shells.len();
            self.stats.default_shell = Some(entry.default_shell.clone());
            self.stats.last_detection_time = Some(
                entry
                    .timestamp
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        } else {
            drop(cache_guard);
            let _ = Self::get_cached_shells();
            self.update_stats();
        }
    }

    pub fn get_stats(&self) -> &ShellManagerStats {
        &self.stats
    }

    fn get_cache() -> &'static Mutex<Option<ShellCacheEntry>> {
        SHELL_CACHE.get_or_init(|| Mutex::new(None))
    }

    fn lock_cache<'a>(
        cache: &'a Mutex<Option<ShellCacheEntry>>,
    ) -> std::sync::MutexGuard<'a, Option<ShellCacheEntry>> {
        cache.lock().unwrap_or_else(|poisoned| {
            warn!("Shell cache mutex poisoned, recovering");
            poisoned.into_inner()
        })
    }

    pub fn get_cached_shells() -> Vec<ShellInfo> {
        let mut cache_guard = Self::lock_cache(Self::get_cache());

        let config = ConfigManager::config_get();
        if let Some(entry) = cache_guard.as_mut() {
            if !entry.is_expired(config.shell_cache_ttl()) {
                entry.access();
                return entry.shells.clone();
            }
        }

        // Cache expired or missing, re-detect
        let shells = Self::detect_available_shells_internal();
        let default_shell = Self::get_default_shell_internal();

        // Update cache
        *cache_guard = Some(ShellCacheEntry::new(shells.clone(), default_shell));

        shells
    }

    pub fn get_cached_default_shell() -> ShellInfo {
        let cache = Self::get_cache();
        let mut cache_guard = Self::lock_cache(cache);

        let config = ConfigManager::config_get();
        if let Some(entry) = cache_guard.as_mut() {
            if !entry.is_expired(config.shell_cache_ttl()) {
                entry.access();
                return entry.default_shell.clone();
            }
        }

        // Cache expired or missing, re-detect
        drop(cache_guard);
        let _ = Self::get_cached_shells(); // This will update the cache

        // Re-acquire
        let cache_guard = Self::lock_cache(cache);
        cache_guard
            .as_ref()
            .map(|entry| entry.default_shell.clone())
            .unwrap_or_else(Self::get_default_shell_internal)
    }

    pub fn refresh_cache() {
        let cache = Self::get_cache();
        let mut cache_guard = Self::lock_cache(cache);
        *cache_guard = None;
        drop(cache_guard);
    }

    /// Check cache status
    pub fn cache_status() -> (bool, Option<SystemTime>, u64) {
        let cache = Self::get_cache();
        let cache_guard = Self::lock_cache(cache);

        if let Some(entry) = cache_guard.as_ref() {
            let config = ConfigManager::config_get();
            (
                !entry.is_expired(config.shell_cache_ttl()),
                Some(entry.timestamp),
                entry.access_count,
            )
        } else {
            (false, None, 0)
        }
    }

    /// Detect available shells on system (public interface, uses cache)
    pub fn detect_available_shells() -> Vec<ShellInfo> {
        Self::get_cached_shells()
    }

    /// Internal shell detection implementation (does not use cache)
    fn detect_available_shells_internal() -> Vec<ShellInfo> {
        let mut shells = Vec::new();
        let config = ConfigManager::config_get();

        // Get shell paths from config
        let shell_paths = if cfg!(windows) {
            &config.shell.default_paths.windows
        } else {
            &config.shell.default_paths.unix
        };

        // Detect shell paths in config
        for path in shell_paths {
            if Self::validate_shell(path) {
                if let Some(shell_name) = std::path::Path::new(path).file_name() {
                    if let Some(name_str) = shell_name.to_str() {
                        let display_name = Self::get_shell_display_name(name_str);

                        // Avoid adding shells with the same name repeatedly
                        if !shells.iter().any(|s: &ShellInfo| s.name == name_str) {
                            shells.push(ShellInfo::new(name_str, path, display_name));
                        }
                    }
                }
            }
        }

        // Try to find other shells from PATH environment variable
        if let Ok(path_env) = std::env::var("PATH") {
            // Use platform-specific PATH separator
            let path_separator = if cfg!(windows) { ';' } else { ':' };
            for path_dir in path_env.split(path_separator) {
                // Select shells to search based on platform
                let shell_names = if cfg!(windows) {
                    &["bash.exe", "zsh.exe", "fish.exe"][..]
                } else {
                    &["zsh", "bash", "fish"][..]
                };

                for shell_name in shell_names {
                    // Use PathBuf to properly handle path joining
                    let shell_path = std::path::PathBuf::from(path_dir)
                        .join(shell_name)
                        .to_string_lossy()
                        .to_string();

                    if Self::validate_shell(&shell_path)
                        && !shells.iter().any(|s| s.path == shell_path)
                    {
                        let base_name = if cfg!(windows) {
                            shell_name.strip_suffix(".exe").unwrap_or(shell_name)
                        } else {
                            shell_name
                        };

                        let display_name = match base_name {
                            "zsh" => "Zsh",
                            "bash" => "Bash",
                            "fish" => "Fish",
                            _ => shell_name,
                        };
                        shells.push(ShellInfo::new(base_name, &shell_path, display_name));
                    }
                }
            }
        }

        shells
    }

    /// Get default shell (public interface, uses cache)
    pub fn terminal_get_default_shell() -> ShellInfo {
        Self::get_cached_default_shell()
    }

    /// Internal default shell retrieval implementation (does not use cache)
    fn get_default_shell_internal() -> ShellInfo {
        #[cfg(windows)]
        {
            // Default shell detection for Windows platform
            let windows_shells = [
                ("bash", "C:\\Program Files\\Git\\bin\\bash.exe", "Git Bash"),
                (
                    "bash",
                    "C:\\Program Files\\Git\\usr\\bin\\bash.exe",
                    "Git Bash",
                ),
                ("zsh", "C:\\Program Files\\Git\\usr\\bin\\zsh.exe", "Zsh"),
                ("fish", "C:\\Program Files\\Git\\usr\\bin\\fish.exe", "Fish"),
            ];

            for (name, path, display_name) in &windows_shells {
                if Self::validate_shell(path) {
                    return ShellInfo::new(name, path, display_name);
                }
            }

            // Fallback option
            ShellInfo::new("cmd", "C:\\Windows\\System32\\cmd.exe", "Command Prompt")
        }

        #[cfg(not(windows))]
        {
            // First try to get default shell from environment variable
            if let Ok(shell_path) = std::env::var("SHELL") {
                if Self::validate_shell(&shell_path) {
                    // Extract shell name from path
                    if let Some(shell_name) = std::path::Path::new(&shell_path).file_name() {
                        if let Some(name_str) = shell_name.to_str() {
                            let display_name = Self::get_shell_display_name(name_str);
                            return ShellInfo::new(name_str, &shell_path, display_name);
                        }
                    }
                }
            }

            let preferred_shells = [
                ("zsh", "/bin/zsh", "Zsh"),
                ("bash", "/bin/bash", "Bash"),
                ("fish", "/usr/bin/fish", "Fish"),
                ("sh", "/bin/sh", "sh"),
            ];

            for (name, path, display_name) in &preferred_shells {
                if Self::validate_shell(path) {
                    return ShellInfo::new(name, path, display_name);
                }
            }

            // Final fallback option
            warn!("No available shell found, using fallback");
            ShellInfo::new("bash", "/bin/bash", "Bash")
        }
    }

    /// Get shell's display name
    fn get_shell_display_name(name: &str) -> &'static str {
        match name {
            "zsh" => "Zsh",
            "bash" => "Bash",
            "fish" => "Fish",
            "sh" => "sh",
            _ => "Unknown Shell",
        }
    }

    /// Verify if shell is available
    pub fn validate_shell(path: &str) -> bool {
        if path.trim().is_empty() {
            return false;
        }

        let path_obj = std::path::Path::new(path);
        let exists = path_obj.exists();
        let is_executable = path_obj.is_file();

        exists && is_executable
    }

    /// Find shell by name (uses cache)
    pub fn terminal_find_shell_by_name(name: &str) -> Option<ShellInfo> {
        if name.trim().is_empty() {
            return None;
        }

        let shells = Self::get_cached_shells();
        shells.into_iter().find(|shell| shell.name == name)
    }

    /// Find shell by path (uses cache)
    pub fn terminal_find_shell_by_path(path: &str) -> Option<ShellInfo> {
        if path.trim().is_empty() {
            return None;
        }

        let shells = Self::get_cached_shells();
        shells.into_iter().find(|shell| shell.path == path)
    }

    /// Get detailed statistics for shell manager
    pub fn get_detailed_stats() -> ShellManagerStats {
        let cache = Self::get_cache();
        let cache_guard = Self::lock_cache(cache);

        let mut stats = ShellManagerStats::default();

        if let Some(entry) = cache_guard.as_ref() {
            stats.available_shells = entry.shells.len();
            stats.default_shell = Some(entry.default_shell.clone());
            stats.last_detection_time = Some(
                entry
                    .timestamp
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
            stats.cache_hits = entry.access_count;
        }

        stats
    }
}

impl Default for ShellManager {
    fn default() -> Self {
        Self::new()
    }
}
