//! Security Headers Middleware for dx-server
//!
//! Provides automatic security headers on all responses to protect against
//! common web vulnerabilities including XSS, clickjacking, and MIME sniffing.
//!
//! ## Headers Added
//! - Content-Security-Policy (CSP)
//! - Strict-Transport-Security (HSTS)
//! - X-Frame-Options
//! - X-Content-Type-Options
//! - X-XSS-Protection
//! - Referrer-Policy

use axum::{
    http::{HeaderValue, Request, Response, header::HeaderName},
    middleware::Next,
};

/// Frame options for X-Frame-Options header
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOptions {
    /// Prevents any domain from framing the content
    Deny,
    /// Only allows the same origin to frame the content
    SameOrigin,
}

impl FrameOptions {
    /// Get the header value string
    pub fn as_str(&self) -> &'static str {
        match self {
            FrameOptions::Deny => "DENY",
            FrameOptions::SameOrigin => "SAMEORIGIN",
        }
    }
}

/// Referrer policy options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferrerPolicy {
    NoReferrer,
    NoReferrerWhenDowngrade,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

impl ReferrerPolicy {
    /// Get the header value string
    pub fn as_str(&self) -> &'static str {
        match self {
            ReferrerPolicy::NoReferrer => "no-referrer",
            ReferrerPolicy::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            ReferrerPolicy::Origin => "origin",
            ReferrerPolicy::OriginWhenCrossOrigin => "origin-when-cross-origin",
            ReferrerPolicy::SameOrigin => "same-origin",
            ReferrerPolicy::StrictOrigin => "strict-origin",
            ReferrerPolicy::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            ReferrerPolicy::UnsafeUrl => "unsafe-url",
        }
    }
}

/// Content Security Policy configuration
#[derive(Debug, Clone)]
pub struct ContentSecurityPolicy {
    pub default_src: Vec<String>,
    pub script_src: Vec<String>,
    pub style_src: Vec<String>,
    pub img_src: Vec<String>,
    pub connect_src: Vec<String>,
    pub frame_ancestors: Vec<String>,
    pub font_src: Vec<String>,
    pub object_src: Vec<String>,
    pub base_uri: Vec<String>,
    pub form_action: Vec<String>,
}

impl Default for ContentSecurityPolicy {
    fn default() -> Self {
        Self::production()
    }
}

