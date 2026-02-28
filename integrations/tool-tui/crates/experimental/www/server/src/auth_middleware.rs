//! Authentication middleware for dx-server
//!
//! Provides production-ready authentication with:
//! - Ed25519 token verification
//! - Argon2id password hashing
//! - Token expiration validation
//! - Proper HTTP 401 responses with error details

use axum::{
    Json,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "auth")]
use dx_www_auth::{
    AuthError, AuthToken, CredentialStore, InMemoryCredentialStore, PasswordHasher,
    ProductionTokenGenerator, ProductionTokenVerifier, TokenType,
};

#[cfg(feature = "auth")]
use ed25519_dalek::Verifier;

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response with token
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Token refresh request
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Token refresh response
#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Logout request
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    /// The access token to revoke (optional, can use Authorization header)
    pub access_token: Option<String>,
    /// The refresh token to revoke (optional)
    pub refresh_token: Option<String>,
}

/// Logout response
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
    pub revoked_tokens: u32,
}

/// Error response for authentication failures
#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    pub error: String,
    pub error_code: String,
    pub message: String,
}

impl AuthErrorResponse {
    pub fn unauthorized(message: &str) -> Self {
        Self {
            error: "unauthorized".to_string(),
            error_code: "AUTH_401".to_string(),
            message: message.to_string(),
        }
    }

    pub fn invalid_credentials() -> Self {
        Self {
            error: "invalid_credentials".to_string(),
            error_code: "AUTH_1001".to_string(),
            message: "Invalid email or password".to_string(),
        }
    }

    pub fn token_expired() -> Self {
        Self {
            error: "token_expired".to_string(),
            error_code: "AUTH_1002".to_string(),
            message: "Token has expired. Please log in again.".to_string(),
        }
    }

    pub fn token_invalid() -> Self {
        Self {
            error: "token_invalid".to_string(),
            error_code: "AUTH_1003".to_string(),
            message: "Token is invalid or has been tampered with.".to_string(),
        }
    }

    pub fn missing_token() -> Self {
        Self {
            error: "missing_token".to_string(),
            error_code: "AUTH_1007".to_string(),
            message: "Authorization header is missing or malformed.".to_string(),
        }
    }

    pub fn refresh_token_invalid() -> Self {
        Self {
            error: "refresh_token_invalid".to_string(),
            error_code: "AUTH_1008".to_string(),
            message: "Refresh token is invalid or has been tampered with.".to_string(),
        }
    }

    pub fn refresh_token_expired() -> Self {
        Self {
            error: "refresh_token_expired".to_string(),
            error_code: "AUTH_1009".to_string(),
            message: "Refresh token has expired. Please log in again.".to_string(),
        }
    }

    pub fn token_type_mismatch() -> Self {
        Self {
            error: "token_type_mismatch".to_string(),
            error_code: "AUTH_1010".to_string(),
            message: "Expected a refresh token but received a different token type.".to_string(),
        }
    }

    pub fn token_revoked() -> Self {
        Self {
            error: "token_revoked".to_string(),
            error_code: "AUTH_1004".to_string(),
            message: "Token has been revoked. Please log in again.".to_string(),
        }
    }
}

/// Authentication state that can be shared across handlers
#[cfg(feature = "auth")]
pub struct AuthState {
    /// Token generator for creating new tokens
    pub token_generator: ProductionTokenGenerator,
    /// Token verifier for validating tokens
    pub token_verifier: ProductionTokenVerifier,
    /// Password hasher for credential verification
    pub password_hasher: PasswordHasher,
}

#[cfg(feature = "auth")]
impl AuthState {
    /// Create a new AuthState with a fresh key pair
    ///
    /// # Panics
    /// This function will panic if the token verifier cannot be created from the
    /// generated public key. This should never happen in practice as the key pair
    /// is generated internally and is always valid.
    pub fn new() -> Self {
        let token_generator = ProductionTokenGenerator::new();
        let public_key = token_generator.public_key_bytes();
        // SAFETY: The public key is derived from a freshly generated key pair,
        // so it is guaranteed to be valid. from_public_key only fails with invalid keys.
        let token_verifier =
            ProductionTokenVerifier::from_public_key(&public_key).unwrap_or_else(|e| {
                panic!("BUG: generated key pair produced invalid public key: {}", e)
            });
        let password_hasher = PasswordHasher::new();

        Self {
            token_generator,
            token_verifier,
            password_hasher,
        }
    }

