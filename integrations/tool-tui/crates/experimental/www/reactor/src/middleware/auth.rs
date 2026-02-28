//! Authentication middleware.

use super::{Middleware, MiddlewareError, MiddlewareResult, Request, Response};

/// JWT authentication middleware.
///
/// Verifies JWT tokens in the Authorization header and injects claims
/// into request extensions.
pub struct AuthMiddleware;

impl Middleware for AuthMiddleware {
    fn before(req: &mut Request) -> MiddlewareResult<()> {
        // Get the Authorization header
        let auth_header = req
            .header("Authorization")
            .ok_or_else(|| MiddlewareError::Unauthorized("missing Authorization header".into()))?
            .to_string();

        // Check for Bearer token
        if !auth_header.starts_with("Bearer ") {
            return Err(MiddlewareError::Unauthorized("invalid Authorization format".into()));
        }

        let token = auth_header[7..].to_string();

        // Verify the token (simplified - real impl would use proper JWT verification)
        if token.is_empty() {
            return Err(MiddlewareError::Unauthorized("empty token".into()));
        }

        // In a real implementation, we would:
        // 1. Decode the JWT
        // 2. Verify the signature
        // 3. Check expiration
        // 4. Extract claims

        // For now, just mark as authenticated
        req.set_extension("authenticated", "true");
        req.set_extension("token", token);

        Ok(())
    }

    fn after(_req: &Request, _res: &mut Response) {
        // No-op for auth middleware
    }
}