impl ContentSecurityPolicy {
    /// Create a strict production CSP
    pub fn production() -> Self {
        Self {
            default_src: vec!["'self'".to_string()],
            script_src: vec!["'self'".to_string()],
            style_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string()],
            img_src: vec![
                "'self'".to_string(),
                "data:".to_string(),
                "https:".to_string(),
            ],
            connect_src: vec!["'self'".to_string()],
            frame_ancestors: vec!["'none'".to_string()],
            font_src: vec!["'self'".to_string()],
            object_src: vec!["'none'".to_string()],
            base_uri: vec!["'self'".to_string()],
            form_action: vec!["'self'".to_string()],
        }
    }

    /// Create a relaxed development CSP that allows hot reloading
    pub fn development() -> Self {
        Self {
            default_src: vec!["'self'".to_string()],
            script_src: vec![
                "'self'".to_string(),
                "'unsafe-inline'".to_string(),
                "'unsafe-eval'".to_string(),
            ],
            style_src: vec!["'self'".to_string(), "'unsafe-inline'".to_string()],
            img_src: vec![
                "'self'".to_string(),
                "data:".to_string(),
                "blob:".to_string(),
                "https:".to_string(),
            ],
            connect_src: vec![
                "'self'".to_string(),
                "ws:".to_string(),
                "wss:".to_string(),
                "http://localhost:*".to_string(),
            ],
            frame_ancestors: vec!["'self'".to_string()],
            font_src: vec!["'self'".to_string(), "data:".to_string()],
            object_src: vec!["'none'".to_string()],
            base_uri: vec!["'self'".to_string()],
            form_action: vec!["'self'".to_string()],
        }
    }

    /// Build the CSP header value
    pub fn build(&self) -> String {
        let mut directives = Vec::new();

        if !self.default_src.is_empty() {
            directives.push(format!("default-src {}", self.default_src.join(" ")));
        }
        if !self.script_src.is_empty() {
            directives.push(format!("script-src {}", self.script_src.join(" ")));
        }
        if !self.style_src.is_empty() {
            directives.push(format!("style-src {}", self.style_src.join(" ")));
        }
        if !self.img_src.is_empty() {
            directives.push(format!("img-src {}", self.img_src.join(" ")));
        }
        if !self.connect_src.is_empty() {
            directives.push(format!("connect-src {}", self.connect_src.join(" ")));
        }
        if !self.frame_ancestors.is_empty() {
            directives.push(format!("frame-ancestors {}", self.frame_ancestors.join(" ")));
        }
        if !self.font_src.is_empty() {
            directives.push(format!("font-src {}", self.font_src.join(" ")));
        }
        if !self.object_src.is_empty() {
            directives.push(format!("object-src {}", self.object_src.join(" ")));
        }
        if !self.base_uri.is_empty() {
            directives.push(format!("base-uri {}", self.base_uri.join(" ")));
        }
        if !self.form_action.is_empty() {
            directives.push(format!("form-action {}", self.form_action.join(" ")));
        }

        directives.join("; ")
    }
}

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Content Security Policy directives
    pub csp: ContentSecurityPolicy,
    /// HSTS max-age in seconds (default: 1 year)
    pub hsts_max_age: u64,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// HSTS preload flag
    pub hsts_preload: bool,
    /// X-Frame-Options value
    pub frame_options: FrameOptions,
    /// Referrer policy
    pub referrer_policy: ReferrerPolicy,
    /// Whether to use relaxed settings for development
    pub development_mode: bool,
    /// Whether to enable X-XSS-Protection (legacy)
    pub enable_xss_protection: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self::production()
    }
}

impl SecurityConfig {
    /// Create production-safe defaults
    pub fn production() -> Self {
        Self {
            csp: ContentSecurityPolicy::production(),
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            hsts_preload: false,
            frame_options: FrameOptions::Deny,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
            development_mode: false,
            enable_xss_protection: true,
        }
    }

    /// Create development-friendly defaults
    pub fn development() -> Self {
        Self {
            csp: ContentSecurityPolicy::development(),
            hsts_max_age: 0, // Disabled in development
            hsts_include_subdomains: false,
            hsts_preload: false,
            frame_options: FrameOptions::SameOrigin,
            referrer_policy: ReferrerPolicy::NoReferrerWhenDowngrade,
            development_mode: true,
            enable_xss_protection: true,
        }
    }

    /// Build the HSTS header value
    pub fn build_hsts_header(&self) -> String {
        if self.hsts_max_age == 0 {
            return String::new();
        }

        let mut value = format!("max-age={}", self.hsts_max_age);
        if self.hsts_include_subdomains {
            value.push_str("; includeSubDomains");
        }
        if self.hsts_preload {
            value.push_str("; preload");
        }
        value
    }

    /// Build the CSP header value
    pub fn build_csp_header(&self) -> String {
        self.csp.build()
    }
}

/// Security headers middleware layer
#[derive(Clone)]
pub struct SecurityHeadersLayer {
    config: SecurityConfig,
}

impl SecurityHeadersLayer {
    /// Create a new security headers layer with the given config
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }

    /// Create with production defaults
    pub fn production() -> Self {
        Self::new(SecurityConfig::production())
    }

    /// Create with development defaults
    pub fn development() -> Self {
        Self::new(SecurityConfig::development())
    }
}

impl<S> tower::Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Security headers service
#[derive(Clone)]
pub struct SecurityHeadersService<S> {
    inner: S,
    config: SecurityConfig,
}

impl<S, ReqBody, ResBody> tower::Service<Request<ReqBody>> for SecurityHeadersService<S>
where
    S: tower::Service<Request<ReqBody>, Response = Response<ResBody>>,
    S::Future: Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = SecurityHeadersFuture<S::Future>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        SecurityHeadersFuture {
            future: self.inner.call(req),
            config: self.config.clone(),
        }
    }
}

