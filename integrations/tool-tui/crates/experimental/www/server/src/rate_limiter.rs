//! Rate Limiter Middleware for dx-server
//!
//! Provides sliding window rate limiting to protect against abuse and DoS attacks.
//!
//! ## Features
//! - Sliding window algorithm (prevents burst attacks at window boundaries)
//! - IP-based tracking
//! - Configurable limits per endpoint category
//! - Retry-After header on 429 responses

use axum::{
    body::Body,
    http::{Request, Response, StatusCode, header::HeaderValue},
    middleware::Next,
};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Endpoint categories for different rate limits
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum EndpointCategory {
    /// Authentication endpoints (login, refresh, logout)
    Auth,
    /// API endpoints (RPC, data queries)
    Api,
    /// Static file serving
    Static,
}

/// Rate limit configuration for an endpoint category
#[derive(Debug, Clone, Copy)]
pub struct RateLimit {
    /// Maximum requests allowed in the window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_seconds: u64,
}

impl RateLimit {
    /// Create a new rate limit
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
        }
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone, PartialEq)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed {
        /// Remaining requests in the current window
        remaining: u32,
    },
    /// Request is rate limited
    Limited {
        /// Seconds until the client can retry
        retry_after: u64,
    },
}

/// A single request timestamp for sliding window tracking
#[derive(Debug, Clone)]
struct RequestTimestamp {
    time: Instant,
}

/// Sliding window counter for a single key
#[derive(Debug)]
struct SlidingWindowCounter {
    /// Timestamps of requests in the current window
    timestamps: Vec<RequestTimestamp>,
    /// Window duration
    window: Duration,
}

impl SlidingWindowCounter {
    fn new(window: Duration) -> Self {
        Self {
            timestamps: Vec::new(),
            window,
        }
    }

    /// Clean up old timestamps outside the window
    fn cleanup(&mut self, now: Instant) {
        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        self.timestamps.retain(|ts| ts.time > cutoff);
    }

    /// Record a new request and return the current count
    fn record(&mut self, now: Instant) -> u32 {
        self.cleanup(now);
        self.timestamps.push(RequestTimestamp { time: now });
        self.timestamps.len() as u32
    }

    /// Get the current count without recording
    fn count(&mut self, now: Instant) -> u32 {
        self.cleanup(now);
        self.timestamps.len() as u32
    }

    /// Get time until the oldest request expires (for Retry-After)
    fn time_until_slot_available(&self, now: Instant) -> u64 {
        // Find the oldest timestamp
        let Some(oldest) = self.timestamps.iter().map(|ts| ts.time).min() else {
            return 0;
        };
        let expires_at = oldest + self.window;

        if expires_at > now {
            (expires_at - now).as_secs() + 1 // Round up
        } else {
            0
        }
    }
}

/// Rate limit store trait for different backends
#[async_trait::async_trait]
pub trait RateLimitStore: Send + Sync {
    /// Increment counter and return current count
    async fn increment(&self, key: &str, window_seconds: u64) -> Result<u32, RateLimitError>;

    /// Get time until window resets
    async fn ttl(&self, key: &str) -> Result<u64, RateLimitError>;

    /// Check current count without incrementing
    async fn count(&self, key: &str, window_seconds: u64) -> Result<u32, RateLimitError>;
}

/// In-memory rate limit store using sliding window
#[derive(Debug)]
pub struct InMemoryRateLimitStore {
    /// Counters per key
    counters: DashMap<String, SlidingWindowCounter>,
}

impl Default for InMemoryRateLimitStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRateLimitStore {
    /// Create a new in-memory store
    pub fn new() -> Self {
        Self {
            counters: DashMap::new(),
        }
    }

    /// Clean up expired entries (call periodically)
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.counters.retain(|_, counter| {
            counter.cleanup(now);
            !counter.timestamps.is_empty()
        });
    }
}

#[async_trait::async_trait]
impl RateLimitStore for InMemoryRateLimitStore {
    async fn increment(&self, key: &str, window_seconds: u64) -> Result<u32, RateLimitError> {
        let now = Instant::now();
        let window = Duration::from_secs(window_seconds);

        let count = self
            .counters
            .entry(key.to_string())
            .or_insert_with(|| SlidingWindowCounter::new(window))
            .record(now);

        Ok(count)
    }

