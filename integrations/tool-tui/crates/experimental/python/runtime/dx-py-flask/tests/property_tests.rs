//! Property-based tests for Flask compatibility
//!
//! These tests validate the correctness properties defined in the design document.
//! **Property 6: Flask Routing Correctness**
//! **Validates: Requirements 4.1, 4.2, 4.3, 4.4**

use dx_py_flask::jinja::{JinjaContext, JinjaEngine, JinjaTemplate, JinjaValue};
use dx_py_flask::werkzeug::{HttpMethod, Request, Response, Route, UrlRouter};
use dx_py_flask::wsgi::{SimpleWsgiApp, WsgiApp, WsgiEnviron, WsgiResponse};
use proptest::prelude::*;
use std::collections::HashMap;

// ============================================================================
// Generators for property-based testing
// ============================================================================

/// Generate valid route segment (alphanumeric, no special chars)
fn route_segment_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}".prop_map(|s| s)
}

/// Generate valid parameter name
fn param_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,10}".prop_map(|s| s)
}

/// Generate parameter type
fn param_type_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("string"), Just("int"), Just("float"), Just("path"),]
}

/// Generate a valid route pattern
fn route_pattern_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            // Static segment
            route_segment_strategy().prop_map(|s| format!("/{}", s)),
            // Parameter segment
            (param_type_strategy(), param_name_strategy())
                .prop_map(|(t, n)| format!("/<{}:{}>", t, n)),
            // Simple parameter (no type)
            param_name_strategy().prop_map(|n| format!("/<{}>", n)),
        ],
        1..5,
    )
    .prop_map(|segments| segments.join(""))
}

/// Generate HTTP method
fn http_method_strategy() -> impl Strategy<Value = HttpMethod> {
    prop_oneof![
        Just(HttpMethod::Get),
        Just(HttpMethod::Post),
        Just(HttpMethod::Put),
        Just(HttpMethod::Delete),
        Just(HttpMethod::Patch),
    ]
}

/// Generate endpoint name
fn endpoint_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{2,20}".prop_map(|s| s)
}

/// Generate a valid Jinja template variable name
fn jinja_var_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,10}".prop_map(|s| s)
}

