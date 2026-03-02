use crate::storage::error::{DatabaseError, DatabaseResult};
use crate::storage::paths::StoragePaths;
use crate::storage::sql_scripts::{SqlScript, SqlScriptCatalog};
use crate::storage::DATABASE_FILE_NAME;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::ChaCha20Poly1305;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::{ConnectOptions, Executor};
use std::fmt;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing;

const KEY_FILE_NAME: &str = "master.key";
const KEY_FILE_VERSION: &str = "v1";
const NONCE_LEN: usize = 12;

#[derive(Debug, Clone)]
pub enum PoolSize {
    Fixed(NonZeroU32),
    Adaptive { min: NonZeroU32, max: NonZeroU32 },
}

impl PoolSize {
    fn resolve(&self) -> (NonZeroU32, NonZeroU32) {
        match self {
            PoolSize::Fixed(size) => (*size, *size),
            PoolSize::Adaptive { min, max } => {
                let cpu = std::thread::available_parallelism()
                    .map(|n| n.get() as u32)
                    .unwrap_or(4);
                let suggested = (cpu * 2).clamp(min.get(), max.get());
                (*min, NonZeroU32::new(suggested).unwrap())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseOptions {
    pub encryption: bool,
    pub pool_size: PoolSize,
    pub connection_timeout: Duration,
    pub statement_timeout: Duration,
    pub wal: bool,
    pub sql_dir: Option<PathBuf>,
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            encryption: true,
            pool_size: PoolSize::Adaptive {
                min: NonZeroU32::new(4).unwrap(),
                max: NonZeroU32::new(32).unwrap(),
            },
            connection_timeout: Duration::from_secs(10),
            statement_timeout: Duration::from_secs(30),
            wal: true,
            sql_dir: None,
        }
    }
}

pub struct DatabaseManager {
    pool: SqlitePool,
    paths: StoragePaths,
    options: DatabaseOptions,
    scripts: Arc<[SqlScript]>,
    key_vault: Arc<KeyVault>,
}

impl fmt::Debug for DatabaseManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseManager")
            .field("paths", &self.paths)
            .field("options", &self.options)
            .field("script_count", &self.scripts.len())
            .finish()
    }
}

