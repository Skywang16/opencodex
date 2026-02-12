use super::types::{OAuthResult, PkceCodes, TokenResponse};
use crate::storage::repositories::ai_models::OAuthConfig;
use async_trait::async_trait;
use reqwest::RequestBuilder;

/// OAuth Provider unified interface
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Provider identifier
    fn provider_id(&self) -> &str;

    /// Display name
    fn display_name(&self) -> &str;

    /// Generate authorization URL
    fn generate_authorize_url(&self, pkce: &PkceCodes, state: &str) -> OAuthResult<String>;

    /// Exchange authorization code for tokens
    async fn exchange_code_for_tokens(
        &self,
        code: &str,
        pkce: &PkceCodes,
    ) -> OAuthResult<TokenResponse>;

    /// Refresh access token
    async fn refresh_access_token(&self, refresh_token: &str) -> OAuthResult<TokenResponse>;

    /// Extract metadata from tokens
    fn extract_metadata(&self, tokens: &TokenResponse) -> OAuthResult<serde_json::Value>;

    /// Prepare API request (add authentication headers, etc.)
    async fn prepare_request(
        &self,
        request: RequestBuilder,
        oauth_config: &OAuthConfig,
    ) -> OAuthResult<RequestBuilder>;

    /// Check if token needs refresh
    fn should_refresh_token(&self, expires_at: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        let threshold = 5 * 60; // Refresh 5 minutes early
        expires_at - now < threshold
    }
}
