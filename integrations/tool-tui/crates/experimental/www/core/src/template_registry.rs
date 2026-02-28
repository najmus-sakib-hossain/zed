//! DX-WWW Template Registry
//!
//! Provides template loading and rendering for dx-www code generation.
//! Integrates with dx-generator for template management.
//!
//! ## Available Templates
//!
//! - `component` - React-style component
//! - `page` - Page route component
//! - `page-dynamic` - Dynamic page route with parameters
//! - `api-route` - API route handler
//! - `api-route-schema` - API route with validation schema
//! - `layout` - Layout wrapper component
//! - `middleware` - Request middleware
//! - `hook` - Custom React-style hook
//! - `content` - DXM content file

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// Template Types
// ============================================================================

/// Available www template types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WwwTemplateType {
    /// React-style component
    Component,
    /// Component test file
    ComponentTest,
    /// Page route component
    Page,
    /// Dynamic page route with parameters
    PageDynamic,
    /// API route handler
    ApiRoute,
    /// API route with validation schema
    ApiRouteSchema,
    /// Layout wrapper component
    Layout,
    /// Request middleware
    Middleware,
    /// Custom React-style hook
    Hook,
    /// DXM content file
    Content,
}

impl WwwTemplateType {
    /// Get the template file name
    pub fn file_name(&self) -> &'static str {
        match self {
            Self::Component => "component.dxt.hbs",
            Self::ComponentTest => "component-test.dxt.hbs",
            Self::Page => "page.dxt.hbs",
            Self::PageDynamic => "page-dynamic.dxt.hbs",
            Self::ApiRoute => "api-route.dxt.hbs",
            Self::ApiRouteSchema => "api-route-schema.dxt.hbs",
            Self::Layout => "layout.dxt.hbs",
            Self::Middleware => "middleware.dxt.hbs",
            Self::Hook => "hook.dxt.hbs",
            Self::Content => "content.dxt.hbs",
        }
    }

    /// Get the template display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Component => "DX-WWW Component",
            Self::ComponentTest => "DX-WWW Component Test",
            Self::Page => "DX-WWW Page",
            Self::PageDynamic => "DX-WWW Dynamic Page",
            Self::ApiRoute => "DX-WWW API Route",
            Self::ApiRouteSchema => "DX-WWW API Route with Schema",
            Self::Layout => "DX-WWW Layout",
            Self::Middleware => "DX-WWW Middleware",
            Self::Hook => "DX-WWW Custom Hook",
            Self::Content => "DX-WWW Content",
        }
    }

    /// Get the template description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Component => "Creates a DX-WWW functional component with TypeScript",
            Self::ComponentTest => "Creates a test file for a DX-WWW component",
            Self::Page => "Creates a DX-WWW page route component",
            Self::PageDynamic => "Creates a DX-WWW dynamic page route with parameter",
            Self::ApiRoute => "Creates a DX-WWW API route handler",
            Self::ApiRouteSchema => "Creates a DX-WWW API route handler with validation schema",
            Self::Layout => "Creates a DX-WWW layout wrapper component",
            Self::Middleware => "Creates a DX-WWW request middleware",
            Self::Hook => "Creates a DX-WWW custom React-style hook",
            Self::Content => "Creates a DXM content file with frontmatter",
        }
    }

    /// Get all template types
    pub fn all() -> &'static [WwwTemplateType] {
        &[
            Self::Component,
            Self::ComponentTest,
            Self::Page,
            Self::PageDynamic,
            Self::ApiRoute,
            Self::ApiRouteSchema,
            Self::Layout,
            Self::Middleware,
            Self::Hook,
            Self::Content,
        ]
    }
}

impl std::fmt::Display for WwwTemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Template Parameters
// ============================================================================

/// Parameters for template rendering
#[derive(Debug, Clone, Default)]
pub struct TemplateParams {
    params: HashMap<String, String>,
}

impl TemplateParams {
    /// Create new empty parameters
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a parameter value
    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Get a parameter value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(String::as_str)
    }

    /// Check if a parameter exists
    pub fn has(&self, key: &str) -> bool {
        self.params.contains_key(key)
    }

    /// Get all parameters
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.params.iter()
    }
}

// ============================================================================
// Template Registry
// ============================================================================

/// WWW Template Registry
///
/// Manages loading and rendering of dx-www templates.
#[derive(Debug)]
pub struct WwwTemplateRegistry {
    /// Base directory for templates
    base_dir: PathBuf,
    /// Cached template contents
    cache: HashMap<WwwTemplateType, String>,
}