/// Future for security headers service
#[pin_project::pin_project]
pub struct SecurityHeadersFuture<F> {
    #[pin]
    future: F,
    config: SecurityConfig,
}

impl<F, ResBody, E> std::future::Future for SecurityHeadersFuture<F>
where
    F: std::future::Future<Output = Result<Response<ResBody>, E>>,
{
    type Output = Result<Response<ResBody>, E>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        match this.future.poll(cx) {
            std::task::Poll::Ready(Ok(mut response)) => {
                add_security_headers(response.headers_mut(), this.config);
                std::task::Poll::Ready(Ok(response))
            }
            other => other,
        }
    }
}

/// Add security headers to a response
fn add_security_headers(headers: &mut axum::http::HeaderMap, config: &SecurityConfig) {
    // Content-Security-Policy
    let csp = config.build_csp_header();
    if !csp.is_empty() {
        if let Ok(value) = HeaderValue::from_str(&csp) {
            headers.insert(HeaderName::from_static("content-security-policy"), value);
        }
    }

    // Strict-Transport-Security (only in production)
    if !config.development_mode {
        let hsts = config.build_hsts_header();
        if !hsts.is_empty() {
            if let Ok(value) = HeaderValue::from_str(&hsts) {
                headers.insert(HeaderName::from_static("strict-transport-security"), value);
            }
        }
    }

    // X-Frame-Options
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static(config.frame_options.as_str()),
    );

    // X-Content-Type-Options
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );

    // X-XSS-Protection (legacy, but still useful for older browsers)
    if config.enable_xss_protection {
        headers.insert(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        );
    }

    // Referrer-Policy
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static(config.referrer_policy.as_str()),
    );
}

/// Middleware function for adding security headers
pub async fn security_headers_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let config = SecurityConfig::production();
    let mut response = next.run(req).await;
    add_security_headers(response.headers_mut(), &config);
    response
}

/// Middleware function for adding security headers with custom config
pub fn security_headers_middleware_with_config(
    config: SecurityConfig,
) -> impl Fn(
    Request<axum::body::Body>,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response<axum::body::Body>> + Send>>
+ Clone {
    move |req, next| {
        let config = config.clone();
        Box::pin(async move {
            let mut response = next.run(req).await;
            add_security_headers(response.headers_mut(), &config);
            response
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_config() {
        let config = SecurityConfig::production();
        assert!(!config.development_mode);
        assert_eq!(config.hsts_max_age, 31536000);
        assert!(config.hsts_include_subdomains);
        assert_eq!(config.frame_options, FrameOptions::Deny);
    }

    #[test]
    fn test_development_config() {
        let config = SecurityConfig::development();
        assert!(config.development_mode);
        assert_eq!(config.hsts_max_age, 0);
        assert_eq!(config.frame_options, FrameOptions::SameOrigin);
    }

    #[test]
    fn test_csp_build() {
        let csp = ContentSecurityPolicy::production();
        let header = csp.build();
        assert!(header.contains("default-src 'self'"));
        assert!(header.contains("script-src 'self'"));
        assert!(header.contains("frame-ancestors 'none'"));
    }

    #[test]
    fn test_hsts_build() {
        let config = SecurityConfig::production();
        let hsts = config.build_hsts_header();
        assert!(hsts.contains("max-age=31536000"));
        assert!(hsts.contains("includeSubDomains"));
    }

    #[test]
    fn test_hsts_disabled_in_development() {
        let config = SecurityConfig::development();
        let hsts = config.build_hsts_header();
        assert!(hsts.is_empty());
    }

    #[test]
    fn test_frame_options() {
        assert_eq!(FrameOptions::Deny.as_str(), "DENY");
        assert_eq!(FrameOptions::SameOrigin.as_str(), "SAMEORIGIN");
    }

    #[test]
    fn test_referrer_policy() {
        assert_eq!(ReferrerPolicy::NoReferrer.as_str(), "no-referrer");
        assert_eq!(
            ReferrerPolicy::StrictOriginWhenCrossOrigin.as_str(),
            "strict-origin-when-cross-origin"
        );
    }
}
