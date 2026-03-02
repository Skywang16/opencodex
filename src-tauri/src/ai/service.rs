use crate::ai::error::{AIServiceError, AIServiceResult};
use crate::storage::repositories::{AIModelConfig, AIModels, AuthType};
use crate::storage::DatabaseManager;
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct AIService {
    database: Arc<DatabaseManager>,
}

impl AIService {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        Self { database }
    }

    pub fn database(&self) -> &DatabaseManager {
        &self.database
    }

    pub async fn initialize(&self) -> AIServiceResult<()> {
        Ok(())
    }

    // ── Model CRUD ───────────────────────────────────────────────────────────

    pub async fn get_models(&self) -> AIServiceResult<Vec<AIModelConfig>> {
        AIModels::new(&self.database)
            .find_all()
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.find_all",
                source: err,
            })
    }

    pub async fn get_model(&self, id: &str) -> AIServiceResult<Option<AIModelConfig>> {
        AIModels::new(&self.database)
            .find_by_id(id)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.find_by_id",
                source: err,
            })
    }

    pub async fn add_model(&self, mut model: AIModelConfig) -> AIServiceResult<AIModelConfig> {
        if model.id.is_empty() {
            model.id = uuid::Uuid::new_v4().to_string();
        }
        let now = Utc::now();
        model.created_at = now;
        model.updated_at = now;

        AIModels::new(&self.database)
            .save(&model)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.save",
                source: err,
            })?;
        Ok(model)
    }

    /// Full update: overwrites the entire model row (save uses UPSERT).
    pub async fn update_model(&self, mut model: AIModelConfig) -> AIServiceResult<AIModelConfig> {
        // Verify model exists
        AIModels::new(&self.database)
            .find_by_id(&model.id)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.find_by_id",
                source: err,
            })?
            .ok_or_else(|| AIServiceError::ModelNotFound {
                model_id: model.id.clone(),
            })?;

        model.updated_at = Utc::now();

        AIModels::new(&self.database)
            .save(&model)
            .await
            .map_err(|err| AIServiceError::Repository {
                operation: "ai_models.save",
                source: err,
            })?;
        Ok(model)
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

    // ── Connection test ──────────────────────────────────────────────────────

    pub async fn test_model(&self, model: &AIModelConfig) -> AIServiceResult<String> {
        let timeout = Duration::from_secs(12);
        let (url, headers) = self.build_test_request(model)?;

        let payload = if model.provider_id == "anthropic" {
            json!({
                "model": &model.model,
                "messages": [{"role": "user", "content": "Hello"}],
                "max_tokens": 1,
            })
        } else if model.auth_type == AuthType::OAuth {
            json!({
                "model": &model.model,
                "input": "Hello",
                "stream": false,
            })
        } else {
            json!({
                "model": &model.model,
                "messages": [{"role": "user", "content": "Hello"}],
                "max_tokens": 1,
            })
        };

        self.execute_http_probe(model.provider_id.clone(), url, headers, payload, timeout)
            .await
    }

    fn build_test_request(&self, model: &AIModelConfig) -> AIServiceResult<(String, HeaderMap)> {
        match model.auth_type {
            AuthType::ApiKey => {
                let api_key = model.api_key.as_deref().unwrap_or_default();
                let base = model
                    .api_url
                    .as_deref()
                    .ok_or_else(|| AIServiceError::Configuration {
                        message: "API URL is required".to_string(),
                    })?
                    .trim_end_matches('/');

                if model.provider_id == "anthropic" {
                    let url = format!("{base}/messages");
                    let headers = header_map(&[
                        ("x-api-key", api_key.to_string()),
                        ("anthropic-version", "2023-06-01".to_string()),
                    ])?;
                    Ok((url, headers))
                } else {
                    let url = format!("{base}/chat/completions");
                    let headers = header_map(&[("authorization", format!("Bearer {api_key}"))])?;
                    Ok((url, headers))
                }
            }
            AuthType::OAuth => {
                let access_token = model.oauth_access_token.as_deref().ok_or_else(|| {
                    AIServiceError::Configuration {
                        message: "OAuth access token missing".to_string(),
                    }
                })?;
                let url = "https://chatgpt.com/backend-api/codex/responses".to_string();
                let mut headers =
                    header_map(&[("authorization", format!("Bearer {access_token}"))])?;
                headers.insert(
                    HeaderName::from_static("originator"),
                    HeaderValue::from_static("opencode"),
                );
                if let Some(meta) = &model.oauth_metadata {
                    if let Some(account_id) = meta.get("account_id").and_then(|v| v.as_str()) {
                        if let Ok(v) = HeaderValue::from_str(account_id) {
                            headers.insert(HeaderName::from_static("chatgpt-account-id"), v);
                        }
                    }
                }
                Ok((url, headers))
            }
        }
    }

    async fn execute_http_probe(
        &self,
        provider: String,
        url: String,
        headers: HeaderMap,
        payload: Value,
        timeout: Duration,
    ) -> AIServiceResult<String> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(AIServiceError::HttpClient)?;

        let mut h = headers;
        h.entry(CONTENT_TYPE)
            .or_insert(HeaderValue::from_static("application/json"));

        let resp = client
            .post(&url)
            .headers(h)
            .json(&payload)
            .send()
            .await
            .map_err(|err| AIServiceError::ProviderRequest {
                provider: provider.clone(),
                source: err,
            })?;

        let status = resp.status();
        if status.is_success() {
            return Ok("ok".to_string());
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            let body = resp.text().await.unwrap_or_default();
            return Err(AIServiceError::ProviderApi {
                provider,
                status,
                message: format!("Authentication failed: {body}"),
            });
        }
        const TOLERATED: &[StatusCode] = &[
            StatusCode::BAD_REQUEST,
            StatusCode::TOO_MANY_REQUESTS,
        ];
        if TOLERATED.contains(&status) {
            return Ok("ok".to_string());
        }
        let body = resp.text().await.unwrap_or_default();
        Err(AIServiceError::ProviderApi {
            provider,
            status,
            message: body,
        })
    }
}

fn header_map(entries: &[(&'static str, String)]) -> AIServiceResult<HeaderMap> {
    let mut headers = HeaderMap::with_capacity(entries.len());
    for (name, value) in entries {
        let header_name = HeaderName::from_static(name);
        let header_value = HeaderValue::from_str(value.trim())
            .map_err(|err| AIServiceError::InvalidHeaderValue { name, source: err })?;
        headers.insert(header_name, header_value);
    }
    Ok(headers)
}
