//! JWT-based authentication for the gateway.

use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::AuthConfig;

/// JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user/client ID)
    pub sub: String,
    /// Expiry timestamp
    pub exp: i64,
    /// Issued at timestamp
    pub iat: i64,
    /// JWT ID
    pub jti: String,
    /// Scopes/permissions
    pub scopes: Vec<String>,
}

/// Authentication manager
pub struct AuthManager {
    jwt_secret: String,
    token_expiry_secs: u64,
    api_keys: Vec<String>,
    auth_required: bool,
}

impl AuthManager {
    /// Create a new auth manager from config
    pub fn new(config: &AuthConfig) -> Self {
        let jwt_secret = config.jwt_secret.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

        Self {
            jwt_secret,
            token_expiry_secs: config.token_expiry_secs,
            api_keys: config.api_keys.clone(),
            auth_required: config.required,
        }
    }

    /// Check if authentication is required
    pub fn is_required(&self) -> bool {
        self.auth_required
    }

    /// Generate a JWT token for a client
    pub fn generate_token(&self, client_id: &str, scopes: Vec<String>) -> anyhow::Result<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.token_expiry_secs as i64);

        let claims = Claims {
            sub: client_id.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            scopes,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;

        Ok(token)
    }

    /// Validate a JWT token and return claims
    pub fn validate_token(&self, token: &str) -> anyhow::Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    /// Validate an API key
    pub fn validate_api_key(&self, key: &str) -> bool {
        self.api_keys.iter().any(|k| k == key)
    }

    /// Authenticate a client (token or API key)
    pub fn authenticate(&self, credential: &str) -> anyhow::Result<Claims> {
        // Try JWT first
        if let Ok(claims) = self.validate_token(credential) {
            return Ok(claims);
        }

        // Try API key
        if self.validate_api_key(credential) {
            return Ok(Claims {
                sub: "api-key-user".into(),
                exp: (Utc::now() + Duration::hours(24)).timestamp(),
                iat: Utc::now().timestamp(),
                jti: Uuid::new_v4().to_string(),
                scopes: vec!["*".into()],
            });
        }

        anyhow::bail!("Invalid credentials")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AuthConfig {
        AuthConfig {
            required: true,
            jwt_secret: Some("test-secret-key-for-tests".into()),
            token_expiry_secs: 3600,
            api_keys: vec!["test-api-key-123".into()],
        }
    }

    #[test]
    fn test_generate_and_validate_token() {
        let auth = AuthManager::new(&test_config());
        let token = auth
            .generate_token("client-1", vec!["read".into(), "write".into()])
            .expect("generate token");

        let claims = auth.validate_token(&token).expect("validate token");
        assert_eq!(claims.sub, "client-1");
        assert_eq!(claims.scopes, vec!["read", "write"]);
    }

    #[test]
    fn test_api_key_validation() {
        let auth = AuthManager::new(&test_config());
        assert!(auth.validate_api_key("test-api-key-123"));
        assert!(!auth.validate_api_key("wrong-key"));
    }

    #[test]
    fn test_authenticate_with_token() {
        let auth = AuthManager::new(&test_config());
        let token = auth.generate_token("user1", vec!["admin".into()]).expect("generate");
        let claims = auth.authenticate(&token).expect("auth");
        assert_eq!(claims.sub, "user1");
    }

    #[test]
    fn test_authenticate_with_api_key() {
        let auth = AuthManager::new(&test_config());
        let claims = auth.authenticate("test-api-key-123").expect("auth");
        assert_eq!(claims.sub, "api-key-user");
    }

    #[test]
    fn test_authenticate_invalid() {
        let auth = AuthManager::new(&test_config());
        assert!(auth.authenticate("invalid-cred").is_err());
    }
}
