//! Integration tests for dx-www framework
//!
//! These tests verify the full compilation pipeline from TSX source to HTIP binary.

use dx_compiler;
use dx_www_integration_tests::fixtures::*;
use dx_www_integration_tests::helpers::*;
use std::fs;
use tempfile::TempDir;

// Re-export for convenience
use dx_compiler as dx_www_compiler;

/// Test: Hello World - Minimal TSX compilation
///
/// Verifies that a minimal TSX component can be compiled through the full pipeline
/// and produces a valid HTIP stream.
///
/// Requirements: 12.1, 12.2
#[test]
fn test_hello_world_compilation() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let entry = temp.path().join("HelloWorld.tsx");
    let output = temp.path().join("dist");

    // Write the TSX file
    fs::write(&entry, HELLO_WORLD_TSX).expect("Failed to write TSX file");

    // Compile
    let result = dx_www_compiler::compile_tsx(&entry, &output, false);
    assert!(result.is_ok(), "Compilation should succeed: {:?}", result.err());

    let compile_result = result.unwrap();

    // Verify HTIP stream exists
    assert!(compile_result.htip_path.exists(), "HTIP stream should exist");

    // Read and validate HTIP stream
    let htip_data = fs::read(&compile_result.htip_path).expect("Failed to read HTIP stream");
    let validation = validate_htip_stream(&htip_data);
    assert!(validation.is_ok(), "HTIP stream should be valid: {:?}", validation.err());

    let validation = validation.unwrap();
    assert_eq!(validation.version, 2, "HTIP version should be 2");
    assert!(validation.total_size > 0, "HTIP should have content");
}

/// Test: Counter App - State management and event handling
///
/// Verifies that a component with state and event handlers compiles correctly.
///
/// Requirements: 12.1, 12.3
#[test]
fn test_counter_app_compilation() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let entry = temp.path().join("Counter.tsx");
    let output = temp.path().join("dist");

    // Write the TSX file
    fs::write(&entry, COUNTER_APP_TSX).expect("Failed to write TSX file");

    // Compile
    let result = dx_www_compiler::compile_tsx(&entry, &output, false);
    assert!(result.is_ok(), "Compilation should succeed: {:?}", result.err());

    let compile_result = result.unwrap();

    // Verify HTIP stream exists and is valid
    assert!(compile_result.htip_path.exists(), "HTIP stream should exist");

    let htip_data = fs::read(&compile_result.htip_path).expect("Failed to read HTIP stream");
    let validation = validate_htip_stream(&htip_data);
    assert!(validation.is_ok(), "HTIP stream should be valid: {:?}", validation.err());

    // Verify templates.json exists
    assert!(compile_result.templates_path.exists(), "Templates JSON should exist");

    // Verify templates contain expected structure
    let templates_json =
        fs::read_to_string(&compile_result.templates_path).expect("Failed to read templates JSON");
    assert!(
        templates_json.contains("counter") || templates_json.contains("Counter"),
        "Templates should contain counter component"
    );
}

/// Test: Form Validation - Form inputs with validation
///
/// Verifies that forms with validation logic compile correctly.
///
/// Requirements: 12.1, 12.3
#[test]
fn test_form_validation_compilation() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let entry = temp.path().join("LoginForm.tsx");
    let output = temp.path().join("dist");

    // Write the TSX file
    fs::write(&entry, FORM_VALIDATION_TSX).expect("Failed to write TSX file");

    // Compile
    let result = dx_www_compiler::compile_tsx(&entry, &output, false);
    assert!(result.is_ok(), "Compilation should succeed: {:?}", result.err());

    let compile_result = result.unwrap();

    // Verify HTIP stream exists and is valid
    assert!(compile_result.htip_path.exists(), "HTIP stream should exist");

    let htip_data = fs::read(&compile_result.htip_path).expect("Failed to read HTIP stream");
    let validation = validate_htip_stream(&htip_data);
    assert!(validation.is_ok(), "HTIP stream should be valid: {:?}", validation.err());

    // Verify templates contain form elements
    let templates_json =
        fs::read_to_string(&compile_result.templates_path).expect("Failed to read templates JSON");
    assert!(
        templates_json.contains("form") || templates_json.contains("Form"),
        "Templates should contain form component"
    );
}

/// Test: Routing - Multi-page navigation
///
/// Verifies that routing components compile correctly.
///
/// Requirements: 12.1, 12.3
#[test]
fn test_routing_compilation() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let entry = temp.path().join("App.tsx");
    let output = temp.path().join("dist");

    // Write the TSX file
    fs::write(&entry, ROUTING_TSX).expect("Failed to write TSX file");

    // Compile
    let result = dx_www_compiler::compile_tsx(&entry, &output, false);
    assert!(result.is_ok(), "Compilation should succeed: {:?}", result.err());

    let compile_result = result.unwrap();

    // Verify HTIP stream exists and is valid
    assert!(compile_result.htip_path.exists(), "HTIP stream should exist");

    let htip_data = fs::read(&compile_result.htip_path).expect("Failed to read HTIP stream");
    let validation = validate_htip_stream(&htip_data);
    assert!(validation.is_ok(), "HTIP stream should be valid: {:?}", validation.err());
}