// ============================================================================
// Property 6: Flask Routing Correctness
// For any valid Flask route pattern and HTTP request matching that pattern,
// DX-Py should invoke the same view function and produce the same response.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 6.1: Route registration preserves endpoint mapping
    /// For any valid route pattern and endpoint, registering the route
    /// should allow matching requests to resolve to that endpoint.
    #[test]
    fn prop_route_registration_preserves_endpoint(
        pattern in route_pattern_strategy(),
        endpoint in endpoint_strategy(),
        method in http_method_strategy(),
    ) {
        let route = Route::new(&pattern, &endpoint, vec![method]);

        // Route creation should succeed for valid patterns
        prop_assert!(route.is_ok(), "Failed to create route for pattern: {}", pattern);

        let route = route.unwrap();
        prop_assert_eq!(&route.endpoint, &endpoint);
        prop_assert!(route.allows_method(method));
    }

    /// Property 6.2: Static route matching is deterministic
    /// For any static route (no parameters), matching should be deterministic
    /// and return the correct endpoint.
    #[test]
    fn prop_static_route_matching_deterministic(
        segments in prop::collection::vec(route_segment_strategy(), 1..4),
        endpoint in endpoint_strategy(),
        method in http_method_strategy(),
    ) {
        let pattern = format!("/{}", segments.join("/"));
        let path = pattern.clone();

        let mut router = UrlRouter::new();
        router.route(&pattern, &endpoint, vec![method]).unwrap();

        // Matching the exact path should return the endpoint
        let result = router.match_route(&path, method);
        prop_assert!(result.is_ok(), "Failed to match path: {}", path);

        let route_match = result.unwrap();
        prop_assert_eq!(route_match.endpoint, endpoint);
        prop_assert!(route_match.params.is_empty());
    }

    /// Property 6.3: Parameterized route extracts correct values
    /// For any route with parameters, matching should extract the correct
    /// parameter values from the URL.
    #[test]
    fn prop_parameterized_route_extracts_values(
        prefix in route_segment_strategy(),
        param_name in param_name_strategy(),
        param_value in "[a-z0-9]{1,20}",
        endpoint in endpoint_strategy(),
    ) {
        let pattern = format!("/{}/{}", prefix, format_args!("<{}>", param_name));
        let path = format!("/{}/{}", prefix, param_value);

        let mut router = UrlRouter::new();
        router.route(&pattern, &endpoint, vec![HttpMethod::Get]).unwrap();

        let result = router.match_route(&path, HttpMethod::Get);
        prop_assert!(result.is_ok(), "Failed to match path: {} against pattern: {}", path, pattern);

        let route_match = result.unwrap();
        prop_assert_eq!(route_match.endpoint, endpoint);
        prop_assert_eq!(
            route_match.params.get(&param_name),
            Some(&param_value.to_string()),
            "Parameter {} not extracted correctly", param_name
        );
    }

    /// Property 6.4: Typed int parameter only matches integers
    /// For any route with an int parameter, only numeric paths should match.
    #[test]
    fn prop_int_param_only_matches_integers(
        prefix in route_segment_strategy(),
        param_name in param_name_strategy(),
        int_value in 1u32..1000000,
        endpoint in endpoint_strategy(),
    ) {
        let pattern = format!("/{}/{}", prefix, format_args!("<int:{}>", param_name));
        let valid_path = format!("/{}/{}", prefix, int_value);
        let invalid_path = format!("/{}/abc", prefix);

        let mut router = UrlRouter::new();
        router.route(&pattern, &endpoint, vec![HttpMethod::Get]).unwrap();

        // Valid integer path should match
        let result = router.match_route(&valid_path, HttpMethod::Get);
        prop_assert!(result.is_ok(), "Failed to match valid int path: {}", valid_path);

        // Invalid non-integer path should not match
        let result = router.match_route(&invalid_path, HttpMethod::Get);
        prop_assert!(result.is_err(), "Should not match non-integer path: {}", invalid_path);
    }

    /// Property 6.5: Method filtering works correctly
    /// For any route with specific methods, only those methods should match.
    #[test]
    fn prop_method_filtering_correct(
        pattern in route_pattern_strategy(),
        endpoint in endpoint_strategy(),
        allowed_method in http_method_strategy(),
    ) {
        // Create route with only one allowed method
        let route = Route::new(&pattern, &endpoint, vec![allowed_method]).unwrap();

        // Allowed method should be allowed
        prop_assert!(route.allows_method(allowed_method));

        // Other methods should not be allowed (unless they happen to be the same)
        let all_methods = vec![
            HttpMethod::Get, HttpMethod::Post, HttpMethod::Put,
            HttpMethod::Delete, HttpMethod::Patch
        ];
        for method in all_methods {
            if method != allowed_method {
                prop_assert!(!route.allows_method(method));
            }
        }
    }

    /// Property 6.6: Router returns MethodNotAllowed for wrong method
    /// For any route, requesting with a disallowed method should return MethodNotAllowed.
    #[test]
    fn prop_router_method_not_allowed(
        segments in prop::collection::vec(route_segment_strategy(), 1..3),
        endpoint in endpoint_strategy(),
    ) {
        let pattern = format!("/{}", segments.join("/"));
        let path = pattern.clone();

        let mut router = UrlRouter::new();
        // Register only GET
        router.route(&pattern, &endpoint, vec![HttpMethod::Get]).unwrap();

        // POST should return MethodNotAllowed
        let result = router.match_route(&path, HttpMethod::Post);
        prop_assert!(
            matches!(result, Err(dx_py_flask::werkzeug::RoutingError::MethodNotAllowed(_))),
            "Expected MethodNotAllowed for POST on GET-only route"
        );
    }

    /// Property 6.7: Router returns NotFound for non-existent paths
    /// For any router, requesting a non-registered path should return NotFound.
    #[test]
    fn prop_router_not_found(
        registered_segment in route_segment_strategy(),
        unregistered_segment in route_segment_strategy(),
        endpoint in endpoint_strategy(),
    ) {
        // Ensure segments are different
        prop_assume!(registered_segment != unregistered_segment);

        let mut router = UrlRouter::new();
        router.route(format!("/{}", registered_segment), &endpoint, vec![HttpMethod::Get]).unwrap();

        let result = router.match_route(&format!("/{}", unregistered_segment), HttpMethod::Get);
        prop_assert!(
            matches!(result, Err(dx_py_flask::werkzeug::RoutingError::NotFound(_))),
            "Expected NotFound for unregistered path"
        );
    }
}

