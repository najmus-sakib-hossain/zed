//! Authentication protocol for DX Agent gateway.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Authentication request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    /// Authentication method
    pub method: AuthMethod,
    /// Client metadata
    pub client_info: ClientInfo,
}

/// Supported authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    /// Token-based authentication
    Token { token: String },
    /// API key authentication
    ApiKey { key: String },
    /// OAuth2 authorization code
    OAuth2 { code: String, redirect_uri: String },
    /// DM pairing code (for messaging channels)
    PairingCode { code: String, channel: String },
}

/// Client information sent during auth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub client_id: String,
    pub client_version: String,
    pub platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
}

/// Authentication challenge from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub challenge_id: String,
    pub challenge_type: ChallengeType,
    pub expires_at: DateTime<Utc>,
}

/// Types of auth challenges
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeType {
    /// Display pairing code on device
    PairingCode { code: String },
    /// OAuth2 redirect
    OAuth2Redirect { url: String },
    /// TOTP verification
    Totp,
}

/// Authentication response from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<AuthToken>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// JWT-style authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// The JWT token string
    pub access_token: String,
    /// Token type (Bearer)
    pub token_type: String,
    /// Expiry in seconds
    pub expires_in: u64,
    /// Refresh token for renewal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Granted scopes
    pub scopes: Vec<String>,
}

impl AuthResponse {
    pub fn success(token: AuthToken) -> Self {
        Self {
            success: true,
            token: Some(token),
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            token: None,
            error: Some(error.into()),
        }
    }
}

impl AuthToken {
    pub fn new(access_token: String, expires_in: u64, scopes: Vec<String>) -> Self {
        Self {
            access_token,
            token_type: "Bearer".into(),
            expires_in,
            refresh_token: None,
            scopes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_request_token() {
        let req = AuthRequest {
            method: AuthMethod::Token {
                token: "abc123".into(),
            },
            client_info: ClientInfo {
                client_id: "cli-1".into(),
                client_version: "0.1.0".into(),
                platform: "windows".into(),
                device_name: Some("dev-machine".into()),
            },
        };
        let json = serde_json::to_string(&req).expect("serialize");
        assert!(json.contains("abc123"));
    }

    #[test]
    fn test_auth_response_success() {
        let token = AuthToken::new("tok_xyz".into(), 3600, vec!["read".into(), "write".into()]);
        let resp = AuthResponse::success(token);
        assert!(resp.success);
        assert!(resp.error.is_none());
    }
}