    async fn ttl(&self, key: &str) -> Result<u64, RateLimitError> {
        let now = Instant::now();

        if let Some(counter) = self.counters.get(key) {
            Ok(counter.time_until_slot_available(now))
        } else {
            Ok(0)
        }
    }

    async fn count(&self, key: &str, window_seconds: u64) -> Result<u32, RateLimitError> {
        let now = Instant::now();
        let window = Duration::from_secs(window_seconds);

        let count = self
            .counters
            .entry(key.to_string())
            .or_insert_with(|| SlidingWindowCounter::new(window))
            .count(now);

        Ok(count)
    }
}

/// Rate limiter errors
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RateLimitError {
    #[error("Store error: {0}")]
    StoreError(String),
}

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Rate limits by endpoint category
    pub limits: std::collections::HashMap<EndpointCategory, RateLimit>,
    /// Default rate limit for uncategorized endpoints
    pub default_limit: RateLimit,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self::production()
    }
}

impl RateLimiterConfig {
    /// Create production rate limits
    pub fn production() -> Self {
        let mut limits = std::collections::HashMap::new();

        // Auth endpoints: 10 requests per minute (prevent brute force)
        limits.insert(EndpointCategory::Auth, RateLimit::new(10, 60));

        // API endpoints: 100 requests per minute
        limits.insert(EndpointCategory::Api, RateLimit::new(100, 60));

        // Static files: 1000 requests per minute
        limits.insert(EndpointCategory::Static, RateLimit::new(1000, 60));

        Self {
            limits,
            default_limit: RateLimit::new(60, 60), // 1 per second default
        }
    }

    /// Create development rate limits (more permissive)
    pub fn development() -> Self {
        let mut limits = std::collections::HashMap::new();

        limits.insert(EndpointCategory::Auth, RateLimit::new(100, 60));
        limits.insert(EndpointCategory::Api, RateLimit::new(1000, 60));
        limits.insert(EndpointCategory::Static, RateLimit::new(10000, 60));

        Self {
            limits,
            default_limit: RateLimit::new(600, 60),
        }
    }

    /// Get rate limit for a category
    pub fn get_limit(&self, category: EndpointCategory) -> RateLimit {
        self.limits.get(&category).copied().unwrap_or(self.default_limit)
    }
}

/// Sliding window rate limiter
#[derive(Clone)]
pub struct RateLimiter {
    /// Storage backend
    store: Arc<dyn RateLimitStore>,
    /// Configuration
    config: RateLimiterConfig,
}

impl RateLimiter {
    /// Create a new rate limiter with the given store and config
    pub fn new(store: Arc<dyn RateLimitStore>, config: RateLimiterConfig) -> Self {
        Self { store, config }
    }

    /// Create with in-memory store and production config
    pub fn production() -> Self {
        Self::new(Arc::new(InMemoryRateLimitStore::new()), RateLimiterConfig::production())
    }

    /// Create with in-memory store and development config
    pub fn development() -> Self {
        Self::new(Arc::new(InMemoryRateLimitStore::new()), RateLimiterConfig::development())
    }

    /// Check if a request should be rate limited
    pub async fn check(
        &self,
        ip: &str,
        category: EndpointCategory,
    ) -> Result<RateLimitResult, RateLimitError> {
        let limit = self.config.get_limit(category);
        let key = format!("{}:{:?}", ip, category);

        // Get current count without incrementing first
        let current_count = self.store.count(&key, limit.window_seconds).await?;

        if current_count >= limit.max_requests {
            // Already at limit, return retry-after
            let retry_after = self.store.ttl(&key).await?;
            return Ok(RateLimitResult::Limited {
                retry_after: retry_after.max(1),
            });
        }

        // Increment and check again (atomic operation)
        let new_count = self.store.increment(&key, limit.window_seconds).await?;

        if new_count > limit.max_requests {
            // Race condition: another request got in first
            let retry_after = self.store.ttl(&key).await?;
            Ok(RateLimitResult::Limited {
                retry_after: retry_after.max(1),
            })
        } else {
            Ok(RateLimitResult::Allowed {
                remaining: limit.max_requests - new_count,
            })
        }
    }

    /// Get the rate limit for a category
    pub fn get_limit(&self, category: EndpointCategory) -> RateLimit {
        self.config.get_limit(category)
    }
}

