//! Property-based tests for dx-www-server crate.
//!
//! These tests verify universal properties that should hold across all inputs.

use axum::{Router, body::Body, http::Request, routing::get};
use proptest::prelude::*;
use std::sync::Arc;
use tower::ServiceExt;

// Import security headers module
use dx_www_server::security_headers::{
    ContentSecurityPolicy, FrameOptions, ReferrerPolicy, SecurityConfig, SecurityHeadersLayer,
};

// ============================================================================
// Property 12: Security Headers Presence
// **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**
//
// *For any* HTTP response from the server, it SHALL contain all required
// security headers: Content-Security-Policy, Strict-Transport-Security,
// X-Frame-Options, X-Content-Type-Options, X-XSS-Protection, and Referrer-Policy.
// ============================================================================

/// Helper to create a test router with security headers middleware
fn create_test_router(config: SecurityConfig) -> Router {
    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/api/test", get(|| async { "API Response" }))
        .route("/static/file.js", get(|| async { "console.log('test');" }))
        .layer(SecurityHeadersLayer::new(config))
}

/// Feature: production-readiness, Property 12: Security Headers Presence
///
/// All responses should contain Content-Security-Policy header.
#[tokio::test]
async fn property_12_csp_header_present() {
    let config = SecurityConfig::production();
    let app = create_test_router(config);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(
        response.headers().contains_key("content-security-policy"),
        "Response should contain Content-Security-Policy header"
    );

    let csp = response.headers().get("content-security-policy").unwrap().to_str().unwrap();
    assert!(csp.contains("default-src"), "CSP should contain default-src directive");
}

/// Feature: production-readiness, Property 12: Security Headers Presence
///
/// All responses should contain Strict-Transport-Security header in production mode.
#[tokio::test]
async fn property_12_hsts_header_present_in_production() {
    let config = SecurityConfig::production();
    let app = create_test_router(config);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(
        response.headers().contains_key("strict-transport-security"),
        "Response should contain Strict-Transport-Security header in production"
    );

    let hsts = response.headers().get("strict-transport-security").unwrap().to_str().unwrap();
    assert!(hsts.contains("max-age="), "HSTS should contain max-age directive");
}

/// Feature: production-readiness, Property 12: Security Headers Presence
///
/// HSTS header should NOT be present in development mode.
#[tokio::test]
async fn property_12_hsts_header_absent_in_development() {
    let config = SecurityConfig::development();
    let app = create_test_router(config);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // HSTS should not be present in development mode
    assert!(
        !response.headers().contains_key("strict-transport-security"),
        "Response should NOT contain Strict-Transport-Security header in development"
    );
}

/// Feature: production-readiness, Property 12: Security Headers Presence
///
/// All responses should contain X-Frame-Options header.
#[tokio::test]
async fn property_12_x_frame_options_header_present() {
    let config = SecurityConfig::production();
    let app = create_test_router(config);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(
        response.headers().contains_key("x-frame-options"),
        "Response should contain X-Frame-Options header"
    );

    let frame_options = response.headers().get("x-frame-options").unwrap().to_str().unwrap();
    assert!(
        frame_options == "DENY" || frame_options == "SAMEORIGIN",
        "X-Frame-Options should be DENY or SAMEORIGIN"
    );
}

/// Feature: production-readiness, Property 12: Security Headers Presence
///
/// All responses should contain X-Content-Type-Options header.
#[tokio::test]
async fn property_12_x_content_type_options_header_present() {
    let config = SecurityConfig::production();
    let app = create_test_router(config);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(
        response.headers().contains_key("x-content-type-options"),
        "Response should contain X-Content-Type-Options header"
    );

    let content_type_options =
        response.headers().get("x-content-type-options").unwrap().to_str().unwrap();
    assert_eq!(content_type_options, "nosniff", "X-Content-Type-Options should be 'nosniff'");
}

/// Feature: production-readiness, Property 12: Security Headers Presence
///
/// All responses should contain X-XSS-Protection header.
#[tokio::test]
async fn property_12_x_xss_protection_header_present() {
    let config = SecurityConfig::production();
    let app = create_test_router(config);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(
        response.headers().contains_key("x-xss-protection"),
        "Response should contain X-XSS-Protection header"
    );

    let xss_protection = response.headers().get("x-xss-protection").unwrap().to_str().unwrap();
    assert!(xss_protection.contains("1"), "X-XSS-Protection should be enabled");
}

/// Feature: production-readiness, Property 12: Security Headers Presence
///
/// All responses should contain Referrer-Policy header.
#[tokio::test]
async fn property_12_referrer_policy_header_present() {
    let config = SecurityConfig::production();
    let app = create_test_router(config);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(
        response.headers().contains_key("referrer-policy"),
        "Response should contain Referrer-Policy header"
    );
}

/// Feature: production-readiness, Property 12: Security Headers Presence
///
/// All six required security headers should be present in production responses.
#[tokio::test]
async fn property_12_all_security_headers_present() {
    let config = SecurityConfig::production();
    let app = create_test_router(config);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let required_headers = [
        "content-security-policy",
        "strict-transport-security",
        "x-frame-options",
        "x-content-type-options",
        "x-xss-protection",
        "referrer-policy",
    ];

    for header in &required_headers {
        assert!(
            response.headers().contains_key(*header),
            "Response should contain {} header",
            header
        );
    }
}

proptest! {
    /// Feature: production-readiness, Property 12: Security Headers Presence (any path)
    ///
    /// For any valid URL path, security headers should be present.
    #[test]
    fn property_12_headers_present_for_any_path(path in "/[a-zA-Z0-9/_-]{0,50}") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = SecurityConfig::production();
            let app = Router::new()
                .fallback(|| async { "Fallback" })
                .layer(SecurityHeadersLayer::new(config));

            let response = app
                .oneshot(Request::builder().uri(&path).body(Body::empty()).unwrap())
                .await
                .unwrap();

            // All security headers should be present regardless of path
            prop_assert!(
                response.headers().contains_key("content-security-policy"),
                "CSP header missing for path: {}", path
            );
            prop_assert!(
                response.headers().contains_key("x-frame-options"),
                "X-Frame-Options header missing for path: {}", path
            );
            prop_assert!(
                response.headers().contains_key("x-content-type-options"),
                "X-Content-Type-Options header missing for path: {}", path
            );
            prop_assert!(
                response.headers().contains_key("referrer-policy"),
                "Referrer-Policy header missing for path: {}", path
            );

            Ok(())
        })?;
    }

    /// Feature: production-readiness, Property 12: Security Headers Presence (any HTTP method)
    ///
    /// For any HTTP method, security headers should be present.
    #[test]
    fn property_12_headers_present_for_any_method(method_idx in 0usize..5) {
        let methods = ["GET", "POST", "PUT", "DELETE", "PATCH"];
        let method = methods[method_idx];

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = SecurityConfig::production();
            let app = Router::new()
                .fallback(|| async { "Fallback" })
                .layer(SecurityHeadersLayer::new(config));

            let response = app
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri("/test")
                        .body(Body::empty())
                        .unwrap()
                )
                .await
                .unwrap();

            // All security headers should be present regardless of method
            prop_assert!(
                response.headers().contains_key("content-security-policy"),
                "CSP header missing for method: {}", method
            );
            prop_assert!(
                response.headers().contains_key("x-frame-options"),
                "X-Frame-Options header missing for method: {}", method
            );

            Ok(())
        })?;
    }
}

// ============================================================================
// CSP Configuration Tests
// ============================================================================

/// Feature: production-readiness, Property 12: Security Headers Presence (CSP directives)
///
/// Production CSP should contain all required directives.
#[test]
fn property_12_production_csp_has_required_directives() {
    let csp = ContentSecurityPolicy::production();
    let header = csp.build();

    // Required directives for production
    assert!(header.contains("default-src"), "CSP should have default-src");
    assert!(header.contains("script-src"), "CSP should have script-src");
    assert!(header.contains("style-src"), "CSP should have style-src");
    assert!(header.contains("frame-ancestors"), "CSP should have frame-ancestors");
    assert!(header.contains("object-src"), "CSP should have object-src");
}

/// Feature: production-readiness, Property 12: Security Headers Presence (CSP strictness)
///
/// Production CSP should be strict (no unsafe-eval in script-src).
#[test]
fn property_12_production_csp_is_strict() {
    let csp = ContentSecurityPolicy::production();
    let header = csp.build();

    // Production should not allow unsafe-eval
    assert!(
        !header.contains("'unsafe-eval'"),
        "Production CSP should not contain 'unsafe-eval'"
    );

    // frame-ancestors should be 'none' in production
    assert!(
        header.contains("frame-ancestors 'none'"),
        "Production CSP should have frame-ancestors 'none'"
    );
}

/// Feature: production-readiness, Property 12: Security Headers Presence (development CSP)
///
/// Development CSP should allow hot reloading features.
#[test]
fn property_12_development_csp_allows_hot_reload() {
    let csp = ContentSecurityPolicy::development();
    let header = csp.build();

    // Development should allow unsafe-eval for hot reloading
    assert!(
        header.contains("'unsafe-eval'"),
        "Development CSP should contain 'unsafe-eval' for hot reloading"
    );

    // Development should allow WebSocket connections
    assert!(
        header.contains("ws:") || header.contains("wss:"),
        "Development CSP should allow WebSocket connections"
    );
}

// ============================================================================
// HSTS Configuration Tests
// ============================================================================

/// Feature: production-readiness, Property 12: Security Headers Presence (HSTS config)
///
/// Production HSTS should have appropriate max-age.
#[test]
fn property_12_production_hsts_has_appropriate_max_age() {
    let config = SecurityConfig::production();
    let hsts = config.build_hsts_header();

    // Should have at least 1 year max-age (31536000 seconds)
    assert!(hsts.contains("max-age=31536000"), "Production HSTS should have 1 year max-age");

    // Should include subdomains
    assert!(hsts.contains("includeSubDomains"), "Production HSTS should include subdomains");
}

proptest! {
    /// Feature: production-readiness, Property 12: Security Headers Presence (custom HSTS)
    ///
    /// Custom HSTS max-age should be reflected in the header.
    #[test]
    fn property_12_custom_hsts_max_age(max_age in 1u64..=63072000) {
        let mut config = SecurityConfig::production();
        config.hsts_max_age = max_age;
        let hsts = config.build_hsts_header();

        prop_assert!(
            hsts.contains(&format!("max-age={}", max_age)),
            "HSTS header should contain custom max-age: {}", max_age
        );
    }
}

// ============================================================================
// Frame Options Tests
// ============================================================================

