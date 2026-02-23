use super::pkce::{generate_pkce, generate_state};
use super::provider_trait::OAuthProvider;
use super::providers::OpenAiCodexProvider;
use super::server::OAuthCallbackServer;
use super::types::{OAuthError, OAuthFlowInfo, OAuthResult, PkceCodes};
use crate::storage::database::DatabaseManager;
use crate::storage::repositories::ai_models::{
    OAuthConfig as StorageOAuthConfig, OAuthProvider as StorageOAuthProvider,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, info};

/// Pending OAuth flow
struct PendingFlow {
    receiver: oneshot::Receiver<OAuthResult<(String, PkceCodes)>>,
}

/// OAuth Manager - manages multiple providers and coordinates authorization flows
pub struct OAuthManager {
    providers: HashMap<String, Box<dyn OAuthProvider>>,
    callback_server: Arc<Mutex<OAuthCallbackServer>>,
    pending_flows: Mutex<HashMap<String, PendingFlow>>,
    #[allow(dead_code)]
    db: Arc<DatabaseManager>,
}

impl OAuthManager {
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        let mut providers: HashMap<String, Box<dyn OAuthProvider>> = HashMap::new();

        // Register OpenAI Codex Provider
        let openai_provider = OpenAiCodexProvider::new();
        providers.insert(
            openai_provider.provider_id().to_string(),
            Box::new(openai_provider),
        );

        // More providers can be added here in the future:
        // providers.insert("claude_pro".to_string(), Box::new(ClaudeProProvider::new()));
        // providers.insert("gemini_advanced".to_string(), Box::new(GeminiProvider::new()));

        let callback_server = OAuthCallbackServer::new();

        Self {
            providers,
            callback_server,
            pending_flows: Mutex::new(HashMap::new()),
            db,
        }
    }

    /// Start OAuth flow
    pub async fn start_oauth_flow(&self, provider_type: &str) -> OAuthResult<OAuthFlowInfo> {
        let provider = self
            .providers
            .get(provider_type)
            .ok_or_else(|| OAuthError::InvalidProvider(provider_type.to_string()))?;

        // Ensure callback server is started
        OAuthCallbackServer::ensure_started(self.callback_server.clone()).await;

        // Generate PKCE and state
        let pkce = generate_pkce()?;
        let state = generate_state()?;

        // Generate authorization URL
        let authorize_url = provider.generate_authorize_url(&pkce, &state)?;

        // Register callback wait and get receiver
        let mut server = self.callback_server.lock().await;
        let receiver = server.register_flow(state.clone(), pkce.clone()).await;
        drop(server);

        // Save pending flow
        let pending = PendingFlow { receiver };
        self.pending_flows
            .lock()
            .await
            .insert(state.clone(), pending);

        info!("OAuth flow started for provider: {}", provider_type);

        Ok(OAuthFlowInfo {
            flow_id: state,
            authorize_url,
            provider: provider_type.to_string(),
        })
    }

    /// Wait for OAuth callback
    pub async fn wait_for_callback(
        &self,
        flow_id: &str,
        provider_type: &str,
    ) -> OAuthResult<StorageOAuthConfig> {
        let provider = self
            .providers
            .get(provider_type)
            .ok_or_else(|| OAuthError::InvalidProvider(provider_type.to_string()))?;

        // Retrieve pending flow
        let pending = self
            .pending_flows
            .lock()
            .await
            .remove(flow_id)
            .ok_or_else(|| OAuthError::FlowNotFound(flow_id.to_string()))?;

        // Wait for callback (with timeout)
        let timeout = tokio::time::Duration::from_secs(5 * 60); // 5 minute timeout

        let result = tokio::time::timeout(timeout, pending.receiver)
            .await
            .map_err(|_| OAuthError::Timeout)?
            .map_err(|_| OAuthError::Other("Channel closed".to_string()))??;

        let (code, pkce) = result;

        // Exchange authorization code for tokens
        debug!("Exchanging code for tokens");
        let tokens = provider.exchange_code_for_tokens(&code, &pkce).await?;

        // Extract metadata
        let metadata = provider.extract_metadata(&tokens)?;

        // Calculate expiration time
        let expires_at = tokens
            .expires_in
            .map(|secs| chrono::Utc::now().timestamp() + secs as i64);

        // Convert to storage format
        let storage_provider = match provider_type {
            "openai_codex" => StorageOAuthProvider::OpenAiCodex,
            "claude_pro" => StorageOAuthProvider::ClaudePro,
            "gemini_advanced" => StorageOAuthProvider::GeminiAdvanced,
            _ => return Err(OAuthError::InvalidProvider(provider_type.to_string())),
        };

        Ok(StorageOAuthConfig {
            provider: storage_provider,
            refresh_token: tokens.refresh_token,
            access_token: Some(tokens.access_token),
            expires_at,
            metadata: Some(metadata),
        })
    }

    /// Cancel OAuth flow
    pub async fn cancel_flow(&self, flow_id: &str) -> OAuthResult<()> {
        // Remove pending flow
        self.pending_flows.lock().await.remove(flow_id);

        // Also remove from callback server
        let mut server = self.callback_server.lock().await;
        server.cancel_flow(flow_id).await;

        info!("OAuth flow cancelled: {}", flow_id);
        Ok(())
    }

    /// Refresh token
    pub async fn refresh_token(&self, oauth_config: &mut StorageOAuthConfig) -> OAuthResult<()> {
        let provider_id = oauth_config.provider.to_string();

        let provider = self
            .providers
            .get(&provider_id)
            .ok_or_else(|| OAuthError::InvalidProvider(provider_id.clone()))?;

        debug!("Refreshing token for provider: {}", provider_id);

        let tokens = provider
            .refresh_access_token(&oauth_config.refresh_token)
            .await?;

        // Extract metadata first to avoid borrow issues
        let metadata = provider.extract_metadata(&tokens)?;

        // Update configuration
        oauth_config.access_token = Some(tokens.access_token);
        oauth_config.refresh_token = tokens.refresh_token;
        oauth_config.expires_at = tokens
            .expires_in
            .map(|secs| chrono::Utc::now().timestamp() + secs as i64);
        oauth_config.metadata = Some(metadata);

        info!("Token refreshed for provider: {}", provider_id);
        Ok(())
    }

    /// Get provider
    pub fn get_provider(&self, provider_type: &str) -> Option<&dyn OAuthProvider> {
        self.providers.get(provider_type).map(|p| p.as_ref())
    }

    /// Check if token needs refresh
    pub fn should_refresh_token(&self, oauth_config: &StorageOAuthConfig) -> bool {
        // If no access_token, refresh is needed
        if oauth_config.access_token.is_none() {
            return true;
        }

        // If expiration time exists, check if it's about to expire
        if let Some(expires_at) = oauth_config.expires_at {
            let provider_id = oauth_config.provider.to_string();
            if let Some(provider) = self.providers.get(&provider_id) {
                return provider.should_refresh_token(expires_at);
            }
        }

        // If no expiration time info, conservatively don't refresh (use access_token if available)
        false
    }
}
