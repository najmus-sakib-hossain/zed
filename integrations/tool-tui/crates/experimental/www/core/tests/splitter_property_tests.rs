//! Property-based tests for the splitter module
//!
//! Feature: production-readiness, Property 12: Splitter No Dummy Templates
//! Feature: production-readiness, Property 13: Splitter Conditional Binding Generation
//! Feature: production-readiness, Property 14: Splitter Iteration Binding Generation

use proptest::prelude::*;

use dx_compiler::parser::{Component, StateDef};
use dx_compiler::splitter::{BindingFlag, Template, split_components};

/// Generate a valid state field
fn state_field(index: usize) -> StateDef {
    StateDef {
        name: format!("state{}", index),
        setter_name: format!("setState{}", index),
        initial_value: "0".to_string(),
        type_annotation: "number".to_string(),
    }
}

/// Generate a simple JSX body with dynamic expressions
fn simple_jsx_body() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "<div>Hello World</div>".to_string(),
        "<div>{state.state0}</div>".to_string(),
        "<div class=\"container\"><span>{state.state0}</span></div>".to_string(),
        "<div><h1>Title</h1><p>{state.state0}</p></div>".to_string(),
        "<button onClick={handleClick}>{state.state0}</button>".to_string(),
    ])
}

/// Generate JSX with conditional rendering (&&)
fn conditional_and_jsx() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "<div>{state.state0 && <span>Visible</span>}</div>".to_string(),
        "<div>{state.state1 && <p>Content</p>}</div>".to_string(),
        "<div>{isVisible && <div>Show me</div>}</div>".to_string(),
        "<ul>{hasItems && <li>Item</li>}</ul>".to_string(),
    ])
}

/// Generate JSX with conditional rendering (ternary)
fn conditional_ternary_jsx() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "<div>{state.state0 ? \"Yes\" : \"No\"}</div>".to_string(),
        "<div>{isActive ? <span>Active</span> : <span>Inactive</span>}</div>".to_string(),
        "<button>{loading ? \"Loading...\" : \"Submit\"}</button>".to_string(),
    ])
}

/// Generate JSX with iteration (.map())
fn iteration_jsx() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "<ul>{state.items.map((item) => <li key={item.id}>{item.name}</li>)}</ul>".to_string(),
        "<div>{state.list.map((x) => <span>{x}</span>)}</div>".to_string(),
        "<ol>{data.map((d) => <li key={d.key}>{d.value}</li>)}</ol>".to_string(),
    ])
}

/// Create a test component with given JSX body
fn create_test_component(name: &str, jsx_body: &str, state_count: usize) -> Component {
    let state: Vec<StateDef> = (0..state_count).map(state_field).collect();

    Component {
        name: name.to_string(),
        props: vec![],
        state,
        jsx_body: jsx_body.to_string(),
        hooks: vec![],
        is_async: false,
        has_children: false,
    }
}

