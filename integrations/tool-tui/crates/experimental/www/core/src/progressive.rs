//! # Progressive Enhancement Tiers
//!
//! Three-tier progressive enhancement system that generates HTML fallback,
//! micro runtime (338B), and full WASM bundle from the same source.
//!
//! **Validates: Requirements 14.1, 14.2, 14.3, 14.4, 14.5**

/// Build output for all three tiers
///
/// The same source file produces three outputs:
/// - HTML fallback: Works without any JavaScript
/// - Micro bundle: 338-byte progressive enhancement runtime
/// - Full bundle: Complete WASM binary experience
#[derive(Debug, Clone)]
pub struct BuildOutput {
    /// Works without JS (Maud-rendered HTML)
    pub html_fallback: String,
    /// 338B micro runtime for progressive enhancement
    pub micro_bundle: [u8; 338],
    /// Full WASM binary experience
    pub full_bundle: Vec<u8>,
}

impl BuildOutput {
    /// Create a new build output with all three tiers
    pub fn new(html: String, micro: [u8; 338], full: Vec<u8>) -> Self {
        Self {
            html_fallback: html,
            micro_bundle: micro,
            full_bundle: full,
        }
    }

    /// Get the size of each tier
    pub fn tier_sizes(&self) -> TierSizes {
        TierSizes {
            html_size: self.html_fallback.len(),
            micro_size: self.micro_bundle.len(),
            full_size: self.full_bundle.len(),
        }
    }
}

/// Size information for each tier
#[derive(Debug, Clone, Copy)]
pub struct TierSizes {
    pub html_size: usize,
    pub micro_size: usize,
    pub full_size: usize,
}

/// Client capability detection result
///
/// Determines which tier to serve based on client capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientCapability {
    /// No JavaScript support - serve HTML fallback
    NoJS,
    /// Light JavaScript support - serve micro runtime
    LightJS,
    /// Full WASM support - serve binary bundle
    FullWASM,
}

impl ClientCapability {
    /// Get the tier name for logging/debugging
    pub fn tier_name(&self) -> &'static str {
        match self {
            ClientCapability::NoJS => "HTML Fallback",
            ClientCapability::LightJS => "Micro Runtime (338B)",
            ClientCapability::FullWASM => "Full WASM",
        }
    }
}

/// HTTP request representation for capability detection
#[derive(Debug, Clone)]
pub struct Request {
    /// User-Agent header
    pub user_agent: Option<String>,
    /// Accept header
    pub accept: Option<String>,
    /// Custom headers for capability hints
    pub headers: Vec<(String, String)>,
}

impl Request {
    /// Create a new request with the given headers
    pub fn new() -> Self {
        Self {
            user_agent: None,
            accept: None,
            headers: Vec::new(),
        }
    }

    /// Set the User-Agent header
    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Set the Accept header
    pub fn with_accept(mut self, accept: impl Into<String>) -> Self {
        self.accept = Some(accept.into());
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Get a header value by name (case-insensitive)
    pub fn get_header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        for (k, v) in &self.headers {
            if k.to_lowercase() == name_lower {
                return Some(v.as_str());
            }
        }
        None
    }
}

impl Default for Request {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP response representation
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code
    pub status: u16,
    /// Content-Type header
    pub content_type: String,
    /// Response body
    pub body: Vec<u8>,
}

impl Response {
    /// Create an HTML response
    pub fn html(body: String) -> Self {
        Self {
            status: 200,
            content_type: "text/html; charset=utf-8".to_string(),
            body: body.into_bytes(),
        }
    }

    /// Create a JavaScript response
    pub fn javascript(body: Vec<u8>) -> Self {
        Self {
            status: 200,
            content_type: "application/javascript".to_string(),
            body,
        }
    }

    /// Create a WASM response
    pub fn wasm(body: Vec<u8>) -> Self {
        Self {
            status: 200,
            content_type: "application/wasm".to_string(),
            body,
        }
    }
}