impl WwwTemplateRegistry {
    /// Create a new registry with the given base directory
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            cache: HashMap::new(),
        }
    }

    /// Create a registry with the default template directory
    pub fn default_dir() -> Self {
        Self::new(".dx/templates/www")
    }

    /// Get the base directory
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Load a template from disk
    pub fn load(&mut self, template_type: WwwTemplateType) -> Result<&str> {
        if !self.cache.contains_key(&template_type) {
            let path = self.base_dir.join(template_type.file_name());
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to load template: {}", path.display()))?;
            self.cache.insert(template_type, content);
        }
        Ok(self.cache.get(&template_type).unwrap())
    }

    /// Check if a template exists
    pub fn exists(&self, template_type: WwwTemplateType) -> bool {
        self.base_dir.join(template_type.file_name()).exists()
    }

    /// List all available templates
    pub fn list_available(&self) -> Vec<WwwTemplateType> {
        WwwTemplateType::all().iter().filter(|t| self.exists(**t)).copied().collect()
    }

    /// Render a template with parameters
    pub fn render(
        &mut self,
        template_type: WwwTemplateType,
        params: &TemplateParams,
    ) -> Result<String> {
        let template = self.load(template_type)?;
        render_template(template, params)
    }

    /// Clear the template cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for WwwTemplateRegistry {
    fn default() -> Self {
        Self::default_dir()
    }
}

// ============================================================================
// Template Rendering
// ============================================================================

/// Render a template with parameters
///
/// Supports basic Handlebars-style placeholders:
/// - `{{ name }}` - Simple substitution
/// - `{{ name | kebab-case }}` - With transform
/// - `{{#if condition}}...{{/if}}` - Conditional blocks
/// - `{{#unless condition}}...{{/unless}}` - Negative conditional
pub fn render_template(template: &str, params: &TemplateParams) -> Result<String> {
    let mut output = template.to_string();

    // Skip frontmatter (everything between {{!-- and --}})
    if output.contains("{{!--") {
        if let Some(end) = output.find("--}}") {
            output = output[end + 4..].to_string();
        }
    }

    // Process conditional blocks first
    output = process_conditionals(&output, params)?;

    // Process simple substitutions
    output = process_substitutions(&output, params)?;

    Ok(output.trim().to_string())
}

/// Process conditional blocks ({{#if}}, {{#unless}}, {{else}})
fn process_conditionals(template: &str, params: &TemplateParams) -> Result<String> {
    let mut output = template.to_string();

    // Process {{#if condition}}...{{else}}...{{/if}}
    let if_regex = regex::Regex::new(r"\{\{#if\s+(\w+)\}\}([\s\S]*?)\{\{/if\}\}")?;

    loop {
        let captures = if_regex.captures(&output);
        if captures.is_none() {
            break;
        }
        let cap = captures.unwrap();
        let condition = cap.get(1).unwrap().as_str();
        let content = cap.get(2).unwrap().as_str();

        // Check for {{else}}
        let (if_content, else_content) = if let Some(else_pos) = content.find("{{else}}") {
            (&content[..else_pos], &content[else_pos + 8..])
        } else {
            (content, "")
        };

        let replacement = if params.has(condition) && params.get(condition) != Some("false") {
            if_content
        } else {
            else_content
        };

        output = output.replace(cap.get(0).unwrap().as_str(), replacement);
    }

    // Process {{#unless condition}}...{{/unless}}
    let unless_regex = regex::Regex::new(r"\{\{#unless\s+(\w+)\}\}([\s\S]*?)\{\{/unless\}\}")?;

    loop {
        let captures = unless_regex.captures(&output);
        if captures.is_none() {
            break;
        }
        let cap = captures.unwrap();
        let condition = cap.get(1).unwrap().as_str();
        let content = cap.get(2).unwrap().as_str();

        let replacement = if !params.has(condition) || params.get(condition) == Some("false") {
            content
        } else {
            ""
        };

        output = output.replace(cap.get(0).unwrap().as_str(), replacement);
    }

    Ok(output)
}

