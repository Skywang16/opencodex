/*!
 * Global preferences storage
 *
 * Used for persisting simple key-value pairs like project/user rules
 */

use crate::storage::database::DatabaseManager;
use crate::storage::error::RepositoryResult;
use sqlx::Row;
use std::collections::HashMap;

pub struct AppPreferences<'a> {
    db: &'a DatabaseManager,
}

impl<'a> AppPreferences<'a> {
    pub fn new(db: &'a DatabaseManager) -> Self {
        Self { db }
    }

    fn pool(&self) -> &sqlx::SqlitePool {
        self.db.pool()
    }

    /// Get value for specified key
    pub async fn get(&self, key: &str) -> RepositoryResult<Option<String>> {
        let row = sqlx::query("SELECT value FROM app_preferences WHERE key = ? LIMIT 1")
            .bind(key)
            .fetch_optional(self.pool())
            .await?;

        Ok(row
            .and_then(|r| r.try_get::<Option<String>, _>("value").ok())
            .flatten())
    }

    /// Batch get values for multiple keys
    pub async fn get_batch(&self, keys: &[&str]) -> RepositoryResult<HashMap<String, String>> {
        if keys.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT key, value FROM app_preferences WHERE key IN ({})",
            placeholders
        );
        let mut q = sqlx::query(&query);
        for key in keys {
            q = q.bind(*key);
        }
        let rows = q.fetch_all(self.pool()).await?;
        let mut result = HashMap::new();
        for row in rows {
            let k: String = row.try_get("key").unwrap_or_default();
            let v: Option<String> = row.try_get("value").unwrap_or(None);
            if let Some(v) = v {
                result.insert(k, v);
            }
        }
        Ok(result)
    }

    /// Set value for specified key; delete when value is None
    pub async fn set(&self, key: &str, value: Option<&str>) -> RepositoryResult<()> {
        match value {
            Some(v) => {
                sqlx::query(
                    r#"
                    INSERT INTO app_preferences (key, value, updated_at)
                    VALUES (?, ?, CURRENT_TIMESTAMP)
                    ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                    "#,
                )
                .bind(key)
                .bind(v)
                .execute(self.pool())
                .await?;
            }
            None => {
                sqlx::query("DELETE FROM app_preferences WHERE key = ?")
                    .bind(key)
                    .execute(self.pool())
                    .await?;
            }
        }

        Ok(())
    }
}