/// Feature: production-readiness, Property 12: Security Headers Presence (frame options)
///
/// Frame options should be correctly formatted.
#[test]
fn property_12_frame_options_values() {
    assert_eq!(FrameOptions::Deny.as_str(), "DENY");
    assert_eq!(FrameOptions::SameOrigin.as_str(), "SAMEORIGIN");
}

// ============================================================================
// Referrer Policy Tests
// ============================================================================

/// Feature: production-readiness, Property 12: Security Headers Presence (referrer policy)
///
/// All referrer policy values should be correctly formatted.
#[test]
fn property_12_referrer_policy_values() {
    assert_eq!(ReferrerPolicy::NoReferrer.as_str(), "no-referrer");
    assert_eq!(ReferrerPolicy::NoReferrerWhenDowngrade.as_str(), "no-referrer-when-downgrade");
    assert_eq!(ReferrerPolicy::Origin.as_str(), "origin");
    assert_eq!(ReferrerPolicy::OriginWhenCrossOrigin.as_str(), "origin-when-cross-origin");
    assert_eq!(ReferrerPolicy::SameOrigin.as_str(), "same-origin");
    assert_eq!(ReferrerPolicy::StrictOrigin.as_str(), "strict-origin");
    assert_eq!(
        ReferrerPolicy::StrictOriginWhenCrossOrigin.as_str(),
        "strict-origin-when-cross-origin"
    );
    assert_eq!(ReferrerPolicy::UnsafeUrl.as_str(), "unsafe-url");
}

proptest! {
    /// Feature: production-readiness, Property 12: Security Headers Presence (referrer policy in response)
    ///
    /// The configured referrer policy should appear in the response header.
    #[test]
    fn property_12_referrer_policy_in_response(policy_idx in 0usize..8) {
        let policies = [
            ReferrerPolicy::NoReferrer,
            ReferrerPolicy::NoReferrerWhenDowngrade,
            ReferrerPolicy::Origin,
            ReferrerPolicy::OriginWhenCrossOrigin,
            ReferrerPolicy::SameOrigin,
            ReferrerPolicy::StrictOrigin,
            ReferrerPolicy::StrictOriginWhenCrossOrigin,
            ReferrerPolicy::UnsafeUrl,
        ];
        let policy = policies[policy_idx];

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut config = SecurityConfig::production();
            config.referrer_policy = policy;
            let app = create_test_router(config);

            let response = app
                .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
                .await
                .unwrap();

            let header_value = response
                .headers()
                .get("referrer-policy")
                .unwrap()
                .to_str()
                .unwrap();

            prop_assert_eq!(
                header_value,
                policy.as_str(),
                "Referrer-Policy header should match configured policy"
            );

            Ok(())
        })?;
    }
}

// ============================================================================
// Rate Limiter Property Tests
// ============================================================================

use dx_www_server::rate_limiter::{
    EndpointCategory, InMemoryRateLimitStore, RateLimit, RateLimitResult, RateLimitStore,
    RateLimiter, RateLimiterConfig, categorize_path,
};

// ============================================================================
// Property 13: Rate Limit Counting Accuracy
// **Validates: Requirements 5.1**
//
// *For any* sequence of N requests from the same IP within a time window,
// the rate limiter SHALL report a count of exactly N.
// ============================================================================

proptest! {
    /// Feature: production-readiness, Property 13: Rate Limit Counting Accuracy
    ///
    /// For any number of requests, the counter should accurately track the count.
    #[test]
    fn property_13_counting_accuracy(num_requests in 1u32..50) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryRateLimitStore::new();

            for i in 1..=num_requests {
                let count = store.increment("test_ip:Auth", 60).await.unwrap();
                prop_assert_eq!(count, i, "Count should be {} after {} requests", i, i);
            }

            Ok(())
        })?;
    }

    /// Feature: production-readiness, Property 13: Rate Limit Counting Accuracy (different keys)
    ///
    /// Different keys should have independent counters.
    #[test]
    fn property_13_independent_counters(
        ip1 in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
        ip2 in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
        requests1 in 1u32..20,
        requests2 in 1u32..20
    ) {
        prop_assume!(ip1 != ip2);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryRateLimitStore::new();

            // Make requests for IP1
            for _ in 0..requests1 {
                store.increment(&format!("{}:Auth", ip1), 60).await.unwrap();
            }

            // Make requests for IP2
            for _ in 0..requests2 {
                store.increment(&format!("{}:Auth", ip2), 60).await.unwrap();
            }

            // Verify counts are independent
            let count1 = store.count(&format!("{}:Auth", ip1), 60).await.unwrap();
            let count2 = store.count(&format!("{}:Auth", ip2), 60).await.unwrap();

            prop_assert_eq!(count1, requests1, "IP1 count should be {}", requests1);
            prop_assert_eq!(count2, requests2, "IP2 count should be {}", requests2);

            Ok(())
        })?;
    }
}

/// Feature: production-readiness, Property 13: Rate Limit Counting Accuracy (zero initial)
///
/// A new key should start with zero count.
#[tokio::test]
async fn property_13_zero_initial_count() {
    let store = InMemoryRateLimitStore::new();
    let count = store.count("new_key", 60).await.unwrap();
    assert_eq!(count, 0, "New key should have zero count");
}

// ============================================================================
// Property 14: Rate Limit Enforcement
// **Validates: Requirements 5.2, 5.3**
//
// *For any* IP that has made max_requests requests within the window,
// the next request SHALL receive HTTP 429 with a valid Retry-After header.
// ============================================================================

proptest! {
    /// Feature: production-readiness, Property 14: Rate Limit Enforcement
    ///
    /// After max_requests, the next request should be limited.
    #[test]
    fn property_14_enforcement_at_limit(max_requests in 5u32..20) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut config = RateLimiterConfig::production();
            config.limits.insert(EndpointCategory::Auth, RateLimit::new(max_requests, 60));

            let limiter = RateLimiter::new(
                Arc::new(InMemoryRateLimitStore::new()),
                config,
            );

            // Make max_requests requests
            for i in 0..max_requests {
                let result = limiter.check("test_ip", EndpointCategory::Auth).await.unwrap();
                prop_assert!(
                    matches!(result, RateLimitResult::Allowed { .. }),
                    "Request {} should be allowed (limit: {})", i + 1, max_requests
                );
            }

            // Next request should be limited
            let result = limiter.check("test_ip", EndpointCategory::Auth).await.unwrap();
            prop_assert!(
                matches!(result, RateLimitResult::Limited { .. }),
                "Request {} should be limited (limit: {})", max_requests + 1, max_requests
            );

            Ok(())
        })?;
    }

    /// Feature: production-readiness, Property 14: Rate Limit Enforcement (retry-after positive)
    ///
    /// When rate limited, retry-after should be a positive value.
    #[test]
    fn property_14_retry_after_positive(max_requests in 1u32..10) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut config = RateLimiterConfig::production();
            config.limits.insert(EndpointCategory::Auth, RateLimit::new(max_requests, 60));

            let limiter = RateLimiter::new(
                Arc::new(InMemoryRateLimitStore::new()),
                config,
            );

            // Exhaust the limit
            for _ in 0..max_requests {
                let _ = limiter.check("test_ip", EndpointCategory::Auth).await;
            }

            // Check retry-after
            let result = limiter.check("test_ip", EndpointCategory::Auth).await.unwrap();
            if let RateLimitResult::Limited { retry_after } = result {
                prop_assert!(retry_after > 0, "Retry-after should be positive");
                prop_assert!(retry_after <= 60, "Retry-after should not exceed window");
            } else {
                prop_assert!(false, "Should be limited");
            }

            Ok(())
        })?;
    }
}

/// Feature: production-readiness, Property 14: Rate Limit Enforcement (remaining decrements)
///
/// The remaining count should decrement with each request.
#[tokio::test]
async fn property_14_remaining_decrements() {
    let mut config = RateLimiterConfig::production();
    config.limits.insert(EndpointCategory::Auth, RateLimit::new(5, 60));

    let limiter = RateLimiter::new(Arc::new(InMemoryRateLimitStore::new()), config);

    for i in 0..5 {
        let result = limiter.check("test_ip", EndpointCategory::Auth).await.unwrap();
        if let RateLimitResult::Allowed { remaining } = result {
            assert_eq!(remaining, 4 - i, "Remaining should decrement");
        } else {
            panic!("Should be allowed");
        }
    }
}

// ============================================================================
// Property 15: Sliding Window Correctness
// **Validates: Requirements 5.5**
//
// *For any* request made at time T, it SHALL only count against windows
// that include time T, preventing burst attacks at window boundaries.
// ============================================================================

/// Feature: production-readiness, Property 15: Sliding Window Correctness
///
/// The sliding window should prevent burst attacks at boundaries.
#[tokio::test]
async fn property_15_sliding_window_prevents_boundary_burst() {
    let mut config = RateLimiterConfig::production();
    config.limits.insert(EndpointCategory::Auth, RateLimit::new(10, 1)); // 10 per second

    let limiter = RateLimiter::new(Arc::new(InMemoryRateLimitStore::new()), config);

    // Make 10 requests (exhaust limit)
    for _ in 0..10 {
        let _ = limiter.check("test_ip", EndpointCategory::Auth).await;
    }

    // Should be limited
    let result = limiter.check("test_ip", EndpointCategory::Auth).await.unwrap();
    assert!(
        matches!(result, RateLimitResult::Limited { .. }),
        "Should be limited after exhausting quota"
    );

    // Wait for window to expire
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Should be allowed again
    let result = limiter.check("test_ip", EndpointCategory::Auth).await.unwrap();
    assert!(
        matches!(result, RateLimitResult::Allowed { .. }),
        "Should be allowed after window expires"
    );
}

