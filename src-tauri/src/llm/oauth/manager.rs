use super::pkce::{generate_pkce, generate_state};
use super::provider_trait::OAuthProvider;
use super::providers::OpenAiCodexProvider;
use super::server::OAuthCallbackServer;
use super::types::{OAuthError, OAuthFlowInfo, OAuthResult, OAuthTokenResult, PkceCodes};
use crate::storage::database::DatabaseManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{oneshot, Mutex as TokioMutex};
use tracing::{debug, info};

struct PendingFlow {
    receiver: oneshot::Receiver<OAuthResult<(String, PkceCodes)>>,
}

pub struct OAuthManager {
    providers: HashMap<String, Box<dyn OAuthProvider>>,
    callback_server: Arc<Mutex<OAuthCallbackServer>>,
    pending_flows: TokioMutex<HashMap<String, PendingFlow>>,
    db: Arc<DatabaseManager>,
}

impl OAuthManager {
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        let mut providers: HashMap<String, Box<dyn OAuthProvider>> = HashMap::new();
        let openai_provider = OpenAiCodexProvider::new();
        providers.insert(
            openai_provider.provider_id().to_string(),
            Box::new(openai_provider),
        );

        Self {
            providers,
            callback_server: OAuthCallbackServer::new(),
            pending_flows: TokioMutex::new(HashMap::new()),
            db,
        }
    }

    pub async fn start_oauth_flow(&self, provider_type: &str) -> OAuthResult<OAuthFlowInfo> {
        let provider = self
            .providers
            .get(provider_type)
            .ok_or_else(|| OAuthError::InvalidProvider(provider_type.to_string()))?;

        OAuthCallbackServer::ensure_started(self.callback_server.clone()).await;

        let pkce = generate_pkce()?;
        let state = generate_state()?;
        let authorize_url = provider.generate_authorize_url(&pkce, &state)?;

        let receiver = match self.callback_server.lock() {
            Ok(mut srv) => srv.register_flow(state.clone(), pkce.clone()),
            Err(poisoned) => poisoned.into_inner().register_flow(state.clone(), pkce.clone()),
        };
        self.pending_flows
            .lock()
            .await
            .insert(state.clone(), PendingFlow { receiver });

        info!("OAuth flow started for provider: {}", provider_type);
        Ok(OAuthFlowInfo {
            flow_id: state,
            authorize_url,
            provider: provider_type.to_string(),
        })
    }

    /// Wait for OAuth callback; returns token bundle for the frontend to merge into a model config.
    pub async fn wait_for_callback(
        &self,
        flow_id: &str,
        provider_type: &str,
    ) -> OAuthResult<OAuthTokenResult> {
        let provider = self
            .providers
            .get(provider_type)
            .ok_or_else(|| OAuthError::InvalidProvider(provider_type.to_string()))?;

        let pending = self
            .pending_flows
            .lock()
            .await
            .remove(flow_id)
            .ok_or_else(|| OAuthError::FlowNotFound(flow_id.to_string()))?;

        let timeout = tokio::time::Duration::from_secs(5 * 60);
        let result = tokio::time::timeout(timeout, pending.receiver)
            .await
            .map_err(|_| OAuthError::Timeout)?
            .map_err(|_| OAuthError::Other("Channel closed".to_string()))??;

        let (code, pkce) = result;
        debug!("Exchanging code for tokens");
        let tokens = provider.exchange_code_for_tokens(&code, &pkce).await?;
        let metadata = provider.extract_metadata(&tokens)?;
        let expires_at = tokens
            .expires_in
            .map(|secs| chrono::Utc::now().timestamp() + secs as i64);

        let provider_id = Self::oauth_to_provider_id(provider_type);

        let api_url = match provider_type {
            "openai_codex" => Some("https://chatgpt.com/backend-api/codex".to_string()),
            _ => None,
        };

        Ok(OAuthTokenResult {
            provider_id: provider_id.to_string(),
            api_url,
            oauth_refresh_token: tokens.refresh_token,
            oauth_access_token: tokens.access_token,
            oauth_expires_at: expires_at,
            oauth_metadata: Some(metadata),
        })
    }

    pub async fn cancel_flow(&self, flow_id: &str) -> OAuthResult<()> {
        self.pending_flows.lock().await.remove(flow_id);
        match self.callback_server.lock() {
            Ok(mut srv) => srv.cancel_flow(flow_id),
            Err(poisoned) => poisoned.into_inner().cancel_flow(flow_id),
        };
        info!("OAuth flow cancelled: {}", flow_id);
        Ok(())
    }

    /// Refresh token in-place on an AIModelConfig
    pub async fn refresh_token(
        &self,
        model: &mut crate::storage::repositories::AIModelConfig,
    ) -> OAuthResult<()> {
        let provider = self
            .providers
            .get(&self.credential_to_provider_id(&model.provider_id))
            .ok_or_else(|| OAuthError::InvalidProvider(model.provider_id.clone()))?;

        let refresh_token = model
            .oauth_refresh_token
            .as_deref()
            .ok_or_else(|| OAuthError::Other("No refresh token".to_string()))?;

        debug!("Refreshing token for provider: {}", model.provider_id);
        let tokens = provider.refresh_access_token(refresh_token).await?;
        let metadata = provider.extract_metadata(&tokens)?;

        model.oauth_access_token = Some(tokens.access_token);
        model.oauth_refresh_token = Some(tokens.refresh_token);
        model.oauth_expires_at = tokens
            .expires_in
            .map(|s| chrono::Utc::now().timestamp() + s as i64);
        model.oauth_metadata = Some(metadata);
        model.updated_at = chrono::Utc::now();

        if let Err(e) = crate::storage::repositories::AIModels::new(&self.db)
            .save(model)
            .await
        {
            tracing::warn!("Failed to persist refreshed token to database: {}", e);
        }

        info!("Token refreshed for model: {}", model.id);
        Ok(())
    }

    pub fn should_refresh_token(
        &self,
        model: &crate::storage::repositories::AIModelConfig,
    ) -> bool {
        if model.oauth_access_token.is_none() {
            return true;
        }
        if let Some(expires_at) = model.oauth_expires_at {
            let provider_key = self.credential_to_provider_id(&model.provider_id);
            if let Some(provider) = self.providers.get(&provider_key) {
                return provider.should_refresh_token(expires_at);
            }
        }
        false
    }

    const PROVIDER_MAP: &[(&'static str, &'static str)] = &[
        ("openai_codex", "openai"),
        ("claude_pro", "anthropic"),
        ("gemini_advanced", "gemini"),
    ];

    fn oauth_to_provider_id(oauth_type: &str) -> &str {
        Self::PROVIDER_MAP
            .iter()
            .find(|(k, _)| *k == oauth_type)
            .map(|(_, v)| *v)
            .unwrap_or(oauth_type)
    }

    fn credential_to_provider_id(&self, provider_id: &str) -> String {
        Self::PROVIDER_MAP
            .iter()
            .find(|(_, v)| *v == provider_id)
            .map(|(k, _)| k.to_string())
            .unwrap_or_else(|| provider_id.to_string())
    }
}
