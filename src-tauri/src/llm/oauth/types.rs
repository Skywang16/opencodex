use serde::{Deserialize, Serialize};

/// PKCE code pair
#[derive(Debug, Clone)]
pub struct PkceCodes {
    pub verifier: String,
    pub challenge: String,
}

/// Token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: Option<String>,
    pub token_type: String,
    pub expires_in: Option<u64>,
}

/// OAuth flow handle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlowInfo {
    pub flow_id: String,
    pub authorize_url: String,
    pub provider: String,
}

/// OAuth error
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("OAuth flow not found: {0}")]
    FlowNotFound(String),

    #[error("OAuth callback timeout")]
    Timeout,

    #[error("OAuth cancelled by user")]
    Cancelled,

    #[error("Token exchange failed: {0}")]
    TokenExchange(String),

    #[error("Token refresh failed: {0}")]
    TokenRefresh(String),

    #[error("Invalid provider: {0}")]
    InvalidProvider(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Other error: {0}")]
    Other(String),
}

pub type OAuthResult<T> = Result<T, OAuthError>;

/// Result returned by OAuth flow — lightweight token bundle for the frontend.
/// The caller merges these fields into an AIModelConfig when saving.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthTokenResult {
    pub provider_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    pub oauth_refresh_token: String,
    pub oauth_access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_metadata: Option<serde_json::Value>,
}
