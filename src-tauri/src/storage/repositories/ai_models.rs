/*!
 * AI model data access backed by a JSON config file.
 */

use crate::storage::database::DatabaseManager;
use crate::storage::error::{RepositoryError, RepositoryResult};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::Mutex;

static MODELS_FILE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const MODELS_FILE_NAME: &str = "models.json";
const MODELS_FILE_VERSION: u32 = 1;

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
    OpenAiCodex,
    ClaudePro,
    GeminiAdvanced,
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
    #[serde(default)]
    pub auth_type: AuthType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_config: Option<OAuthConfig>,
    #[serde(default)]
    pub options: Option<Value>,
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
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AIModelsConfig {
    #[serde(default = "default_file_version")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "ModelDefaults::is_empty")]
    pub defaults: ModelDefaults,
    #[serde(default, skip_serializing_if = "AgentModelConfig::is_empty")]
    pub agents: AgentModelConfig,
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderModels>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelDefaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model_id: Option<String>,
}

impl ModelDefaults {
    fn is_empty(&self) -> bool {
        self.chat_model_id.is_none() && self.embedding_model_id.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelConfig {
    #[serde(default, skip_serializing_if = "AgentModelDefaults::is_empty")]
    pub defaults: AgentModelDefaults,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bindings: BTreeMap<String, String>,
}

impl AgentModelConfig {
    fn is_empty(&self) -> bool {
        self.defaults.is_empty() && self.bindings.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelDefaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent: Option<String>,
}

impl AgentModelDefaults {
    fn is_empty(&self) -> bool {
        self.subagent.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModels {
    #[serde(default)]
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub id: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub model_type: ModelType,
    #[serde(default)]
    pub auth_type: AuthType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_config: Option<OAuthConfig>,
    #[serde(default)]
    pub options: Option<Value>,
    #[serde(default = "default_timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_timestamp")]
    pub updated_at: DateTime<Utc>,
}

fn default_file_version() -> u32 {
    MODELS_FILE_VERSION
}

impl Default for ModelEntry {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: String::new(),
            model: String::new(),
            display_name: None,
            model_type: ModelType::default(),
            auth_type: AuthType::default(),
            api_url: None,
            api_key: None,
            oauth_config: None,
            options: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl ModelEntry {
    fn into_config(self, provider: String) -> AIModelConfig {
        AIModelConfig {
            id: self.id,
            provider,
            model: self.model,
            display_name: self.display_name,
            model_type: self.model_type,
            auth_type: self.auth_type,
            api_url: self.api_url,
            api_key: self.api_key,
            oauth_config: self.oauth_config,
            options: self.options,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl From<&AIModelConfig> for ModelEntry {
    fn from(model: &AIModelConfig) -> Self {
        Self {
            id: model.id.clone(),
            model: model.model.clone(),
            display_name: model.display_name.clone(),
            model_type: model.model_type.clone(),
            auth_type: model.auth_type.clone(),
            api_url: model.api_url.clone(),
            api_key: model.api_key.clone(),
            oauth_config: model.oauth_config.clone(),
            options: model.options.clone(),
            created_at: model.created_at,
            updated_at: model.updated_at,
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

    pub async fn find_all(&self) -> RepositoryResult<Vec<AIModelConfig>> {
        let file = self.load_config_file().await?;
        Ok(flatten_models(file))
    }

    pub async fn get_config(&self) -> RepositoryResult<AIModelsConfig> {
        self.load_config_file().await
    }

    pub async fn get_agent_model_binding(
        &self,
        agent_name: &str,
        _is_subagent: bool,
    ) -> RepositoryResult<Option<String>> {
        let config = self.load_config_file().await?;
        let binding = config.agents.bindings.get(agent_name).cloned();
        Ok(binding.filter(|value| !value.trim().is_empty()))
    }

    pub fn config_path(&self) -> PathBuf {
        self.db.config_dir().join(MODELS_FILE_NAME)
    }

    pub async fn save(&self, model: &AIModelConfig) -> RepositoryResult<()> {
        let _guard = MODELS_FILE_LOCK.lock().await;
        let file = self.load_config_file_locked().await?;
        let defaults = file.defaults.clone();
        let agents = file.agents.clone();
        let mut models = flatten_models(file);

        if let Some(existing) = models.iter_mut().find(|entry| entry.id == model.id) {
            *existing = model.clone();
        } else {
            models.push(model.clone());
        }

        let file = build_models_file(models, defaults, agents);
        self.write_config_file_locked(&file).await
    }

    pub async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<AIModelConfig>> {
        let models = self.find_all().await?;
        Ok(models.into_iter().find(|model| model.id == id))
    }

    pub async fn delete(&self, id: &str) -> RepositoryResult<()> {
        let _guard = MODELS_FILE_LOCK.lock().await;
        let file = self.load_config_file_locked().await?;
        let defaults = file.defaults.clone();
        let agents = file.agents.clone();
        let mut models = flatten_models(file);
        let before = models.len();
        models.retain(|model| model.id != id);

        if models.len() == before {
            return Err(RepositoryError::AiModelNotFound { id: id.to_string() });
        }

        let file = build_models_file(models, defaults, agents);
        self.write_config_file_locked(&file).await
    }

    async fn load_config_file(&self) -> RepositoryResult<AIModelsConfig> {
        let _guard = MODELS_FILE_LOCK.lock().await;
        self.load_config_file_locked().await
    }

    async fn load_config_file_locked(&self) -> RepositoryResult<AIModelsConfig> {
        let path = self.config_path();
        ensure_parent_dir(&path).await?;

        match fs::read_to_string(&path).await {
            Ok(raw) => {
                let mut file: AIModelsConfig = serde_json::from_str(&raw)?;
                if file.version == 0 {
                    file.version = MODELS_FILE_VERSION;
                }
                Ok(file)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                let file = AIModelsConfig {
                    version: MODELS_FILE_VERSION,
                    defaults: ModelDefaults::default(),
                    agents: AgentModelConfig::default(),
                    providers: BTreeMap::new(),
                };
                self.write_config_file_locked(&file).await?;
                Ok(file)
            }
            Err(err) => Err(RepositoryError::internal(format!(
                "Failed to read models config {}: {err}",
                path.display()
            ))),
        }
    }

    async fn write_config_file_locked(&self, file: &AIModelsConfig) -> RepositoryResult<()> {
        let path = self.config_path();
        ensure_parent_dir(&path).await?;
        let json = serde_json::to_string_pretty(file)?;
        fs::write(&path, format!("{json}\n")).await.map_err(|err| {
            RepositoryError::internal(format!(
                "Failed to write models config {}: {err}",
                path.display()
            ))
        })
    }
}

fn flatten_models(file: AIModelsConfig) -> Vec<AIModelConfig> {
    let mut models = Vec::new();

    for (provider, group) in file.providers {
        for model in group.models {
            models.push(model.into_config(provider.clone()));
        }
    }

    models.sort_by(|left, right| {
        left.provider
            .cmp(&right.provider)
            .then_with(|| {
                display_name_or_model(left.display_name.as_deref(), left.model.as_str()).cmp(
                    display_name_or_model(right.display_name.as_deref(), right.model.as_str()),
                )
            })
            .then_with(|| left.id.cmp(&right.id))
    });

    models
}

fn build_models_file(
    models: Vec<AIModelConfig>,
    defaults: ModelDefaults,
    agents: AgentModelConfig,
) -> AIModelsConfig {
    let mut providers: BTreeMap<String, ProviderModels> = BTreeMap::new();
    let mut available_chat = Vec::new();
    let mut available_embedding = Vec::new();

    for model in models {
        if model.model_type == ModelType::Chat {
            available_chat.push(model.id.clone());
        }
        if model.model_type == ModelType::Embedding {
            available_embedding.push(model.id.clone());
        }

        let provider = model.provider.clone();
        providers
            .entry(provider)
            .or_default()
            .models
            .push(ModelEntry::from(&model));
    }

    for group in providers.values_mut() {
        group.models.sort_by(|left, right| {
            display_name_or_model(left.display_name.as_deref(), left.model.as_str())
                .cmp(display_name_or_model(
                    right.display_name.as_deref(),
                    right.model.as_str(),
                ))
                .then_with(|| left.id.cmp(&right.id))
        });
    }

    let defaults = normalize_defaults(defaults, &available_chat, &available_embedding);

    AIModelsConfig {
        version: MODELS_FILE_VERSION,
        defaults,
        agents,
        providers,
    }
}

fn display_name_or_model<'a>(display_name: Option<&'a str>, model: &'a str) -> &'a str {
    match display_name {
        Some(name) => name,
        None => model,
    }
}

fn normalize_defaults(
    defaults: ModelDefaults,
    available_chat: &[String],
    available_embedding: &[String],
) -> ModelDefaults {
    let chat_model_id = defaults
        .chat_model_id
        .filter(|id| available_chat.iter().any(|candidate| candidate == id));

    let embedding_model_id = defaults
        .embedding_model_id
        .filter(|id| available_embedding.iter().any(|candidate| candidate == id));

    ModelDefaults {
        chat_model_id,
        embedding_model_id,
    }
}

async fn ensure_parent_dir(path: &Path) -> RepositoryResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(|err| {
            RepositoryError::internal(format!(
                "Failed to create models config directory {}: {err}",
                parent.display()
            ))
        })?;
    }
    Ok(())
}