// ============================================================================
// Property 6 (continued): Jinja2 Template Correctness
// For any valid template and context, rendering should produce correct output.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 6.8: Variable substitution is correct
    /// For any variable name and value, template rendering should substitute correctly.
    #[test]
    fn prop_jinja_variable_substitution(
        var_name in jinja_var_name_strategy(),
        var_value in "[a-zA-Z0-9]{1,30}",
    ) {
        let template_src = format!("Hello, {{{{ {} }}}}!", var_name);
        let template = JinjaTemplate::parse("test", &template_src).unwrap();

        let mut ctx = JinjaContext::new();
        ctx.set(&var_name, var_value.clone());

        let result = template.render(&ctx).unwrap();
        prop_assert!(
            result.contains(&var_value),
            "Rendered output should contain variable value. Got: {}", result
        );
    }

    /// Property 6.9: If statement evaluates truthiness correctly
    /// For any boolean value, if statement should render correct branch.
    #[test]
    fn prop_jinja_if_truthiness(
        var_name in jinja_var_name_strategy(),
        condition in any::<bool>(),
    ) {
        let template_src = format!(
            "{{% if {} %}}TRUE{{% else %}}FALSE{{% endif %}}",
            var_name
        );
        let template = JinjaTemplate::parse("test", &template_src).unwrap();

        let mut ctx = JinjaContext::new();
        ctx.set(&var_name, condition);

        let result = template.render(&ctx).unwrap();
        if condition {
            prop_assert_eq!(result, "TRUE");
        } else {
            prop_assert_eq!(result, "FALSE");
        }
    }

    /// Property 6.10: For loop iterates over all items
    /// For any list of items, for loop should render each item.
    #[test]
    fn prop_jinja_for_loop_iteration(
        items in prop::collection::vec("[a-z]{1,5}", 1..5),
    ) {
        let template_src = "{% for item in items %}[{{ item }}]{% endfor %}";
        let template = JinjaTemplate::parse("test", template_src).unwrap();

        let jinja_items: Vec<JinjaValue> = items.iter()
            .map(|s| JinjaValue::String(s.clone()))
            .collect();

        let mut ctx = JinjaContext::new();
        ctx.set("items", JinjaValue::List(jinja_items));

        let result = template.render(&ctx).unwrap();

        // Each item should appear in the output
        for item in &items {
            prop_assert!(
                result.contains(&format!("[{}]", item)),
                "Output should contain [{}]. Got: {}", item, result
            );
        }
    }

    /// Property 6.11: HTML escaping prevents XSS
    /// For any string with HTML special characters, output should be escaped.
    #[test]
    fn prop_jinja_html_escaping(
        var_name in jinja_var_name_strategy(),
    ) {
        let dangerous_value = "<script>alert('xss')</script>";
        let template_src = format!("{{{{ {} }}}}", var_name);
        let template = JinjaTemplate::parse("test", &template_src).unwrap();

        let mut ctx = JinjaContext::new();
        ctx.set(&var_name, dangerous_value);

        let result = template.render(&ctx).unwrap();

        // Should not contain raw < or >
        prop_assert!(!result.contains('<'), "Output should escape <");
        prop_assert!(!result.contains('>'), "Output should escape >");
        // Should contain escaped versions
        prop_assert!(result.contains("&lt;"), "Output should contain &lt;");
        prop_assert!(result.contains("&gt;"), "Output should contain &gt;");
    }

    /// Property 6.12: Filter application is correct
    /// For any string, upper filter should uppercase it.
    #[test]
    fn prop_jinja_filter_upper(
        var_name in jinja_var_name_strategy(),
        value in "[a-z]{1,20}",
    ) {
        let template_src = format!("{{{{ {}|upper }}}}", var_name);
        let template = JinjaTemplate::parse("test", &template_src).unwrap();

        let mut ctx = JinjaContext::new();
        ctx.set(&var_name, value.clone());

        let result = template.render(&ctx).unwrap();
        prop_assert_eq!(result, value.to_uppercase());
    }
}

