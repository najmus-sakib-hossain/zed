//! # Authentication System
//!
//! Secure OAuth2 flows and token management for all integrations.
//! Stores tokens securely using the system keyring.

use oauth2::{CsrfToken, PkceCodeChallenge};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::{AgentError, Result};

/// Authentication provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProvider {
    /// Provider name (github, google, spotify, etc.)
    pub name: String,

    /// OAuth2 client ID
    pub client_id: String,

    /// OAuth2 client secret (optional for PKCE flows)
    pub client_secret: Option<String>,

    /// Authorization endpoint URL
    pub auth_url: String,

    /// Token endpoint URL
    pub token_url: String,

    /// Required scopes
    pub scopes: Vec<String>,

    /// Redirect URI for OAuth callback
    pub redirect_uri: String,
}

impl AuthProvider {
    /// Create a new auth provider for common services
    pub fn github() -> Self {
        Self {
            name: "github".to_string(),
            client_id: String::new(),
            client_secret: None,
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            scopes: vec!["repo".to_string(), "user".to_string()],
            redirect_uri: "http://localhost:8765/callback".to_string(),
        }
    }

    pub fn google() -> Self {
        Self {
            name: "google".to_string(),
            client_id: String::new(),
            client_secret: None,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            scopes: vec!["https://www.googleapis.com/auth/gmail.readonly".to_string()],
            redirect_uri: "http://localhost:8765/callback".to_string(),
        }
    }

    pub fn spotify() -> Self {
        Self {
            name: "spotify".to_string(),
            client_id: String::new(),
            client_secret: None,
            auth_url: "https://accounts.spotify.com/authorize".to_string(),
            token_url: "https://accounts.spotify.com/api/token".to_string(),
            scopes: vec![
                "user-read-playback-state".to_string(),
                "user-modify-playback-state".to_string(),
            ],
            redirect_uri: "http://localhost:8765/callback".to_string(),
        }
    }

    pub fn notion() -> Self {
        Self {
            name: "notion".to_string(),
            client_id: String::new(),
            client_secret: None,
            auth_url: "https://api.notion.com/v1/oauth/authorize".to_string(),
            token_url: "https://api.notion.com/v1/oauth/token".to_string(),
            scopes: vec![],
            redirect_uri: "http://localhost:8765/callback".to_string(),
        }
    }

    pub fn discord() -> Self {
        Self {
            name: "discord".to_string(),
            client_id: String::new(),
            client_secret: None,
            auth_url: "https://discord.com/api/oauth2/authorize".to_string(),
            token_url: "https://discord.com/api/oauth2/token".to_string(),
            scopes: vec!["bot".to_string(), "messages.read".to_string()],
            redirect_uri: "http://localhost:8765/callback".to_string(),
        }
    }

    pub fn slack() -> Self {
        Self {
            name: "slack".to_string(),
            client_id: String::new(),
            client_secret: None,
            auth_url: "https://slack.com/oauth/v2/authorize".to_string(),
            token_url: "https://slack.com/api/oauth.v2.access".to_string(),
            scopes: vec!["chat:write".to_string(), "channels:read".to_string()],
            redirect_uri: "http://localhost:8765/callback".to_string(),
        }
    }
}

/// OAuth2 flow handler
pub struct OAuthFlow {
    provider: AuthProvider,
    pkce_verifier: Option<String>,
    csrf_token: Option<String>,
}

impl OAuthFlow {
    pub fn new(provider: AuthProvider) -> Self {
        Self {
            provider,
            pkce_verifier: None,
            csrf_token: None,
        }
    }

    /// Start the OAuth2 authorization flow
    /// Returns the authorization URL to open in a browser
    pub fn start(&mut self) -> Result<String> {
        // Generate PKCE challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let csrf_token = CsrfToken::new_random();

        self.pkce_verifier = Some(pkce_verifier.secret().clone());
        self.csrf_token = Some(csrf_token.secret().clone());

        // Build authorization URL
        let scopes: String = self.provider.scopes.join(" ");
        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&scope={}&response_type=code&state={}&code_challenge={}&code_challenge_method=S256",
            self.provider.auth_url,
            self.provider.client_id,
            urlencoding::encode(&self.provider.redirect_uri),
            urlencoding::encode(&scopes),
            csrf_token.secret(),
            pkce_challenge.as_str()
        );