proptest! {
    /// Feature: production-readiness, Property 15: Sliding Window Correctness (category isolation)
    ///
    /// Different endpoint categories should have isolated rate limits.
    #[test]
    fn property_15_category_isolation(auth_requests in 1u32..15, _api_requests in 1u32..50) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let limiter = RateLimiter::production();

            // Make auth requests
            for _ in 0..auth_requests.min(10) {
                let _ = limiter.check("test_ip", EndpointCategory::Auth).await;
            }

            // API requests should still be allowed (separate limit)
            let result = limiter.check("test_ip", EndpointCategory::Api).await.unwrap();
            prop_assert!(
                matches!(result, RateLimitResult::Allowed { .. }),
                "API should be allowed even if Auth is exhausted"
            );

            Ok(())
        })?;
    }

    /// Feature: production-readiness, Property 15: Sliding Window Correctness (IP isolation)
    ///
    /// Different IPs should have isolated rate limits.
    #[test]
    fn property_15_ip_isolation(
        ip1 in "192\\.168\\.1\\.[0-9]{1,3}",
        ip2 in "10\\.0\\.0\\.[0-9]{1,3}"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut config = RateLimiterConfig::production();
            config.limits.insert(EndpointCategory::Auth, RateLimit::new(5, 60));

            let limiter = RateLimiter::new(
                Arc::new(InMemoryRateLimitStore::new()),
                config,
            );

            // Exhaust limit for IP1
            for _ in 0..5 {
                let _ = limiter.check(&ip1, EndpointCategory::Auth).await;
            }

            // IP1 should be limited
            let result1 = limiter.check(&ip1, EndpointCategory::Auth).await.unwrap();
            prop_assert!(
                matches!(result1, RateLimitResult::Limited { .. }),
                "IP1 should be limited"
            );

            // IP2 should still be allowed
            let result2 = limiter.check(&ip2, EndpointCategory::Auth).await.unwrap();
            prop_assert!(
                matches!(result2, RateLimitResult::Allowed { .. }),
                "IP2 should be allowed"
            );

            Ok(())
        })?;
    }
}

/// Feature: production-readiness, Property 15: Sliding Window Correctness (path categorization)
///
/// Paths should be correctly categorized.
#[test]
fn property_15_path_categorization() {
    // Auth paths
    assert_eq!(categorize_path("/api/auth/login"), EndpointCategory::Auth);
    assert_eq!(categorize_path("/api/auth/refresh"), EndpointCategory::Auth);
    assert_eq!(categorize_path("/api/auth/logout"), EndpointCategory::Auth);

    // API paths
    assert_eq!(categorize_path("/api/users"), EndpointCategory::Api);
    assert_eq!(categorize_path("/api/data"), EndpointCategory::Api);
    assert_eq!(categorize_path("/rpc"), EndpointCategory::Api);

    // Static paths
    assert_eq!(categorize_path("/static/app.js"), EndpointCategory::Static);
    assert_eq!(categorize_path("/bundle.js"), EndpointCategory::Static);
    assert_eq!(categorize_path("/styles.css"), EndpointCategory::Static);
    assert_eq!(categorize_path("/app.wasm"), EndpointCategory::Static);
    assert_eq!(categorize_path("/favicon.ico"), EndpointCategory::Static);
}

// ============================================================================
// Property 16: CSRF Token Uniqueness
// **Validates: Requirements 6.1**
//
// *For any* two CSRF tokens generated for different sessions, they SHALL be
// cryptographically distinct (collision probability < 2^-128).
// ============================================================================

use dx_www_server::csrf::CsrfManager;
use std::collections::HashSet;

/// Feature: production-readiness, Property 16: CSRF Token Uniqueness
///
/// Tokens generated for the same session should be unique (different nonces).
#[test]
fn property_16_same_session_tokens_unique() {
    let manager = CsrfManager::new();
    let session_id = "session123";

    // Generate multiple tokens for the same session
    let mut tokens = HashSet::new();
    for _ in 0..100 {
        let token = manager.generate(session_id);
        assert!(tokens.insert(token.value.clone()), "Token collision detected for same session");
    }
}

/// Feature: production-readiness, Property 16: CSRF Token Uniqueness
///
/// Tokens generated for different sessions should be unique.
#[test]
fn property_16_different_session_tokens_unique() {
    let manager = CsrfManager::new();

    let mut tokens = HashSet::new();
    for i in 0..100 {
        let session_id = format!("session_{}", i);
        let token = manager.generate(&session_id);
        assert!(
            tokens.insert(token.value.clone()),
            "Token collision detected across different sessions"
        );
    }
}

proptest! {
    /// Feature: production-readiness, Property 16: CSRF Token Uniqueness
    ///
    /// For any two session IDs, generated tokens should be cryptographically distinct.
    #[test]
    fn property_16_tokens_unique_for_any_sessions(
        session1 in "[a-zA-Z0-9]{8,32}",
        session2 in "[a-zA-Z0-9]{8,32}"
    ) {
        let manager = CsrfManager::new();

        let token1 = manager.generate(&session1);
        let token2 = manager.generate(&session2);

        // Tokens should always be different due to random nonce
        prop_assert_ne!(
            token1.value, token2.value,
            "Tokens for sessions '{}' and '{}' should be unique",
            session1, session2
        );
    }

    /// Feature: production-readiness, Property 16: CSRF Token Uniqueness
    ///
    /// Multiple tokens for the same session should all be unique.
    #[test]
    fn property_16_multiple_tokens_same_session_unique(
        session_id in "[a-zA-Z0-9]{8,32}",
        num_tokens in 2usize..50
    ) {
        let manager = CsrfManager::new();

        let mut tokens = HashSet::new();
        for _ in 0..num_tokens {
            let token = manager.generate(&session_id);
            prop_assert!(
                tokens.insert(token.value.clone()),
                "Token collision detected after generating {} tokens for session '{}'",
                tokens.len(), session_id
            );
        }

        prop_assert_eq!(
            tokens.len(), num_tokens,
            "Should have {} unique tokens", num_tokens
        );
    }

    /// Feature: production-readiness, Property 16: CSRF Token Uniqueness
    ///
    /// Tokens from different managers (different secrets) should be unique.
    #[test]
    fn property_16_different_managers_unique_tokens(
        session_id in "[a-zA-Z0-9]{8,32}"
    ) {
        let manager1 = CsrfManager::new();
        let manager2 = CsrfManager::new();

        let token1 = manager1.generate(&session_id);
        let token2 = manager2.generate(&session_id);

        // Different managers have different secrets, so tokens should differ
        prop_assert_ne!(
            token1.value, token2.value,
            "Tokens from different managers should be unique"
        );
    }

    /// Feature: production-readiness, Property 16: CSRF Token Uniqueness
    ///
    /// Token values should have sufficient entropy (length check as proxy).
    #[test]
    fn property_16_token_has_sufficient_entropy(
        session_id in "[a-zA-Z0-9]{1,64}"
    ) {
        let manager = CsrfManager::new();
        let token = manager.generate(&session_id);

        // Token should be base64 encoded: 56 bytes -> ~75 chars
        // Minimum expected length for cryptographic security
        prop_assert!(
            token.value.len() >= 70,
            "Token should have sufficient length for cryptographic security, got {}",
            token.value.len()
        );
    }
}

/// Feature: production-readiness, Property 16: CSRF Token Uniqueness
///
/// Large-scale uniqueness test - generate many tokens and verify no collisions.
#[test]
fn property_16_large_scale_uniqueness() {
    let manager = CsrfManager::new();
    let mut tokens = HashSet::new();

    // Generate 1000 tokens across 100 sessions
    for session_num in 0..100 {
        let session_id = format!("session_{}", session_num);
        for _ in 0..10 {
            let token = manager.generate(&session_id);
            assert!(
                tokens.insert(token.value.clone()),
                "Token collision detected in large-scale test"
            );
        }
    }

    assert_eq!(tokens.len(), 1000, "Should have 1000 unique tokens");
}

/// Feature: production-readiness, Property 16: CSRF Token Uniqueness
///
/// Tokens with same secret but generated at different times should be unique.
#[test]
fn property_16_temporal_uniqueness() {
    let secret = [42u8; 32];
    let manager = CsrfManager::with_secret(secret);

    let mut tokens = HashSet::new();
    for _ in 0..100 {
        let token = manager.generate("same_session");
        assert!(
            tokens.insert(token.value.clone()),
            "Temporal collision detected - tokens should be unique even with same secret"
        );
    }
}

// ============================================================================
// Property 17: CSRF Validation Strictness
// **Validates: Requirements 6.3, 6.4, 6.5**
//
// *For any* POST/PUT/DELETE request, it SHALL be rejected with HTTP 403 if
// the CSRF token is missing, invalid, expired, or bound to a different session.
// ============================================================================

use chrono::Duration;
use dx_www_server::csrf::{CSRF_HEADER, CsrfError};

/// Feature: production-readiness, Property 17: CSRF Validation Strictness
///
/// Missing CSRF token should result in validation failure.
#[test]
fn property_17_missing_token_rejected() {
    let manager = CsrfManager::new();
    let session_id = "session123";

    // Validate with empty token should fail
    let result = manager.validate("", session_id);
    assert!(
        matches!(result, Err(CsrfError::Invalid)),
        "Empty token should be rejected as invalid"
    );
}

/// Feature: production-readiness, Property 17: CSRF Validation Strictness
///
/// Invalid (malformed) CSRF token should result in validation failure.
#[test]
fn property_17_invalid_token_rejected() {
    let manager = CsrfManager::new();
    let session_id = "session123";

    // Various invalid tokens
    let invalid_tokens = [
        "not_base64!@#$",
        "dG9vX3Nob3J0", // too short (valid base64 but wrong length)
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", // wrong length
        "invalid_token_format",
    ];

    for token in &invalid_tokens {
        let result = manager.validate(token, session_id);
        assert!(
            matches!(result, Err(CsrfError::Invalid)),
            "Invalid token '{}' should be rejected",
            token
        );
    }
}

/// Feature: production-readiness, Property 17: CSRF Validation Strictness
///
/// Expired CSRF token should result in validation failure.
#[test]
fn property_17_expired_token_rejected() {
    // Create manager with very short TTL
    let manager = CsrfManager::with_secret([42u8; 32]).with_ttl(Duration::seconds(-1));
    let session_id = "session123";

    let token = manager.generate(session_id);

    // Token should be expired immediately
    let result = manager.validate(&token.value, session_id);
    assert!(matches!(result, Err(CsrfError::Expired)), "Expired token should be rejected");
}

/// Feature: production-readiness, Property 17: CSRF Validation Strictness
///
/// Token bound to different session should result in validation failure.
#[test]
fn property_17_wrong_session_rejected() {
    let manager = CsrfManager::new();

    let token = manager.generate("session_A");

    // Validate with different session should fail
    let result = manager.validate(&token.value, "session_B");
    assert!(
        matches!(result, Err(CsrfError::Invalid)),
        "Token for different session should be rejected"
    );
}