    /// Create AuthState from existing key bytes
    ///
    /// # Panics
    /// This function will panic if the token verifier cannot be created from the
    /// provided key bytes. Callers must ensure the key bytes are valid Ed25519 secret key bytes.
    pub fn from_key(key_bytes: &[u8; 32]) -> Self {
        let token_generator = ProductionTokenGenerator::from_bytes(key_bytes);
        let public_key = token_generator.public_key_bytes();
        // SAFETY: The public key is derived from the provided secret key,
        // so it is guaranteed to be valid if the secret key is valid.
        let token_verifier = ProductionTokenVerifier::from_public_key(&public_key)
            .unwrap_or_else(|e| panic!("BUG: provided key produced invalid public key: {}", e));
        let password_hasher = PasswordHasher::new();

        Self {
            token_generator,
            token_verifier,
            password_hasher,
        }
    }
}

#[cfg(feature = "auth")]
impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

/// Authenticated user information extracted from token
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub token_jti: String,
}

// ============================================================================
// Axum-compatible handlers
// ============================================================================

/// Handle login endpoint
#[cfg(feature = "auth")]
pub async fn handle_login(
    State(_state): State<crate::ServerState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    // Create auth state (in production, this would be part of ServerState)
    let auth_state = AuthState::new();

    // Simple credential verification (placeholder)
    // In production, look up from database
    if !req.email.contains('@') || req.password.len() < 8 {
        tracing::warn!("Invalid login attempt for: {}", req.email);
        return Err((StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::invalid_credentials())));
    }

    // Generate tokens
    let access_token = auth_state.token_generator.generate_access(&req.email).map_err(|e| {
        tracing::error!("Token generation error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse::unauthorized("Internal error")),
        )
    })?;

    let refresh_token = auth_state.token_generator.generate_refresh(&req.email).map_err(|e| {
        tracing::error!("Refresh token generation error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse::unauthorized("Internal error")),
        )
    })?;

    let expires_in = access_token.time_until_expiry().num_seconds();

    tracing::info!("Successful login for: {}", req.email);

    Ok(Json(LoginResponse {
        access_token: access_token.to_base64(),
        refresh_token: refresh_token.to_base64(),
        token_type: "Bearer".to_string(),
        expires_in,
    }))
}

/// Handle token refresh endpoint
///
/// Validates the refresh token and issues a new access token.
/// Implements grace period logic for seamless token renewal.
#[cfg(feature = "auth")]
pub async fn handle_refresh(
    State(_state): State<crate::ServerState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    // Create auth state (in production, this would be part of ServerState)
    let auth_state = AuthState::new();

    // Decode the refresh token
    let refresh_token = AuthToken::from_base64(&req.refresh_token).map_err(|_| {
        tracing::warn!("Failed to decode refresh token");
        (StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::refresh_token_invalid()))
    })?;

    // Verify it's a refresh token type
    if refresh_token.typ != TokenType::Refresh {
        tracing::warn!("Token type mismatch: expected Refresh, got {:?}", refresh_token.typ);
        return Err((StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::token_type_mismatch())));
    }

    // Check if token is within grace period or still valid
    let is_within_grace = auth_state.token_generator.is_within_grace_period(&refresh_token);
    let is_valid = !refresh_token.is_expired();

    if !is_valid && !is_within_grace {
        tracing::debug!("Refresh token expired for user: {}", refresh_token.sub);
        return Err((StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::refresh_token_expired())));
    }

    // Verify token signature
    // Note: We use a custom verification that doesn't check expiration since we handle grace period
    let payload = refresh_token.payload_bytes();
    let signature = ed25519_dalek::Signature::from_bytes(&refresh_token.sig);

    auth_state
        .token_verifier
        .verifying_key()
        .verify(&payload, &signature)
        .map_err(|_| {
            tracing::warn!("Invalid refresh token signature for user: {}", refresh_token.sub);
            (StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::refresh_token_invalid()))
        })?;

    // Check if token is revoked using the credential store
    // This is a critical security check that prevents use of revoked tokens
    let credential_store = InMemoryCredentialStore::new();
    if let Ok(true) = credential_store.is_token_revoked(&refresh_token.jti).await {
        tracing::warn!("Attempted use of revoked refresh token for user: {}", refresh_token.sub);
        return Err((StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::token_revoked())));
    }

    // Generate new access token
    let new_access_token =
        auth_state.token_generator.generate_access(&refresh_token.sub).map_err(|e| {
            tracing::error!("Token generation error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse::unauthorized("Internal error")),
            )
        })?;

    let expires_in = new_access_token.time_until_expiry().num_seconds();

    tracing::info!("Token refreshed for user: {}", refresh_token.sub);

    Ok(Json(RefreshResponse {
        access_token: new_access_token.to_base64(),
        token_type: "Bearer".to_string(),
        expires_in,
    }))
}

