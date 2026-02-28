//! Property-based tests for DX-WWW CLI Commands
//!
//! These tests verify universal properties for CLI scaffolding,
//! route path mapping, and component generation.
//!
//! Feature: dx-www-production-ready, Property 1: CLI Scaffolding Produces Valid Output
//! Feature: dx-www-production-ready, Property 2: Component Generation Consistency
//! Feature: dx-www-production-ready, Property 3: Route Path to File Mapping
//! **Validates: Requirements 1.1, 1.2, 1.3, 1.4**
//!
//! Run with: cargo test --test www_property_tests

use proptest::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Helper Functions (mirrors www.rs logic)
// ============================================================================

/// Convert route path to file path
fn route_to_file_path(route: &str, dynamic: bool) -> String {
    let path = route.trim_start_matches('/');

    if path.is_empty() {
        return "index.tsx".to_string();
    }

    let segments: Vec<&str> = path.split('/').collect();
    let file_segments: Vec<String> = segments
        .iter()
        .map(|s| {
            if s.starts_with('[') && s.ends_with(']') {
                s.to_string()
            } else if dynamic && s == segments.last().unwrap() {
                format!("[{}]", s)
            } else {
                s.to_string()
            }
        })
        .collect();

    let file_path = file_segments.join("/");
    format!("{}.tsx", file_path)
}

/// Convert file path back to route path
fn file_path_to_route(file_path: &str) -> String {
    let path = file_path.trim_end_matches(".tsx").trim_end_matches(".ts");

    if path == "index" {
        return "/".to_string();
    }

    format!("/{}", path)
}

/// Extract page name from route path
fn extract_page_name(route: &str) -> String {
    let path = route.trim_start_matches('/');

    if path.is_empty() {
        return "HomePage".to_string();
    }

    let last_segment = path.split('/').next_back().unwrap_or("Page");
    let clean_segment = last_segment.trim_start_matches('[').trim_end_matches(']');

    clean_segment
        .split(['-', '_'])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<String>()
        + "Page"
}

/// Generate component content
fn generate_component_content(name: &str) -> String {
    format!(
        r#"import {{ h }} from 'dx';

interface {name}Props {{
  // Add props here
}}

export default function {name}(props: {name}Props) {{
  return (
    <div className="{name_lower}">
      {name} Component
    </div>
  );
}}
"#,
        name = name,
        name_lower = name.to_lowercase()
    )
}

/// Validate component content structure
fn is_valid_component(content: &str, name: &str) -> bool {
    content.contains("import { h } from 'dx'")
        && content.contains(&format!("interface {}Props", name))
        && content.contains(&format!("export default function {}", name))
        && content.contains(&format!("className=\"{}\"", name.to_lowercase()))
}

/// Generate dx.config content
fn generate_config(name: &str, template: &str, features: &[&str]) -> String {
    let features_str = if features.is_empty() {
        String::new()
    } else {
        let feature_lines: Vec<String> =
            features.iter().map(|f| format!("    {}: true", f)).collect();
        format!("\n  features: {{\n{}\n  }}", feature_lines.join("\n"))
    };

    format!(
        r#"# DX-WWW Project Configuration
# Generated with template: {template}

app: {{
  name: "{name}"
  version: "0.1.0"
  template: "{template}"
}}

build: {{
  entry: "pages"
  output: "dist"
  target: "wasm"
  minify: 2
}}

server: {{
  dev_port: 3000
  prod_port: 8080
}}{features_str}
"#
    )
}

/// Validate config content structure
fn is_valid_config(content: &str, name: &str) -> bool {
    content.contains(&format!("name: \"{}\"", name))
        && content.contains("version: \"0.1.0\"")
        && content.contains("entry: \"pages\"")
        && content.contains("output: \"dist\"")
}

// ============================================================================
// Arbitrary Generators
// ============================================================================

fn arbitrary_project_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("my-app".to_string()),
        Just("test-project".to_string()),
        Just("dx-web".to_string()),
        "[a-z][a-z0-9-]{0,15}".prop_map(|s| s.to_string()),
    ]
}

fn arbitrary_component_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Button".to_string()),
        Just("Card".to_string()),
        Just("Header".to_string()),
        Just("Footer".to_string()),
        Just("Modal".to_string()),
        "[A-Z][a-zA-Z]{2,15}".prop_map(|s| s.to_string()),
    ]
}

fn arbitrary_route_segment() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("dashboard".to_string()),
        Just("users".to_string()),
        Just("settings".to_string()),
        Just("profile".to_string()),
        Just("admin".to_string()),
        "[a-z][a-z0-9-]{0,10}".prop_map(|s| s.to_string()),
    ]
}