proptest! {
    /// Feature: production-readiness, Property 17: CSRF Validation Strictness
    ///
    /// For any session ID, a token generated for that session should only validate
    /// for that exact session.
    #[test]
    fn property_17_session_binding_strict(
        session1 in "[a-zA-Z0-9]{8,32}",
        session2 in "[a-zA-Z0-9]{8,32}"
    ) {
        prop_assume!(session1 != session2);

        let manager = CsrfManager::new();
        let token = manager.generate(&session1);

        // Should validate for correct session
        prop_assert!(
            manager.validate(&token.value, &session1).is_ok(),
            "Token should validate for its own session"
        );

        // Should NOT validate for different session
        prop_assert!(
            matches!(manager.validate(&token.value, &session2), Err(CsrfError::Invalid)),
            "Token should NOT validate for different session"
        );
    }

    /// Feature: production-readiness, Property 17: CSRF Validation Strictness
    ///
    /// For any random byte sequence, validation should either succeed (if valid)
    /// or return an appropriate error (Invalid or Expired), never panic.
    #[test]
    fn property_17_random_input_handled_gracefully(
        random_bytes in prop::collection::vec(any::<u8>(), 0..100),
        session_id in "[a-zA-Z0-9]{8,32}"
    ) {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

        let manager = CsrfManager::new();
        let random_token = URL_SAFE_NO_PAD.encode(&random_bytes);

        // Should not panic, should return an error
        let result = manager.validate(&random_token, &session_id);
        prop_assert!(
            result.is_err(),
            "Random token should be rejected"
        );
    }

    /// Feature: production-readiness, Property 17: CSRF Validation Strictness
    ///
    /// Tokens should be rejected after expiration time.
    #[test]
    fn property_17_expiration_enforced(
        session_id in "[a-zA-Z0-9]{8,32}",
        ttl_seconds in 60i64..3600  // Use longer TTL to avoid timing issues
    ) {
        let manager = CsrfManager::with_secret([42u8; 32])
            .with_ttl(Duration::seconds(ttl_seconds));
        let token = manager.generate(&session_id);

        // Should be valid at creation time
        prop_assert!(
            manager.validate_at(&token.value, &session_id, token.created_at).is_ok(),
            "Token should be valid at creation time"
        );

        // Should be valid 1 second before expiration
        let before_expiry = token.expires_at - Duration::seconds(1);
        prop_assert!(
            manager.validate_at(&token.value, &session_id, before_expiry).is_ok(),
            "Token should be valid before expiration"
        );

        // Should be invalid at expiration time
        prop_assert!(
            matches!(
                manager.validate_at(&token.value, &session_id, token.expires_at),
                Err(CsrfError::Expired)
            ),
            "Token should be expired at expiration time"
        );

        // Should be invalid after expiration
        let after = token.expires_at + Duration::hours(1);
        prop_assert!(
            matches!(
                manager.validate_at(&token.value, &session_id, after),
                Err(CsrfError::Expired)
            ),
            "Token should be expired after expiration time"
        );
    }

    /// Feature: production-readiness, Property 17: CSRF Validation Strictness
    ///
    /// Tampered tokens should be rejected.
    #[test]
    fn property_17_tampered_token_rejected(
        session_id in "[a-zA-Z0-9]{8,32}",
        tamper_position in 0usize..56
    ) {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

        let manager = CsrfManager::new();
        let token = manager.generate(&session_id);

        // Decode, tamper, re-encode
        let mut bytes = URL_SAFE_NO_PAD.decode(&token.value).unwrap();
        if tamper_position < bytes.len() {
            bytes[tamper_position] ^= 0xFF; // Flip all bits at position
        }
        let tampered = URL_SAFE_NO_PAD.encode(&bytes);

        // Tampered token should be rejected
        let result = manager.validate(&tampered, &session_id);
        prop_assert!(
            result.is_err(),
            "Tampered token should be rejected"
        );
    }
}

/// Feature: production-readiness, Property 17: CSRF Validation Strictness
///
/// Valid token should pass validation.
#[test]
fn property_17_valid_token_accepted() {
    let manager = CsrfManager::new();
    let session_id = "session123";

    let token = manager.generate(session_id);

    // Valid token should pass
    assert!(
        manager.validate(&token.value, session_id).is_ok(),
        "Valid token should be accepted"
    );
}

// ============================================================================
// Property 18: CSRF Token Location Flexibility
// **Validates: Requirements 6.6**
//
// *For any* valid CSRF token, it SHALL be accepted whether provided in a
// form field or a custom header.
// ============================================================================

use dx_www_server::csrf::{CSRF_FIELD, extract_csrf_from_form_body};

/// Feature: production-readiness, Property 18: CSRF Token Location Flexibility
///
/// Token should be extractable from form body.
#[test]
fn property_18_token_from_form_body() {
    let manager = CsrfManager::new();
    let session_id = "session123";
    let token = manager.generate(session_id);

    // Create form body with CSRF token
    let form_body = format!("{}={}&name=test", CSRF_FIELD, token.value);

    // Extract token from form body
    let extracted = extract_csrf_from_form_body(&form_body);
    assert_eq!(
        extracted,
        Some(token.value.clone()),
        "Token should be extractable from form body"
    );

    // Validate extracted token
    assert!(
        manager.validate(&extracted.unwrap(), session_id).is_ok(),
        "Extracted token should be valid"
    );
}

/// Feature: production-readiness, Property 18: CSRF Token Location Flexibility
///
/// Token should be extractable from form body at any position.
#[test]
fn property_18_token_position_in_form() {
    let manager = CsrfManager::new();
    let session_id = "session123";
    let token = manager.generate(session_id);

    // Token at beginning
    let form1 = format!("{}={}&name=test&value=123", CSRF_FIELD, token.value);
    assert_eq!(
        extract_csrf_from_form_body(&form1),
        Some(token.value.clone()),
        "Token at beginning should be extractable"
    );

    // Token in middle
    let form2 = format!("name=test&{}={}&value=123", CSRF_FIELD, token.value);
    assert_eq!(
        extract_csrf_from_form_body(&form2),
        Some(token.value.clone()),
        "Token in middle should be extractable"
    );

    // Token at end
    let form3 = format!("name=test&value=123&{}={}", CSRF_FIELD, token.value);
    assert_eq!(
        extract_csrf_from_form_body(&form3),
        Some(token.value.clone()),
        "Token at end should be extractable"
    );
}

/// Feature: production-readiness, Property 18: CSRF Token Location Flexibility
///
/// Missing token in form body should return None.
#[test]
fn property_18_missing_token_in_form() {
    let form_body = "name=test&value=123";
    assert_eq!(extract_csrf_from_form_body(form_body), None, "Missing token should return None");
}

/// Feature: production-readiness, Property 18: CSRF Token Location Flexibility
///
/// Header constant should be correct.
#[test]
fn property_18_header_constant_correct() {
    assert_eq!(CSRF_HEADER, "x-csrf-token", "CSRF header should be 'x-csrf-token'");
}

/// Feature: production-readiness, Property 18: CSRF Token Location Flexibility
///
/// Form field constant should be correct.
#[test]
fn property_18_form_field_constant_correct() {
    assert_eq!(CSRF_FIELD, "_csrf", "CSRF form field should be '_csrf'");
}

proptest! {
    /// Feature: production-readiness, Property 18: CSRF Token Location Flexibility
    ///
    /// For any valid token, it should be extractable from form body regardless
    /// of other form fields present.
    #[test]
    fn property_18_token_extractable_with_other_fields(
        session_id in "[a-zA-Z0-9]{8,32}",
        field_count in 0usize..10,
        field_names in prop::collection::vec("[a-zA-Z][a-zA-Z0-9]{0,10}", 0..10),
        field_values in prop::collection::vec("[a-zA-Z0-9]{0,20}", 0..10)
    ) {
        let manager = CsrfManager::new();
        let token = manager.generate(&session_id);

        // Build form body with random fields
        let mut parts: Vec<String> = Vec::new();
        for i in 0..field_count.min(field_names.len()).min(field_values.len()) {
            // Skip if field name would conflict with CSRF field
            if field_names[i] != CSRF_FIELD {
                parts.push(format!("{}={}", field_names[i], field_values[i]));
            }
        }

        // Add CSRF token at random position
        let csrf_part = format!("{}={}", CSRF_FIELD, token.value);
        if parts.is_empty() {
            parts.push(csrf_part);
        } else {
            let insert_pos = field_count % (parts.len() + 1);
            parts.insert(insert_pos, csrf_part);
        }

        let form_body = parts.join("&");

        // Extract and validate
        let extracted = extract_csrf_from_form_body(&form_body);
        prop_assert_eq!(
            extracted.clone(),
            Some(token.value.clone()),
            "Token should be extractable from form body: {}", form_body
        );

        // Validate extracted token
        prop_assert!(
            manager.validate(&extracted.unwrap(), &session_id).is_ok(),
            "Extracted token should be valid"
        );
    }

    /// Feature: production-readiness, Property 18: CSRF Token Location Flexibility
    ///
    /// URL-encoded tokens in form body should be correctly decoded and validated.
    #[test]
    fn property_18_url_encoded_token_handled(
        session_id in "[a-zA-Z0-9]{8,32}"
    ) {
        let manager = CsrfManager::new();
        let token = manager.generate(&session_id);

        // URL encode the token (replace + with %2B, / with %2F, = with %3D)
        let encoded_token = token.value
            .replace('+', "%2B")
            .replace('/', "%2F")
            .replace('=', "%3D");

        let form_body = format!("{}={}&name=test", CSRF_FIELD, encoded_token);

        // Extract should decode the token
        let extracted = extract_csrf_from_form_body(&form_body);
        prop_assert!(
            extracted.is_some(),
            "URL-encoded token should be extractable"
        );

        // The extracted token should match the original
        prop_assert_eq!(
            extracted.unwrap(),
            token.value,
            "Decoded token should match original"
        );
    }
}

/// Feature: production-readiness, Property 18: CSRF Token Location Flexibility
///
/// Empty form body should return None.
#[test]
fn property_18_empty_form_body() {
    assert_eq!(extract_csrf_from_form_body(""), None, "Empty form body should return None");
}

/// Feature: production-readiness, Property 18: CSRF Token Location Flexibility
///
/// Token validation should work the same regardless of extraction source.
#[test]
fn property_18_validation_source_independent() {
    let manager = CsrfManager::new();
    let session_id = "session123";
    let token = manager.generate(session_id);

    // Extract from form body
    let form_body = format!("{}={}", CSRF_FIELD, token.value);
    let from_form = extract_csrf_from_form_body(&form_body).unwrap();

    // Direct token value
    let direct = token.value.clone();

    // Both should validate identically
    assert!(
        manager.validate(&from_form, session_id).is_ok(),
        "Token from form should validate"
    );
    assert!(manager.validate(&direct, session_id).is_ok(), "Direct token should validate");

    // Both should be equal
    assert_eq!(from_form, direct, "Extracted and direct tokens should be equal");
}

// ============================================================================
// Property 26: Error Response Sanitization
// **Validates: Requirements 7.5**
//
// *For any* unhandled exception in production mode, the error response SHALL NOT
// contain stack traces, internal paths, or sensitive configuration details.
// ============================================================================

