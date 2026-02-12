/*!
 * Storage path management module
 *
 * Provides unified path management functionality, including config paths, state paths, data paths, etc.
 * Supports cross-platform path handling and path validation
 */

use crate::storage::error::{StoragePathsError, StoragePathsResult};
use std::fs;
use std::path::{Path, PathBuf};

/// Storage path manager
#[derive(Debug, Clone)]
pub struct StoragePaths {
    /// Application root directory
    pub app_dir: PathBuf,
    /// Config directory
    pub config_dir: PathBuf,
    /// State directory
    pub state_dir: PathBuf,
    /// Data directory
    pub data_dir: PathBuf,

    /// Backups directory
    pub backups_dir: PathBuf,
    /// Logs directory
    pub logs_dir: PathBuf,
}

impl StoragePaths {
    /// Create a new path manager
    pub fn new(app_dir: PathBuf) -> StoragePathsResult<Self> {
        let config_dir = app_dir.join(super::CONFIG_DIR_NAME);
        let state_dir = app_dir.join(super::STATE_DIR_NAME);
        let data_dir = app_dir.join(super::DATA_DIR_NAME);

        let backups_dir = app_dir.join(super::BACKUPS_DIR_NAME);
        let logs_dir = app_dir.join("logs");

        let paths = Self {
            app_dir,
            config_dir,
            state_dir,
            data_dir,

            backups_dir,
            logs_dir,
        };

        // Validate paths
        paths.validate()?;

        Ok(paths)
    }

    /// Get database file path
    pub fn database_file(&self) -> PathBuf {
        self.data_dir.join(super::DATABASE_FILE_NAME)
    }

    /// Get backup file path
    pub fn backup_file(&self, filename: &str) -> PathBuf {
        self.backups_dir.join(filename)
    }

    /// Get log file path
    pub fn log_file(&self, filename: &str) -> PathBuf {
        self.logs_dir.join(filename)
    }

    /// Ensure all directories exist
    pub fn ensure_directories(&self) -> StoragePathsResult<()> {
        let directories = [
            &self.app_dir,
            &self.config_dir,
            &self.state_dir,
            &self.data_dir,
            &self.backups_dir,
            &self.logs_dir,
        ];

        for dir in &directories {
            if !dir.exists() {
                fs::create_dir_all(dir)
                    .map_err(|e| StoragePathsError::directory_create(dir.to_path_buf(), e))?;
                // Directory created successfully
            }
        }

        Ok(())
    }

    /// Validate path validity
    pub fn validate(&self) -> StoragePathsResult<()> {
        if !self.app_dir.exists() {
            fs::create_dir_all(&self.app_dir)
                .map_err(|e| StoragePathsError::directory_create(self.app_dir.clone(), e))?;
        }

        if let Err(e) = fs::metadata(&self.app_dir) {
            return Err(StoragePathsError::directory_access(self.app_dir.clone(), e));
        }

        Ok(())
    }

    /// Get directory size (in bytes)
    pub fn get_directory_size(&self, dir: &Path) -> StoragePathsResult<u64> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;

        fn visit_dir(dir: &Path, total_size: &mut u64) -> std::io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    visit_dir(&path, total_size)?;
                } else {
                    let metadata = entry.metadata()?;
                    *total_size += metadata.len();
                }
            }
            Ok(())
        }

        visit_dir(dir, &mut total_size)
            .map_err(|e| StoragePathsError::directory_size(dir.to_path_buf(), e))?;

        Ok(total_size)
    }
}

/// Storage path builder
pub struct StoragePathsBuilder {
    app_dir: Option<PathBuf>,
    custom_config_dir: Option<PathBuf>,
    custom_state_dir: Option<PathBuf>,
    custom_data_dir: Option<PathBuf>,
}

impl StoragePathsBuilder {
    pub fn new() -> Self {
        Self {
            app_dir: None,
            custom_config_dir: None,
            custom_state_dir: None,
            custom_data_dir: None,
        }
    }

    pub fn app_dir(mut self, dir: PathBuf) -> Self {
        self.app_dir = Some(dir);
        self
    }

    pub fn config_dir(mut self, dir: PathBuf) -> Self {
        self.custom_config_dir = Some(dir);
        self
    }

    pub fn state_dir(mut self, dir: PathBuf) -> Self {
        self.custom_state_dir = Some(dir);
        self
    }

    pub fn data_dir(mut self, dir: PathBuf) -> Self {
        self.custom_data_dir = Some(dir);
        self
    }

    pub fn build(self) -> StoragePathsResult<StoragePaths> {
        let Some(app_dir) = self.app_dir else {
            return Err(StoragePathsError::AppDirectoryMissing);
        };

        let config_dir = self
            .custom_config_dir
            .unwrap_or_else(|| app_dir.join(super::CONFIG_DIR_NAME));
        let state_dir = self
            .custom_state_dir
            .unwrap_or_else(|| app_dir.join(super::STATE_DIR_NAME));
        let data_dir = self
            .custom_data_dir
            .unwrap_or_else(|| app_dir.join(super::DATA_DIR_NAME));

        let backups_dir = app_dir.join(super::BACKUPS_DIR_NAME);
        let logs_dir = app_dir.join("logs");

        let paths = StoragePaths {
            app_dir,
            config_dir,
            state_dir,
            data_dir,

            backups_dir,
            logs_dir,
        };

        paths.validate()?;
        Ok(paths)
    }
}

impl Default for StoragePathsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
