//! Property tests for feature flag exclusion.
//!
//! **Feature: dx-js-compatibility, Property 20: Feature Flag Exclusion**
//! **Validates: Requirements 1.3, 1.4**
//!
//! *For any* disabled feature flag F, attempting to use APIs from that feature
//! SHALL result in a compile-time error (feature not enabled).
//!
//! Note: This property is primarily verified at compile-time through Rust's
//! feature flag system. These tests verify that enabled features work correctly
//! and that the feature detection functions report accurate information.

use dx_js_compatibility::{enabled_features, version};

/// Test that version is available regardless of features.
#[test]
fn test_version_always_available() {
    let v = version();
    assert!(!v.is_empty());
    assert!(v.contains('.'));
}

/// Test that enabled_features returns correct information.
#[test]
fn test_enabled_features_reports_correctly() {
    let features = enabled_features();

    // With default features, we should have node-core, web-core, bun-core
    #[cfg(feature = "node-core")]
    assert!(features.contains(&"node-core"));

    #[cfg(feature = "web-core")]
    assert!(features.contains(&"web-core"));

    #[cfg(feature = "bun-core")]
    assert!(features.contains(&"bun-core"));

    // Features not enabled should not be in the list
    #[cfg(not(feature = "bun-sqlite"))]
    assert!(!features.contains(&"bun-sqlite"));

    #[cfg(not(feature = "bun-s3"))]
    assert!(!features.contains(&"bun-s3"));

    #[cfg(not(feature = "bun-ffi"))]
    assert!(!features.contains(&"bun-ffi"));
}

/// Test that node module is available when node-core is enabled.
#[cfg(feature = "node-core")]
#[test]
fn test_node_core_available() {
    // This compiles only if node-core feature is enabled
    use dx_js_compatibility::node;

    // Verify we can access node modules
    let _ = node::path::SEP;
}

/// Test that web module is available when web-core is enabled.
#[cfg(feature = "web-core")]
#[test]
fn test_web_core_available() {
    // This compiles only if web-core feature is enabled
    use dx_js_compatibility::web;

    // Verify we can access web modules
    let _ = web::fetch::Headers::new();
}

/// Test that bun module is available when bun-core is enabled.
#[cfg(feature = "bun-core")]
#[test]
fn test_bun_core_available() {
    // This compiles only if bun-core feature is enabled
    use dx_js_compatibility::bun;

    // Verify we can access bun modules
    let _ = bun::hash::hash(b"test");
}

/// Test that sqlite module is available when bun-sqlite is enabled.
#[cfg(feature = "bun-sqlite")]
#[test]
fn test_sqlite_available() {
    use dx_js_compatibility::sqlite;

    // Verify we can access sqlite module
    let _ = sqlite::Database::memory();
}

/// Test that s3 module is available when bun-s3 is enabled.
#[cfg(feature = "bun-s3")]
#[test]
fn test_s3_available() {
    use dx_js_compatibility::s3;

    // Verify we can access s3 types
    let _ = s3::S3Config {
        access_key_id: String::new(),
        secret_access_key: String::new(),
        endpoint: None,
        region: None,
        bucket: String::new(),
    };
}

/// Test that ffi module is available when bun-ffi is enabled.
#[cfg(feature = "bun-ffi")]
#[test]
fn test_ffi_available() {
    use dx_js_compatibility::ffi;

    // Verify we can access ffi types
    let _ = ffi::FfiType::Void;
}

/// Test that shell module is available when bun-shell is enabled.
#[cfg(feature = "bun-shell")]
#[test]
fn test_shell_available() {
    use dx_js_compatibility::shell;

    // Verify we can access shell types
    let _ = shell::ShellCommand::new("echo");
}

/// Test that compile module is available when compile is enabled.
#[cfg(feature = "compile")]
#[test]
fn test_compile_available() {
    use dx_js_compatibility::compile;

    // Verify we can access compile types
    let _ = compile::Target::LinuxX64;
}

/// Test that hmr module is available when hmr is enabled.
#[cfg(feature = "hmr")]
#[test]
fn test_hmr_available() {
    use dx_js_compatibility::hmr;

    // Verify we can access hmr types
    let _ = hmr::UpdateType::Js;
}

/// Test that plugin module is available when plugins is enabled.
#[cfg(feature = "plugins")]
#[test]
fn test_plugins_available() {
    use dx_js_compatibility::plugin;

    // Verify we can access plugin types
    let _ = plugin::Loader::Js;
}

/// Test that macros module is available when macros is enabled.
#[cfg(feature = "macros")]
#[test]
fn test_macros_available() {
    use dx_js_compatibility::macros;

    // Verify we can access macro types
    let _ = macros::MacroContext::new();
}

/// Test that html module is available when html-rewriter is enabled.
#[cfg(feature = "html-rewriter")]
#[test]
fn test_html_rewriter_available() {
    use dx_js_compatibility::html;

    // Verify we can access html types
    let _ = html::HTMLRewriter::new();
}

// ============================================================================
// Compile-time verification tests
// ============================================================================
//
// The following tests verify that feature flags work correctly at compile time.
// If a feature is disabled, the corresponding module should not be accessible.
// This is enforced by Rust's type system and cannot be tested at runtime.
//
// To verify feature exclusion works:
// 1. Build with only specific features: cargo build --no-default-features --features "node-core"
// 2. Verify that attempting to use disabled features results in compile errors
//
// Example verification commands:
// - cargo build --no-default-features --features "node-core"
// - cargo build --no-default-features --features "web-core"
// - cargo build --no-default-features --features "bun-core"
// - cargo build --no-default-features --features "full"
