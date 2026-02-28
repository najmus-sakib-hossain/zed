//! Configurable error pages for dx-server.
//!
//! This module provides customizable error page rendering with support for:
//! - Custom HTML templates for different HTTP status codes (404, 500, etc.)
//! - Request ID inclusion for error correlation
//! - Environment-aware error detail exposure
//! - Default fallback templates

use axum::{
    http::{StatusCode, header},
    response::Response,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration for error pages.
///
/// Allows customization of error page templates for different HTTP status codes.
/// Templates can include placeholders that will be replaced at render time:
/// - `{{status_code}}` - The HTTP status code (e.g., 404, 500)
/// - `{{status_text}}` - The HTTP status text (e.g., "Not Found", "Internal Server Error")
/// - `{{message}}` - The error message
/// - `{{request_id}}` - The unique request ID for correlation
/// - `{{details}}` - Additional error details (only shown in development mode)
#[derive(Debug, Clone)]
pub struct ErrorPageConfig {
    /// Custom templates by status code
    templates: HashMap<u16, String>,
    /// Whether to show detailed error information
    show_details: bool,
    /// Default template for unspecified status codes
    default_template: String,
}

impl Default for ErrorPageConfig {
    fn default() -> Self {
        Self::production()
    }
}

impl ErrorPageConfig {
    /// Create a new error page configuration with production defaults.
    ///
    /// Production mode hides internal error details from users.
    pub fn production() -> Self {
        Self {
            templates: HashMap::new(),
            show_details: false,
            default_template: DEFAULT_ERROR_TEMPLATE.to_string(),
        }
    }

    /// Create a new error page configuration with development defaults.
    ///
    /// Development mode shows full error details for debugging.
    pub fn development() -> Self {
        Self {
            templates: HashMap::new(),
            show_details: true,
            default_template: DEFAULT_ERROR_TEMPLATE.to_string(),
        }
    }

    /// Set a custom template for a specific status code.
    ///
    /// # Arguments
    ///
    /// * `status_code` - The HTTP status code (e.g., 404, 500)
    /// * `template` - The HTML template string with placeholders
    ///
    /// # Example
    ///
    /// ```rust
    /// use dx_www_server::error_pages::ErrorPageConfig;
    ///
    /// let mut config = ErrorPageConfig::production();
    /// config.set_template(404, r#"
    ///     <html>
    ///         <body>
    ///             <h1>Page Not Found</h1>
    ///             <p>Request ID: {{request_id}}</p>
    ///         </body>
    ///     </html>
    /// "#.to_string());
    /// ```
    pub fn set_template(&mut self, status_code: u16, template: String) {
        self.templates.insert(status_code, template);
    }

    /// Set the default template for status codes without custom templates.
    pub fn set_default_template(&mut self, template: String) {
        self.default_template = template;
    }

    /// Set whether to show detailed error information.
    pub fn set_show_details(&mut self, show: bool) {
        self.show_details = show;
    }

    /// Check if detailed error information should be shown.
    pub fn show_details(&self) -> bool {
        self.show_details
    }

    /// Get the template for a specific status code.
    ///
    /// Returns the custom template if set, otherwise returns the default template.
    pub fn get_template(&self, status_code: u16) -> &str {
        self.templates
            .get(&status_code)
            .map(|s| s.as_str())
            .unwrap_or(&self.default_template)
    }

    /// Render an error page for the given status code and message.
    ///
    /// # Arguments
    ///
    /// * `status` - The HTTP status code
    /// * `message` - The error message to display
    /// * `details` - Optional detailed error information (only shown if `show_details` is true)
    /// * `request_id` - The unique request ID for correlation
    ///
    /// # Returns
    ///
    /// An Axum Response with the rendered error page.
    pub fn render(
        &self,
        status: StatusCode,
        message: &str,
        details: Option<&str>,
        request_id: &str,
    ) -> Response {
        let template = self.get_template(status.as_u16());

        let status_text = status.canonical_reason().unwrap_or("Error");
        let details_html = if self.show_details {
            details.unwrap_or("")
        } else {
            ""
        };

        let html = template
            .replace("{{status_code}}", &status.as_u16().to_string())
            .replace("{{status_text}}", status_text)
            .replace("{{message}}", message)
            .replace("{{request_id}}", request_id)
            .replace("{{details}}", details_html);

        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header("X-Request-ID", request_id)
            .body(axum::body::Body::from(html))
            .unwrap_or_else(|_| Response::new(axum::body::Body::from("Internal Server Error")))
    }

    /// Render an error page with an auto-generated request ID.
    pub fn render_with_new_request_id(
        &self,
        status: StatusCode,
        message: &str,
        details: Option<&str>,
    ) -> Response {
        let request_id = Uuid::new_v4().to_string();
        self.render(status, message, details, &request_id)
    }
}

/// Default error page template.
///
/// This template provides a clean, professional error page with:
/// - Status code and text
/// - Error message
/// - Request ID for support correlation
/// - Optional details section (controlled by `show_details`)
pub const DEFAULT_ERROR_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{status_code}} {{status_text}} - DX WWW</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f8fafc;
            color: #1e293b;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 1rem;
        }
        .container {
            max-width: 600px;
            width: 100%;
            background: white;
            border-radius: 1rem;
            box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1);
            padding: 2rem;
        }
        .status-code {
            font-size: 4rem;
            font-weight: 700;
            color: #dc2626;
            line-height: 1;
        }
        .status-text {
            font-size: 1.5rem;
            font-weight: 600;
            color: #475569;
            margin-top: 0.5rem;
        }
        .message {
            margin-top: 1.5rem;
            padding: 1rem;
            background: #fef2f2;
            border: 1px solid #fecaca;
            border-radius: 0.5rem;
            color: #991b1b;
        }
        .details {
            margin-top: 1rem;
            padding: 1rem;
            background: #f1f5f9;
            border-radius: 0.5rem;
            font-family: monospace;
            font-size: 0.875rem;
            white-space: pre-wrap;
            word-break: break-all;
            color: #475569;
        }
        .request-id {
            margin-top: 1.5rem;
            font-size: 0.75rem;
            color: #94a3b8;
        }
        .request-id code {
            background: #f1f5f9;
            padding: 0.25rem 0.5rem;
            border-radius: 0.25rem;
            font-family: monospace;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="status-code">{{status_code}}</div>
        <div class="status-text">{{status_text}}</div>
        <div class="message">{{message}}</div>
        {{details}}
        <div class="request-id">Request ID: <code>{{request_id}}</code></div>
    </div>
</body>
</html>"#;

/// Pre-configured 404 Not Found template.
pub const NOT_FOUND_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>404 Not Found - DX WWW</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f8fafc;
            color: #1e293b;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 1rem;
        }
        .container {
            max-width: 600px;
            width: 100%;
            text-align: center;
        }
        .status-code {
            font-size: 8rem;
            font-weight: 700;
            color: #667eea;
            line-height: 1;
        }
        .status-text {
            font-size: 1.5rem;
            font-weight: 600;
            color: #475569;
            margin-top: 0.5rem;
        }
        .message {
            margin-top: 1.5rem;
            color: #64748b;
        }
        .home-link {
            display: inline-block;
            margin-top: 2rem;
            padding: 0.75rem 1.5rem;
            background: #667eea;
            color: white;
            text-decoration: none;
            border-radius: 0.5rem;
            font-weight: 500;
            transition: background 0.2s;
        }
        .home-link:hover {
            background: #5a67d8;
        }
        .request-id {
            margin-top: 2rem;
            font-size: 0.75rem;
            color: #94a3b8;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="status-code">404</div>
        <div class="status-text">Page Not Found</div>
        <p class="message">{{message}}</p>
        <a href="/" class="home-link">Go Home</a>
        <div class="request-id">Request ID: {{request_id}}</div>
    </div>
</body>
</html>"#;

/// Pre-configured 500 Internal Server Error template.
pub const INTERNAL_ERROR_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>500 Internal Server Error - DX WWW</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f8fafc;
            color: #1e293b;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 1rem;
        }
        .container {
            max-width: 600px;
            width: 100%;
            text-align: center;
        }
        .status-code {
            font-size: 8rem;
            font-weight: 700;
            color: #dc2626;
            line-height: 1;
        }
        .status-text {
            font-size: 1.5rem;
            font-weight: 600;
            color: #475569;
            margin-top: 0.5rem;
        }
        .message {
            margin-top: 1.5rem;
            padding: 1rem;
            background: #fef2f2;
            border: 1px solid #fecaca;
            border-radius: 0.5rem;
            color: #991b1b;
        }
        .details {
            margin-top: 1rem;
            padding: 1rem;
            background: #f1f5f9;
            border-radius: 0.5rem;
            font-family: monospace;
            font-size: 0.875rem;
            white-space: pre-wrap;
            word-break: break-all;
            color: #475569;
            text-align: left;
        }
        .request-id {
            margin-top: 2rem;
            font-size: 0.75rem;
            color: #94a3b8;
        }
        .request-id code {
            background: #f1f5f9;
            padding: 0.25rem 0.5rem;
            border-radius: 0.25rem;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="status-code">500</div>
        <div class="status-text">Internal Server Error</div>
        <div class="message">{{message}}</div>
        {{details}}
        <div class="request-id">Request ID: <code>{{request_id}}</code></div>
    </div>
</body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_config_hides_details() {
        let config = ErrorPageConfig::production();
        assert!(!config.show_details());
    }

    #[test]
    fn test_development_config_shows_details() {
        let config = ErrorPageConfig::development();
        assert!(config.show_details());
    }

    #[test]
    fn test_custom_template() {
        let mut config = ErrorPageConfig::production();
        config.set_template(404, "Custom 404: {{message}}".to_string());

        assert_eq!(config.get_template(404), "Custom 404: {{message}}");
        assert_eq!(config.get_template(500), DEFAULT_ERROR_TEMPLATE);
    }

    #[test]
    fn test_render_replaces_placeholders() {
        let config = ErrorPageConfig::production();
        let response =
            config.render(StatusCode::NOT_FOUND, "Page not found", None, "test-request-id");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.headers().contains_key("x-request-id"));
        assert_eq!(
            response.headers().get("x-request-id").unwrap().to_str().unwrap(),
            "test-request-id"
        );
    }

    #[test]
    fn test_render_with_details_in_development() {
        let config = ErrorPageConfig::development();
        let response = config.render(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong",
            Some("Stack trace here"),
            "test-id",
        );

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_render_hides_details_in_production() {
        let config = ErrorPageConfig::production();
        let response = config.render(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong",
            Some("Stack trace here"),
            "test-id",
        );

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_render_with_new_request_id() {
        let config = ErrorPageConfig::production();
        let response = config.render_with_new_request_id(StatusCode::NOT_FOUND, "Not found", None);

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.headers().contains_key("x-request-id"));

        // Request ID should be a valid UUID
        let request_id = response.headers().get("x-request-id").unwrap().to_str().unwrap();
        assert!(Uuid::parse_str(request_id).is_ok());
    }
}
