//! BlobStore: content-addressable storage
//!
//! Uses SHA-256 hash as content identifier for deduplication storage
//! Supports streaming processing of large files

use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::time::{SystemTime, UNIX_EPOCH};

use super::config::CheckpointConfig;
use super::models::CheckpointResult;

/// Content-addressable storage
pub struct BlobStore {
    pool: SqlitePool,
    config: CheckpointConfig,
}

impl BlobStore {
    pub fn new(pool: SqlitePool, config: CheckpointConfig) -> Self {
        Self { pool, config }
    }

    /// Compute SHA-256 hash of content
    pub fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Store content, return SHA-256 hash
    /// If content already exists, increment reference count
    pub async fn store(&self, content: &[u8]) -> CheckpointResult<String> {
        // Check file size limit
        if self.config.is_file_too_large(content.len() as u64) {
            return Err(super::models::CheckpointError::FileTooLarge(
                content.len() as u64
            ));
        }

        let hash = Self::compute_hash(content);

        // Check if already exists
        if self.exists(&hash).await? {
            self.increment_ref(&hash).await?;
            return Ok(hash);
        }

        let size = content.len() as i64;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Insert new blob
        let result = sqlx::query(
            r#"
            INSERT INTO checkpoint_blobs (hash, content, size, ref_count, created_at)
            VALUES (?, ?, ?, 1, ?)
            "#,
        )
        .bind(&hash)
        .bind(content)
        .bind(size)
        .bind(now)
        .execute(&self.pool)
        .await?;

        tracing::debug!(
            "BlobStore: stored blob hash={}, size={}, rows_affected={}",
            hash,
            size,
            result.rows_affected()
        );

        Ok(hash)
    }

    /// Get content by hash
    pub async fn get(&self, hash: &str) -> CheckpointResult<Option<Vec<u8>>> {
        let row = sqlx::query("SELECT content FROM checkpoint_blobs WHERE hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get("content")))
    }

    /// Check if hash exists
    pub async fn exists(&self, hash: &str) -> CheckpointResult<bool> {
        let row = sqlx::query("SELECT 1 FROM checkpoint_blobs WHERE hash = ?")
            .bind(hash)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.is_some())
    }

    /// Decrement reference count
    pub async fn decrement_ref(&self, hash: &str) -> CheckpointResult<()> {
        sqlx::query(
            r#"
            UPDATE checkpoint_blobs
            SET ref_count = ref_count - 1
            WHERE hash = ? AND ref_count > 0
            "#,
        )
        .bind(hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Increment reference count
    pub async fn increment_ref(&self, hash: &str) -> CheckpointResult<()> {
        sqlx::query("UPDATE checkpoint_blobs SET ref_count = ref_count + 1 WHERE hash = ?")
            .bind(hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Garbage collection: clean up blobs with reference count 0
    pub async fn gc(&self) -> CheckpointResult<u64> {
        let result = sqlx::query("DELETE FROM checkpoint_blobs WHERE ref_count <= 0")
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            tracing::info!("BlobStore GC: deleted {} orphaned blobs", deleted);
        }

        Ok(deleted)
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> CheckpointResult<BlobStoreStats> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as blob_count,
                SUM(size) as total_size,
                SUM(ref_count) as total_refs,
                COUNT(CASE WHEN ref_count = 0 THEN 1 END) as orphaned_count
            FROM checkpoint_blobs
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(BlobStoreStats {
            blob_count: row.get("blob_count"),
            total_size: row.get("total_size"),
            total_refs: row.get("total_refs"),
            orphaned_count: row.get("orphaned_count"),
        })
    }
}

/// BlobStore statistics
#[derive(Debug, Clone)]
pub struct BlobStoreStats {
    pub blob_count: i64,
    pub total_size: i64,
    pub total_refs: i64,
    pub orphaned_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::CheckpointError;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE checkpoint_blobs (
                hash TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                size INTEGER NOT NULL,
                ref_count INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_store_and_get() {
        let pool = setup_test_db().await;
        let config = CheckpointConfig::default();
        let store = BlobStore::new(pool, config);

        let content = b"Hello, World!";
        let hash = store.store(content).await.unwrap();

        let retrieved = store.get(&hash).await.unwrap().unwrap();
        assert_eq!(content, retrieved.as_slice());
    }

    #[tokio::test]
    async fn test_deduplication() {
        let pool = setup_test_db().await;
        let config = CheckpointConfig::default();
        let store = BlobStore::new(pool, config);

        let content = b"Hello, World!";
        let hash1 = store.store(content).await.unwrap();
        let hash2 = store.store(content).await.unwrap();

        assert_eq!(hash1, hash2);

        let stats = store.get_stats().await.unwrap();
        assert_eq!(stats.blob_count, 1);
        assert_eq!(stats.total_refs, 2);
    }

    #[tokio::test]
    async fn test_file_size_limit() {
        let pool = setup_test_db().await;
        let config = CheckpointConfig {
            max_file_size: 10, // 10 bytes limit
            ..Default::default()
        };
        let store = BlobStore::new(pool, config);

        let large_content = vec![0u8; 20]; // 20 bytes
        let result = store.store(&large_content).await;

        assert!(matches!(result, Err(CheckpointError::FileTooLarge(_))));
    }
}