use dx_www_server::error_handler::{
    Environment, ErrorHandler, SENSITIVE_PATTERNS, contains_sensitive_info, sanitize_string,
};

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// Production error handler should hide internal error details.
#[test]
fn property_26_production_hides_internal_details() {
    let handler = ErrorHandler::production();
    assert!(handler.is_production());
    assert!(!handler.is_development());
}

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// Development error handler should show internal error details.
#[test]
fn property_26_development_shows_internal_details() {
    let handler = ErrorHandler::development();
    assert!(handler.is_development());
    assert!(!handler.is_production());
}

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// Production error responses should not contain stack traces.
#[tokio::test]
async fn property_26_no_stack_traces_in_production() {
    let handler = ErrorHandler::production();
    let error = std::io::Error::new(
        std::io::ErrorKind::Other,
        "Stack trace: at main.rs:42\n  at lib.rs:100",
    );
    let request_id = "test-123";

    let response = handler.internal_error(&error, request_id);

    // Get the response body
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);

    // Should not contain stack trace details
    assert!(
        !body_str.contains("main.rs:42"),
        "Production response should not contain stack trace line numbers"
    );
    assert!(
        !body_str.contains("lib.rs:100"),
        "Production response should not contain stack trace file references"
    );
}

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// Production error responses should not contain internal file paths.
#[tokio::test]
async fn property_26_no_file_paths_in_production() {
    let handler = ErrorHandler::production();
    let error = std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "File not found: /home/user/app/config/secrets.json",
    );
    let request_id = "test-123";

    let response = handler.internal_error(&error, request_id);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);

    // Should not contain file paths
    assert!(
        !body_str.contains("/home/user"),
        "Production response should not contain Unix file paths"
    );
    assert!(
        !body_str.contains("secrets.json"),
        "Production response should not contain sensitive file names"
    );
}

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// Production error responses should not contain Windows file paths.
#[tokio::test]
async fn property_26_no_windows_paths_in_production() {
    let handler = ErrorHandler::production();
    let error = std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "File not found: C:\\Users\\admin\\AppData\\config.json",
    );
    let request_id = "test-123";

    let response = handler.internal_error(&error, request_id);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);

    // Should not contain Windows paths
    assert!(
        !body_str.contains("C:\\Users"),
        "Production response should not contain Windows file paths"
    );
}

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// Production error responses should not contain database connection details.
#[tokio::test]
async fn property_26_no_database_details_in_production() {
    let handler = ErrorHandler::production();
    let error = std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        "Connection to database postgres://admin:password123@localhost:5432/mydb failed",
    );
    let request_id = "test-123";

    let response = handler.internal_error(&error, request_id);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);

    // Should not contain database connection details
    assert!(
        !body_str.contains("postgres://"),
        "Production response should not contain database URLs"
    );
    assert!(
        !body_str.contains("password123"),
        "Production response should not contain passwords"
    );
    assert!(
        !body_str.contains("localhost:5432"),
        "Production response should not contain database host details"
    );
}

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// Production error responses should not contain sensitive configuration.
#[tokio::test]
async fn property_26_no_sensitive_config_in_production() {
    let handler = ErrorHandler::production();
    let error =
        std::io::Error::new(std::io::ErrorKind::Other, "Invalid API_KEY: sk_live_abc123xyz");
    let request_id = "test-123";

    let response = handler.internal_error(&error, request_id);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);

    // Should not contain API keys
    assert!(
        !body_str.contains("sk_live_abc123xyz"),
        "Production response should not contain API keys"
    );
}

proptest! {
    /// Feature: production-readiness, Property 26: Error Response Sanitization
    ///
    /// For any error message containing sensitive patterns, production mode
    /// should sanitize the response.
    #[test]
    fn property_26_sanitizes_sensitive_patterns(
        pattern_idx in 0usize..SENSITIVE_PATTERNS.len(),
        random_value in "[a-zA-Z0-9]{8,32}"
    ) {
        let pattern = SENSITIVE_PATTERNS[pattern_idx];
        let message = format!("Error: {}={}", pattern, random_value);

        // Should be detected as sensitive
        prop_assert!(
            contains_sensitive_info(&message),
            "Message containing '{}' should be detected as sensitive",
            pattern
        );

        // Sanitize should redact
        let sanitized = sanitize_string(&message);
        prop_assert_eq!(
            sanitized,
            "[REDACTED]",
            "Sensitive message should be redacted"
        );
    }

    /// Feature: production-readiness, Property 26: Error Response Sanitization
    ///
    /// For any non-sensitive message, sanitization should preserve the content.
    #[test]
    fn property_26_preserves_non_sensitive_messages(
        message in "[A-Za-z ]{10,50}"
    ) {
        // Skip if message accidentally contains a sensitive pattern
        prop_assume!(!contains_sensitive_info(&message));

        let sanitized = sanitize_string(&message);
        prop_assert_eq!(
            sanitized,
            message,
            "Non-sensitive message should be preserved"
        );
    }

    /// Feature: production-readiness, Property 26: Error Response Sanitization
    ///
    /// For any Unix-style file path, production mode should sanitize it.
    #[test]
    fn property_26_sanitizes_unix_paths(
        dir1 in "[a-z]{3,10}",
        dir2 in "[a-z]{3,10}",
        filename in "[a-z]{3,10}",
        ext in "(rs|json|toml|yaml|txt)"
    ) {
        let handler = ErrorHandler::production();
        let path = format!("/{}/{}/{}.{}", dir1, dir2, filename, ext);
        let message = format!("Error reading file {}", path);

        let sanitized = handler.sanitize_message(&message);

        // Should not contain the original path
        prop_assert!(
            !sanitized.contains(&path),
            "Sanitized message should not contain path: {}",
            path
        );
        prop_assert!(
            sanitized.contains("[path]"),
            "Sanitized message should contain [path] placeholder"
        );
    }

    /// Feature: production-readiness, Property 26: Error Response Sanitization
    ///
    /// For any Windows-style file path, production mode should sanitize it.
    #[test]
    fn property_26_sanitizes_windows_paths(
        drive in "[A-Z]",
        dir1 in "[A-Za-z]{3,10}",
        dir2 in "[A-Za-z]{3,10}",
        filename in "[a-z]{3,10}",
        ext in "(rs|json|toml|yaml|txt)"
    ) {
        let handler = ErrorHandler::production();
        let path = format!("{}:\\{}\\{}\\{}.{}", drive, dir1, dir2, filename, ext);
        let message = format!("Error reading file {}", path);

        let sanitized = handler.sanitize_message(&message);

        // Should not contain the original path
        prop_assert!(
            !sanitized.contains(&format!("{}:\\", drive)),
            "Sanitized message should not contain Windows drive letter"
        );
    }

    /// Feature: production-readiness, Property 26: Error Response Sanitization
    ///
    /// Production error responses should always include a request ID.
    #[test]
    fn property_26_includes_request_id(
        request_id in "[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let handler = ErrorHandler::production();
            let error = std::io::Error::new(std::io::ErrorKind::Other, "Test error");

            let response = handler.internal_error(&error, &request_id);

            // Response should have request ID header
            prop_assert!(
                response.headers().contains_key("x-request-id"),
                "Response should contain X-Request-ID header"
            );

            let header_value = response
                .headers()
                .get("x-request-id")
                .unwrap()
                .to_str()
                .unwrap();

            prop_assert_eq!(
                header_value,
                request_id,
                "X-Request-ID header should match provided request ID"
            );

            Ok(())
        })?;
    }

    /// Feature: production-readiness, Property 26: Error Response Sanitization
    ///
    /// Development mode should show full error details.
    #[test]
    fn property_26_development_shows_details(
        error_message in "[A-Za-z0-9 ]{10,50}"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let handler = ErrorHandler::development();
            let error = std::io::Error::new(std::io::ErrorKind::Other, error_message.clone());
            let request_id = "test-123";

            let response = handler.internal_error(&error, request_id);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body_str = String::from_utf8_lossy(&body);

            // Development mode should show the error message
            prop_assert!(
                body_str.contains(&error_message),
                "Development response should contain error message: {}",
                error_message
            );

            Ok(())
        })?;
    }
}

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// All sensitive patterns should be detected.
#[test]
fn property_26_all_sensitive_patterns_detected() {
    for pattern in SENSITIVE_PATTERNS {
        let message = format!("Error: {}=value123", pattern);
        assert!(
            contains_sensitive_info(&message),
            "Pattern '{}' should be detected as sensitive",
            pattern
        );
    }
}

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// Error handler should generate unique request IDs.
#[test]
fn property_26_unique_request_ids() {
    let handler = ErrorHandler::production();
    let mut ids = std::collections::HashSet::new();

    for _ in 0..100 {
        let id = handler.generate_request_id();
        assert!(ids.insert(id.clone()), "Request ID should be unique: {}", id);
    }

    assert_eq!(ids.len(), 100, "Should have 100 unique request IDs");
}

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// Production mode should return generic messages for different error types.
#[tokio::test]
async fn property_26_generic_messages_for_all_error_types() {
    let handler = ErrorHandler::production();
    let request_id = "test-123";

    // Test not found
    let response = handler.not_found("/secret/path", request_id);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(!body_str.contains("/secret/path"), "Not found response should not contain path");

    // Test unauthorized
    let response = handler.unauthorized("Invalid token xyz123", request_id);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(
        !body_str.contains("xyz123"),
        "Unauthorized response should not contain token details"
    );

    // Test forbidden
    let response = handler.forbidden("User admin lacks permission", request_id);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(
        !body_str.contains("admin"),
        "Forbidden response should not contain user details"
    );
}

/// Feature: production-readiness, Property 26: Error Response Sanitization
///
/// Environment detection should work correctly.
#[test]
fn property_26_environment_detection() {
    // Test explicit environment creation
    let prod = ErrorHandler::new(Environment::Production);
    assert!(prod.is_production());

    let dev = ErrorHandler::new(Environment::Development);
    assert!(dev.is_development());
}

// ============================================================================
// Property 3: Graceful Shutdown Completion
// **Validates: Requirements 3.1**
//
// *For any* set of in-flight requests when SIGTERM is received, if the total
// processing time is less than the configured timeout, all requests SHALL
// complete successfully before the server terminates.
// ============================================================================

use dx_www_server::ops::shutdown::ConnectionGuard;
use dx_www_server::ops::{GracefulShutdown, ShutdownConfig};