        info!("OAuth flow started for {}", self.provider.name);
        Ok(auth_url)
    }

    /// Exchange the authorization code for tokens
    pub async fn exchange(&self, code: &str, state: &str) -> Result<TokenResponse> {
        // Verify CSRF token
        if let Some(expected) = &self.csrf_token {
            if state != expected {
                return Err(AgentError::AuthFailed {
                    provider: self.provider.name.clone(),
                    message: "CSRF token mismatch".to_string(),
                });
            }
        }

        // Exchange code for tokens
        let client = reqwest::Client::new();

        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("redirect_uri", &self.provider.redirect_uri);
        params.insert("client_id", &self.provider.client_id);

        if let Some(verifier) = &self.pkce_verifier {
            params.insert("code_verifier", verifier);
        }

        if let Some(secret) = &self.provider.client_secret {
            params.insert("client_secret", secret);
        }

        let response = client
            .post(&self.provider.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AgentError::NetworkError(e.to_string()))?;

        let token_response: TokenResponse =
            response.json().await.map_err(|e| AgentError::AuthFailed {
                provider: self.provider.name.clone(),
                message: e.to_string(),
            })?;

        info!("OAuth tokens obtained for {}", self.provider.name);
        Ok(token_response)
    }

    /// Refresh an expired access token
    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenResponse> {
        let client = reqwest::Client::new();

        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token);
        params.insert("client_id", &self.provider.client_id);

        if let Some(secret) = &self.provider.client_secret {
            params.insert("client_secret", secret);
        }

        let response = client
            .post(&self.provider.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AgentError::NetworkError(e.to_string()))?;

        let token_response: TokenResponse =
            response.json().await.map_err(|e| AgentError::AuthFailed {
                provider: self.provider.name.clone(),
                message: e.to_string(),
            })?;

        info!("Tokens refreshed for {}", self.provider.name);
        Ok(token_response)
    }
}

/// OAuth2 token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Secure token storage using system keyring
pub struct TokenStore {
    service_name: String,
}

impl TokenStore {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    /// Store a token securely
    pub fn store(&self, provider: &str, token: &TokenResponse) -> Result<()> {
        let entry = keyring::Entry::new(&self.service_name, provider).map_err(|e| {
            AgentError::AuthFailed {
                provider: provider.to_string(),
                message: format!("Keyring error: {}", e),
            }
        })?;

        let token_json = serde_json::to_string(token)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        entry
            .set_password(&token_json)
            .map_err(|e| AgentError::AuthFailed {
                provider: provider.to_string(),
                message: format!("Failed to store token: {}", e),
            })?;

        info!("Token stored for {}", provider);
        Ok(())
    }

    /// Retrieve a stored token
    pub fn get(&self, provider: &str) -> Result<Option<TokenResponse>> {
        let entry = keyring::Entry::new(&self.service_name, provider).map_err(|e| {
            AgentError::AuthFailed {
                provider: provider.to_string(),
                message: format!("Keyring error: {}", e),
            }
        })?;

        match entry.get_password() {
            Ok(token_json) => {
                let token: TokenResponse = serde_json::from_str(&token_json)
                    .map_err(|e| AgentError::SerializationError(e.to_string()))?;
                Ok(Some(token))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AgentError::AuthFailed {
                provider: provider.to_string(),
                message: format!("Failed to retrieve token: {}", e),
            }),
        }
    }

    /// Delete a stored token
    pub fn delete(&self, provider: &str) -> Result<()> {
        let entry = keyring::Entry::new(&self.service_name, provider).map_err(|e| {
            AgentError::AuthFailed {
                provider: provider.to_string(),
                message: format!("Keyring error: {}", e),
            }
        })?;

        entry
            .delete_credential()
            .map_err(|e| AgentError::AuthFailed {
                provider: provider.to_string(),
                message: format!("Failed to delete token: {}", e),
            })?;

        info!("Token deleted for {}", provider);
        Ok(())
    }

    /// Check if a token exists for a provider
    pub fn has_token(&self, provider: &str) -> bool {
        self.get(provider).map(|t| t.is_some()).unwrap_or(false)
    }
}