/// Determine endpoint category from request path
pub fn categorize_path(path: &str) -> EndpointCategory {
    if path.starts_with("/api/auth") {
        EndpointCategory::Auth
    } else if path.starts_with("/api") || path.starts_with("/rpc") {
        EndpointCategory::Api
    } else if path.starts_with("/static")
        || path.ends_with(".js")
        || path.ends_with(".css")
        || path.ends_with(".wasm")
        || path.ends_with(".ico")
    {
        EndpointCategory::Static
    } else {
        EndpointCategory::Api // Default to API for unknown paths
    }
}

/// Extract client IP from request
pub fn extract_client_ip<B>(req: &Request<B>) -> String {
    // Check X-Forwarded-For header first (for proxied requests)
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(value) = forwarded.to_str() {
            // Take the first IP in the chain
            if let Some(ip) = value.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(value) = real_ip.to_str() {
            return value.trim().to_string();
        }
    }

    // Fallback to unknown (in production, you'd get this from the connection)
    "unknown".to_string()
}

/// Rate limiter middleware layer
#[derive(Clone)]
pub struct RateLimiterLayer {
    limiter: RateLimiter,
}

impl RateLimiterLayer {
    /// Create a new rate limiter layer
    pub fn new(limiter: RateLimiter) -> Self {
        Self { limiter }
    }

    /// Create with production defaults
    pub fn production() -> Self {
        Self::new(RateLimiter::production())
    }

    /// Create with development defaults
    pub fn development() -> Self {
        Self::new(RateLimiter::development())
    }
}

impl<S> tower::Layer<S> for RateLimiterLayer {
    type Service = RateLimiterService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimiterService {
            inner,
            limiter: self.limiter.clone(),
        }
    }
}

/// Rate limiter service
#[derive(Clone)]
pub struct RateLimiterService<S> {
    inner: S,
    limiter: RateLimiter,
}

impl<S> tower::Service<Request<Body>> for RateLimiterService<S>
where
    S: tower::Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let limiter = self.limiter.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let ip = extract_client_ip(&req);
            let path = req.uri().path().to_string();
            let category = categorize_path(&path);

            match limiter.check(&ip, category).await {
                Ok(RateLimitResult::Allowed { remaining }) => {
                    let mut response = inner.call(req).await?;

                    // Add rate limit headers (use from_static for numeric values to avoid unwrap)
                    let limit = limiter.get_limit(category);
                    if let Ok(limit_val) = HeaderValue::from_str(&limit.max_requests.to_string()) {
                        response.headers_mut().insert("x-ratelimit-limit", limit_val);
                    }
                    if let Ok(remaining_val) = HeaderValue::from_str(&remaining.to_string()) {
                        response.headers_mut().insert("x-ratelimit-remaining", remaining_val);
                    }

                    Ok(response)
                }
                Ok(RateLimitResult::Limited { retry_after }) => {
                    let mut response = Response::new(Body::from("Too Many Requests"));
                    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;

                    if let Ok(retry_val) = HeaderValue::from_str(&retry_after.to_string()) {
                        response.headers_mut().insert("retry-after", retry_val);
                    }

                    let limit = limiter.get_limit(category);
                    if let Ok(limit_val) = HeaderValue::from_str(&limit.max_requests.to_string()) {
                        response.headers_mut().insert("x-ratelimit-limit", limit_val);
                    }
                    response
                        .headers_mut()
                        .insert("x-ratelimit-remaining", HeaderValue::from_static("0"));

                    Ok(response)
                }
                Err(_) => {
                    // On error, allow the request (fail open)
                    inner.call(req).await
                }
            }
        })
    }
}

