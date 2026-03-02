/*!
 * AI model configuration data access — unified single-table design.
 * Each row is a complete model config: auth + model selection + metadata.
 */

use crate::storage::database::DatabaseManager;
use crate::storage::error::{RepositoryError, RepositoryResult};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use tracing::error;

fn default_timestamp() -> DateTime<Utc> {
    Utc::now()
}

/// AI Provider identifier (matches models.dev provider id)
pub type AIProvider = String;

/// Authentication type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    #[default]
    ApiKey,
    #[serde(rename = "oauth")]
    OAuth,
}

impl std::fmt::Display for AuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthType::ApiKey => write!(f, "api_key"),
            AuthType::OAuth => write!(f, "oauth"),
        }
    }
}

impl std::str::FromStr for AuthType {
    type Err = crate::storage::error::RepositoryError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "api_key" => Ok(AuthType::ApiKey),
            "oauth" => Ok(AuthType::OAuth),
            _ => Err(crate::storage::error::RepositoryError::Validation {
                reason: format!("Unknown auth type: {s}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ModelType {
    #[serde(rename = "chat")]
    #[default]
    Chat,
    #[serde(rename = "embedding")]
    Embedding,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::Chat => write!(f, "chat"),
            ModelType::Embedding => write!(f, "embedding"),
        }
    }
}

impl std::str::FromStr for ModelType {
    type Err = crate::storage::error::RepositoryError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chat" => Ok(ModelType::Chat),
            "embedding" => Ok(ModelType::Embedding),
            _ => Err(crate::storage::error::RepositoryError::Validation {
                reason: format!("Unknown model type: {s}"),
            }),
        }
    }
}

/// A complete model configuration entry: auth + model + metadata in one row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIModelConfig {
    pub id: String,
    // ── provider & auth ─────────────────────────────────────────────
    pub provider_id: String,
    pub auth_type: AuthType,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_metadata: Option<Value>,
    // ── model selection ─────────────────────────────────────────────
    pub model: String,
    #[serde(default)]
    pub model_type: ModelType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
    // ── models.dev metadata ─────────────────────────────────────────
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output: Option<u32>,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub tool_call: bool,
    #[serde(default)]
    pub attachment: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<Value>,

    #[serde(default = "default_timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_timestamp")]
    pub updated_at: DateTime<Utc>,
}

/// AI model data access struct
pub struct AIModels<'a> {
    db: &'a DatabaseManager,
}

const SELECT_COLS: &str = r#"
    id, provider_id, auth_type, display_name, api_url,
    api_key_encrypted, oauth_refresh_token_encrypted,
    oauth_access_token_encrypted, oauth_expires_at, oauth_metadata,
    model_name, model_type, options_json,
    context_window, max_output, reasoning, tool_call, attachment, cost_json,
    created_at, updated_at
"#;

impl<'a> AIModels<'a> {
    pub fn new(db: &'a DatabaseManager) -> Self {
        Self { db }
    }

