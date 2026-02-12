/*!
 * Unified Error Handling Module
 *
 * Provides an internationalization-based error handling system with convenient macros and wrappers.
 * Integrates with ApiResponse and i18n modules to implement a unified error response format.
 */

use crate::utils::i18n::I18nManager;
use crate::utils::{ApiResponse, EmptyData, TauriApiResult};
use std::collections::HashMap;

/// Unified error response macro - uses i18n key
///
/// Usage:
/// - `api_error!("common.operation_failed")` - simple error
/// - `api_error!("error.with_param", "name" => "filename")` - error with parameters
#[macro_export]
macro_rules! api_error {
    ($key:expr) => {
        $crate::utils::ApiResponse::error($crate::t!($key))
    };

    ($key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {
        $crate::utils::ApiResponse::error($crate::t!($key, $($param_key => $param_value),+))
    };
}

/// Unified success response macro
///
/// Usage:
/// - `api_success!(data)` - success response with data
/// - `api_success!()` - success response without data
/// - `api_success!(data, "success.message")` - success response with message
#[macro_export]
macro_rules! api_success {
    () => {
        $crate::utils::ApiResponse::ok($crate::utils::EmptyData::default())
    };

    ($data:expr) => {
        $crate::utils::ApiResponse::ok($data)
    };

    ($data:expr, $message_key:expr) => {
        $crate::utils::ApiResponse::ok_with_message($data, $crate::t!($message_key))
    };
}

/// Tauri command wrapper trait
///
/// Provides convenient conversion methods for Result types, automatically handling errors and returning ApiResponse
pub trait TauriCommandWrapper<T> {
    /// Convert to TauriApiResult using default error message
    fn to_api_response(self) -> TauriApiResult<T>;

    /// Convert to TauriApiResult using specified error i18n key
    fn to_api_response_with_error(self, error_key: &str) -> TauriApiResult<T>;

    /// Convert to TauriApiResult using error i18n key with parameters
    fn to_api_response_with_params(
        self,
        error_key: &str,
        params: HashMap<String, String>,
    ) -> TauriApiResult<T>;
}

impl<T, E> TauriCommandWrapper<T> for Result<T, E>
where
    E: std::fmt::Display + std::fmt::Debug,
{
    fn to_api_response(self) -> TauriApiResult<T> {
        match self {
            Ok(data) => Ok(api_success!(data)),
            Err(_) => Ok(api_error!("common.operation_failed")),
        }
    }

    fn to_api_response_with_error(self, error_key: &str) -> TauriApiResult<T> {
        match self {
            Ok(data) => Ok(api_success!(data)),
            Err(_) => Ok(api_error!(error_key)),
        }
    }

    fn to_api_response_with_params(
        self,
        error_key: &str,
        params: HashMap<String, String>,
    ) -> TauriApiResult<T> {
        match self {
            Ok(data) => Ok(api_success!(data)),
            Err(_) => {
                let message = I18nManager::get_text(error_key, Some(&params));
                Ok(ApiResponse::error(message))
            }
        }
    }
}

/// Provides convenient conversion methods for Option types
pub trait OptionToApiResponse<T> {
    /// Convert Option to TauriApiResult, using default error when None
    fn to_api_response(self) -> TauriApiResult<T>;

    /// Convert Option to TauriApiResult, using specified error key when None
    fn to_api_response_or_error(self, error_key: &str) -> TauriApiResult<T>;
}

impl<T> OptionToApiResponse<T> for Option<T> {
    fn to_api_response(self) -> TauriApiResult<T> {
        match self {
            Some(data) => Ok(api_success!(data)),
            None => Ok(api_error!("common.not_found")),
        }
    }

    fn to_api_response_or_error(self, error_key: &str) -> TauriApiResult<T> {
        match self {
            Some(data) => Ok(api_success!(data)),
            None => Ok(api_error!(error_key)),
        }
    }
}

/// Parameter validation macro
///
/// Usage:
/// - `validate_param!(value > 0, "common.invalid_params")` - conditional validation
/// - `validate_not_empty!(text, "common.invalid_params")` - non-empty validation
#[macro_export]
macro_rules! validate_param {
    ($condition:expr, $error_key:expr) => {
        if !($condition) {
            return Ok($crate::api_error!($error_key));
        }
    };

    ($condition:expr, $error_key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {
        if !($condition) {
            return Ok($crate::api_error!($error_key, $($param_key => $param_value),+));
        }
    };
}

/// Non-empty string validation macro
#[macro_export]
macro_rules! validate_not_empty {
    ($value:expr, $error_key:expr) => {
        $crate::validate_param!(!$value.trim().is_empty(), $error_key);
    };

    ($value:expr, $error_key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {
        $crate::validate_param!(!$value.trim().is_empty(), $error_key, $($param_key => $param_value),+);
    };
}

/// ID validation macro (greater than 0)
#[macro_export]
macro_rules! validate_id {
    ($id:expr, $error_key:expr) => {
        $crate::validate_param!($id > 0, $error_key);
    };

    ($id:expr, $error_key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {
        $crate::validate_param!($id > 0, $error_key, $($param_key => $param_value),+);
    };
}

/// Range validation macro
#[macro_export]
macro_rules! validate_range {
    ($value:expr, $min:expr, $max:expr, $error_key:expr) => {
        $crate::validate_param!($value >= $min && $value <= $max, $error_key);
    };

    ($value:expr, $min:expr, $max:expr, $error_key:expr, $($param_key:expr => $param_value:expr),+ $(,)?) => {
        $crate::validate_param!($value >= $min && $value <= $max, $error_key, $($param_key => $param_value),+);
    };
}

/// Error handling utility functions
pub struct ErrorHandler;

impl ErrorHandler {
    /// Create error response with parameters
    pub fn create_error_with_params<T>(
        error_key: &str,
        params: HashMap<String, String>,
    ) -> TauriApiResult<T> {
        let message = I18nManager::get_text(error_key, Some(&params));
        Ok(ApiResponse::error(message))
    }

    /// Create simple error response
    pub fn create_error<T>(error_key: &str) -> TauriApiResult<T> {
        Ok(api_error!(error_key))
    }

    /// Create success response
    pub fn create_success<T>(data: T) -> TauriApiResult<T> {
        Ok(api_success!(data))
    }

    /// Create empty success response
    pub fn create_empty_success() -> TauriApiResult<EmptyData> {
        Ok(api_success!())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_result_wrapper() {
        let _success_result: Result<i32, &str> = Ok(42);
        let _error_result: Result<i32, &str> = Err("test error");

        // i18n initialization required for testing
    }

    #[test]
    fn test_option_wrapper() {
        let _some_value: Option<i32> = Some(42);
        let _none_value: Option<i32> = None;

        // i18n initialization required for testing
    }
}