// ============================================================================
// Property 6 (continued): WSGI Protocol Correctness
// For any valid WSGI environ, the protocol should be handled correctly.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 6.13: WSGI environ from request preserves data
    /// For any request, converting to WSGI environ should preserve all data.
    #[test]
    fn prop_wsgi_environ_preserves_request(
        method in http_method_strategy(),
        path in "/[a-z]{1,10}(/[a-z]{1,10}){0,3}",
    ) {
        let request = Request::new(method, &path);
        let environ = WsgiEnviron::from_request(&request);

        prop_assert_eq!(environ.request_method, method.as_str());
        prop_assert_eq!(environ.path_info, path);
    }

    /// Property 6.14: WSGI response round-trip preserves data
    /// For any response, converting to WSGI and back should preserve data.
    #[test]
    fn prop_wsgi_response_roundtrip(
        status_code in prop::sample::select(vec![200u16, 201, 204, 400, 404, 500]),
        body in prop::collection::vec(any::<u8>(), 0..100),
    ) {
        let response = Response::new(status_code).with_body(body.clone());
        let wsgi_response = WsgiResponse::from_response(&response);
        let roundtrip = wsgi_response.to_response();

        prop_assert_eq!(roundtrip.status_code, status_code);
        prop_assert_eq!(roundtrip.body, body);
    }

    /// Property 6.15: WSGI app invocation returns valid response
    /// For any WSGI app and environ, invocation should return a valid response.
    #[test]
    fn prop_wsgi_app_returns_valid_response(
        path in "/[a-z]{1,10}",
        response_body in "[a-zA-Z0-9 ]{0,50}",
    ) {
        let expected_body = response_body.clone();
        let app = SimpleWsgiApp::new(move |_environ| {
            Ok(WsgiResponse::new("200 OK").with_body(expected_body.as_bytes().to_vec()))
        });

        let environ = WsgiEnviron::new().with_path(&path);
        let result = app.call(&environ);

        prop_assert!(result.is_ok());
        let response = result.unwrap();
        prop_assert_eq!(&response.status, "200 OK");
        prop_assert_eq!(response.get_body(), response_body.as_bytes());
    }
}

// ============================================================================
// Integration tests for Flask routing correctness
// ============================================================================

#[test]
fn test_flask_routing_integration() {
    // Create a router with multiple routes
    let mut router = UrlRouter::new();
    router.route("/", "index", vec![HttpMethod::Get]).unwrap();
    router
        .route("/users", "users_list", vec![HttpMethod::Get, HttpMethod::Post])
        .unwrap();
    router
        .route(
            "/users/<int:id>",
            "user_detail",
            vec![HttpMethod::Get, HttpMethod::Put, HttpMethod::Delete],
        )
        .unwrap();
    router
        .route("/users/<int:user_id>/posts/<int:post_id>", "user_post", vec![HttpMethod::Get])
        .unwrap();

    // Test index route
    let m = router.match_route("/", HttpMethod::Get).unwrap();
    assert_eq!(m.endpoint, "index");

    // Test users list
    let m = router.match_route("/users", HttpMethod::Get).unwrap();
    assert_eq!(m.endpoint, "users_list");

    // Test user detail with parameter
    let m = router.match_route("/users/42", HttpMethod::Get).unwrap();
    assert_eq!(m.endpoint, "user_detail");
    assert_eq!(m.params.get("id"), Some(&"42".to_string()));

    // Test nested parameters
    let m = router.match_route("/users/1/posts/99", HttpMethod::Get).unwrap();
    assert_eq!(m.endpoint, "user_post");
    assert_eq!(m.params.get("user_id"), Some(&"1".to_string()));
    assert_eq!(m.params.get("post_id"), Some(&"99".to_string()));
}

#[test]
fn test_jinja_template_integration() {
    let mut engine = JinjaEngine::new();

    // Add a template with various features
    engine
        .add_template(
            "page",
            r#"
<!DOCTYPE html>
<html>
<head><title>{{ title }}</title></head>
<body>
{% if user %}
<h1>Welcome, {{ user.name }}!</h1>
{% else %}
<h1>Welcome, Guest!</h1>
{% endif %}
<ul>
{% for item in items %}
<li>{{ item }}</li>
{% endfor %}
</ul>
</body>
</html>
"#,
        )
        .unwrap();

    // Create context
    let mut ctx = JinjaContext::new();
    ctx.set("title", "Test Page");

    let mut user = HashMap::new();
    user.insert("name".to_string(), JinjaValue::String("Alice".to_string()));
    ctx.set("user", JinjaValue::Dict(user));

    ctx.set(
        "items",
        JinjaValue::List(vec![
            JinjaValue::String("Item 1".to_string()),
            JinjaValue::String("Item 2".to_string()),
        ]),
    );

    let result = engine.render("page", &ctx).unwrap();

    assert!(result.contains("Test Page"));
    assert!(result.contains("Welcome, Alice!"));
    assert!(result.contains("Item 1"));
    assert!(result.contains("Item 2"));
}

#[test]
fn test_wsgi_integration() {
    // Create a simple WSGI app that echoes the path
    let app = SimpleWsgiApp::new(|environ| {
        let body = format!("Path: {}", environ.path_info);
        Ok(WsgiResponse::new("200 OK")
            .with_header("Content-Type", "text/plain")
            .with_body(body.into_bytes()))
    });

    let environ = WsgiEnviron::new().with_method("GET").with_path("/test/path");

    let response = app.call(&environ).unwrap();
    assert_eq!(response.status, "200 OK");
    assert_eq!(String::from_utf8(response.get_body()).unwrap(), "Path: /test/path");
}