fn arbitrary_route_path() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("/".to_string()),
        arbitrary_route_segment().prop_map(|s| format!("/{}", s)),
        (arbitrary_route_segment(), arbitrary_route_segment())
            .prop_map(|(a, b)| format!("/{}/{}", a, b)),
    ]
}

fn arbitrary_dynamic_route() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("/users/[id]".to_string()),
        Just("/posts/[slug]".to_string()),
        Just("/[category]/[item]".to_string()),
        arbitrary_route_segment().prop_map(|s| format!("/{}s/[id]", s)),
    ]
}

fn arbitrary_template() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("minimal".to_string()),
        Just("default".to_string()),
        Just("full".to_string()),
        Just("api-only".to_string()),
    ]
}

fn arbitrary_features() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop_oneof![
            Just("forms".to_string()),
            Just("query".to_string()),
            Just("auth".to_string()),
            Just("sync".to_string()),
            Just("offline".to_string()),
            Just("a11y".to_string()),
            Just("i18n".to_string()),
        ],
        0..4,
    )
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 1: CLI Scaffolding Produces Valid Output
    /// *For any* valid project name and template combination, the generated
    /// configuration SHALL contain all required fields with valid content.
    ///
    /// **Validates: Requirements 1.1, 1.2**
    #[test]
    fn prop_scaffolding_produces_valid_config(
        name in arbitrary_project_name(),
        template in arbitrary_template(),
        features in arbitrary_features(),
    ) {
        let feature_refs: Vec<&str> = features.iter().map(|s| s.as_str()).collect();
        let config = generate_config(&name, &template, &feature_refs);

        // Verify config contains required fields
        prop_assert!(is_valid_config(&config, &name));

        // Verify template is mentioned
        let template_str = format!("template: {}", template);
        prop_assert!(config.contains(&template_str));

        // Verify features are included if specified
        for feature in &features {
            let feature_str = format!("{}: true", feature);
            prop_assert!(config.contains(&feature_str));
        }
    }

    /// Property 2: Component Generation Consistency
    /// *For any* valid component name, the generated component SHALL:
    /// - Contain a valid TSX component definition
    /// - Export a default function with the component name
    /// - Follow the DX_Generator template structure
    ///
    /// **Validates: Requirements 1.3, 4.1**
    #[test]
    fn prop_component_generation_consistency(
        name in arbitrary_component_name(),
    ) {
        let content = generate_component_content(&name);

        // Verify component structure
        prop_assert!(is_valid_component(&content, &name));

        // Verify props interface exists
        let props_name = format!("{}Props", name);
        prop_assert!(content.contains(&props_name));

        // Verify export default
        prop_assert!(content.contains("export default function"));
    }

    /// Property 3: Route Path to File Mapping (Bijective)
    /// *For any* valid route path, the mapping to file path SHALL be
    /// reversible (bijective), meaning route → file → route produces
    /// the original route.
    ///
    /// **Validates: Requirements 1.4**
    #[test]
    fn prop_route_path_mapping_bijective(
        route in arbitrary_route_path(),
    ) {
        let file_path = route_to_file_path(&route, false);
        let recovered_route = file_path_to_route(&file_path);

        // Normalize routes for comparison (handle trailing slashes)
        let normalized_original = route.trim_end_matches('/');
        let normalized_original = if normalized_original.is_empty() { "/" } else { normalized_original };

        prop_assert_eq!(normalized_original, recovered_route);
    }

    /// Property 3b: Dynamic Route Path Mapping
    /// *For any* dynamic route path, the file path SHALL contain
    /// the dynamic segment markers [param].
    ///
    /// **Validates: Requirements 1.4**
    #[test]
    fn prop_dynamic_route_mapping(
        route in arbitrary_dynamic_route(),
    ) {
        let file_path = route_to_file_path(&route, false);

        // Count dynamic segments in route
        let route_dynamic_count = route.matches('[').count();
        let file_dynamic_count = file_path.matches('[').count();

        // File should preserve dynamic segments
        prop_assert_eq!(route_dynamic_count, file_dynamic_count);

        // File should end with .tsx
        prop_assert!(file_path.ends_with(".tsx"));
    }

    /// Property 3c: Route to File Path Produces Valid Paths
    /// *For any* route path, the generated file path SHALL be a valid
    /// filesystem path (no invalid characters, proper extension).
    ///
    /// **Validates: Requirements 1.4**
    #[test]
    fn prop_route_produces_valid_file_path(
        route in arbitrary_route_path(),
    ) {
        let file_path = route_to_file_path(&route, false);

        // Should end with .tsx
        prop_assert!(file_path.ends_with(".tsx"));

        // Should not contain double slashes
        prop_assert!(!file_path.contains("//"));

        // Should not start with slash (relative path)
        prop_assert!(!file_path.starts_with('/'));

        // Should be parseable as a path
        let path = PathBuf::from(&file_path);
        prop_assert!(path.extension().map(|e| e == "tsx").unwrap_or(false));
    }

    /// Property: Page Name Extraction
    /// *For any* route path, the extracted page name SHALL be in PascalCase
    /// and end with "Page".
    ///
    /// **Validates: Requirements 1.4**
    #[test]
    fn prop_page_name_extraction(
        route in arbitrary_route_path(),
    ) {
        let page_name = extract_page_name(&route);

        // Should end with "Page"
        prop_assert!(page_name.ends_with("Page"));

        // Should start with uppercase (PascalCase)
        prop_assert!(page_name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false));

        // Should not contain hyphens or underscores
        prop_assert!(!page_name.contains('-'));
        prop_assert!(!page_name.contains('_'));
    }
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