/// Process simple substitutions ({{ name }}, {{ name | transform }})
fn process_substitutions(template: &str, params: &TemplateParams) -> Result<String> {
    let mut output = template.to_string();

    // Match {{ name }} or {{ name | transform }}
    let placeholder_regex = regex::Regex::new(r"\{\{\s*(\w+)(?:\s*\|\s*(\S+))?\s*\}\}")?;

    for cap in placeholder_regex.captures_iter(template) {
        let full_match = cap.get(0).unwrap().as_str();
        let name = cap.get(1).unwrap().as_str();
        let transform = cap.get(2).map(|m| m.as_str());

        if let Some(value) = params.get(name) {
            let transformed = match transform {
                Some("kebab-case") => to_kebab_case(value),
                Some("snake_case") => to_snake_case(value),
                Some("camelCase") => to_camel_case(value),
                Some("PascalCase") => to_pascal_case(value),
                Some("UPPER_CASE") => value.to_uppercase(),
                Some("lower_case") => value.to_lowercase(),
                _ => value.to_string(),
            };
            output = output.replace(full_match, &transformed);
        }
    }

    // Handle default values: {{ name | default 'value' }}
    let default_regex =
        regex::Regex::new(r#"\{\{\s*(\w+)\s*\|\s*default\s+['"]?([^'"}\s]+)['"]?\s*\}\}"#)?;

    for cap in default_regex.captures_iter(template) {
        let full_match = cap.get(0).unwrap().as_str();
        let name = cap.get(1).unwrap().as_str();
        let default_value = cap.get(2).unwrap().as_str();

        let value = params.get(name).unwrap_or(default_value);
        output = output.replace(full_match, value);
    }

    Ok(output)
}

// ============================================================================
// String Transforms
// ============================================================================

/// Convert to kebab-case
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert to camelCase
fn to_camel_case(s: &str) -> String {
    let parts: Vec<&str> = s.split(['-', '_']).collect();
    let mut result = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            result.push_str(&part.to_lowercase());
        } else {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_uppercase().next().unwrap());
                result.extend(chars);
            }
        }
    }
    result
}

