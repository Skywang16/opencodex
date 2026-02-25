/*!
 * AI model data access
 */

use crate::storage::database::DatabaseManager;
use crate::storage::error::RepositoryResult;
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

/// AI Provider identifier (dynamic, from models.dev)
pub type AIProvider = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ModelType {
    #[serde(rename = "chat")]
    #[default]
    Chat,
    #[serde(rename = "embedding")]
    Embedding,
}

/// Authentication type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    #[default]
    ApiKey,
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

/// OAuth Provider type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProvider {
    OpenAiCodex,    // OpenAI ChatGPT Plus/Pro
    ClaudePro,      // Claude Pro subscription
    GeminiAdvanced, // Google Gemini Advanced
}

impl std::fmt::Display for OAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthProvider::OpenAiCodex => write!(f, "openai_codex"),
            OAuthProvider::ClaudePro => write!(f, "claude_pro"),
            OAuthProvider::GeminiAdvanced => write!(f, "gemini_advanced"),
        }
    }
}

impl std::str::FromStr for OAuthProvider {
    type Err = crate::storage::error::RepositoryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "openai_codex" => Ok(OAuthProvider::OpenAiCodex),
            "claude_pro" => Ok(OAuthProvider::ClaudePro),
            "gemini_advanced" => Ok(OAuthProvider::GeminiAdvanced),
            _ => Err(crate::storage::error::RepositoryError::Validation {
                reason: format!("Unknown OAuth provider: {s}"),
            }),
        }
    }
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthConfig {
    pub provider: OAuthProvider,
    pub refresh_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// Provider-specific data (OpenAI's account_id, Claude's organization_id, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIModelConfig {
    pub id: String,
    pub provider: AIProvider,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub model_type: ModelType,

    // Authentication configuration
    #[serde(default)]
    pub auth_type: AuthType,

    // API Key authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    // OAuth authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_config: Option<OAuthConfig>,

    // General configuration
    #[serde(default)]
    pub options: Option<Value>,
    #[serde(default)]
    pub use_custom_base_url: Option<bool>,
    #[serde(default = "default_timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_timestamp")]
    pub updated_at: DateTime<Utc>,
}

impl AIModelConfig {
    /// Create API Key authenticated model
    pub fn new(provider: AIProvider, api_url: String, api_key: String, model: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            provider,
            model: model.clone(),
            display_name: Some(model),
            model_type: ModelType::Chat,
            auth_type: AuthType::ApiKey,
            api_url: Some(api_url),
            api_key: Some(api_key),
            oauth_config: None,
            options: None,
            use_custom_base_url: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// AI model data access struct
pub struct AIModels<'a> {
    db: &'a DatabaseManager,
}

impl<'a> AIModels<'a> {
    pub fn new(db: &'a DatabaseManager) -> Self {
        Self { db }
    }