/// Feature: production-excellence, Property 3: Graceful Shutdown Completion
///
/// When all in-flight requests complete before timeout, shutdown should succeed.
#[tokio::test]
async fn property_3_graceful_shutdown_completes_when_requests_finish_before_timeout() {
    let config = ShutdownConfig::with_timeout(std::time::Duration::from_secs(5));
    let shutdown = Arc::new(GracefulShutdown::new(config));

    // Start some connections
    shutdown.connection_started();
    shutdown.connection_started();
    shutdown.connection_started();

    assert_eq!(shutdown.active_connections(), 3);

    // Spawn task to finish connections after a short delay (well before timeout)
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        shutdown_clone.connection_finished();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        shutdown_clone.connection_finished();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        shutdown_clone.connection_finished();
    });

    // Shutdown should wait for connections and succeed
    let result = shutdown.shutdown().await;
    assert!(
        result.is_ok(),
        "Shutdown should succeed when all requests complete before timeout"
    );
    assert_eq!(shutdown.active_connections(), 0);
}

proptest! {
    /// Feature: production-excellence, Property 3: Graceful Shutdown Completion
    ///
    /// **Validates: Requirements 3.1**
    ///
    /// For any number of in-flight requests with processing times less than
    /// the configured timeout, all requests SHALL complete successfully.
    #[test]
    fn property_3_all_requests_complete_when_processing_time_less_than_timeout(
        // Processing time per request in milliseconds (10-100ms each)
        // The number of requests is determined by the length of this vector
        processing_times in prop::collection::vec(10u64..100, 1..20),
        // Timeout in milliseconds (must be greater than total processing time)
        timeout_buffer_ms in 500u64..2000
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Number of requests is determined by processing_times length
            let num_requests = processing_times.len();

            // Calculate total processing time (sequential worst case)
            let total_processing_ms: u64 = processing_times.iter().sum();

            // Set timeout to be greater than total processing time
            let timeout_ms = total_processing_ms + timeout_buffer_ms;
            let config = ShutdownConfig::with_timeout(std::time::Duration::from_millis(timeout_ms));
            let shutdown = Arc::new(GracefulShutdown::new(config));

            // Start the specified number of connections
            for _ in 0..num_requests {
                shutdown.connection_started();
            }

            prop_assert_eq!(
                shutdown.active_connections(),
                num_requests,
                "Should have {} active connections",
                num_requests
            );

            // Spawn tasks to simulate request processing
            let shutdown_clone = shutdown.clone();
            let processing_times_clone = processing_times.clone();

            tokio::spawn(async move {
                for processing_time in processing_times_clone {
                    tokio::time::sleep(std::time::Duration::from_millis(processing_time)).await;
                    shutdown_clone.connection_finished();
                }
            });

            // Initiate shutdown - should succeed since processing time < timeout
            let result = shutdown.shutdown().await;

            prop_assert!(
                result.is_ok(),
                "Shutdown should succeed when total processing time ({} ms) < timeout ({} ms)",
                total_processing_ms,
                timeout_ms
            );

            prop_assert_eq!(
                shutdown.active_connections(),
                0,
                "All connections should be drained after successful shutdown"
            );

            Ok(())
        })?;
    }

    /// Feature: production-excellence, Property 3: Graceful Shutdown Completion
    ///
    /// **Validates: Requirements 3.1**
    ///
    /// For any set of concurrent requests that all complete before timeout,
    /// shutdown should succeed regardless of completion order.
    #[test]
    fn property_3_concurrent_requests_complete_in_any_order(
        num_requests in 2usize..10,
        // Random delays for each request (simulating different processing times)
        delays in prop::collection::vec(5u64..50, 2..10)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Use a generous timeout that's definitely longer than all delays
            let config = ShutdownConfig::with_timeout(std::time::Duration::from_secs(2));
            let shutdown = Arc::new(GracefulShutdown::new(config));

            // Start connections
            let actual_requests = num_requests.min(delays.len());
            for _ in 0..actual_requests {
                shutdown.connection_started();
            }

            // Spawn concurrent tasks that complete in different orders
            let mut handles = Vec::new();
            for i in 0..actual_requests {
                let shutdown_clone = shutdown.clone();
                let delay = delays[i];
                handles.push(tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    shutdown_clone.connection_finished();
                }));
            }

            // Initiate shutdown
            let result = shutdown.shutdown().await;

            // Wait for all tasks to complete
            for handle in handles {
                let _ = handle.await;
            }

            prop_assert!(
                result.is_ok(),
                "Shutdown should succeed when all concurrent requests complete before timeout"
            );

            prop_assert_eq!(
                shutdown.active_connections(),
                0,
                "All connections should be drained"
            );

            Ok(())
        })?;
    }

    /// Feature: production-excellence, Property 3: Graceful Shutdown Completion
    ///
    /// **Validates: Requirements 3.1**
    ///
    /// For any timeout configuration, if no requests are in-flight,
    /// shutdown should complete immediately.
    #[test]
    fn property_3_immediate_shutdown_with_no_requests(
        timeout_secs in 1u64..60
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = ShutdownConfig::with_timeout(std::time::Duration::from_secs(timeout_secs));
            let shutdown = GracefulShutdown::new(config);

            prop_assert_eq!(
                shutdown.active_connections(),
                0,
                "Should start with no active connections"
            );

            let start = std::time::Instant::now();
            let result = shutdown.shutdown().await;
            let elapsed = start.elapsed();

            prop_assert!(
                result.is_ok(),
                "Shutdown should succeed with no active connections"
            );

            // Should complete almost immediately (within 100ms)
            prop_assert!(
                elapsed < std::time::Duration::from_millis(100),
                "Shutdown with no connections should complete quickly, took {:?}",
                elapsed
            );

            Ok(())
        })?;
    }
}

/// Feature: production-excellence, Property 3: Graceful Shutdown Completion
///
/// **Validates: Requirements 3.1**
///
/// ConnectionGuard should properly track request lifecycle.
#[tokio::test]
async fn property_3_connection_guard_tracks_lifecycle() {
    let shutdown = Arc::new(GracefulShutdown::new(ShutdownConfig::default()));

    assert_eq!(shutdown.active_connections(), 0);

    // Create guards that automatically track connections
    {
        let _guard1 = ConnectionGuard::new(shutdown.clone());
        assert_eq!(shutdown.active_connections(), 1);

        {
            let _guard2 = ConnectionGuard::new(shutdown.clone());
            assert_eq!(shutdown.active_connections(), 2);
        }
        // guard2 dropped
        assert_eq!(shutdown.active_connections(), 1);
    }
    // guard1 dropped
    assert_eq!(shutdown.active_connections(), 0);
}

/// Feature: production-excellence, Property 3: Graceful Shutdown Completion
///
/// **Validates: Requirements 3.1**
///
/// Multiple concurrent requests using ConnectionGuard should all complete
/// before shutdown succeeds.
#[tokio::test]
async fn property_3_multiple_guards_complete_before_shutdown() {
    let shutdown = Arc::new(GracefulShutdown::new(ShutdownConfig::with_timeout(
        std::time::Duration::from_secs(5),
    )));

    // Spawn multiple "request handlers" using ConnectionGuard
    let mut handles = Vec::new();
    for i in 0..5 {
        let shutdown_clone = shutdown.clone();
        handles.push(tokio::spawn(async move {
            let _guard = ConnectionGuard::new(shutdown_clone);
            // Simulate varying processing times
            tokio::time::sleep(std::time::Duration::from_millis(10 * (i + 1) as u64)).await;
            // Guard dropped here, connection count decremented
        }));
    }

    // Small delay to ensure all tasks have started
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Initiate shutdown
    let result = shutdown.shutdown().await;

    // Wait for all handles
    for handle in handles {
        let _ = handle.await;
    }

    assert!(result.is_ok(), "Shutdown should succeed when all guarded requests complete");
    assert_eq!(shutdown.active_connections(), 0);
}

/// Feature: production-excellence, Property 3: Graceful Shutdown Completion
///
/// **Validates: Requirements 3.1**
///
/// Shutdown should properly signal subscribers before waiting for connections.
#[tokio::test]
async fn property_3_shutdown_signals_subscribers() {
    let shutdown = Arc::new(GracefulShutdown::new(ShutdownConfig::with_timeout(
        std::time::Duration::from_secs(5),
    )));

    let mut receiver = shutdown.subscribe();

    // Start a connection
    shutdown.connection_started();

    // Spawn task to listen for shutdown signal and then finish connection
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        receiver.wait().await;
        // After receiving signal, finish the connection
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        shutdown_clone.connection_finished();
    });

    // Small delay to ensure subscriber is waiting
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Initiate shutdown
    let result = shutdown.shutdown().await;

    assert!(
        result.is_ok(),
        "Shutdown should succeed after signaling subscribers and waiting for connections"
    );
}

// ============================================================================
// Property 4: Shutdown Timeout Enforcement
// **Validates: Requirements 3.2**
//
// *For any* graceful shutdown with timeout T, if requests are still in-flight
// after T seconds, the server SHALL forcefully terminate those connections
// and exit within T + 1 seconds.
// ============================================================================

use dx_www_server::ops::shutdown::ShutdownError;

/// Feature: production-excellence, Property 4: Shutdown Timeout Enforcement
///
/// **Validates: Requirements 3.2**
///
/// When connections don't complete within timeout, shutdown should return
/// timeout error with correct connection count.
#[tokio::test]
async fn property_4_shutdown_returns_timeout_error_with_connection_count() {
    let timeout_ms = 100;
    let config = ShutdownConfig::with_timeout(std::time::Duration::from_millis(timeout_ms));
    let shutdown = Arc::new(GracefulShutdown::new(config));

    // Start connections that won't complete
    let num_connections = 3;
    for _ in 0..num_connections {
        shutdown.connection_started();
    }

    assert_eq!(shutdown.active_connections(), num_connections);

    // Initiate shutdown - should timeout since connections never finish
    let result = shutdown.shutdown().await;

    match result {
        Err(ShutdownError::Timeout(duration, count)) => {
            assert_eq!(
                duration,
                std::time::Duration::from_millis(timeout_ms),
                "Timeout duration should match configured timeout"
            );
            assert_eq!(
                count, num_connections,
                "Connection count should match number of in-flight connections"
            );
        }
        Ok(()) => panic!("Shutdown should have timed out, not succeeded"),
        Err(e) => panic!("Expected Timeout error, got: {:?}", e),
    }
}

