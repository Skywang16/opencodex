use super::types::{OAuthResult, PkceCodes, TokenResponse};
use async_trait::async_trait;

/// OAuth Provider unified interface
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn display_name(&self) -> &str;

    fn generate_authorize_url(&self, pkce: &PkceCodes, state: &str) -> OAuthResult<String>;

    async fn exchange_code_for_tokens(
        &self,
        code: &str,
        pkce: &PkceCodes,
    ) -> OAuthResult<TokenResponse>;

    async fn refresh_access_token(&self, refresh_token: &str) -> OAuthResult<TokenResponse>;

    fn extract_metadata(&self, tokens: &TokenResponse) -> OAuthResult<serde_json::Value>;

    fn should_refresh_token(&self, expires_at: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        expires_at - now < 5 * 60
    }
}