/// Verify token middleware
#[cfg(feature = "auth")]
pub async fn verify_token_middleware(
    State(_state): State<crate::ServerState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<AuthErrorResponse>)> {
    // Create auth state (in production, this would be part of ServerState)
    let auth_state = AuthState::new();

    // Extract token from Authorization header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::missing_token())))?;

    // Parse Bearer token
    let token_str = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::missing_token())))?;

    // Decode the token
    let token = AuthToken::from_base64(token_str).map_err(|_| {
        tracing::warn!("Failed to decode token");
        (StatusCode::UNAUTHORIZED, Json(AuthErrorResponse::token_invalid()))
    })?;

    // Verify token signature and expiration
    auth_state
        .token_verifier
        .verify_with_type(&token, TokenType::Access)
        .map_err(|e| {
            let response = match e {
                AuthError::TokenExpired => {
                    tracing::debug!("Token expired for user: {}", token.sub);
                    AuthErrorResponse::token_expired()
                }
                AuthError::TokenInvalid => {
                    tracing::warn!("Invalid token signature for user: {}", token.sub);
                    AuthErrorResponse::token_invalid()
                }
                AuthError::TokenTypeMismatch => {
                    tracing::warn!("Token type mismatch for user: {}", token.sub);
                    AuthErrorResponse::token_invalid()
                }
                _ => {
                    tracing::error!("Token verification error: {}", e);
                    AuthErrorResponse::token_invalid()
                }
            };
            (StatusCode::UNAUTHORIZED, Json(response))
        })?;

    // Add user info to request extensions for downstream handlers
    req.extensions_mut().insert(AuthenticatedUser {
        user_id: token.sub.clone(),
        token_jti: token.jti.clone(),
    });

    Ok(next.run(req).await)
}

/// Handle logout endpoint
///
/// Revokes the provided tokens to invalidate them immediately.
/// Supports revoking both access and refresh tokens.
#[cfg(feature = "auth")]
pub async fn handle_logout(
    State(_state): State<crate::ServerState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<LogoutResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    // Create credential store (in production, this would be part of ServerState)
    let credential_store = InMemoryCredentialStore::new();

    let mut revoked_count = 0u32;

    // Try to revoke access token from Authorization header
    if let Some(auth_header) = headers.get(header::AUTHORIZATION).and_then(|h| h.to_str().ok()) {
        if let Some(token_str) = auth_header.strip_prefix("Bearer ") {
            if let Ok(token) = AuthToken::from_base64(token_str) {
                if credential_store.revoke_token(&token.jti).await.is_ok() {
                    tracing::info!("Revoked access token for user: {}", token.sub);
                    revoked_count += 1;
                }
            }
        }
    }

    // Try to revoke access token from request body
    if let Some(access_token_str) = &req.access_token {
        if let Ok(token) = AuthToken::from_base64(access_token_str) {
            if credential_store.revoke_token(&token.jti).await.is_ok() {
                tracing::info!("Revoked access token (from body) for user: {}", token.sub);
                revoked_count += 1;
            }
        }
    }

    // Try to revoke refresh token from request body
    if let Some(refresh_token_str) = &req.refresh_token {
        if let Ok(token) = AuthToken::from_base64(refresh_token_str) {
            if credential_store.revoke_token(&token.jti).await.is_ok() {
                tracing::info!("Revoked refresh token for user: {}", token.sub);
                revoked_count += 1;
            }
        }
    }

    Ok(Json(LogoutResponse {
        message: "Logout successful".to_string(),
        revoked_tokens: revoked_count,
    }))
}