    /// Query all models (including decrypted keys)
    pub async fn find_all(&self) -> RepositoryResult<Vec<AIModelConfig>> {
        let rows = sqlx::query(
            r#"
            SELECT id, provider, api_url, api_key_encrypted, model_name, display_name, model_type,
                   config_json, use_custom_base_url, created_at, updated_at,
                   auth_type, oauth_provider, oauth_refresh_token_encrypted,
                   oauth_access_token_encrypted, oauth_token_expires_at, oauth_metadata
            FROM ai_models
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut models = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")?;
            let provider: String = row.try_get("provider")?;
            let model_type_str: String = row.try_get("model_type")?;
            let model_type = model_type_str.parse()?;

            let auth_type_str: String = row.try_get("auth_type")?;
            let auth_type = auth_type_str.parse()?;

            let options = row
                .try_get::<Option<String>, _>("config_json")?
                .and_then(|s| serde_json::from_str(&s).ok());

            let use_custom_base_url = row
                .try_get::<Option<i64>, _>("use_custom_base_url")?
                .map(|v| v != 0);

            // Decrypt API key
            let api_key = if let Some(encrypted_base64) =
                row.try_get::<Option<String>, _>("api_key_encrypted")?
            {
                if !encrypted_base64.is_empty() {
                    match BASE64.decode(&encrypted_base64) {
                        Ok(encrypted_bytes) => self
                            .db
                            .decrypt_data(&encrypted_bytes)
                            .await
                            .unwrap_or_else(|e| {
                                error!("Failed to decrypt API key ({}): {}", id, e);
                                String::new()
                            }),
                        Err(e) => {
                            error!("Base64 decode failed ({}): {}", id, e);
                            String::new()
                        }
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // Decrypt OAuth tokens
            let oauth_config = if auth_type == AuthType::OAuth {
                let provider_str: Option<String> = row.try_get("oauth_provider")?;
                if let Some(provider_str) = provider_str {
                    let oauth_provider: OAuthProvider = provider_str.parse()?;

                    // Decrypt refresh token
                    let refresh_token = if let Some(encrypted_base64) =
                        row.try_get::<Option<String>, _>("oauth_refresh_token_encrypted")?
                    {
                        if !encrypted_base64.is_empty() {
                            match BASE64.decode(&encrypted_base64) {
                                Ok(encrypted_bytes) => {
                                    self.db.decrypt_data(&encrypted_bytes).await.unwrap_or_else(
                                        |e| {
                                            error!(
                                                "Failed to decrypt OAuth refresh token ({}): {}",
                                                id, e
                                            );
                                            String::new()
                                        },
                                    )
                                }
                                Err(e) => {
                                    error!("Base64 decode failed ({}): {}", id, e);
                                    String::new()
                                }
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    // Decrypt access token
                    let access_token = if let Some(encrypted_base64) =
                        row.try_get::<Option<String>, _>("oauth_access_token_encrypted")?
                    {
                        if !encrypted_base64.is_empty() {
                            match BASE64.decode(&encrypted_base64) {
                                Ok(encrypted_bytes) => Some(
                                    self.db.decrypt_data(&encrypted_bytes).await.unwrap_or_else(
                                        |e| {
                                            error!(
                                                "Failed to decrypt OAuth access token ({}): {}",
                                                id, e
                                            );
                                            String::new()
                                        },
                                    ),
                                ),
                                Err(e) => {
                                    error!("Base64 decode failed ({}): {}", id, e);
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let expires_at = row.try_get("oauth_token_expires_at")?;
                    let metadata = row
                        .try_get::<Option<String>, _>("oauth_metadata")?
                        .and_then(|s| serde_json::from_str(&s).ok());

                    Some(OAuthConfig {
                        provider: oauth_provider,
                        refresh_token,
                        access_token,
                        expires_at,
                        metadata,
                    })
                } else {
                    None
                }
            } else {
                None
            };

            models.push(AIModelConfig {
                id,
                provider,
                auth_type,
                api_url: row.try_get("api_url")?,
                api_key: if api_key.is_empty() {
                    None
                } else {
                    Some(api_key)
                },
                model: row.try_get("model_name")?,
                display_name: row.try_get("display_name")?,
                model_type,
                oauth_config,
                options,
                use_custom_base_url,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(models)
    }

    /// Save model (automatically encrypt keys)
    ///
    /// UPSERT based on primary key (id):
    /// - If id doesn't exist, insert new record
    /// - If id exists, update existing record
    pub async fn save(&self, model: &AIModelConfig) -> RepositoryResult<()> {
        use crate::storage::error::RepositoryError;

        // Encrypt API key
        let encrypted_key = if let Some(api_key) = &model.api_key {
            if !api_key.is_empty() {
                let encrypted_bytes = self.db.encrypt_data(api_key).await?;
                Some(BASE64.encode(&encrypted_bytes))
            } else {
                None
            }
        } else {
            None
        };

        // Encrypt OAuth tokens
        let (
            oauth_provider,
            oauth_refresh_token_encrypted,
            oauth_access_token_encrypted,
            oauth_expires_at,
            oauth_metadata,
        ) = if let Some(oauth_config) = &model.oauth_config {
            let refresh_token_encrypted = if !oauth_config.refresh_token.is_empty() {
                let encrypted_bytes = self.db.encrypt_data(&oauth_config.refresh_token).await?;
                Some(BASE64.encode(&encrypted_bytes))
            } else {
                None
            };

            let access_token_encrypted = if let Some(access_token) = &oauth_config.access_token {
                if !access_token.is_empty() {
                    let encrypted_bytes = self.db.encrypt_data(access_token).await?;
                    Some(BASE64.encode(&encrypted_bytes))
                } else {
                    None
                }
            } else {
                None
            };

            let metadata_json = oauth_config
                .metadata
                .as_ref()
                .map(|m| serde_json::to_string(m).unwrap_or_default());

            (
                Some(oauth_config.provider.to_string()),
                refresh_token_encrypted,
                access_token_encrypted,
                oauth_config.expires_at,
                metadata_json,
            )
        } else {
            (None, None, None, None, None)
        };

        let config_json = model
            .options
            .as_ref()
            .map(|opts| serde_json::to_string(opts).unwrap_or_default());

        // UPSERT based on primary key (id)
        let result = sqlx::query(
            r#"
            INSERT INTO ai_models
            (id, provider, api_url, api_key_encrypted, model_name, display_name, model_type,
             config_json, use_custom_base_url, created_at, updated_at,
             auth_type, oauth_provider, oauth_refresh_token_encrypted,
             oauth_access_token_encrypted, oauth_token_expires_at, oauth_metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                api_url = excluded.api_url,
                api_key_encrypted = excluded.api_key_encrypted,
                model_name = excluded.model_name,
                display_name = excluded.display_name,
                model_type = excluded.model_type,
                config_json = excluded.config_json,
                use_custom_base_url = excluded.use_custom_base_url,
                auth_type = excluded.auth_type,
                oauth_provider = excluded.oauth_provider,
                oauth_refresh_token_encrypted = excluded.oauth_refresh_token_encrypted,
                oauth_access_token_encrypted = excluded.oauth_access_token_encrypted,
                oauth_token_expires_at = excluded.oauth_token_expires_at,
                oauth_metadata = excluded.oauth_metadata,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&model.id)
        .bind(model.provider.to_string())
        .bind(&model.api_url)
        .bind(encrypted_key)
        .bind(&model.model)
        .bind(&model.display_name)
        .bind(model.model_type.to_string())
        .bind(config_json)
        .bind(model.use_custom_base_url.map(|v| v as i64))
        .bind(model.created_at)
        .bind(model.updated_at)
        .bind(model.auth_type.to_string())
        .bind(oauth_provider)
        .bind(oauth_refresh_token_encrypted)
        .bind(oauth_access_token_encrypted)
        .bind(oauth_expires_at)
        .bind(oauth_metadata)
        .execute(self.db.pool())
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(RepositoryError::AiModelAlreadyExists {
                    provider: model.provider.to_string(),
                    model: model.model.clone(),
                })
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Find model by ID
    pub async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<AIModelConfig>> {
        let models = self.find_all().await?;
        Ok(models.into_iter().find(|m| m.id == id))
    }

    /// Delete model
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
