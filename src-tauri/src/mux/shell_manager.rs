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
    default_shell: Option<ShellInfo>,
    timestamp: SystemTime,
    access_count: u64,
}

impl ShellCacheEntry {
    fn new(shells: Vec<ShellInfo>, default_shell: Option<ShellInfo>) -> Self {
        Self {
            shells,
            default_shell,
            timestamp: SystemTime::now(),
            access_count: 0,
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        match self.timestamp.elapsed() {
            Ok(elapsed) => elapsed > ttl,
            Err(err) => {
                warn!("Shell cache timestamp is in the future: {}", err);
                true
            }
        }
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
            self.stats.default_shell = entry.default_shell.clone();
            self.stats.last_detection_time = match entry.timestamp.duration_since(UNIX_EPOCH) {
                Ok(duration) => Some(duration.as_secs()),
                Err(err) => {
                    warn!("Shell cache timestamp predates UNIX_EPOCH: {}", err);
                    None
                }
            };
        } else {
            drop(cache_guard);
            Self::get_cached_shells();
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
        match cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Shell cache mutex poisoned, recovering");
                poisoned.into_inner()
            }
        }
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
        let default_shell = Self::select_default_shell(&shells);

        // Update cache
        *cache_guard = Some(ShellCacheEntry::new(shells.clone(), default_shell));

        shells
    }

    pub fn get_cached_default_shell() -> Result<ShellInfo, String> {
        let cache = Self::get_cache();
        let mut cache_guard = Self::lock_cache(cache);

        let config = ConfigManager::config_get();
        if let Some(entry) = cache_guard.as_mut() {
            if !entry.is_expired(config.shell_cache_ttl()) {
                entry.access();
                return entry
                    .default_shell
                    .clone()
                    .ok_or_else(|| "No valid default shell detected in shell cache".to_string());
            }
        }

        // Cache expired or missing, re-detect
        drop(cache_guard);
        Self::get_cached_shells(); // This will update the cache

        // Re-acquire
        let cache_guard = Self::lock_cache(cache);
        match cache_guard.as_ref() {
            Some(entry) => entry.default_shell.clone().ok_or_else(|| {
                "No valid default shell detected after refreshing shell cache".to_string()
            }),
            None => Err("Shell cache unavailable after refresh".to_string()),
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
        match std::env::var("PATH") {
            Ok(path_env) => {
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
                                match shell_name.strip_suffix(".exe") {
                                    Some(base_name) => base_name,
                                    None => shell_name,
                                }
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
            Err(err) => {
                warn!(
                    "PATH environment variable unavailable during shell detection: {}",
                    err
                );
            }
        }

        shells
    }

    /// Get default shell (public interface, uses cache)
    pub fn terminal_get_default_shell() -> Result<ShellInfo, String> {
        Self::get_cached_default_shell()
    }

    fn select_default_shell(shells: &[ShellInfo]) -> Option<ShellInfo> {
        #[cfg(not(windows))]
        {
            match std::env::var("SHELL") {
                Ok(shell_path) => {
                    if Self::validate_shell(&shell_path) {
                        if let Some(shell_name) = std::path::Path::new(&shell_path).file_name() {
                            if let Some(name_str) = shell_name.to_str() {
                                let display_name = Self::get_shell_display_name(name_str);
                                return Some(ShellInfo::new(name_str, &shell_path, display_name));
                            }
                        }

                        warn!(
                            "SHELL path '{}' is valid but has no UTF-8 file name",
                            shell_path
                        );
                    } else {
                        warn!(
                            "SHELL environment variable points to invalid shell: {}",
                            shell_path
                        );
                    }
                }
                Err(err) => {
                    warn!(
                        "SHELL environment variable unavailable during shell detection: {}",
                        err
                    );
                }
            }
        }

        let preferred_names: &[&str] = if cfg!(windows) {
            &["bash", "zsh", "fish", "cmd"]
        } else {
            &["zsh", "bash", "fish", "sh"]
        };

        for name in preferred_names {
            if let Some(shell) = shells.iter().find(|shell| shell.name == *name) {
                return Some(shell.clone());
            }
        }

        if shells.is_empty() {
            warn!("No available shells detected on the system");
            None
        } else {
            Some(shells[0].clone())
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
}

impl Default for ShellManager {
    fn default() -> Self {
        Self::new()
    }
}
