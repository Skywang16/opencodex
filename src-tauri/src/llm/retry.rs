//! LLM request retry with exponential backoff.

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

use crate::llm::error::{AnthropicError, GeminiError, LlmProviderError, OpenAiError};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 10,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Config for stream requests (fewer retries since stream must restart)
    pub fn for_stream() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 500,
            max_delay_ms: 30000,
            ..Default::default()
        }
    }

    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base = self.initial_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        let delay_ms = base.min(self.max_delay_ms as f64) as u64;

        let final_ms = if self.jitter {
            let jitter = delay_ms / 4;
            let rand = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
                % 1000) as u64;
            delay_ms.saturating_sub(jitter) + (rand * jitter * 2 / 1000)
        } else {
            delay_ms
        };

        Duration::from_millis(final_ms)
    }
}

/// Check if error is retryable
pub fn is_retryable_error(error: &LlmProviderError) -> bool {
    match error {
        LlmProviderError::OpenAi(e) => is_openai_retryable(e),
        LlmProviderError::Anthropic(e) => is_anthropic_retryable(e),
        LlmProviderError::Gemini(e) => is_gemini_retryable(e),
        _ => false,
    }
}

fn is_openai_retryable(e: &OpenAiError) -> bool {
    match e {
        OpenAiError::Http { source } => is_reqwest_retryable(source),
        OpenAiError::Api { status, .. } => is_status_retryable(*status),
        OpenAiError::Stream { .. } => true,
        _ => false,
    }
}

fn is_anthropic_retryable(e: &AnthropicError) -> bool {
    match e {
        AnthropicError::Http { source } => is_reqwest_retryable(source),
        AnthropicError::Api { status, .. } => is_status_retryable(*status),
        AnthropicError::Stream { .. } => true,
        _ => false,
    }
}

fn is_gemini_retryable(e: &GeminiError) -> bool {
    match e {
        GeminiError::Http { source } => is_reqwest_retryable(source),
        GeminiError::Api { status, .. } => is_status_retryable(*status),
        GeminiError::Stream { .. } => true,
        _ => false,
    }
}

fn is_reqwest_retryable(e: &reqwest::Error) -> bool {
    e.is_timeout()
        || e.is_connect()
        || e.is_request()
        || e.is_body()
        || e.status().is_some_and(|s| is_status_retryable(s))
}

fn is_status_retryable(status: reqwest::StatusCode) -> bool {
    status.is_server_error() || matches!(status.as_u16(), 408 | 425 | 429 | 499)
}

/// Get retry reason for logging
pub fn error_retry_reason(e: &LlmProviderError) -> &'static str {
    match e {
        LlmProviderError::OpenAi(OpenAiError::Http { source })
        | LlmProviderError::Anthropic(AnthropicError::Http { source })
        | LlmProviderError::Gemini(GeminiError::Http { source }) => {
            if source.is_timeout() {
                "timeout"
            } else if source.is_connect() {
                "connection"
            } else {
                "network"
            }
        }
        LlmProviderError::OpenAi(OpenAiError::Api { status, .. })
        | LlmProviderError::Anthropic(AnthropicError::Api { status, .. })
        | LlmProviderError::Gemini(GeminiError::Api { status, .. }) => {
            if status.as_u16() == 429 {
                "rate_limit"
            } else if status.is_server_error() {
                "server"
            } else {
                "api"
            }
        }
        _ => "stream",
    }
}

/// Execute async operation with retry
pub async fn retry_async<F, Fut, T, E>(
    config: RetryConfig,
    mut op: F,
    is_retryable: impl Fn(&E) -> bool,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_err: Option<E> = None;

    for attempt in 0..=config.max_retries {
        match op().await {
            Ok(v) => {
                if attempt > 0 {
                    tracing::info!("LLM succeeded after {} retries", attempt);
                }
                return Ok(v);
            }
            Err(e) => {
                let is_last = attempt == config.max_retries;
                if !is_last && is_retryable(&e) {
                    let delay = config.delay_for_attempt(attempt);
                    tracing::warn!(
                        "LLM failed (attempt {}/{}): {}. Retry in {:?}",
                        attempt + 1,
                        config.max_retries + 1,
                        e,
                        delay
                    );
                    sleep(delay).await;
                    last_err = Some(e);
                } else {
                    if is_last && is_retryable(&e) {
                        tracing::error!(
                            "LLM failed after {} retries: {}",
                            config.max_retries + 1,
                            e
                        );
                    }
                    return Err(e);
                }
            }
        }
    }

    Err(last_err.expect("retry loop should have returned"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_calculation() {
        let config = RetryConfig {
            jitter: false,
            ..Default::default()
        };
        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(1000));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(2000));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(4000));
    }

    #[test]
    fn test_status_retryable() {
        use reqwest::StatusCode;
        assert!(is_status_retryable(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_status_retryable(StatusCode::TOO_MANY_REQUESTS));
        assert!(!is_status_retryable(StatusCode::BAD_REQUEST));
        assert!(!is_status_retryable(StatusCode::UNAUTHORIZED));
    }
}