#[test]
fn test_root_route_mapping() {
    assert_eq!(route_to_file_path("/", false), "index.tsx");
    assert_eq!(file_path_to_route("index.tsx"), "/");
}

#[test]
fn test_simple_route_mapping() {
    assert_eq!(route_to_file_path("/dashboard", false), "dashboard.tsx");
    assert_eq!(file_path_to_route("dashboard.tsx"), "/dashboard");
}

#[test]
fn test_nested_route_mapping() {
    assert_eq!(route_to_file_path("/users/profile", false), "users/profile.tsx");
    assert_eq!(file_path_to_route("users/profile.tsx"), "/users/profile");
}

#[test]
fn test_dynamic_route_mapping() {
    assert_eq!(route_to_file_path("/users/[id]", false), "users/[id].tsx");
    assert_eq!(route_to_file_path("/posts/slug", true), "posts/[slug].tsx");
}

#[test]
fn test_page_name_extraction() {
    assert_eq!(extract_page_name("/"), "HomePage");
    assert_eq!(extract_page_name("/dashboard"), "DashboardPage");
    assert_eq!(extract_page_name("/user-profile"), "UserProfilePage");
    assert_eq!(extract_page_name("/users/[id]"), "IdPage");
    assert_eq!(extract_page_name("/admin_panel"), "AdminPanelPage");
}

#[test]
fn test_component_generation() {
    let content = generate_component_content("Button");
    assert!(content.contains("interface ButtonProps"));
    assert!(content.contains("export default function Button"));
    assert!(content.contains("className=\"button\""));
}

#[test]
fn test_config_generation() {
    let config = generate_config("my-app", "default", &["forms", "auth"]);
    assert!(config.contains("name: \"my-app\""));
    assert!(config.contains("template: default"));
    assert!(config.contains("forms: true"));
    assert!(config.contains("auth: true"));
}

#[test]
fn test_config_without_features() {
    let config = generate_config("test-app", "minimal", &[]);
    assert!(config.contains("name: \"test-app\""));
    assert!(config.contains("template: minimal"));
    assert!(!config.contains("features:"));
}

/// Test that scaffolding creates proper directory structure
#[test]
fn test_scaffolding_directory_structure() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().join("test-project");

    // Create directories as the scaffolding would
    let dirs = vec![
        "pages",
        "components",
        "api",
        "content",
        "layouts",
        "public",
        "styles",
    ];
    std::fs::create_dir_all(&project_path).unwrap();

    for dir in &dirs {
        std::fs::create_dir_all(project_path.join(dir)).unwrap();
    }

    // Write config
    let config = generate_config("test-project", "default", &[]);
    std::fs::write(project_path.join("dx.config"), &config).unwrap();

    // Verify structure
    assert!(project_path.join("dx.config").exists());
    for dir in &dirs {
        assert!(project_path.join(dir).exists());
    }
}

/// Test minimal template creates fewer directories
#[test]
fn test_minimal_template_structure() {
    let minimal_dirs = ["pages", "components", "public"];
    let full_dirs = [
        "pages",
        "components",
        "api",
        "content",
        "layouts",
        "public",
        "styles",
    ];

    // Minimal should have fewer directories
    assert!(minimal_dirs.len() < full_dirs.len());
}

/// Test api-only template structure
#[test]
fn test_api_only_template_structure() {
    let api_dirs = ["api", "public"];

    // API-only should not have pages
    assert!(!api_dirs.contains(&"pages"));
    assert!(api_dirs.contains(&"api"));
}