impl DatabaseManager {
    pub async fn new(paths: StoragePaths, options: DatabaseOptions) -> DatabaseResult<Self> {
        let db_path = paths.data_dir.join(DATABASE_FILE_NAME);
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|err| {
                DatabaseError::io(
                    format!("create database directory {}", parent.display()),
                    err,
                )
            })?;
        }

        let (min_conn, max_conn) = options.pool_size.resolve();

        let connect_options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(if options.wal {
                SqliteJournalMode::Wal
            } else {
                SqliteJournalMode::Delete
            })
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(options.statement_timeout)
            .disable_statement_logging();

        let pool = SqlitePoolOptions::new()
            .min_connections(min_conn.get())
            .max_connections(max_conn.get())
            .acquire_timeout(options.connection_timeout)
            .idle_timeout(Some(Duration::from_secs(30)))
            .max_lifetime(Some(Duration::from_secs(60 * 15)))
            .connect_with(connect_options)
            .await
            .map_err(|err| {
                DatabaseError::internal(format!(
                    "Failed to connect SQLite: {} ({err})",
                    db_path.display()
                ))
            })?;

        let sql_dir = resolve_sql_dir(&options);
        let scripts = SqlScriptCatalog::load(sql_dir)
            .await
            .map_err(DatabaseError::from)?
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .into();

        let key_vault = Arc::new(KeyVault::new(paths.config_dir.join(KEY_FILE_NAME)));

        Ok(Self {
            pool,
            paths,
            options,
            scripts,
            key_vault,
        })
    }

    pub async fn initialize(&self) -> DatabaseResult<()> {
        if self.options.encryption {
            self.key_vault.master_key().await?;
        }

        self.pool
            .execute("PRAGMA foreign_keys = ON")
            .await
            .map_err(|err| {
                DatabaseError::internal(format!("Failed to enable foreign_keys pragma: {err}"))
            })?;

        if self.options.encryption {
            self.pool
                .execute("PRAGMA secure_delete = ON")
                .await
                .map_err(|err| {
                    DatabaseError::internal(format!("Failed to enable secure_delete pragma: {err}"))
                })?;
        }

        self.execute_sql_scripts().await?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn encrypt_data(&self, data: &str) -> DatabaseResult<Vec<u8>> {
        if !self.options.encryption {
            return Err(DatabaseError::EncryptionNotEnabled);
        }
        let key_bytes = self.key_vault.master_key().await?;
        let cipher = ChaCha20Poly1305::new(key_bytes.as_ref().into());
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = nonce_bytes.as_ref().into();
        let ciphertext = cipher
            .encrypt(nonce, data.as_bytes())
            .map_err(DatabaseError::from)?;
        let mut result = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub async fn decrypt_data(&self, encrypted: &[u8]) -> DatabaseResult<String> {
        if !self.options.encryption {
            return Err(DatabaseError::EncryptionNotEnabled);
        }
        if encrypted.len() <= NONCE_LEN {
            return Err(DatabaseError::InvalidEncryptedData);
        }
        let key_bytes = self.key_vault.master_key().await?;
        let cipher = ChaCha20Poly1305::new(key_bytes.as_ref().into());
        let (nonce_bytes, payload) = encrypted.split_at(NONCE_LEN);
        let nonce = nonce_bytes.into();
        let plaintext = cipher
            .decrypt(nonce, payload)
            .map_err(DatabaseError::from)?;
        String::from_utf8(plaintext).map_err(DatabaseError::from)
    }

    async fn execute_sql_scripts(&self) -> DatabaseResult<()> {
        if self.scripts.is_empty() {
            tracing::warn!("No SQL scripts found for database initialization");
            return Ok(());
        }

        tracing::info!("Executing {} SQL scripts", self.scripts.len());
        for script in self.scripts.iter() {
            tracing::info!("Executing script: {}", script.name);
            for statement in script.statements.iter() {
                if statement.trim().is_empty() {
                    continue;
                }
                sqlx::query(statement)
                    .execute(&self.pool)
                    .await
                    .map_err(|err| {
                        DatabaseError::internal(format!(
                            "Failed to execute SQL statement in script {}: {err}\nStatement: {statement}",
                            script.name
                        ))
                    })?;
            }
        }

        tracing::info!("All SQL scripts executed successfully");
        Ok(())
    }
}

struct KeyVault {
    path: PathBuf,
    key: OnceLock<[u8; 32]>,
}

impl KeyVault {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            key: OnceLock::new(),
        }
    }

    async fn master_key(&self) -> DatabaseResult<[u8; 32]> {
        if let Some(&key) = self.key.get() {
            return Ok(key);
        }

        let key = match self.load_from_disk().await {
            Ok(Some(k)) => k,
            _ => self.derive_from_device().await?,
        };

        let _ = self.key.set(key);
        Ok(key)
    }

    async fn load_from_disk(&self) -> DatabaseResult<Option<[u8; 32]>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let raw = tokio::fs::read_to_string(&self.path).await.map_err(|err| {
            DatabaseError::io(format!("read key file {}", self.path.display()), err)
        })?;
        let mut lines = raw.lines();
        let first = lines.next().unwrap_or_default();
        let encoded = if first == KEY_FILE_VERSION {
            lines.next().unwrap_or_default()
        } else {
            first
        };
        if encoded.is_empty() {
            return Ok(None);
        }
        let decoded = BASE64.decode(encoded).map_err(DatabaseError::from)?;
        if decoded.len() != 32 {
            return Err(DatabaseError::InvalidKeyLength);
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(Some(bytes))
    }

    async fn derive_from_device(&self) -> DatabaseResult<[u8; 32]> {
        let device_id = self.get_device_identifier()?;

        // PBKDF2-like iterated hashing: 100k rounds of HMAC(SHA-256)
        const ITERATIONS: u32 = 100_000;
        let salt = b"opencodex-secret-v1";

        let mut key = {
            let mut h = Sha256::new();
            h.update(device_id.as_bytes());
            h.update(salt);
            h.finalize()
        };

        for _ in 1..ITERATIONS {
            let mut h = Sha256::new();
            h.update(key);
            h.update(device_id.as_bytes());
            key = h.finalize();
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&key);

        self.persist(bytes).await?;
        Ok(bytes)
    }

    fn get_device_identifier(&self) -> DatabaseResult<String> {
        // Use machine-uid crate to get cross-platform machine unique identifier
        // Windows: Read from registry HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Cryptography\MachineGuid
        // macOS: Use ioreg to get IOPlatformUUID
        // Linux: Read /var/lib/dbus/machine-id or /etc/machine-id
        machine_uid::get().map_err(|e| {
            DatabaseError::internal(format!(
                "Failed to get machine UID for key derivation: {e}"
            ))
        })
    }

    async fn persist(&self, bytes: [u8; 32]) -> DatabaseResult<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|err| {
                DatabaseError::io(format!("create key directory {}", parent.display()), err)
            })?;
        }
        let encoded = BASE64.encode(bytes);
        let payload = format!("{KEY_FILE_VERSION}\n{encoded}\n");
        let tmp_path = self.path.with_extension("tmp");
        tokio::fs::write(&tmp_path, payload.as_bytes())
            .await
            .map_err(|err| {
                DatabaseError::io(format!("write key temp file {}", tmp_path.display()), err)
            })?;
        tokio::fs::rename(&tmp_path, &self.path)
            .await
            .map_err(|err| {
                DatabaseError::io(format!("replace key file {}", self.path.display()), err)
            })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&self.path)
                .await
                .map_err(|err| DatabaseError::io("read key file metadata", err))?
                .permissions();
            perms.set_mode(0o600);
            tokio::fs::set_permissions(&self.path, perms)
                .await
                .map_err(|err| DatabaseError::io("set key file permissions", err))?;
        }

        Ok(())
    }
}

fn resolve_sql_dir(options: &DatabaseOptions) -> PathBuf {
    if let Some(custom) = &options.sql_dir {
        return custom.clone();
    }

    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sql")
    } else {
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(err) => {
                tracing::warn!("Failed to get current executable path: {err}");
                PathBuf::from(".")
            }
        };
        if let Some(contents) = exe
            .ancestors()
            .find(|p| p.file_name() == Some(std::ffi::OsStr::new("Contents")))
        {
            contents.join("Resources/sql")
        } else {
            exe.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("sql")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn key_vault_generates_and_persists() {
        let temp_dir = TempDir::new().unwrap();
        let vault = KeyVault::new(temp_dir.path().join(KEY_FILE_NAME));
        let key1 = vault.master_key().await.unwrap();
        let key2 = vault.master_key().await.unwrap();
        assert_eq!(key1, key2);
    }

    #[tokio::test]
    async fn encryption_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let paths = crate::storage::paths::StoragePathsBuilder::new()
            .app_dir(temp_dir.path().to_path_buf())
            .build()
            .unwrap();
        paths.ensure_directories().unwrap();

        let manager = DatabaseManager::new(paths.clone(), DatabaseOptions::default())
            .await
            .unwrap();
        manager.initialize().await.unwrap();

        let encrypted = manager.encrypt_data("hello world").await.unwrap();
        let decrypted = manager.decrypt_data(&encrypted).await.unwrap();
        assert_eq!(decrypted, "hello world");
    }
}
