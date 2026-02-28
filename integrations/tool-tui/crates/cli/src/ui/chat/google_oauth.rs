//! Google OAuth authentication for Gemini API access

use anyhow::Result;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    RefreshToken, Scope, TokenResponse, TokenUrl, basic::BasicClient, reqwest::async_http_client,
};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use tokio::sync::oneshot;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
}

impl GoogleOAuthConfig {
    /// Load OAuth config from client-secret.json
    pub fn load_from_file() -> Result<Self> {
        let path = PathBuf::from("client-sercet.json");
        let content = std::fs::read_to_string(&path)?;

        #[derive(Deserialize)]
        struct ClientSecretFile {
            web: WebConfig,
        }

        #[derive(Deserialize)]
        struct WebConfig {
            client_id: String,
            client_secret: String,
            redirect_uris: Vec<String>,
        }

        let config: ClientSecretFile = serde_json::from_str(&content)?;
        let redirect_uri = config
            .web
            .redirect_uris
            .first()
            .ok_or_else(|| anyhow::anyhow!("No redirect URI found"))?
            .clone();

        Ok(Self {
            client_id: config.web.client_id,
            client_secret: config.web.client_secret,
            redirect_uri,
        })
    }

    /// Get token file path
    fn token_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".dx_google_token.json")
    }

    /// Load stored token
    pub fn load_token() -> Result<Option<StoredToken>> {
        let path = Self::token_path();
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)?;
        let token: StoredToken = serde_json::from_str(&content)?;
        Ok(Some(token))
    }

    /// Save token to disk
    pub fn save_token(token: &StoredToken) -> Result<()> {
        let path = Self::token_path();
        let content = serde_json::to_string_pretty(token)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Perform OAuth flow
    pub async fn authenticate(&self) -> Result<StoredToken> {
        let client = BasicClient::new(
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?,
            Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?),
        )
        .set_redirect_uri(RedirectUrl::new(self.redirect_uri.clone())?);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/cloud-platform".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/cclog".to_string()))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/experimentsandconfigs".to_string(),
            ))
            .set_pkce_challenge(pkce_challenge)
            .url();

        // Open browser for authentication
        webbrowser::open(auth_url.as_ref())?;

        // Listen for callback
        let code = Self::listen_for_code(csrf_token.secret()).await?;

        // Exchange code for token
        let token_result = client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await?;

        let stored_token = StoredToken {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
            expires_at: token_result.expires_in().map(|d| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + d.as_secs()
            }),
        };

        Self::save_token(&stored_token)?;
        Ok(stored_token)
    }

    /// Listen for OAuth callback
    async fn listen_for_code(expected_state: &str) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        let listener = TcpListener::bind("127.0.0.1:8080")?;

        let expected_state = expected_state.to_string();
        tokio::spawn(async move {
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let mut reader = BufReader::new(&mut stream);
                    let mut request_line = String::new();
                    if reader.read_line(&mut request_line).is_err() {
                        continue;
                    }

                    let redirect_url = request_line.split_whitespace().nth(1).unwrap_or("");
                    if let Ok(url) = Url::parse(&format!("http://localhost{}", redirect_url)) {
                        let code = url
                            .query_pairs()
                            .find(|(k, _)| k == "code")
                            .map(|(_, v)| v.to_string());
                        let state = url
                            .query_pairs()
                            .find(|(k, _)| k == "state")
                            .map(|(_, v)| v.to_string());

                        let response = if state.as_ref().map(|s| s.as_str())
                            == Some(expected_state.as_str())
                            && code.is_some()
                        {
                            "Authentication successful! You can close this tab."
                        } else {
                            "Authentication failed."
                        };

                        let http_response = format!(
                            "HTTP/1.1 200 OK\r\n\r\n<html><body>{}</body></html>",
                            response
                        );
                        use std::io::Write;
                        let _ = stream.write_all(http_response.as_bytes());

                        if let Some(code) = code {
                            let _ = tx.send(code);
                        }
                        break;
                    }
                }
            }
        });

        Ok(rx.await?)
    }

    /// Refresh access token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<StoredToken> {
        let client = BasicClient::new(
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?,
            Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?),
        );

        let token_result = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
            .request_async(async_http_client)
            .await?;

        let stored_token = StoredToken {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: Some(refresh_token.to_string()),
            expires_at: token_result.expires_in().map(|d| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + d.as_secs()
            }),
        };

        Self::save_token(&stored_token)?;
        Ok(stored_token)
    }

    /// Get valid access token (refresh if needed)
    pub async fn get_access_token(&self) -> Result<String> {
        if let Some(token) = Self::load_token()? {
            // Check if token is expired
            if let Some(expires_at) = token.expires_at {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                if now < expires_at {
                    return Ok(token.access_token);
                }

                // Token expired, try to refresh
                if let Some(refresh_token) = token.refresh_token {
                    let new_token = self.refresh_token(&refresh_token).await?;
                    return Ok(new_token.access_token);
                }
            } else {
                // No expiry info, assume valid
                return Ok(token.access_token);
            }
        }

        // No token or refresh failed, do full auth
        let token = self.authenticate().await?;
        Ok(token.access_token)
    }
}

/// Call Gemini API with OAuth token
pub async fn call_gemini(access_token: &str, model: &str, prompt: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    );

    let res: serde_json::Value = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&serde_json::json!({
            "contents": [{
                "role": "user",
                "parts": [{"text": prompt}]
            }]
        }))
        .send()
        .await?
        .json()
        .await?;

    let text = res["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("No response")
        .to_string();

    Ok(text)
}

/// Fetch available Google Antigravity models (includes Claude, Gemini, etc.)
/// Antigravity models are accessed through the Gemini API with special model names
pub async fn fetch_antigravity_models(_access_token: &str) -> Result<Vec<String>> {
    // Hardcoded list of Antigravity models based on OpenCode implementation
    // These are the models available through Google's Antigravity platform
    let models = vec![
        // Gemini models with thinking
        "gemini-3-pro-preview".to_string(),
        "gemini-3-flash".to_string(),
        "gemini-2.5-flash-lite".to_string(),
        // Claude models proxied through Gemini API
        "gemini-claude-sonnet-4-5-thinking".to_string(),
        "gemini-claude-opus-4-5-thinking".to_string(),
    ];

    Ok(models)
}

/// Fetch available Google models using API key
/// Note: OAuth tokens from user authentication don't have permission to access the Generative Language API.
/// Users need to provide an API key from Google AI Studio instead.
pub async fn fetch_google_models_with_api_key(api_key: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", api_key);

    let res: serde_json::Value = client.get(&url).send().await?.json().await?;

    #[derive(Deserialize)]
    struct ModelsResponse {
        models: Vec<Model>,
    }

    #[derive(Deserialize)]
    struct Model {
        name: String,
    }

    let models_response: ModelsResponse = serde_json::from_value(res)?;

    let models: Vec<String> = models_response
        .models
        .into_iter()
        .filter_map(|m| m.name.strip_prefix("models/").map(|s| s.to_string()))
        .filter(|name| name.contains("gemini") || name.contains("gemma"))
        .collect();

    Ok(models)
}
