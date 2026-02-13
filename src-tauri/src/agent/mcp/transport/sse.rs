use crate::agent::mcp::error::{McpError, McpResult};
use crate::agent::mcp::protocol::jsonrpc::{JsonRpcId, JsonRpcRequest, JsonRpcResponse};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SseTransport {
    client: Client,
    url: String,
    request_id: Arc<Mutex<u64>>,
}

impl SseTransport {
    pub async fn new(url: &str, headers: &HashMap<String, String>) -> McpResult<Self> {
        let mut default_headers = reqwest::header::HeaderMap::new();
        for (k, v) in headers {
            if let (Ok(name), Ok(value)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(v),
            ) {
                default_headers.insert(name, value);
            }
        }

        let client = Client::builder()
            .default_headers(default_headers)
            .build()
            .map_err(|e| McpError::Protocol(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            url: url.to_string(),
            request_id: Arc::new(Mutex::new(1)),
        })
    }

    pub async fn request(&self, mut req: JsonRpcRequest) -> McpResult<JsonRpcResponse> {
        let mut id = self.request_id.lock().await;
        req.id = Some(JsonRpcId::Number(*id as i64));
        *id += 1;
        drop(id);

        let response = self
            .client
            .post(&self.url)
            .json(&req)
            .send()
            .await
            .map_err(|e| McpError::Protocol(format!("SSE request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(McpError::Protocol(format!(
                "SSE request failed with status: {}",
                response.status()
            )));
        }

        let resp: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| McpError::Protocol(format!("Failed to parse SSE response: {}", e)))?;

        Ok(resp)
    }

    pub async fn notify(&self, req: JsonRpcRequest) -> McpResult<()> {
        let _ = self
            .client
            .post(&self.url)
            .json(&req)
            .send()
            .await
            .map_err(|e| McpError::Protocol(format!("SSE notify failed: {}", e)))?;
        Ok(())
    }
}