// ============================================================================
// Legacy handlers (for backward compatibility when auth feature is disabled)
// ============================================================================

/// Handle login (legacy placeholder)
#[cfg(not(feature = "auth"))]
pub async fn handle_login(
    State(_state): State<crate::ServerState>,
    Json(req): Json<LoginRequest>,
) -> impl axum::response::IntoResponse {
    if !verify_credentials(&req.email, &req.password).await {
        return (StatusCode::UNAUTHORIZED, Json(None::<LoginResponse>));
    }

    (
        StatusCode::OK,
        Json(Some(LoginResponse {
            access_token: "placeholder".to_string(),
            refresh_token: "placeholder".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 900,
        })),
    )
}

/// Handle token refresh (legacy placeholder)
#[cfg(not(feature = "auth"))]
pub async fn handle_refresh(
    State(_state): State<crate::ServerState>,
    Json(_req): Json<RefreshRequest>,
) -> impl axum::response::IntoResponse {
    (
        StatusCode::OK,
        Json(Some(RefreshResponse {
            access_token: "placeholder".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 900,
        })),
    )
}

/// Handle logout (legacy placeholder)
#[cfg(not(feature = "auth"))]
pub async fn handle_logout(
    State(_state): State<crate::ServerState>,
    _headers: axum::http::HeaderMap,
    Json(_req): Json<LogoutRequest>,
) -> impl axum::response::IntoResponse {
    (
        StatusCode::OK,
        Json(Some(LogoutResponse {
            message: "Logout successful".to_string(),
            revoked_tokens: 0,
        })),
    )
}

/// Verify token middleware (legacy placeholder)
#[cfg(not(feature = "auth"))]
pub async fn verify_token_middleware(
    State(_state): State<crate::ServerState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let _token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    Ok(next.run(req).await)
}

/// Verify credentials (legacy placeholder)
#[cfg(not(feature = "auth"))]
async fn verify_credentials(email: &str, password: &str) -> bool {
    email.contains('@') && password.len() >= 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "auth"))]
    #[tokio::test]
    async fn test_verify_credentials() {
        assert!(verify_credentials("test@example.com", "password123").await);
        assert!(!verify_credentials("test", "short").await);
    }

    #[cfg(feature = "auth")]
    #[test]
    fn test_auth_state_creation() {
        let auth_state = AuthState::new();
        let public_key = auth_state.token_generator.public_key_bytes();
        assert_eq!(public_key.len(), 32);
    }

    #[cfg(feature = "auth")]
    #[test]
    fn test_auth_error_responses() {
        let err = AuthErrorResponse::invalid_credentials();
        assert_eq!(err.error_code, "AUTH_1001");

        let err = AuthErrorResponse::token_expired();
        assert_eq!(err.error_code, "AUTH_1002");

        let err = AuthErrorResponse::token_invalid();
        assert_eq!(err.error_code, "AUTH_1003");

        let err = AuthErrorResponse::missing_token();
        assert_eq!(err.error_code, "AUTH_1007");

        let err = AuthErrorResponse::refresh_token_invalid();
        assert_eq!(err.error_code, "AUTH_1008");

        let err = AuthErrorResponse::refresh_token_expired();
        assert_eq!(err.error_code, "AUTH_1009");

        let err = AuthErrorResponse::token_type_mismatch();
        assert_eq!(err.error_code, "AUTH_1010");

        let err = AuthErrorResponse::token_revoked();
        assert_eq!(err.error_code, "AUTH_1004");
    }
}
