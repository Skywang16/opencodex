use super::types::{OAuthError, OAuthResult, PkceCodes};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tiny_http::Header;
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};
use url::form_urlencoded;

const OAUTH_PORT: u16 = 1455;
const OAUTH_CALLBACK_PATH: &str = "/auth/callback";

/// OAuth callback waiter
struct PendingOAuthFlow {
    pub pkce: PkceCodes,
    pub sender: oneshot::Sender<OAuthResult<(String, PkceCodes)>>,
}

/// OAuth callback server
pub struct OAuthCallbackServer {
    pending_flows: Arc<Mutex<HashMap<String, PendingOAuthFlow>>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    started: bool,
}

impl OAuthCallbackServer {
    /// Create server (does not start immediately, deferred until first use)
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            pending_flows: Arc::new(Mutex::new(HashMap::new())),
            server_handle: None,
            started: false,
        }))
    }

    /// Ensure server is started
    pub async fn ensure_started(server: Arc<Mutex<Self>>) {
        let mut guard = match server.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                warn!("OAuth server mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };
        if guard.started {
            return;
        }
        guard.started = true;
        let flows = guard.pending_flows.clone();

        let handle = tokio::task::spawn_blocking(move || {
            if let Err(e) = Self::run_server(flows) {
                error!("Failed to start OAuth callback server: {}", e);
            }
        });
        guard.server_handle = Some(handle);
        drop(guard);
    }

    /// Run HTTP server (synchronous, must be called from spawn_blocking)
    fn run_server(flows: Arc<Mutex<HashMap<String, PendingOAuthFlow>>>) -> OAuthResult<()> {
        use tiny_http::{Response, Server};

        let http_server = Server::http(format!("127.0.0.1:{OAUTH_PORT}"))
            .map_err(|e| OAuthError::Other(format!("Failed to bind server: {}", e)))?;

        info!("OAuth callback server started on port {}", OAUTH_PORT);

        let html_content_type =
            Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();

        for request in http_server.incoming_requests() {
            let url_str = request.url();
            debug!("OAuth callback received: {}", url_str);

            if url_str.starts_with(OAUTH_CALLBACK_PATH) {
                let query_start = url_str.find('?');
                if let Some(start) = query_start {
                    let query = &url_str[start + 1..];
                    let params: HashMap<String, String> = form_urlencoded::parse(query.as_bytes())
                        .into_owned()
                        .collect();

                    let state = params.get("state").cloned();
                    let code = params.get("code").cloned();
                    let error = params.get("error").cloned();

                    if let Some(state_val) = state {
                        let pending = match flows.lock() {
                            Ok(mut f) => f.remove(&state_val),
                            Err(poisoned) => poisoned.into_inner().remove(&state_val),
                        };
                        if let Some(pending) = pending {
                            if let Some(err) = error {
                                let _ = pending
                                    .sender
                                    .send(Err(OAuthError::Other(format!("OAuth error: {}", err))));
                                let _ = request.respond(
                                    Response::from_string(Self::html_error(&err))
                                        .with_header(html_content_type.clone()),
                                );
                            } else if let Some(code_val) = code {
                                let _ = pending.sender.send(Ok((code_val, pending.pkce)));
                                let _ = request.respond(
                                    Response::from_string(Self::html_success())
                                        .with_header(html_content_type.clone()),
                                );
                            } else {
                                let _ = pending
                                    .sender
                                    .send(Err(OAuthError::Other("Missing code".to_string())));
                                let _ = request.respond(
                                    Response::from_string(Self::html_error("Missing code"))
                                        .with_header(html_content_type.clone()),
                                );
                            }
                            continue;
                        }
                    }

                    warn!("OAuth callback with invalid or missing state");
                    let _ = request.respond(
                        Response::from_string(Self::html_error("Invalid state"))
                            .with_header(html_content_type.clone()),
                    );
                } else {
                    let _ = request.respond(
                        Response::from_string(Self::html_error("Invalid request"))
                            .with_header(html_content_type.clone()),
                    );
                }
            } else {
                let _ = request.respond(Response::from_string("Not found").with_status_code(404));
            }
        }

        Ok(())
    }

    /// Register callback waiter
    pub fn register_flow(
        &mut self,
        state: String,
        pkce: PkceCodes,
    ) -> oneshot::Receiver<OAuthResult<(String, PkceCodes)>> {
        let (sender, receiver) = oneshot::channel();

        let pending = PendingOAuthFlow { pkce, sender };

        match self.pending_flows.lock() {
            Ok(mut flows) => { flows.insert(state, pending); }
            Err(poisoned) => { poisoned.into_inner().insert(state, pending); }
        }

        receiver
    }

    /// Cancel flow
    pub fn cancel_flow(&mut self, state: &str) {
        match self.pending_flows.lock() {
            Ok(mut flows) => { flows.remove(state); }
            Err(poisoned) => { poisoned.into_inner().remove(state); }
        }
    }

    /// Success page HTML
    fn html_success() -> String {
        r#"<!DOCTYPE html>
<html>
<head>
    <title>OpenCodex - Authorization Successful</title>
    <style>
        body { font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #0f172a; color: #e2e8f0; }
        .container { text-align: center; padding: 2rem; }
        h1 { color: #10b981; margin-bottom: 1rem; }
        p { color: #94a3b8; }
    </style>
</head>
<body>
    <div class="container">
        <h1>✓ Authorization Successful</h1>
        <p>You can close this window and return to OpenCodex.</p>
    </div>
    <script>setTimeout(() => window.close(), 2000);</script>
</body>
</html>"#.to_string()
    }

    /// Error page HTML
    fn html_error(error: &str) -> String {
        let escaped = error
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;");
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>OpenCodex - Authorization Failed</title>
    <style>
        body {{ font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #0f172a; color: #e2e8f0; }}
        .container {{ text-align: center; padding: 2rem; }}
        h1 {{ color: #ef4444; margin-bottom: 1rem; }}
        p {{ color: #94a3b8; }}
        .error {{ color: #fca5a5; font-family: monospace; margin-top: 1rem; padding: 1rem; background: rgba(239,68,68,0.1); border-radius: 0.5rem; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>✗ Authorization Failed</h1>
        <p>An error occurred during authorization.</p>
        <div class="error">{}</div>
    </div>
</body>
</html>"#,
            escaped
        )
    }

    /// Get callback URL
    pub fn callback_url() -> String {
        format!("http://localhost:{OAUTH_PORT}{OAUTH_CALLBACK_PATH}")
    }
}

impl Drop for OAuthCallbackServer {
    fn drop(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
    }
}
