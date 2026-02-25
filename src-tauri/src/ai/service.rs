use crate::ai::error::{AIServiceError, AIServiceResult};
use crate::ai::types::{AIModelConfig, AIProvider, ModelType};
use crate::storage::repositories::{AIModels, AuthType, OAuthConfig};
use crate::storage::DatabaseManager;
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

#[derive(Clone)]
pub struct AIService {
    database: Arc<DatabaseManager>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AIModelUpdatePayload {
    provider: Option<AIProvider>,
    auth_type: Option<AuthType>,
    api_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    display_name: Option<String>,
    model_type: Option<ModelType>,
    oauth_config: Option<Option<OAuthConfig>>,
    options: Option<Value>,
    use_custom_base_url: Option<bool>,
}

struct ProviderHttpRequest {
    provider_label: String,
    url: String,
    headers: HeaderMap,
    payload: Value,
    timeout: Duration,
    tolerated: &'static [StatusCode],
}

enum ConnectionProbe {
    Http(ProviderHttpRequest),
}

impl AIService {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    pub async fn initialize(&self) -> AIServiceResult<()> {
        Ok(())
    }

    pub async fn get_models(&self) -> AIServiceResult<Vec<AIModelConfig>> {
        AIModels::new(&self.database)
            .find_all()
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.find_all",
                source: err,
            })
    }

    pub async fn add_model(&self, mut config: AIModelConfig) -> AIServiceResult<()> {
        let repo = AIModels::new(&self.database);

        if config.auth_type == AuthType::ApiKey {
            let existing = repo
                .find_all()
                .await
                .map_err(|err| AIServiceError::Repository {
                    operation: "ai_models.find_all",
                    source: err,
                })?;
            let candidate_url = normalize_url(config.api_url.as_deref());
            let candidate_key = normalize_key(config.api_key.as_deref());
            let candidate_model = config.model.trim();

            let duplicate = existing.iter().any(|model| {
                model.id != config.id
                    && model.auth_type == AuthType::ApiKey
                    && model.model.trim() == candidate_model
                    && normalize_url(model.api_url.as_deref()) == candidate_url
                    && normalize_key(model.api_key.as_deref()) == candidate_key
            });

            if duplicate {
                return Err(AIServiceError::ModelAlreadyExists {
                    provider: config.provider.clone(),
                    model: config.model.clone(),
                });
            }
        }

        repo.save(&config)
            .await
            .map(|_| ())
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.save",
                source: err,
            })
    }

    pub async fn remove_model(&self, model_id: &str) -> AIServiceResult<()> {
        AIModels::new(&self.database)
            .delete(model_id)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.delete",
                source: err,
            })
    }

    pub async fn update_model(&self, model_id: &str, updates: Value) -> AIServiceResult<()> {
        let update_payload: AIModelUpdatePayload =
            serde_json::from_value(updates).map_err(AIServiceError::InvalidUpdatePayload)?;

        let repo = AIModels::new(&self.database);
        let mut existing = repo
            .find_by_id(model_id)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.find_by_id",
                source: err,
            })?
            .ok_or_else(|| AIServiceError::ModelNotFound {
                model_id: model_id.to_string(),
            })?;

        if let Some(provider) = update_payload.provider {
            existing.provider = provider;
        }
        if let Some(auth_type) = update_payload.auth_type {
            existing.auth_type = auth_type;
        }
        if let Some(url) = update_payload.api_url.and_then(trimmed) {
            existing.api_url = Some(url);
        }
        if let Some(api_key) = update_payload.api_key {
            existing.api_key = Some(api_key);
        }
        if let Some(model) = update_payload.model.and_then(trimmed) {
            existing.model = model;
        }
        if let Some(display_name) = update_payload.display_name.and_then(trimmed) {
            existing.display_name = Some(display_name);
        }
        if let Some(model_type) = update_payload.model_type {
            existing.model_type = model_type;
        }
        if let Some(oauth_config) = update_payload.oauth_config {
            existing.oauth_config = oauth_config;
        }
        if let Some(options) = update_payload.options {
            existing.options = Some(options);
        }
        if let Some(use_custom_base_url) = update_payload.use_custom_base_url {
            existing.use_custom_base_url = Some(use_custom_base_url);
        }

        // Keep authentication data consistent when auth mode changes.
        match existing.auth_type {
            AuthType::OAuth => {
                existing.api_key = None;
            }
            AuthType::ApiKey => {
                existing.oauth_config = None;
            }
        }

        existing.updated_at = Utc::now();

        if existing
            .display_name
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            return Err(AIServiceError::Configuration {
                message: "display_name_empty".to_string(),
            });
        }

        if existing.auth_type == AuthType::ApiKey {
            let all_models = repo
                .find_all()
                .await
                .map_err(|err| AIServiceError::Repository {
                    operation: "ai_models.find_all",
                    source: err,
                })?;
            let candidate_url = normalize_url(existing.api_url.as_deref());
            let candidate_key = normalize_key(existing.api_key.as_deref());
            let candidate_model = existing.model.trim();

            let duplicate = all_models.iter().any(|model| {
                model.id != existing.id
                    && model.auth_type == AuthType::ApiKey
                    && model.model.trim() == candidate_model
                    && normalize_url(model.api_url.as_deref()) == candidate_url
                    && normalize_key(model.api_key.as_deref()) == candidate_key
            });

            if duplicate {
                return Err(AIServiceError::ModelAlreadyExists {
                    provider: existing.provider.clone(),
                    model: existing.model.clone(),
                });
            }
        }

        repo.save(&existing)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.save",
                source: err,
            })
    }

    pub async fn test_connection_with_config(
        &self,
        model: &AIModelConfig,
    ) -> AIServiceResult<String> {
        let probe = self.build_probe(model)?;

        match probe {
            ConnectionProbe::Http(request) => self.execute_http_probe(request).await,
        }
    }

    fn build_probe(&self, model: &AIModelConfig) -> AIServiceResult<ConnectionProbe> {
        let timeout = self.resolve_timeout(model);

        // Get authentication credentials (API Key or OAuth access_token)
        let get_auth_token = |model: &AIModelConfig| -> AIServiceResult<String> {
            match model.auth_type {
                AuthType::OAuth => {
                    let oauth = model.oauth_config.as_ref().ok_or_else(|| {
                        AIServiceError::Configuration {
                            message: "OAuth configuration is required for OAuth models".to_string(),
                        }
                    })?;
                    oauth
                        .access_token
                        .clone()
                        .ok_or_else(|| AIServiceError::Configuration {
                            message: "OAuth access token is missing. Please re-authorize."
                                .to_string(),
                        })
                }
                AuthType::ApiKey => {
                    model
                        .api_key
                        .clone()
                        .ok_or_else(|| AIServiceError::Configuration {
                            message: "API key is required".to_string(),
                        })
                }
            }
        };

        if model.provider == "anthropic" {
            // Anthropic uses different API format
            let api_url =
                model
                    .api_url
                    .as_deref()
                    .ok_or_else(|| AIServiceError::Configuration {
                        message: "API URL is required".to_string(),
                    })?;
            let api_key = get_auth_token(model)?;
            let url = join_url(api_url.trim(), "messages");
            let headers = header_map(&[
                ("x-api-key", api_key),
                ("anthropic-version", "2023-06-01".to_string()),
            ])?;
            let payload = json!({
                "model": model.model,
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "Hello"}]
            });
            Ok(ConnectionProbe::Http(ProviderHttpRequest {
                provider_label: "Anthropic".to_string(),
                url,
                headers,
                payload,
                timeout,
                tolerated: TOLERATED_STANDARD_CODES,
            }))
        } else {
            // All other providers use OpenAI-compatible API
            let payload = if model.model_type == ModelType::Embedding {
                basic_embedding_payload(&model.model)
            } else {
                basic_chat_payload(&model.model)
            };
            let endpoint = if model.model_type == ModelType::Embedding {
                "embeddings"
            } else {
                "chat/completions"
            };

            let (url, headers) = if model.auth_type == AuthType::OAuth {
                if model.model_type == ModelType::Embedding {
                    return Err(AIServiceError::Configuration {
                        message:
                            "OAuth authentication currently supports chat models only for connection testing."
                                .to_string(),
                    });
                }
                let oauth =
                    model
                        .oauth_config
                        .as_ref()
                        .ok_or_else(|| AIServiceError::Configuration {
                            message: "OAuth configuration is required".to_string(),
                        })?;
                let access_token = get_auth_token(model)?;

                // OpenAI Codex uses ChatGPT backend API
                let mut headers =
                    header_map(&[("authorization", format!("Bearer {access_token}"))])?;

                // Add ChatGPT-Account-Id (if available)
                if let Some(metadata) = &oauth.metadata {
                    if let Some(account_id) = metadata.get("account_id").and_then(|v| v.as_str()) {
                        headers.insert(
                            HeaderName::from_static("chatgpt-account-id"),
                            HeaderValue::from_str(account_id).map_err(|err| {
                                AIServiceError::InvalidHeaderValue {
                                    name: "chatgpt-account-id",
                                    source: err,
                                }
                            })?,
                        );
                    }
                }

                // OAuth branch already assembled fixed URL/headers above.
                (
                    "https://chatgpt.com/backend-api/conversation".to_string(),
                    headers,
                )
            } else {
                let api_url =
                    model
                        .api_url
                        .as_deref()
                        .ok_or_else(|| AIServiceError::Configuration {
                            message: "API URL is required".to_string(),
                        })?;
                let api_key = get_auth_token(model)?;
                let url = join_url(api_url.trim(), endpoint);
                let headers = header_map(&[("authorization", format!("Bearer {api_key}"))])?;
                (url, headers)
            };

            Ok(ConnectionProbe::Http(ProviderHttpRequest {
                provider_label: model.provider.clone(),
                url,
                headers,
                payload,
                timeout,
                tolerated: TOLERATED_CUSTOM_CODES,
            }))
        }
    }

    async fn execute_http_probe(&self, request: ProviderHttpRequest) -> AIServiceResult<String> {
        let client = Client::builder()
            .timeout(request.timeout)
            .build()
            .map_err(AIServiceError::HttpClient)?;

        let mut headers = request.headers.clone();
        headers
            .entry(CONTENT_TYPE)
            .or_insert(HeaderValue::from_static("application/json"));

        let response = client
            .post(&request.url)
            .headers(headers)
            .json(&request.payload)
            .send()
            .await
            .map_err(|err| AIServiceError::ProviderRequest {
                provider: request.provider_label.clone(),
                source: err,
            })?;

        let status = response.status();

        // Success status codes: 2xx
        if status.is_success() {
            return Ok("Connection successful".to_string());
        }

        // Authentication failure: 401/403 - this is a clear error and should not be tolerated
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unauthorized".to_string());
            warn!(
                "{} authentication failed: {}",
                request.provider_label, status
            );
            return Err(AIServiceError::ProviderApi {
                provider: request.provider_label.clone(),
                status,
                message: format!("Authentication failed: {error_text}"),
            });
        }

        // Tolerated status codes: indicate API endpoint is available and authentication is valid
        // 400: request format error, but server is reachable
        // 429: too many requests, but authentication succeeded
        if request.tolerated.contains(&status) {
            return Ok("Connection successful".to_string());
        }

        // Other error status codes
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "(failed to read response body)".to_string());
        let error_msg = format!(
            "{} API error: {} - {}",
            request.provider_label, status, error_text
        );
        warn!("{}", error_msg);
        Err(AIServiceError::ProviderApi {
            provider: request.provider_label,
            status,
            message: error_msg,
        })
    }

    fn resolve_timeout(&self, model: &AIModelConfig) -> Duration {
        let Some(opts) = model.options.as_ref() else {
            return Duration::from_secs(12);
        };

        // Strict single-field contract: use timeoutSeconds only.
        if let Some(secs) = get_positive_u64(opts, "timeoutSeconds") {
            return Duration::from_secs(secs.clamp(1, 600));
        }

        Duration::from_secs(12)
    }
}