/// Test: SSR Hydration - Server render to client hydration
///
/// Verifies that SSR-capable components compile correctly and the WASM client
/// can render the HTIP stream.
///
/// Requirements: 12.1, 12.4, 12.6
#[test]
fn test_ssr_hydration_compilation() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let entry = temp.path().join("HydratableCounter.tsx");
    let output = temp.path().join("dist");

    // Write the TSX file
    fs::write(&entry, SSR_HYDRATION_TSX).expect("Failed to write TSX file");

    // Compile
    let result = dx_www_compiler::compile_tsx(&entry, &output, false);
    assert!(result.is_ok(), "Compilation should succeed: {:?}", result.err());

    let compile_result = result.unwrap();

    // Verify HTIP stream exists and is valid
    assert!(compile_result.htip_path.exists(), "HTIP stream should exist");

    let htip_data = fs::read(&compile_result.htip_path).expect("Failed to read HTIP stream");
    let validation = validate_htip_stream(&htip_data);
    assert!(validation.is_ok(), "HTIP stream should be valid: {:?}", validation.err());

    // Verify the DXB package exists (for WASM client)
    let dxb_path = output.join("app.dxb");
    assert!(dxb_path.exists(), "DXB package should exist for WASM client");
}

/// Test: Analysis without compilation
///
/// Verifies that TSX files can be analyzed without full compilation.
#[test]
fn test_analyze_tsx() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let entry = temp.path().join("App.tsx");

    // Write a simple TSX file
    fs::write(&entry, HELLO_WORLD_TSX).expect("Failed to write TSX file");

    // Analyze
    let result = dx_www_compiler::analyze_tsx(&entry, false);
    assert!(result.is_ok(), "Analysis should succeed: {:?}", result.err());

    let (metrics, variant) = result.unwrap();
    assert!(metrics.total_jsx_nodes > 0, "Should have some nodes");

    // Hello World should use micro runtime (simple component)
    assert_eq!(
        variant,
        dx_www_compiler::analyzer::RuntimeVariant::Micro,
        "Simple component should use micro runtime"
    );
}

/// Test: can_compile check
///
/// Verifies that the quick compilation check works correctly.
#[test]
fn test_can_compile() {
    let temp = TempDir::new().expect("Failed to create temp dir");

    // Valid TSX
    let valid_entry = temp.path().join("Valid.tsx");
    fs::write(&valid_entry, HELLO_WORLD_TSX).expect("Failed to write TSX file");
    assert!(dx_www_compiler::can_compile(&valid_entry), "Valid TSX should be compilable");

    // Non-existent file should not be compilable
    let nonexistent_entry = temp.path().join("NonExistent.tsx");
    assert!(
        !dx_www_compiler::can_compile(&nonexistent_entry),
        "Non-existent file should not be compilable"
    );
}

/// Test: Multiple components in one file
#[test]
fn test_multiple_components() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let entry = temp.path().join("Components.tsx");
    let output = temp.path().join("dist");

    let tsx = r#"
function Header() {
    return <header><h1>My App</h1></header>;
}

function Footer() {
    return <footer><p>Copyright 2026</p></footer>;
}

export default function App() {
    return (
        <div>
            <Header />
            <main>Content</main>
            <Footer />
        </div>
    );
}
"#;

    fs::write(&entry, tsx).expect("Failed to write TSX file");

    let result = dx_www_compiler::compile_tsx(&entry, &output, false);
    assert!(result.is_ok(), "Compilation should succeed: {:?}", result.err());

    let compile_result = result.unwrap();
    assert!(compile_result.htip_path.exists(), "HTIP binary should exist");
}

/// Test: Component with props
#[test]
fn test_component_with_props() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let entry = temp.path().join("Greeting.tsx");
    let output = temp.path().join("dist");

    let tsx = r#"
interface GreetingProps {
    name: string;
    age?: number;
}

export default function Greeting({ name, age }: GreetingProps) {
    return (
        <div>
            <h1>Hello, {name}!</h1>
            {age && <p>You are {age} years old</p>}
        </div>
    );
}
"#;

    fs::write(&entry, tsx).expect("Failed to write TSX file");

    let result = dx_www_compiler::compile_tsx(&entry, &output, false);
    assert!(result.is_ok(), "Compilation should succeed: {:?}", result.err());
}

/// Test: Component with conditional rendering
#[test]
fn test_conditional_rendering() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let entry = temp.path().join("Conditional.tsx");
    let output = temp.path().join("dist");

    let tsx = r#"
import { useState } from 'dx';

export default function Toggle() {
    const [isVisible, setIsVisible] = useState(false);
    
    return (
        <div>
            <button onClick={() => setIsVisible(!isVisible)}>
                Toggle
            </button>
            {isVisible && <p>Now you see me!</p>}
            {!isVisible && <p>Now you don't!</p>}
        </div>
    );
}
"#;

    fs::write(&entry, tsx).expect("Failed to write TSX file");

    let result = dx_www_compiler::compile_tsx(&entry, &output, false);
    assert!(result.is_ok(), "Compilation should succeed: {:?}", result.err());
}

/// Test: Component with list rendering
#[test]
fn test_list_rendering() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let entry = temp.path().join("List.tsx");
    let output = temp.path().join("dist");

    let tsx = r#"
interface Item {
    id: number;
    text: string;
}

export default function TodoList() {
    const items: Item[] = [
        { id: 1, text: 'Learn dx-www' },
        { id: 2, text: 'Build an app' },
        { id: 3, text: 'Deploy to production' },
    ];
    
    return (
        <ul>
            {items.map(item => (
                <li key={item.id}>{item.text}</li>
            ))}
        </ul>
    );
}
"#;

    fs::write(&entry, tsx).expect("Failed to write TSX file");

    let result = dx_www_compiler::compile_tsx(&entry, &output, false);
    assert!(result.is_ok(), "Compilation should succeed: {:?}", result.err());
}