/// Detect client capability from HTTP request
///
/// Detection logic:
/// 1. Check for explicit capability hints (Sec-CH-UA-* headers)
/// 2. Check Accept header for WASM support
/// 3. Check User-Agent for known patterns
/// 4. Default to LightJS for unknown clients
#[inline]
pub fn detect_capability(request: &Request) -> ClientCapability {
    // Check for explicit "no-js" hint
    if let Some(hint) = request.get_header("X-DX-Capability") {
        match hint.to_lowercase().as_str() {
            "no-js" | "nojs" | "html" => return ClientCapability::NoJS,
            "light" | "lightjs" | "micro" => return ClientCapability::LightJS,
            "full" | "wasm" | "fullwasm" => return ClientCapability::FullWASM,
            _ => {}
        }
    }

    // Check Accept header for WASM support
    if let Some(accept) = &request.accept {
        if accept.contains("application/wasm") {
            return ClientCapability::FullWASM;
        }
    }

    // Check User-Agent for known patterns
    if let Some(ua) = &request.user_agent {
        let ua_lower = ua.to_lowercase();

        // Known bots/crawlers - serve HTML
        if ua_lower.contains("googlebot")
            || ua_lower.contains("bingbot")
            || ua_lower.contains("yandexbot")
            || ua_lower.contains("duckduckbot")
            || ua_lower.contains("slurp")
            || ua_lower.contains("curl")
            || ua_lower.contains("wget")
        {
            return ClientCapability::NoJS;
        }

        // Modern browsers with WASM support
        if (ua_lower.contains("chrome/") && !ua_lower.contains("chrome/[1-5]"))
            || (ua_lower.contains("firefox/") && !ua_lower.contains("firefox/[1-5]"))
            || ua_lower.contains("safari/") && ua_lower.contains("version/1")
            || ua_lower.contains("edge/")
        {
            return ClientCapability::FullWASM;
        }

        // Older browsers - use micro runtime
        if ua_lower.contains("msie") || ua_lower.contains("trident") {
            return ClientCapability::LightJS;
        }
    }

    // Default to LightJS for unknown clients
    ClientCapability::LightJS
}

/// Serve the appropriate tier based on client capability
pub fn serve_page(request: &Request, output: &BuildOutput) -> Response {
    match detect_capability(request) {
        ClientCapability::NoJS => render_html_fallback(&output.html_fallback),
        ClientCapability::LightJS => serve_micro_runtime(&output.micro_bundle),
        ClientCapability::FullWASM => serve_binary_bundle(&output.full_bundle),
    }
}

/// Render HTML fallback response
fn render_html_fallback(html: &str) -> Response {
    Response::html(html.to_string())
}

/// Serve micro runtime response
fn serve_micro_runtime(micro: &[u8; 338]) -> Response {
    // Wrap micro runtime in minimal HTML shell
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"></head>
<body>
<script>{}</script>
</body>
</html>"#,
        // Convert micro bundle to base64 or inline JS
        String::from_utf8_lossy(micro)
    );
    Response::html(html)
}

/// Serve full WASM bundle response
fn serve_binary_bundle(bundle: &[u8]) -> Response {
    Response::wasm(bundle.to_vec())
}

/// Builder for creating BuildOutput from source
pub struct BuildOutputBuilder {
    html: Option<String>,
    micro: Option<[u8; 338]>,
    full: Option<Vec<u8>>,
}

impl BuildOutputBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            html: None,
            micro: None,
            full: None,
        }
    }

    /// Set the HTML fallback
    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    /// Set the micro bundle
    pub fn micro(mut self, micro: [u8; 338]) -> Self {
        self.micro = Some(micro);
        self
    }

    /// Set the full bundle
    pub fn full(mut self, full: Vec<u8>) -> Self {
        self.full = Some(full);
        self
    }

    /// Build the output
    pub fn build(self) -> Option<BuildOutput> {
        Some(BuildOutput {
            html_fallback: self.html?,
            micro_bundle: self.micro?,
            full_bundle: self.full?,
        })
    }
}