fn normalize_url(url: Option<&str>) -> String {
    url.unwrap_or_default().trim().trim_end_matches('/').to_string()
}

fn normalize_key(key: Option<&str>) -> String {
    key.unwrap_or_default().trim().to_string()
}

fn header_map(entries: &[(&'static str, String)]) -> AIServiceResult<HeaderMap> {
    // Pre-allocate capacity to avoid multiple rehashes
    let mut headers = HeaderMap::with_capacity(entries.len());
    for (name, value) in entries {
        let header_name = HeaderName::from_static(name);
        let header_value = HeaderValue::from_str(value.trim())
            .map_err(|err| AIServiceError::InvalidHeaderValue { name, source: err })?;
        headers.insert(header_name, header_value);
    }
    Ok(headers)
}

fn trimmed<S: Into<String>>(value: S) -> Option<String> {
    let s = value.into().trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn basic_chat_payload(model: &str) -> Value {
    json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 1,
        "temperature": 0,
    })
}

fn basic_embedding_payload(model: &str) -> Value {
    json!({
        "model": model,
        "input": "hello",
    })
}

fn get_positive_u64(options: &Value, key: &str) -> Option<u64> {
    if let Some(v) = options.get(key).and_then(Value::as_u64) {
        return Some(v);
    }
    options
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|v| u64::try_from(v).ok())
}

fn join_url(base: &str, suffix: &str) -> String {
    let base = base.trim_end_matches('/');
    let suffix = suffix.trim_start_matches('/');
    format!("{base}/{suffix}")
}

const TOLERATED_STANDARD_CODES: &[StatusCode] =
    &[StatusCode::BAD_REQUEST, StatusCode::TOO_MANY_REQUESTS];

const TOLERATED_CUSTOM_CODES: &[StatusCode] = &[
    StatusCode::BAD_REQUEST,
    StatusCode::TOO_MANY_REQUESTS,
    StatusCode::UNPROCESSABLE_ENTITY,
];
