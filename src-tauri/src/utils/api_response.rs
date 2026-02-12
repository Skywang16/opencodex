use serde::{Deserialize, Serialize};

/// Empty data type for APIs with no data return
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EmptyData;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiResponse<T> {
    pub code: u16,
    pub message: Option<String>,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    /// Success response (no message)
    pub fn ok(data: T) -> Self {
        Self {
            code: 200,
            message: None,
            data: Some(data),
        }
    }

    /// Success response (with message)
    pub fn ok_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            code: 200,
            message: Some(message.into()),
            data: Some(data),
        }
    }

    /// Error response (i18n completed)
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            code: 500,
            message: Some(message.into()),
            data: None,
        }
    }
}

/// Tauri command specific result type
pub type TauriApiResult<T> = Result<ApiResponse<T>, String>;