    pub async fn find_all(&self) -> RepositoryResult<Vec<AIModelConfig>> {
        let sql = format!("SELECT {SELECT_COLS} FROM ai_models ORDER BY created_at ASC");
        let rows = sqlx::query(&sql).fetch_all(self.db.pool()).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.row_to_model(row).await?);
        }
        Ok(out)
    }

    pub async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<AIModelConfig>> {
        let sql = format!("SELECT {SELECT_COLS} FROM ai_models WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(self.db.pool())
            .await?;
        match row {
            Some(r) => Ok(Some(self.row_to_model(r).await?)),
            None => Ok(None),
        }
    }

    pub async fn find_by_model_type(
        &self,
        model_type: &ModelType,
    ) -> RepositoryResult<Vec<AIModelConfig>> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM ai_models WHERE model_type = ? ORDER BY created_at ASC"
        );
        let rows = sqlx::query(&sql)
            .bind(model_type.to_string())
            .fetch_all(self.db.pool())
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.row_to_model(row).await?);
        }
        Ok(out)
    }

    async fn row_to_model(&self, row: sqlx::sqlite::SqliteRow) -> RepositoryResult<AIModelConfig> {
        let id: String = row.try_get("id")?;
        let auth_type: AuthType = row.try_get::<String, _>("auth_type")?.parse()?;
        let model_type: ModelType = row.try_get::<String, _>("model_type")?.parse()?;

        let api_key = self
            .decrypt_optional(&row, "api_key_encrypted", &id, "api_key")
            .await;
        let oauth_refresh_token = self
            .decrypt_optional(&row, "oauth_refresh_token_encrypted", &id, "refresh_token")
            .await;
        let oauth_access_token = self
            .decrypt_optional(&row, "oauth_access_token_encrypted", &id, "access_token")
            .await;

        let oauth_metadata = row
            .try_get::<Option<String>, _>("oauth_metadata")?
            .and_then(|s| serde_json::from_str(&s).ok());
        let options = row
            .try_get::<Option<String>, _>("options_json")?
            .and_then(|s| serde_json::from_str(&s).ok());
        let cost = row
            .try_get::<Option<String>, _>("cost_json")
            .unwrap_or(None)
            .and_then(|s| serde_json::from_str(&s).ok());

        Ok(AIModelConfig {
            id,
            provider_id: row.try_get("provider_id")?,
            auth_type,
            display_name: row.try_get("display_name")?,
            api_url: row.try_get("api_url")?,
            api_key,
            oauth_refresh_token,
            oauth_access_token,
            oauth_expires_at: row.try_get("oauth_expires_at")?,
            oauth_metadata,
            model: row.try_get("model_name")?,
            model_type,
            options,
            context_window: row
                .try_get::<Option<i32>, _>("context_window")
                .unwrap_or(None)
                .map(|v| v as u32),
            max_output: row
                .try_get::<Option<i32>, _>("max_output")
                .unwrap_or(None)
                .map(|v| v as u32),
            reasoning: row.try_get::<bool, _>("reasoning").unwrap_or(false),
            tool_call: row.try_get::<bool, _>("tool_call").unwrap_or(false),
            attachment: row.try_get::<bool, _>("attachment").unwrap_or(false),
            cost,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    async fn decrypt_optional(
        &self,
        row: &sqlx::sqlite::SqliteRow,
        col: &str,
        id: &str,
        label: &str,
    ) -> Option<String> {
        let encrypted_b64: Option<String> = row.try_get(col).ok()?;
        let b64 = encrypted_b64.filter(|s| !s.is_empty())?;
        match BASE64.decode(&b64) {
            Ok(bytes) => match self.db.decrypt_data(&bytes).await {
                Ok(s) if !s.is_empty() => Some(s),
                Ok(_) => None,
                Err(e) => {
                    error!(
                        "Decryption failed for {} (model {}): {}. Data may be corrupted or key rotated.",
                        label, id, e
                    );
                    None
                }
            },
            Err(e) => {
                error!("Base64 decode failed for {} (model {}): {}", label, id, e);
                None
            }
        }
    }

    async fn encrypt_optional(&self, value: Option<&str>) -> RepositoryResult<Option<String>> {
        match value {
            Some(v) if !v.is_empty() => {
                let bytes = self.db.encrypt_data(v).await?;
                Ok(Some(BASE64.encode(&bytes)))
            }
            _ => Ok(None),
        }
    }

    pub async fn save(&self, model: &AIModelConfig) -> RepositoryResult<()> {
        let api_key_enc = self.encrypt_optional(model.api_key.as_deref()).await?;
        let refresh_enc = self
            .encrypt_optional(model.oauth_refresh_token.as_deref())
            .await?;
        let access_enc = self
            .encrypt_optional(model.oauth_access_token.as_deref())
            .await?;
        let metadata_json = model
            .oauth_metadata
            .as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()
            .map_err(|e| RepositoryError::internal(format!("Failed to serialize oauth_metadata: {e}")))?;
        let options_json = model
            .options
            .as_ref()
            .map(|o| serde_json::to_string(o))
            .transpose()
            .map_err(|e| RepositoryError::internal(format!("Failed to serialize options: {e}")))?;
        let cost_json = model
            .cost
            .as_ref()
            .map(|c| serde_json::to_string(c))
            .transpose()
            .map_err(|e| RepositoryError::internal(format!("Failed to serialize cost: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO ai_models
              (id, provider_id, auth_type, display_name, api_url,
               api_key_encrypted, oauth_refresh_token_encrypted,
               oauth_access_token_encrypted, oauth_expires_at, oauth_metadata,
               model_name, model_type, options_json,
               context_window, max_output, reasoning, tool_call, attachment, cost_json,
               created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              provider_id   = excluded.provider_id,
              auth_type     = excluded.auth_type,
              display_name  = excluded.display_name,
              api_url       = excluded.api_url,
              api_key_encrypted = excluded.api_key_encrypted,
              oauth_refresh_token_encrypted = excluded.oauth_refresh_token_encrypted,
              oauth_access_token_encrypted  = excluded.oauth_access_token_encrypted,
              oauth_expires_at  = excluded.oauth_expires_at,
              oauth_metadata    = excluded.oauth_metadata,
              model_name    = excluded.model_name,
              model_type    = excluded.model_type,
              options_json  = excluded.options_json,
              context_window = excluded.context_window,
              max_output    = excluded.max_output,
              reasoning     = excluded.reasoning,
              tool_call     = excluded.tool_call,
              attachment    = excluded.attachment,
              cost_json     = excluded.cost_json,
              updated_at    = excluded.updated_at
            "#,
        )
        .bind(&model.id)
        .bind(&model.provider_id)
        .bind(model.auth_type.to_string())
        .bind(&model.display_name)
        .bind(&model.api_url)
        .bind(api_key_enc)
        .bind(refresh_enc)
        .bind(access_enc)
        .bind(model.oauth_expires_at)
        .bind(metadata_json)
        .bind(&model.model)
        .bind(model.model_type.to_string())
        .bind(options_json)
        .bind(model.context_window.map(|v| v as i32))
        .bind(model.max_output.map(|v| v as i32))
        .bind(model.reasoning)
        .bind(model.tool_call)
        .bind(model.attachment)
        .bind(cost_json)
        .bind(model.created_at)
        .bind(model.updated_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM ai_models WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(crate::storage::error::RepositoryError::AiModelNotFound {
                id: id.to_string(),
            });
        }
        Ok(())
    }
}