/// Check if a template contains dummy/placeholder content
fn has_dummy_content(template: &Template) -> bool {
    let html_lower = template.html.to_lowercase();
    html_lower.contains("dummy")
        || html_lower.contains("todo")
        || html_lower.contains("placeholder")
        || html_lower.contains("fixme")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 12: Splitter No Dummy Templates
    /// For any parsed component, the splitter SHALL NOT produce templates
    /// containing placeholder strings like "dummy", "TODO", or "placeholder".
    /// **Validates: Requirements 2.4**
    #[test]
    fn splitter_no_dummy_templates(jsx_body in simple_jsx_body()) {
        let component = create_test_component("TestComponent", &jsx_body, 2);

        let modules = vec![dx_compiler::parser::ParsedModule {
            path: std::path::PathBuf::from("test.tsx"),
            imports: vec![],
            exports: vec![],
            components: vec![component],
            hash: "test_hash".to_string(),
        }];

        let result = split_components(modules, false);
        prop_assert!(result.is_ok(), "Splitting should succeed");

        let (templates, _bindings, _schemas) = result.unwrap();

        for template in &templates {
            prop_assert!(
                !has_dummy_content(template),
                "Template should not contain dummy content: {}",
                template.html
            );
        }
    }

    /// Property 13: Splitter Conditional Binding Generation
    /// For any component containing conditional rendering (&& or ternary),
    /// the splitter SHALL generate at least one binding entry with a conditional flag.
    /// **Validates: Requirements 2.2**
    #[test]
    fn splitter_conditional_and_binding(jsx_body in conditional_and_jsx()) {
        let component = create_test_component("TestComponent", &jsx_body, 3);

        let modules = vec![dx_compiler::parser::ParsedModule {
            path: std::path::PathBuf::from("test.tsx"),
            imports: vec![],
            exports: vec![],
            components: vec![component],
            hash: "test_hash".to_string(),
        }];

        let result = split_components(modules, false);
        prop_assert!(result.is_ok(), "Splitting should succeed");

        let (_templates, bindings, _schemas) = result.unwrap();

        // Should have at least one conditional binding
        let conditional_bindings: Vec<_> = bindings
            .iter()
            .filter(|b| b.flag == BindingFlag::Conditional)
            .collect();

        prop_assert!(
            !conditional_bindings.is_empty(),
            "Should generate at least one conditional binding for JSX with && expression. JSX: {}, Bindings: {:?}",
            jsx_body,
            bindings
        );
    }

    /// Property 13 (continued): Ternary conditional binding generation
    /// **Validates: Requirements 2.2**
    #[test]
    fn splitter_conditional_ternary_binding(jsx_body in conditional_ternary_jsx()) {
        let component = create_test_component("TestComponent", &jsx_body, 3);

        let modules = vec![dx_compiler::parser::ParsedModule {
            path: std::path::PathBuf::from("test.tsx"),
            imports: vec![],
            exports: vec![],
            components: vec![component],
            hash: "test_hash".to_string(),
        }];

        let result = split_components(modules, false);
        prop_assert!(result.is_ok(), "Splitting should succeed");

        let (_templates, bindings, _schemas) = result.unwrap();

        // Should have at least one conditional binding
        let conditional_bindings: Vec<_> = bindings
            .iter()
            .filter(|b| b.flag == BindingFlag::Conditional)
            .collect();

        prop_assert!(
            !conditional_bindings.is_empty(),
            "Should generate at least one conditional binding for JSX with ternary expression. JSX: {}, Bindings: {:?}",
            jsx_body,
            bindings
        );
    }

    /// Property 14: Splitter Iteration Binding Generation
    /// For any component containing list rendering (.map()),
    /// the splitter SHALL generate at least one binding entry with an iteration flag.
    /// **Validates: Requirements 2.3**
    #[test]
    fn splitter_iteration_binding(jsx_body in iteration_jsx()) {
        // Add items state field for iteration
        let mut component = create_test_component("TestComponent", &jsx_body, 2);
        component.state.push(StateDef {
            name: "items".to_string(),
            setter_name: "setItems".to_string(),
            initial_value: "[]".to_string(),
            type_annotation: "array".to_string(),
        });
        component.state.push(StateDef {
            name: "list".to_string(),
            setter_name: "setList".to_string(),
            initial_value: "[]".to_string(),
            type_annotation: "array".to_string(),
        });

        let modules = vec![dx_compiler::parser::ParsedModule {
            path: std::path::PathBuf::from("test.tsx"),
            imports: vec![],
            exports: vec![],
            components: vec![component],
            hash: "test_hash".to_string(),
        }];

        let result = split_components(modules, false);
        prop_assert!(result.is_ok(), "Splitting should succeed");

        let (_templates, bindings, _schemas) = result.unwrap();

        // Should have at least one iteration binding
        let iteration_bindings: Vec<_> = bindings
            .iter()
            .filter(|b| b.flag == BindingFlag::Iteration)
            .collect();

        prop_assert!(
            !iteration_bindings.is_empty(),
            "Should generate at least one iteration binding for JSX with .map() expression. JSX: {}, Bindings: {:?}",
            jsx_body,
            bindings
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_simple_jsx_splitting() {
        let component = create_test_component("Test", "<div>{state.state0}</div>", 1);

        let modules = vec![dx_compiler::parser::ParsedModule {
            path: std::path::PathBuf::from("test.tsx"),
            imports: vec![],
            exports: vec![],
            components: vec![component],
            hash: "test".to_string(),
        }];

        let result = split_components(modules, false);
        assert!(result.is_ok());

        let (templates, bindings, _) = result.unwrap();
        assert!(!templates.is_empty());
        assert!(!bindings.is_empty());

        // Verify no dummy content
        for template in &templates {
            assert!(!has_dummy_content(template));
        }
    }

    #[test]
    fn test_conditional_and_detection() {
        let component =
            create_test_component("Test", "<div>{state.state0 && <span>Show</span>}</div>", 1);

        let modules = vec![dx_compiler::parser::ParsedModule {
            path: std::path::PathBuf::from("test.tsx"),
            imports: vec![],
            exports: vec![],
            components: vec![component],
            hash: "test".to_string(),
        }];

        let result = split_components(modules, false);
        assert!(result.is_ok());

        let (_, bindings, _) = result.unwrap();
        let conditional = bindings.iter().any(|b| b.flag == BindingFlag::Conditional);
        assert!(conditional, "Should detect conditional binding");
    }

    #[test]
    fn test_iteration_detection() {
        let mut component = create_test_component(
            "Test",
            "<ul>{state.items.map((item) => <li key={item.id}>{item.name}</li>)}</ul>",
            0,
        );
        component.state.push(StateDef {
            name: "items".to_string(),
            setter_name: "setItems".to_string(),
            initial_value: "[]".to_string(),
            type_annotation: "array".to_string(),
        });

        let modules = vec![dx_compiler::parser::ParsedModule {
            path: std::path::PathBuf::from("test.tsx"),
            imports: vec![],
            exports: vec![],
            components: vec![component],
            hash: "test".to_string(),
        }];

        let result = split_components(modules, false);
        assert!(result.is_ok());

        let (_, bindings, _) = result.unwrap();
        let iteration = bindings.iter().any(|b| b.flag == BindingFlag::Iteration);
        assert!(iteration, "Should detect iteration binding");
    }
}
