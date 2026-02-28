//! Standardized JSON Output for DX CLI
//!
//! Provides consistent JSON response structures across all commands.
//! Feature: cli-production-ready, Tasks 10.1-10.4
//! Validates: Requirements 10.4, 10.5

use serde::{Deserialize, Serialize};

use crate::utils::update::CURRENT_VERSION;

/// Standard success response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse<T> {
    /// Always true for success responses
    pub success: bool,
    /// CLI version that generated this response
    pub version: String,
    /// The actual result data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T> SuccessResponse<T> {
    /// Create a success response with data
    pub fn with_data(data: T) -> Self {
        Self {
            success: true,
            version: CURRENT_VERSION.to_string(),
            data: Some(data),
            message: None,
        }
    }

    /// Create a success response with just a message
    pub fn with_message(message: impl Into<String>) -> Self
    where
        T: Default,
    {
        Self {
            success: true,
            version: CURRENT_VERSION.to_string(),
            data: None,
            message: Some(message.into()),
        }
    }

    /// Add a message to the response
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String
    where
        T: Serialize,
    {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| {
            r#"{"success":true,"version":"unknown","error":"serialization_failed"}"#.to_string()
        })
    }

    /// Print as JSON to stdout
    pub fn print(&self)
    where
        T: Serialize,
    {
        println!("{}", self.to_json());
    }
}

/// Create a simple success response without data
pub fn success_message(message: impl Into<String>) -> SuccessResponse<()> {
    SuccessResponse {
        success: true,
        version: CURRENT_VERSION.to_string(),
        data: None,
        message: Some(message.into()),
    }
}

/// Standard error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Always false for error responses
    pub success: bool,
    /// CLI version that generated this response
    pub version: String,
    /// Error message
    pub error: String,
    /// Error code (machine-readable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Helpful hint for resolving the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a basic error response
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            success: false,
            version: CURRENT_VERSION.to_string(),
            error: error.into(),
            code: None,
            hint: None,
            details: None,
        }
    }

    /// Create an error response with a code
    pub fn with_code(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            success: false,
            version: CURRENT_VERSION.to_string(),
            error: error.into(),
            code: Some(code.into()),
            hint: None,
            details: None,
        }
    }

    /// Add an error code
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Add a helpful hint
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Add additional details
    pub fn details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| {
            format!(
                r#"{{"success":false,"version":"{}","error":"serialization_failed"}}"#,
                CURRENT_VERSION
            )
        })
    }

    /// Print as JSON to stderr
    pub fn print(&self) {
        eprintln!("{}", self.to_json());
    }

    /// Create from a DxError
    pub fn from_dx_error(err: &crate::utils::error::DxError) -> Self {
        use crate::utils::error::DxError;

        let (code, hint) = match err {
            DxError::ConfigNotFound { path } => (
                Some("CONFIG_NOT_FOUND".to_string()),
                Some(format!("Create a config file at {}", path.display())),
            ),
            DxError::ConfigInvalid { path, line, .. } => (
                Some("CONFIG_INVALID".to_string()),
                Some(format!("Check {} at line {}", path.display(), line)),
            ),
            DxError::FileNotFound { path } => (
                Some("FILE_NOT_FOUND".to_string()),
                Some(format!("Verify the path exists: {}", path.display())),
            ),
            DxError::PermissionDenied { path } => (
                Some("PERMISSION_DENIED".to_string()),
                Some(format!("Check permissions for: {}", path.display())),
            ),
            DxError::Network { .. } => (
                Some("NETWORK_ERROR".to_string()),
                Some("Check your internet connection".to_string()),
            ),
            DxError::Timeout { timeout_secs } => (
                Some("TIMEOUT".to_string()),
                Some(format!("Operation timed out after {}s", timeout_secs)),
            ),
            DxError::ToolNotInstalled { name } => (
                Some("TOOL_NOT_INSTALLED".to_string()),
                Some(format!("Install {} with `dx forge install {}`", name, name)),
            ),
            _ => (None, None),
        };

        Self {
            success: false,
            version: CURRENT_VERSION.to_string(),
            error: err.to_string(),
            code,
            hint,
            details: None,
        }
    }
}

/// Helper macro for creating JSON output
#[macro_export]
macro_rules! json_output {
    // Success with data
    (success: $data:expr) => {
        $crate::utils::json_output::SuccessResponse::with_data($data)
    };
    // Success with message only
    (message: $msg:expr) => {
        $crate::utils::json_output::success_message($msg)
    };
    // Error
    (error: $err:expr) => {
        $crate::utils::json_output::ErrorResponse::new($err)
    };
    // Error with code
    (error: $err:expr, code: $code:expr) => {
        $crate::utils::json_output::ErrorResponse::with_code($err, $code)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_response_with_data() {
        let resp = SuccessResponse::with_data(vec!["item1", "item2"]);
        assert!(resp.success);
        assert_eq!(resp.version, CURRENT_VERSION);
        assert!(resp.data.is_some());
    }

    #[test]
    fn test_success_response_json() {
        let resp = SuccessResponse::with_data("test");
        let json = resp.to_json();
        assert!(json.contains("\"success\": true"));
        assert!(json.contains("\"version\":"));
        assert!(json.contains("\"data\": \"test\""));
    }

    #[test]
    fn test_error_response() {
        let resp = ErrorResponse::new("Something went wrong");
        assert!(!resp.success);
        assert_eq!(resp.error, "Something went wrong");
    }

    #[test]
    fn test_error_response_with_code_and_hint() {
        let resp = ErrorResponse::with_code("File not found", "FILE_NOT_FOUND")
            .hint("Check if the file exists");

        assert!(!resp.success);
        assert_eq!(resp.code, Some("FILE_NOT_FOUND".to_string()));
        assert!(resp.hint.is_some());
    }

    #[test]
    fn test_error_response_json() {
        let resp = ErrorResponse::with_code("Test error", "TEST_ERROR").hint("This is a hint");
        let json = resp.to_json();

        assert!(json.contains("\"success\": false"));
        assert!(json.contains("\"error\": \"Test error\""));
        assert!(json.contains("\"code\": \"TEST_ERROR\""));
        assert!(json.contains("\"hint\":"));
    }

    #[test]
    fn test_success_message_helper() {
        let resp = success_message("Operation completed");
        assert!(resp.success);
        assert_eq!(resp.message, Some("Operation completed".to_string()));
        assert!(resp.data.is_none());
    }
}