/// Feature: production-excellence, Property 4: Shutdown Timeout Enforcement
///
/// **Validates: Requirements 3.2**
///
/// Shutdown should complete within T + 1 seconds when timeout occurs.
#[tokio::test]
async fn property_4_shutdown_completes_within_timeout_plus_one_second() {
    let timeout_ms = 100;
    let config = ShutdownConfig::with_timeout(std::time::Duration::from_millis(timeout_ms));
    let shutdown = Arc::new(GracefulShutdown::new(config));

    // Start a connection that won't complete
    shutdown.connection_started();

    let start = std::time::Instant::now();
    let result = shutdown.shutdown().await;
    let elapsed = start.elapsed();

    // Should be a timeout error
    assert!(
        matches!(result, Err(ShutdownError::Timeout(_, _))),
        "Should return timeout error"
    );

    // Should complete within T + 1 second (T + 1000ms)
    let max_allowed = std::time::Duration::from_millis(timeout_ms + 1000);
    assert!(
        elapsed <= max_allowed,
        "Shutdown should complete within T + 1 second. Timeout: {}ms, Elapsed: {:?}, Max allowed: {:?}",
        timeout_ms,
        elapsed,
        max_allowed
    );
}

proptest! {
    /// Feature: production-excellence, Property 4: Shutdown Timeout Enforcement
    ///
    /// **Validates: Requirements 3.2**
    ///
    /// For any timeout value T and any number of connections that won't complete,
    /// shutdown SHALL return timeout error with correct connection count.
    #[test]
    fn property_4_timeout_returns_correct_connection_count(
        // Timeout in milliseconds (50-200ms for fast tests)
        timeout_ms in 50u64..200,
        // Number of connections that won't complete
        num_connections in 1usize..10
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = ShutdownConfig::with_timeout(std::time::Duration::from_millis(timeout_ms));
            let shutdown = Arc::new(GracefulShutdown::new(config));

            // Start connections that won't complete
            for _ in 0..num_connections {
                shutdown.connection_started();
            }

            prop_assert_eq!(
                shutdown.active_connections(),
                num_connections,
                "Should have {} active connections",
                num_connections
            );

            // Initiate shutdown - should timeout
            let result = shutdown.shutdown().await;

            match result {
                Err(ShutdownError::Timeout(duration, count)) => {
                    prop_assert_eq!(
                        duration,
                        std::time::Duration::from_millis(timeout_ms),
                        "Timeout duration should match configured timeout"
                    );
                    prop_assert_eq!(
                        count,
                        num_connections,
                        "Connection count should match number of in-flight connections"
                    );
                }
                Ok(()) => {
                    prop_assert!(false, "Shutdown should have timed out, not succeeded");
                }
                Err(e) => {
                    prop_assert!(false, "Expected Timeout error, got: {:?}", e);
                }
            }

            Ok(())
        })?;
    }

    /// Feature: production-excellence, Property 4: Shutdown Timeout Enforcement
    ///
    /// **Validates: Requirements 3.2**
    ///
    /// For any timeout value T, shutdown SHALL complete within T + 1 seconds
    /// when connections don't finish.
    #[test]
    fn property_4_shutdown_completes_within_timeout_plus_buffer(
        // Timeout in milliseconds (50-300ms for reasonable test times)
        timeout_ms in 50u64..300,
        // Number of connections that won't complete
        num_connections in 1usize..5
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = ShutdownConfig::with_timeout(std::time::Duration::from_millis(timeout_ms));
            let shutdown = Arc::new(GracefulShutdown::new(config));

            // Start connections that won't complete
            for _ in 0..num_connections {
                shutdown.connection_started();
            }

            let start = std::time::Instant::now();
            let result = shutdown.shutdown().await;
            let elapsed = start.elapsed();

            // Should be a timeout error
            prop_assert!(
                matches!(result, Err(ShutdownError::Timeout(_, _))),
                "Should return timeout error for {} connections with {}ms timeout",
                num_connections,
                timeout_ms
            );

            // Should complete within T + 1 second (1000ms buffer)
            let max_allowed = std::time::Duration::from_millis(timeout_ms + 1000);
            prop_assert!(
                elapsed <= max_allowed,
                "Shutdown should complete within T + 1 second. Timeout: {}ms, Elapsed: {:?}, Max: {:?}",
                timeout_ms,
                elapsed,
                max_allowed
            );

            // Should complete at least after the timeout (with small tolerance for timing)
            let min_expected = std::time::Duration::from_millis(timeout_ms.saturating_sub(10));
            prop_assert!(
                elapsed >= min_expected,
                "Shutdown should wait at least until timeout. Timeout: {}ms, Elapsed: {:?}",
                timeout_ms,
                elapsed
            );

            Ok(())
        })?;
    }

    /// Feature: production-excellence, Property 4: Shutdown Timeout Enforcement
    ///
    /// **Validates: Requirements 3.2**
    ///
    /// For any mix of completing and non-completing connections, if some
    /// connections remain after timeout, shutdown SHALL return timeout error
    /// with the count of remaining connections.
    #[test]
    fn property_4_partial_completion_reports_remaining_connections(
        // Timeout in milliseconds (longer to ensure completing connections finish)
        timeout_ms in 200u64..400,
        // Number of connections that will complete before timeout
        completing_connections in 1usize..5,
        // Number of connections that won't complete
        non_completing_connections in 1usize..5
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = ShutdownConfig::with_timeout(std::time::Duration::from_millis(timeout_ms));
            let shutdown = Arc::new(GracefulShutdown::new(config));

            let total_connections = completing_connections + non_completing_connections;

            // Start all connections
            for _ in 0..total_connections {
                shutdown.connection_started();
            }

            // Complete some connections BEFORE initiating shutdown to avoid race condition
            // This ensures the completing connections are definitely finished
            for _ in 0..completing_connections {
                shutdown.connection_finished();
            }

            // Small delay to ensure state is consistent
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            // Initiate shutdown - should timeout due to non-completing connections
            let result = shutdown.shutdown().await;

            match result {
                Err(ShutdownError::Timeout(_, count)) => {
                    prop_assert_eq!(
                        count,
                        non_completing_connections,
                        "Should report {} remaining connections (started {}, completed {})",
                        non_completing_connections,
                        total_connections,
                        completing_connections
                    );
                }
                Ok(()) => {
                    prop_assert!(
                        false,
                        "Shutdown should have timed out with {} non-completing connections",
                        non_completing_connections
                    );
                }
                Err(e) => {
                    prop_assert!(false, "Expected Timeout error, got: {:?}", e);
                }
            }

            Ok(())
        })?;
    }
}

/// Feature: production-excellence, Property 4: Shutdown Timeout Enforcement
///
/// **Validates: Requirements 3.2**
///
/// Verify that the timeout error message contains useful information.
#[tokio::test]
async fn property_4_timeout_error_message_is_informative() {
    let timeout_ms = 50;
    let config = ShutdownConfig::with_timeout(std::time::Duration::from_millis(timeout_ms));
    let shutdown = Arc::new(GracefulShutdown::new(config));

    shutdown.connection_started();
    shutdown.connection_started();

    let result = shutdown.shutdown().await;

    match result {
        Err(e @ ShutdownError::Timeout(_, _)) => {
            let error_message = e.to_string();
            // Error message should contain timeout duration
            assert!(
                error_message.contains("50ms") || error_message.contains("50"),
                "Error message should contain timeout duration: {}",
                error_message
            );
            // Error message should contain connection count
            assert!(
                error_message.contains("2"),
                "Error message should contain connection count: {}",
                error_message
            );
        }
        _ => panic!("Expected Timeout error"),
    }
}

/// Feature: production-excellence, Property 4: Shutdown Timeout Enforcement
///
/// **Validates: Requirements 3.2**
///
/// Verify that shutdown is idempotent - calling shutdown twice returns
/// AlreadyShuttingDown error.
#[tokio::test]
async fn property_4_double_shutdown_returns_already_shutting_down() {
    let config = ShutdownConfig::with_timeout(std::time::Duration::from_millis(50));
    let shutdown = Arc::new(GracefulShutdown::new(config));

    // Start a connection that won't complete
    shutdown.connection_started();

    // First shutdown should timeout
    let result1 = shutdown.shutdown().await;
    assert!(
        matches!(result1, Err(ShutdownError::Timeout(_, _))),
        "First shutdown should timeout"
    );

    // Second shutdown should return AlreadyShuttingDown
    let result2 = shutdown.shutdown().await;
    assert!(
        matches!(result2, Err(ShutdownError::AlreadyShuttingDown)),
        "Second shutdown should return AlreadyShuttingDown"
    );
}

/// Feature: production-excellence, Property 4: Shutdown Timeout Enforcement
///
/// **Validates: Requirements 3.2**
///
/// Verify that very short timeouts still work correctly.
#[tokio::test]
async fn property_4_very_short_timeout_works() {
    // Use minimum practical timeout (10ms)
    let timeout_ms = 10;
    let config = ShutdownConfig::with_timeout(std::time::Duration::from_millis(timeout_ms));
    let shutdown = Arc::new(GracefulShutdown::new(config));

    shutdown.connection_started();

    let start = std::time::Instant::now();
    let result = shutdown.shutdown().await;
    let elapsed = start.elapsed();

    assert!(
        matches!(result, Err(ShutdownError::Timeout(_, 1))),
        "Should timeout with 1 connection"
    );

    // Should complete within reasonable time (timeout + 1 second buffer)
    assert!(
        elapsed <= std::time::Duration::from_millis(timeout_ms + 1000),
        "Should complete within T + 1 second even for very short timeout"
    );
}

// ============================================================================
// Property 6: Circuit Breaker State Transitions
// **Validates: Requirements 3.7, 3.8**
//
// *For any* circuit breaker with failure threshold T and reset timeout R:
// - After T consecutive failures, the circuit SHALL transition to Open state
// - After R seconds in Open state, the circuit SHALL transition to Half-Open state
// - A successful call in Half-Open state SHALL transition to Closed state
// - A failed call in Half-Open state SHALL transition back to Open state
// ============================================================================

use dx_www_server::ops::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitState,
};

/// Feature: production-excellence, Property 6: Circuit Breaker State Transitions
///
/// After T consecutive failures, the circuit SHALL transition to Open state.
/// **Validates: Requirements 3.7, 3.8**
#[tokio::test]
async fn property_6_circuit_opens_after_threshold_failures() {
    let config = CircuitBreakerConfig {
        failure_threshold: 5,
        reset_timeout: std::time::Duration::from_secs(30),
        half_open_max_calls: 3,
    };
    let breaker = CircuitBreaker::new(config);

    // Verify initial state is Closed
    assert_eq!(breaker.state(), CircuitState::Closed);

    // Cause exactly threshold failures
    for i in 0..5 {
        let result: Result<i32, CircuitBreakerError<&str>> =
            breaker.call(async { Err::<i32, _>("error") }).await;
        assert!(matches!(result, Err(CircuitBreakerError::Inner(_))));

        if i < 4 {
            // Before reaching threshold, circuit should remain closed
            assert_eq!(
                breaker.state(),
                CircuitState::Closed,
                "Circuit should remain closed after {} failures (threshold is 5)",
                i + 1
            );
        }
    }

    // After threshold failures, circuit should be Open
    assert_eq!(
        breaker.state(),
        CircuitState::Open,
        "Circuit should be Open after reaching failure threshold"
    );
}