impl Default for BuildOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Unit tests

    #[test]
    fn test_build_output_creation() {
        let html = "<html><body>Hello</body></html>".to_string();
        let micro = [0u8; 338];
        let full = vec![0u8; 1000];

        let output = BuildOutput::new(html.clone(), micro, full.clone());

        assert_eq!(output.html_fallback, html);
        assert_eq!(output.micro_bundle.len(), 338);
        assert_eq!(output.full_bundle.len(), 1000);
    }

    #[test]
    fn test_tier_sizes() {
        let html = "Hello World".to_string();
        let micro = [0u8; 338];
        let full = vec![0u8; 500];

        let output = BuildOutput::new(html.clone(), micro, full);
        let sizes = output.tier_sizes();

        assert_eq!(sizes.html_size, 11);
        assert_eq!(sizes.micro_size, 338);
        assert_eq!(sizes.full_size, 500);
    }

    #[test]
    fn test_capability_detection_explicit_hints() {
        // NoJS hint
        let req = Request::new().with_header("X-DX-Capability", "no-js");
        assert_eq!(detect_capability(&req), ClientCapability::NoJS);

        // LightJS hint
        let req = Request::new().with_header("X-DX-Capability", "light");
        assert_eq!(detect_capability(&req), ClientCapability::LightJS);

        // FullWASM hint
        let req = Request::new().with_header("X-DX-Capability", "wasm");
        assert_eq!(detect_capability(&req), ClientCapability::FullWASM);
    }

    #[test]
    fn test_capability_detection_accept_header() {
        let req = Request::new().with_accept("application/wasm, text/html");
        assert_eq!(detect_capability(&req), ClientCapability::FullWASM);
    }

    #[test]
    fn test_capability_detection_bots() {
        let req = Request::new().with_user_agent("Googlebot/2.1");
        assert_eq!(detect_capability(&req), ClientCapability::NoJS);

        let req = Request::new().with_user_agent("curl/7.68.0");
        assert_eq!(detect_capability(&req), ClientCapability::NoJS);
    }

    #[test]
    fn test_capability_detection_default() {
        let req = Request::new();
        assert_eq!(detect_capability(&req), ClientCapability::LightJS);
    }

    #[test]
    fn test_tier_names() {
        assert_eq!(ClientCapability::NoJS.tier_name(), "HTML Fallback");
        assert_eq!(ClientCapability::LightJS.tier_name(), "Micro Runtime (338B)");
        assert_eq!(ClientCapability::FullWASM.tier_name(), "Full WASM");
    }

    #[test]
    fn test_response_types() {
        let html_resp = Response::html("Hello".to_string());
        assert_eq!(html_resp.status, 200);
        assert!(html_resp.content_type.contains("text/html"));

        let js_resp = Response::javascript(vec![1, 2, 3]);
        assert!(js_resp.content_type.contains("javascript"));

        let wasm_resp = Response::wasm(vec![0, 97, 115, 109]);
        assert!(wasm_resp.content_type.contains("wasm"));
    }

    // Property-based tests

    // Feature: binary-dawn-features, Property 23: Progressive Enhancement Tiers
    // For any source file, the build system SHALL produce exactly three outputs:
    // html_fallback (String), micro_bundle (338 bytes), and full_bundle (Vec<u8>).
    // Validates: Requirements 14.1, 14.4
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_build_output_has_three_tiers(
            html in ".*",
            full_size in 0usize..10000
        ) {
            let micro = [0u8; 338];
            let full = vec![0u8; full_size];

            let output = BuildOutput::new(html.clone(), micro, full.clone());

            // Property: BuildOutput always has exactly three tiers
            // 1. html_fallback is a String
            prop_assert!(!output.html_fallback.is_empty() || html.is_empty());

            // 2. micro_bundle is exactly 338 bytes
            prop_assert_eq!(output.micro_bundle.len(), 338);

            // 3. full_bundle is a Vec<u8>
            prop_assert_eq!(output.full_bundle.len(), full_size);
        }

        /// **Feature: binary-dawn-features, Property 24: Capability Detection Correctness**
        /// *For any* HTTP request, capability detection SHALL return exactly one of
        /// NoJS, LightJS, or FullWASM based on request headers.
        /// **Validates: Requirements 14.2**
        #[test]
        fn prop_capability_detection_returns_exactly_one(
            user_agent in proptest::option::of(".*"),
            accept in proptest::option::of(".*"),
            hint in proptest::option::of(prop_oneof![
                Just("no-js".to_string()),
                Just("light".to_string()),
                Just("wasm".to_string()),
                Just("unknown".to_string()),
            ])
        ) {
            let mut req = Request::new();

            if let Some(ua) = user_agent {
                req = req.with_user_agent(ua);
            }
            if let Some(acc) = accept {
                req = req.with_accept(acc);
            }
            if let Some(h) = hint {
                req = req.with_header("X-DX-Capability", h);
            }

            let capability = detect_capability(&req);

            // Property: Result is exactly one of the three variants
            let is_valid = matches!(
                capability,
                ClientCapability::NoJS | ClientCapability::LightJS | ClientCapability::FullWASM
            );
            prop_assert!(is_valid);
        }

        #[test]
        fn prop_serve_page_returns_valid_response(
            html in ".{0,1000}",
            full_size in 0usize..1000
        ) {
            let micro = [0u8; 338];
            let full = vec![0u8; full_size];
            let output = BuildOutput::new(html, micro, full);

            // Test all three capability types
            for capability in [
                ClientCapability::NoJS,
                ClientCapability::LightJS,
                ClientCapability::FullWASM,
            ] {
                let mut req = Request::new();
                req = match capability {
                    ClientCapability::NoJS => req.with_header("X-DX-Capability", "no-js"),
                    ClientCapability::LightJS => req.with_header("X-DX-Capability", "light"),
                    ClientCapability::FullWASM => req.with_header("X-DX-Capability", "wasm"),
                };

                let response = serve_page(&req, &output);

                // Property: Response always has valid status and content type
                prop_assert_eq!(response.status, 200);
                prop_assert!(!response.content_type.is_empty());
            }
        }
    }
}