/// Middleware function for rate limiting
pub async fn rate_limit_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let limiter = RateLimiter::production();
    let ip = extract_client_ip(&req);
    let path = req.uri().path().to_string();
    let category = categorize_path(&path);

    match limiter.check(&ip, category).await {
        Ok(RateLimitResult::Allowed { remaining }) => {
            let mut response = next.run(req).await;

            let limit = limiter.get_limit(category);
            if let Ok(limit_val) = HeaderValue::from_str(&limit.max_requests.to_string()) {
                response.headers_mut().insert("x-ratelimit-limit", limit_val);
            }
            if let Ok(remaining_val) = HeaderValue::from_str(&remaining.to_string()) {
                response.headers_mut().insert("x-ratelimit-remaining", remaining_val);
            }

            response
        }
        Ok(RateLimitResult::Limited { retry_after }) => {
            let mut response = Response::new(Body::from("Too Many Requests"));
            *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;

            if let Ok(retry_val) = HeaderValue::from_str(&retry_after.to_string()) {
                response.headers_mut().insert("retry-after", retry_val);
            }

            response
        }
        Err(_) => {
            // Fail open on errors
            next.run(req).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_production() {
        let config = RateLimiterConfig::production();
        let auth_limit = config.get_limit(EndpointCategory::Auth);
        assert_eq!(auth_limit.max_requests, 10);
        assert_eq!(auth_limit.window_seconds, 60);
    }

    #[test]
    fn test_categorize_path() {
        assert_eq!(categorize_path("/api/auth/login"), EndpointCategory::Auth);
        assert_eq!(categorize_path("/api/auth/refresh"), EndpointCategory::Auth);
        assert_eq!(categorize_path("/api/rpc"), EndpointCategory::Api);
        assert_eq!(categorize_path("/api/users"), EndpointCategory::Api);
        assert_eq!(categorize_path("/static/app.js"), EndpointCategory::Static);
        assert_eq!(categorize_path("/app.wasm"), EndpointCategory::Static);
        assert_eq!(categorize_path("/favicon.ico"), EndpointCategory::Static);
    }

    #[tokio::test]
    async fn test_in_memory_store_increment() {
        let store = InMemoryRateLimitStore::new();

        let count1 = store.increment("test_key", 60).await.unwrap();
        assert_eq!(count1, 1);

        let count2 = store.increment("test_key", 60).await.unwrap();
        assert_eq!(count2, 2);

        let count3 = store.increment("test_key", 60).await.unwrap();
        assert_eq!(count3, 3);
    }

    #[tokio::test]
    async fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(
            Arc::new(InMemoryRateLimitStore::new()),
            RateLimiterConfig::production(),
        );

        // Auth limit is 10 per minute
        for i in 0..10 {
            let result = limiter.check("192.168.1.1", EndpointCategory::Auth).await.unwrap();
            match result {
                RateLimitResult::Allowed { remaining } => {
                    assert_eq!(remaining, 9 - i);
                }
                RateLimitResult::Limited { .. } => {
                    panic!("Should not be limited at request {}", i + 1);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(
            Arc::new(InMemoryRateLimitStore::new()),
            RateLimiterConfig::production(),
        );

        // Exhaust the limit (10 requests)
        for _ in 0..10 {
            let _ = limiter.check("192.168.1.1", EndpointCategory::Auth).await;
        }

        // 11th request should be limited
        let result = limiter.check("192.168.1.1", EndpointCategory::Auth).await.unwrap();
        match result {
            RateLimitResult::Limited { retry_after } => {
                assert!(retry_after > 0, "Retry-after should be positive");
            }
            RateLimitResult::Allowed { .. } => {
                panic!("Should be limited after exceeding max requests");
            }
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_different_ips() {
        let limiter = RateLimiter::new(
            Arc::new(InMemoryRateLimitStore::new()),
            RateLimiterConfig::production(),
        );

        // Exhaust limit for IP 1
        for _ in 0..10 {
            let _ = limiter.check("192.168.1.1", EndpointCategory::Auth).await;
        }

        // IP 2 should still be allowed
        let result = limiter.check("192.168.1.2", EndpointCategory::Auth).await.unwrap();
        assert!(matches!(result, RateLimitResult::Allowed { .. }));
    }

    #[tokio::test]
    async fn test_rate_limiter_different_categories() {
        let limiter = RateLimiter::new(
            Arc::new(InMemoryRateLimitStore::new()),
            RateLimiterConfig::production(),
        );

        // Exhaust auth limit
        for _ in 0..10 {
            let _ = limiter.check("192.168.1.1", EndpointCategory::Auth).await;
        }

        // API category should still be allowed (separate counter)
        let result = limiter.check("192.168.1.1", EndpointCategory::Api).await.unwrap();
        assert!(matches!(result, RateLimitResult::Allowed { .. }));
    }
}