proptest! {
    /// Feature: production-excellence, Property 6: Circuit Breaker State Transitions
    ///
    /// For any failure threshold T (1-20), after T consecutive failures,
    /// the circuit SHALL transition to Open state.
    /// **Validates: Requirements 3.7, 3.8**
    #[test]
    fn property_6_opens_after_any_threshold(threshold in 1u32..=20) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = CircuitBreakerConfig {
                failure_threshold: threshold,
                reset_timeout: std::time::Duration::from_secs(30),
                half_open_max_calls: 3,
            };
            let breaker = CircuitBreaker::new(config);

            // Verify initial state
            prop_assert_eq!(breaker.state(), CircuitState::Closed);

            // Cause threshold - 1 failures (should stay closed)
            for _ in 0..(threshold - 1) {
                let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
            }
            prop_assert_eq!(
                breaker.state(),
                CircuitState::Closed,
                "Circuit should remain Closed before reaching threshold"
            );

            // One more failure should open the circuit
            let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
            prop_assert_eq!(
                breaker.state(),
                CircuitState::Open,
                "Circuit should be Open after {} failures", threshold
            );

            Ok(())
        })?;
    }

    /// Feature: production-excellence, Property 6: Circuit Breaker State Transitions
    ///
    /// For any reset timeout R (10-500ms), after R milliseconds in Open state,
    /// the circuit SHALL transition to Half-Open state.
    /// **Validates: Requirements 3.7, 3.8**
    #[test]
    fn property_6_transitions_to_half_open_after_timeout(timeout_ms in 10u64..=100) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = CircuitBreakerConfig {
                failure_threshold: 1,
                reset_timeout: std::time::Duration::from_millis(timeout_ms),
                half_open_max_calls: 3,
            };
            let breaker = CircuitBreaker::new(config);

            // Trip the circuit
            let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
            prop_assert_eq!(breaker.state(), CircuitState::Open);

            // Wait for reset timeout plus a small buffer
            tokio::time::sleep(std::time::Duration::from_millis(timeout_ms + 20)).await;

            // Should transition to HalfOpen
            prop_assert_eq!(
                breaker.state(),
                CircuitState::HalfOpen,
                "Circuit should be HalfOpen after {}ms timeout", timeout_ms
            );

            Ok(())
        })?;
    }

    /// Feature: production-excellence, Property 6: Circuit Breaker State Transitions
    ///
    /// A successful call in Half-Open state SHALL transition to Closed state.
    /// **Validates: Requirements 3.7, 3.8**
    #[test]
    fn property_6_half_open_success_closes_circuit(threshold in 1u32..=10, timeout_ms in 10u64..=50) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = CircuitBreakerConfig {
                failure_threshold: threshold,
                reset_timeout: std::time::Duration::from_millis(timeout_ms),
                half_open_max_calls: 3,
            };
            let breaker = CircuitBreaker::new(config);

            // Trip the circuit with threshold failures
            for _ in 0..threshold {
                let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
            }
            prop_assert_eq!(breaker.state(), CircuitState::Open);

            // Wait for reset timeout
            tokio::time::sleep(std::time::Duration::from_millis(timeout_ms + 20)).await;
            prop_assert_eq!(breaker.state(), CircuitState::HalfOpen);

            // Successful call in HalfOpen should close the circuit
            let result: Result<i32, CircuitBreakerError<&str>> =
                breaker.call(async { Ok::<_, &str>(42) }).await;
            prop_assert!(result.is_ok());
            prop_assert_eq!(
                breaker.state(),
                CircuitState::Closed,
                "Circuit should be Closed after successful call in HalfOpen state"
            );

            Ok(())
        })?;
    }

    /// Feature: production-excellence, Property 6: Circuit Breaker State Transitions
    ///
    /// A failed call in Half-Open state SHALL transition back to Open state.
    /// **Validates: Requirements 3.7, 3.8**
    #[test]
    fn property_6_half_open_failure_reopens_circuit(threshold in 1u32..=10, timeout_ms in 10u64..=50) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = CircuitBreakerConfig {
                failure_threshold: threshold,
                reset_timeout: std::time::Duration::from_millis(timeout_ms),
                half_open_max_calls: 3,
            };
            let breaker = CircuitBreaker::new(config);

            // Trip the circuit with threshold failures
            for _ in 0..threshold {
                let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
            }
            prop_assert_eq!(breaker.state(), CircuitState::Open);

            // Wait for reset timeout
            tokio::time::sleep(std::time::Duration::from_millis(timeout_ms + 20)).await;
            prop_assert_eq!(breaker.state(), CircuitState::HalfOpen);

            // Failed call in HalfOpen should reopen the circuit
            let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
            prop_assert_eq!(
                breaker.state(),
                CircuitState::Open,
                "Circuit should be Open after failed call in HalfOpen state"
            );

            Ok(())
        })?;
    }

    /// Feature: production-excellence, Property 6: Circuit Breaker State Transitions
    ///
    /// Success in Closed state should reset failure count, preventing premature opening.
    /// **Validates: Requirements 3.7, 3.8**
    #[test]
    fn property_6_success_resets_failure_count(
        threshold in 2u32..=10,
        failures_before_success in 1u32..=9
    ) {
        prop_assume!(failures_before_success < threshold);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = CircuitBreakerConfig {
                failure_threshold: threshold,
                reset_timeout: std::time::Duration::from_secs(30),
                half_open_max_calls: 3,
            };
            let breaker = CircuitBreaker::new(config);

            // Cause some failures (but less than threshold)
            for _ in 0..failures_before_success {
                let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
            }
            prop_assert_eq!(breaker.state(), CircuitState::Closed);
            prop_assert_eq!(breaker.failure_count(), failures_before_success);

            // Successful call should reset failure count
            let _: Result<i32, _> = breaker.call(async { Ok::<i32, &str>(42) }).await;
            prop_assert_eq!(breaker.failure_count(), 0);

            // Now we need threshold failures again to open the circuit
            for _ in 0..(threshold - 1) {
                let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
            }
            prop_assert_eq!(
                breaker.state(),
                CircuitState::Closed,
                "Circuit should still be Closed after {} failures (threshold is {})",
                threshold - 1,
                threshold
            );

            Ok(())
        })?;
    }

    /// Feature: production-excellence, Property 6: Circuit Breaker State Transitions
    ///
    /// Open circuit should fail fast without executing the wrapped function.
    /// **Validates: Requirements 3.7, 3.8**
    #[test]
    fn property_6_open_circuit_fails_fast(threshold in 1u32..=10) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = CircuitBreakerConfig {
                failure_threshold: threshold,
                reset_timeout: std::time::Duration::from_secs(30), // Long timeout to stay open
                half_open_max_calls: 3,
            };
            let breaker = CircuitBreaker::new(config);

            // Trip the circuit
            for _ in 0..threshold {
                let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
            }
            prop_assert_eq!(breaker.state(), CircuitState::Open);

            // Subsequent calls should fail fast with CircuitBreakerError::Open
            for _ in 0..5 {
                let result: Result<i32, CircuitBreakerError<&str>> =
                    breaker.call(async { Ok::<_, &str>(42) }).await;
                prop_assert!(
                    matches!(result, Err(CircuitBreakerError::Open)),
                    "Open circuit should fail fast"
                );
            }

            Ok(())
        })?;
    }
}

/// Feature: production-excellence, Property 6: Circuit Breaker State Transitions
///
/// Complete state machine test: Closed -> Open -> HalfOpen -> Closed
/// **Validates: Requirements 3.7, 3.8**
#[tokio::test]
async fn property_6_complete_state_machine_cycle() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: std::time::Duration::from_millis(50),
        half_open_max_calls: 2,
    };
    let breaker = CircuitBreaker::new(config);

    // State 1: Closed (initial)
    assert_eq!(breaker.state(), CircuitState::Closed);

    // Transition: Closed -> Open (via failures)
    for _ in 0..3 {
        let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
    }
    assert_eq!(breaker.state(), CircuitState::Open);

    // Transition: Open -> HalfOpen (via timeout)
    tokio::time::sleep(std::time::Duration::from_millis(70)).await;
    assert_eq!(breaker.state(), CircuitState::HalfOpen);

    // Transition: HalfOpen -> Closed (via success)
    let result: Result<i32, CircuitBreakerError<&str>> =
        breaker.call(async { Ok::<_, &str>(42) }).await;
    assert!(result.is_ok());
    assert_eq!(breaker.state(), CircuitState::Closed);
}

/// Feature: production-excellence, Property 6: Circuit Breaker State Transitions
///
/// Complete state machine test: Closed -> Open -> HalfOpen -> Open (failure path)
/// **Validates: Requirements 3.7, 3.8**
#[tokio::test]
async fn property_6_state_machine_failure_path() {
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        reset_timeout: std::time::Duration::from_millis(30),
        half_open_max_calls: 2,
    };
    let breaker = CircuitBreaker::new(config);

    // State 1: Closed (initial)
    assert_eq!(breaker.state(), CircuitState::Closed);

    // Transition: Closed -> Open
    for _ in 0..2 {
        let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
    }
    assert_eq!(breaker.state(), CircuitState::Open);

    // Transition: Open -> HalfOpen
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(breaker.state(), CircuitState::HalfOpen);

    // Transition: HalfOpen -> Open (via failure)
    let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
    assert_eq!(breaker.state(), CircuitState::Open);
}

/// Feature: production-excellence, Property 6: Circuit Breaker State Transitions
///
/// Circuit should not transition to HalfOpen before reset timeout expires.
/// **Validates: Requirements 3.7, 3.8**
#[tokio::test]
async fn property_6_no_premature_half_open_transition() {
    let config = CircuitBreakerConfig {
        failure_threshold: 1,
        reset_timeout: std::time::Duration::from_millis(100),
        half_open_max_calls: 1,
    };
    let breaker = CircuitBreaker::new(config);

    // Trip the circuit
    let _: Result<i32, _> = breaker.call(async { Err::<i32, _>("error") }).await;
    assert_eq!(breaker.state(), CircuitState::Open);

    // Check state before timeout expires (at 50ms, timeout is 100ms)
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        breaker.state(),
        CircuitState::Open,
        "Circuit should remain Open before reset timeout"
    );

    // Wait for remaining timeout plus buffer
    tokio::time::sleep(std::time::Duration::from_millis(70)).await;
    assert_eq!(
        breaker.state(),
        CircuitState::HalfOpen,
        "Circuit should be HalfOpen after reset timeout"
    );
}