/// Convert to PascalCase
fn to_pascal_case(s: &str) -> String {
    let parts: Vec<&str> = s.split(['-', '_']).collect();
    let mut result = String::new();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_uppercase().next().unwrap());
            result.extend(chars);
        }
    }
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_type_file_names() {
        assert_eq!(WwwTemplateType::Component.file_name(), "component.dxt.hbs");
        assert_eq!(WwwTemplateType::Page.file_name(), "page.dxt.hbs");
        assert_eq!(WwwTemplateType::ApiRoute.file_name(), "api-route.dxt.hbs");
    }

    #[test]
    fn test_template_params() {
        let params = TemplateParams::new().set("name", "Button").set("layout", "default");

        assert_eq!(params.get("name"), Some("Button"));
        assert_eq!(params.get("layout"), Some("default"));
        assert!(params.has("name"));
        assert!(!params.has("missing"));
    }

    #[test]
    fn test_simple_substitution() {
        let template = "Hello {{ name }}!";
        let params = TemplateParams::new().set("name", "World");

        let result = render_template(template, &params).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_transform_kebab_case() {
        let template = "class=\"{{ name | kebab-case }}\"";
        let params = TemplateParams::new().set("name", "MyComponent");

        let result = render_template(template, &params).unwrap();
        assert_eq!(result, "class=\"my-component\"");
    }

    #[test]
    fn test_conditional_if() {
        let template = "{{#if layout}}with layout{{/if}}";

        let params_with = TemplateParams::new().set("layout", "default");
        let result = render_template(template, &params_with).unwrap();
        assert_eq!(result, "with layout");

        let params_without = TemplateParams::new();
        let result = render_template(template, &params_without).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_conditional_if_else() {
        let template = "{{#if layout}}has layout{{else}}no layout{{/if}}";

        let params_with = TemplateParams::new().set("layout", "default");
        let result = render_template(template, &params_with).unwrap();
        assert_eq!(result, "has layout");

        let params_without = TemplateParams::new();
        let result = render_template(template, &params_without).unwrap();
        assert_eq!(result, "no layout");
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("MyComponent"), "my-component");
        assert_eq!(to_kebab_case("Button"), "button");
        assert_eq!(to_kebab_case("UserProfileCard"), "user-profile-card");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("MyComponent"), "my_component");
        assert_eq!(to_snake_case("Button"), "button");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("my-component"), "myComponent");
        assert_eq!(to_camel_case("user_profile"), "userProfile");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my-component"), "MyComponent");
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
    }

    #[test]
    fn test_all_template_types() {
        let all = WwwTemplateType::all();
        assert!(all.len() >= 10);
        assert!(all.contains(&WwwTemplateType::Component));
        assert!(all.contains(&WwwTemplateType::Page));
        assert!(all.contains(&WwwTemplateType::ApiRoute));
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Feature: dx-www-production-ready
    // Property 8: Template Compilation Validity
    // Validates: Requirements 4.4
    // ========================================================================

    /// Strategy for generating valid component names (PascalCase)
    fn component_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Button".to_string()),
            Just("Card".to_string()),
            Just("UserProfile".to_string()),
            Just("DataTable".to_string()),
            "[A-Z][a-z]{2,10}".prop_map(|s| s.to_string()),
        ]
    }

    /// Strategy for generating valid layout names
    fn layout_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("default".to_string()),
            Just("docs".to_string()),
            Just("blog".to_string()),
            Just("dashboard".to_string()),
        ]
    }

    /// Strategy for generating HTTP methods
    fn http_method_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("GET".to_string()),
            Just("POST".to_string()),
            Just("PUT".to_string()),
            Just("DELETE".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 8.1: Simple substitution preserves parameter values
        /// *For any* valid parameter name and value, rendering a template
        /// with {{ name }} SHALL produce output containing the value.
        ///
        /// **Validates: Requirements 4.4**
        #[test]
        fn prop_simple_substitution_preserves_value(
            name in component_name_strategy(),
        ) {
            let template = "Component: {{ name }}";
            let params = TemplateParams::new().set("name", &name);

            let result = render_template(template, &params).unwrap();

            prop_assert!(result.contains(&name),
                "Output '{}' should contain name '{}'", result, name);
        }

        /// Property 8.2: Kebab-case transform is correct
        /// *For any* PascalCase name, the kebab-case transform SHALL
        /// produce a lowercase hyphenated string.
        ///
        /// **Validates: Requirements 4.4**
        #[test]
        fn prop_kebab_case_transform(
            name in component_name_strategy(),
        ) {
            let template = "class=\"{{ name | kebab-case }}\"";
            let params = TemplateParams::new().set("name", &name);

            let result = render_template(template, &params).unwrap();

            // Result should be lowercase
            let kebab = to_kebab_case(&name);
            prop_assert!(result.contains(&kebab),
                "Output '{}' should contain kebab-case '{}'", result, kebab);

            // Kebab case should be all lowercase
            prop_assert!(kebab.chars().all(|c| c.is_lowercase() || c == '-'),
                "Kebab case '{}' should be all lowercase with hyphens", kebab);
        }

        /// Property 8.3: Conditional blocks respect parameter presence
        /// *For any* parameter, {{#if param}}...{{/if}} SHALL include
        /// content only when the parameter is present.
        ///
        /// **Validates: Requirements 4.4**
        #[test]
        fn prop_conditional_respects_presence(
            layout in layout_name_strategy(),
            has_layout in any::<bool>(),
        ) {
            let template = "{{#if layout}}Layout: {{ layout }}{{/if}}";

            let params = if has_layout {
                TemplateParams::new().set("layout", &layout)
            } else {
                TemplateParams::new()
            };

            let result = render_template(template, &params).unwrap();

            if has_layout {
                prop_assert!(result.contains(&layout),
                    "With layout, output '{}' should contain '{}'", result, layout);
            } else {
                prop_assert!(result.is_empty() || !result.contains("Layout:"),
                    "Without layout, output '{}' should not contain 'Layout:'", result);
            }
        }

        /// Property 8.4: If-else blocks are mutually exclusive
        /// *For any* parameter, {{#if}}...{{else}}...{{/if}} SHALL
        /// include exactly one branch.
        ///
        /// **Validates: Requirements 4.4**
        #[test]
        fn prop_if_else_mutually_exclusive(
            has_param in any::<bool>(),
        ) {
            let template = "{{#if flag}}YES{{else}}NO{{/if}}";

            let params = if has_param {
                TemplateParams::new().set("flag", "true")
            } else {
                TemplateParams::new()
            };

            let result = render_template(template, &params).unwrap();

            let has_yes = result.contains("YES");
            let has_no = result.contains("NO");

            // Exactly one should be present
            prop_assert!(has_yes != has_no,
                "Result '{}' should have exactly one of YES or NO", result);

            if has_param {
                prop_assert!(has_yes, "With flag, should have YES");
            } else {
                prop_assert!(has_no, "Without flag, should have NO");
            }
        }

        /// Property 8.5: Multiple substitutions are independent
        /// *For any* set of parameters, each substitution SHALL be
        /// performed independently without affecting others.
        ///
        /// **Validates: Requirements 4.4**
        #[test]
        fn prop_multiple_substitutions_independent(
            name in component_name_strategy(),
            method in http_method_strategy(),
        ) {
            let template = "Name: {{ name }}, Method: {{ method }}";
            let params = TemplateParams::new()
                .set("name", &name)
                .set("method", &method);

            let result = render_template(template, &params).unwrap();

            prop_assert!(result.contains(&name),
                "Output should contain name '{}'", name);
            prop_assert!(result.contains(&method),
                "Output should contain method '{}'", method);
        }

        /// Property 8.6: Template type file names are unique
        /// *For all* template types, the file names SHALL be unique.
        ///
        /// **Validates: Requirements 4.2**
        #[test]
        fn prop_template_types_unique_file_names(
            idx1 in 0usize..10usize,
            idx2 in 0usize..10usize,
        ) {
            let all_types = WwwTemplateType::all();
            if idx1 < all_types.len() && idx2 < all_types.len() && idx1 != idx2 {
                let type1 = all_types[idx1];
                let type2 = all_types[idx2];

                prop_assert_ne!(type1.file_name(), type2.file_name(),
                    "Template types {:?} and {:?} should have different file names",
                    type1, type2);
            }
        }
    }
}
