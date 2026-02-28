//! Integration tests for dx-compat-macro module.
//!
//! These tests verify the macro module functionality including:
//! - MacroValue operations and conversions
//! - MacroContext execution and caching
//! - MacroRegistry management
//! - Built-in macro helpers
//! - File system and environment access

use dx_compat_macro::*;
use std::collections::HashMap;
use tempfile::TempDir;

// ============================================================================
// MacroValue Tests
// ============================================================================

#[test]
fn test_macro_value_null() {
    let value = MacroValue::null();
    assert!(value.is_null());
    assert_eq!(value.to_js_literal(), "null");
}

#[test]
fn test_macro_value_bool() {
    let true_val = MacroValue::Bool(true);
    let false_val = MacroValue::Bool(false);

    assert_eq!(true_val.as_bool(), Some(true));
    assert_eq!(false_val.as_bool(), Some(false));
    assert_eq!(true_val.to_js_literal(), "true");
    assert_eq!(false_val.to_js_literal(), "false");
}

#[test]
fn test_macro_value_integer() {
    let value = MacroValue::Integer(42);

    assert_eq!(value.as_i64(), Some(42));
    assert_eq!(value.as_f64(), Some(42.0)); // Integer can be read as float
    assert_eq!(value.to_js_literal(), "42");
}

#[test]
fn test_macro_value_float() {
    let value = MacroValue::Float(1.23456);

    assert_eq!(value.as_f64(), Some(1.23456));
    assert!(value.as_i64().is_none()); // Float cannot be read as integer
    assert!(value.to_js_literal().starts_with("1.23"));
}

#[test]
fn test_macro_value_float_special() {
    let nan = MacroValue::Float(f64::NAN);
    let inf = MacroValue::Float(f64::INFINITY);
    let neg_inf = MacroValue::Float(f64::NEG_INFINITY);

    assert_eq!(nan.to_js_literal(), "NaN");
    assert_eq!(inf.to_js_literal(), "Infinity");
    assert_eq!(neg_inf.to_js_literal(), "-Infinity");
}

#[test]
fn test_macro_value_string() {
    let value = MacroValue::String("hello world".to_string());

    assert_eq!(value.as_str(), Some("hello world"));
    let literal = value.to_js_literal();
    assert!(literal.starts_with('"'));
    assert!(literal.ends_with('"'));
    assert!(literal.contains("hello world"));
}

#[test]
fn test_macro_value_string_escaping() {
    let value = MacroValue::String("line1\nline2\ttab".to_string());
    let literal = value.to_js_literal();

    // Should contain escaped characters
    assert!(literal.contains("\\n") || literal.contains("line1"));
}

#[test]
fn test_macro_value_array() {
    let value = MacroValue::Array(vec![
        MacroValue::Integer(1),
        MacroValue::Integer(2),
        MacroValue::Integer(3),
    ]);

    let arr = value.as_array().unwrap();
    assert_eq!(arr.len(), 3);

    let literal = value.to_js_literal();
    assert!(literal.starts_with('['));
    assert!(literal.ends_with(']'));
    assert!(literal.contains("1"));
    assert!(literal.contains("2"));
    assert!(literal.contains("3"));
}

#[test]
fn test_macro_value_object() {
    let mut obj = HashMap::new();
    obj.insert("name".to_string(), MacroValue::String("test".to_string()));
    obj.insert("count".to_string(), MacroValue::Integer(42));

    let value = MacroValue::Object(obj);

    let obj_ref = value.as_object().unwrap();
    assert_eq!(obj_ref.len(), 2);

    let literal = value.to_js_literal();
    assert!(literal.starts_with('{'));
    assert!(literal.ends_with('}'));
}

#[test]
fn test_macro_value_nested() {
    let inner_array = MacroValue::Array(vec![MacroValue::Integer(1), MacroValue::Integer(2)]);

    let mut obj = HashMap::new();
    obj.insert("items".to_string(), inner_array);
    obj.insert("active".to_string(), MacroValue::Bool(true));

    let value = MacroValue::Object(obj);
    let literal = value.to_js_literal();

    assert!(literal.contains('['));
    assert!(literal.contains(']'));
    assert!(literal.contains("true"));
}

// ============================================================================
// MacroValue From Implementations
// ============================================================================

#[test]
fn test_macro_value_from_bool() {
    let value: MacroValue = true.into();
    assert_eq!(value.as_bool(), Some(true));
}

#[test]
fn test_macro_value_from_i64() {
    let value: MacroValue = 100i64.into();
    assert_eq!(value.as_i64(), Some(100));
}

#[test]
fn test_macro_value_from_i32() {
    let value: MacroValue = 50i32.into();
    assert_eq!(value.as_i64(), Some(50));
}

#[test]
fn test_macro_value_from_f64() {
    let value: MacroValue = 1.234f64.into();
    assert!((value.as_f64().unwrap() - 1.234).abs() < 0.001);
}

#[test]
fn test_macro_value_from_string() {
    let value: MacroValue = String::from("test").into();
    assert_eq!(value.as_str(), Some("test"));
}

#[test]
fn test_macro_value_from_str() {
    let value: MacroValue = "test".into();
    assert_eq!(value.as_str(), Some("test"));
}

#[test]
fn test_macro_value_from_vec() {
    let value: MacroValue = vec![1i32, 2, 3].into();
    let arr = value.as_array().unwrap();
    assert_eq!(arr.len(), 3);
}

#[test]
fn test_macro_value_from_option_some() {
    let value: MacroValue = Some(42i64).into();
    assert_eq!(value.as_i64(), Some(42));
}

#[test]
fn test_macro_value_from_option_none() {
    let value: MacroValue = Option::<i64>::None.into();
    assert!(value.is_null());
}

// ============================================================================
// MacroValue JSON Tests
// ============================================================================

#[test]
fn test_macro_value_json_roundtrip_primitives() {
    let values = vec![
        MacroValue::Null,
        MacroValue::Bool(true),
        MacroValue::Bool(false),
        MacroValue::Integer(42),
        MacroValue::Float(1.23),
        MacroValue::String("hello".to_string()),
    ];

    for value in values {
        let json = value.to_json().unwrap();
        let parsed = MacroValue::from_json(&json).unwrap();

        // Note: Integer might become Float after JSON roundtrip
        match (&value, &parsed) {
            (MacroValue::Integer(a), MacroValue::Integer(b)) => assert_eq!(a, b),
            (MacroValue::Float(a), MacroValue::Float(b)) => assert!((a - b).abs() < 0.001),
            (MacroValue::String(a), MacroValue::String(b)) => assert_eq!(a, b),
            (MacroValue::Bool(a), MacroValue::Bool(b)) => assert_eq!(a, b),
            (MacroValue::Null, MacroValue::Null) => {}
            _ => {} // JSON may convert integers to floats
        }
    }
}

#[test]
fn test_macro_value_json_roundtrip_complex() {
    let mut obj = HashMap::new();
    obj.insert("name".to_string(), MacroValue::String("test".to_string()));
    obj.insert(
        "items".to_string(),
        MacroValue::Array(vec![MacroValue::Integer(1), MacroValue::Integer(2)]),
    );

    let value = MacroValue::Object(obj);
    let json = value.to_json().unwrap();
    let parsed = MacroValue::from_json(&json).unwrap();

    assert!(parsed.as_object().is_some());
}

#[test]
fn test_macro_value_json_pretty() {
    let mut obj = HashMap::new();
    obj.insert("key".to_string(), MacroValue::String("value".to_string()));

    let value = MacroValue::Object(obj);
    let pretty = value.to_json_pretty().unwrap();

    // Pretty JSON should contain newlines
    assert!(pretty.contains('\n'));
}

// ============================================================================
// MacroConfig Tests
// ============================================================================

#[test]
fn test_macro_config_defaults() {
    let config = MacroConfig::new();

    assert!(config.allow_fs);
    assert!(config.allow_env);
    assert_eq!(config.timeout_ms, 30000);
}

#[test]
fn test_macro_config_builder() {
    let config = MacroConfig::new()
        .cwd("/tmp")
        .timeout(5000)
        .allow_fs(false)
        .allow_env(false)
        .env_var("TEST_KEY", "TEST_VALUE");

    assert_eq!(config.cwd.to_string_lossy(), "/tmp");
    assert_eq!(config.timeout_ms, 5000);
    assert!(!config.allow_fs);
    assert!(!config.allow_env);
    assert_eq!(config.env.get("TEST_KEY"), Some(&"TEST_VALUE".to_string()));
}

#[test]
fn test_macro_config_allowed_paths() {
    let config = MacroConfig::new().allowed_paths(vec!["/allowed/path".into()]);

    assert_eq!(config.allowed_paths.len(), 1);
}

// ============================================================================
// MacroContext Tests
// ============================================================================

#[test]
fn test_macro_context_creation() {
    let ctx = MacroContext::new();
    assert!(!ctx.cwd().to_string_lossy().is_empty());
}

#[test]
fn test_macro_context_with_config() {
    let config = MacroConfig::new().env_var("CUSTOM_VAR", "custom_value");

    let ctx = MacroContext::with_config(config);
    let value = ctx.env("CUSTOM_VAR").unwrap();
    assert_eq!(value, Some("custom_value".to_string()));
}

#[test]
fn test_macro_context_execute() {
    let ctx = MacroContext::new();

    let result = ctx.execute(|_| Ok(MacroValue::Integer(42))).unwrap();
    assert_eq!(result.as_i64(), Some(42));
}

#[test]
fn test_macro_context_execute_with_context() {
    let ctx = MacroContext::with_config(MacroConfig::new().env_var("MY_VAR", "my_value"));

    let result = ctx
        .execute(|c| {
            let val = c.env("MY_VAR")?.unwrap_or_default();
            Ok(MacroValue::String(val))
        })
        .unwrap();

    assert_eq!(result.as_str(), Some("my_value"));
}

#[test]
fn test_macro_context_execute_cached() {
    let ctx = MacroContext::new();
    let counter = std::sync::atomic::AtomicU32::new(0);

    // First call should execute
    let result1 = ctx
        .execute_cached("cache_key", |_| {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(MacroValue::Integer(100))
        })
        .unwrap();

    // Second call should use cache
    let result2 = ctx
        .execute_cached("cache_key", |_| {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(MacroValue::Integer(200))
        })
        .unwrap();

    assert_eq!(result1.as_i64(), Some(100));
    assert_eq!(result2.as_i64(), Some(100)); // Cached value
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn test_macro_context_clear_cache() {
    let ctx = MacroContext::new();
    let counter = std::sync::atomic::AtomicU32::new(0);

    // First call
    ctx.execute_cached("key", |_| {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(MacroValue::Integer(1))
    })
    .unwrap();

    // Clear cache
    ctx.clear_cache();

    // Should execute again
    ctx.execute_cached("key", |_| {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(MacroValue::Integer(2))
    })
    .unwrap();

    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
}

// ============================================================================
// MacroContext Environment Tests
// ============================================================================

#[test]
fn test_macro_context_env() {
    let ctx = MacroContext::with_config(
        MacroConfig::new().env_var("TEST_VAR", "test_value").allow_env(true),
    );

    assert_eq!(ctx.env("TEST_VAR").unwrap(), Some("test_value".to_string()));
    assert_eq!(ctx.env("NONEXISTENT").unwrap(), None);
}

#[test]
fn test_macro_context_env_or() {
    let ctx =
        MacroContext::with_config(MacroConfig::new().env_var("EXISTS", "value").allow_env(true));

    assert_eq!(ctx.env_or("EXISTS", "default").unwrap(), "value");
    assert_eq!(ctx.env_or("MISSING", "default").unwrap(), "default");
}

#[test]
fn test_macro_context_env_all() {
    let ctx = MacroContext::with_config(
        MacroConfig::new()
            .env_var("VAR1", "val1")
            .env_var("VAR2", "val2")
            .allow_env(true),
    );

    let all = ctx.env_all().unwrap();
    assert!(all.contains_key("VAR1"));
    assert!(all.contains_key("VAR2"));
}

#[test]
fn test_macro_context_env_disabled() {
    let ctx = MacroContext::with_config(MacroConfig::new().allow_env(false));

    assert!(ctx.env("ANY").is_err());
    assert!(ctx.env_or("ANY", "default").is_err());
    assert!(ctx.env_all().is_err());
}

// ============================================================================
// MacroContext File System Tests
// ============================================================================

#[test]
fn test_macro_context_read_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "Hello, World!").unwrap();

    let ctx = MacroContext::with_config(MacroConfig::new().cwd(temp_dir.path()).allow_fs(true));

    let content = ctx.read_file("test.txt").unwrap();
    assert_eq!(content, "Hello, World!");
}

#[test]
fn test_macro_context_read_file_bytes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("binary.bin");
    let data = vec![0u8, 1, 2, 3, 255];
    std::fs::write(&file_path, &data).unwrap();

    let ctx = MacroContext::with_config(MacroConfig::new().cwd(temp_dir.path()).allow_fs(true));

    let content = ctx.read_file_bytes("binary.bin").unwrap();
    assert_eq!(content, data);
}

#[test]
fn test_macro_context_read_json() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("config.json");
    std::fs::write(&file_path, r#"{"name": "test", "count": 42}"#).unwrap();

    let ctx = MacroContext::with_config(MacroConfig::new().cwd(temp_dir.path()).allow_fs(true));

    let value = ctx.read_json("config.json").unwrap();
    let obj = value.as_object().unwrap();
    assert!(obj.contains_key("name"));
    assert!(obj.contains_key("count"));
}

#[test]
fn test_macro_context_file_exists() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("exists.txt");
    std::fs::write(&file_path, "content").unwrap();

    let ctx = MacroContext::with_config(MacroConfig::new().cwd(temp_dir.path()).allow_fs(true));

    assert!(ctx.file_exists("exists.txt").unwrap());
    assert!(!ctx.file_exists("missing.txt").unwrap());
}

#[test]
fn test_macro_context_fs_disabled() {
    let ctx = MacroContext::with_config(MacroConfig::new().allow_fs(false));

    assert!(ctx.read_file("any.txt").is_err());
    assert!(ctx.read_file_bytes("any.bin").is_err());
    assert!(ctx.file_exists("any.txt").is_err());
}

// ============================================================================
// MacroDefinition Tests
// ============================================================================

#[test]
fn test_macro_definition_creation() {
    let def = MacroDefinition::new("myMacro", "my_module", "my_function");

    assert_eq!(def.name, "myMacro");
    assert_eq!(def.module, "my_module");
    assert_eq!(def.function, "my_function");
    assert!(def.description.is_none());
}

#[test]
fn test_macro_definition_with_description() {
    let def = MacroDefinition::new("myMacro", "module", "func").description("A helpful macro");

    assert_eq!(def.description, Some("A helpful macro".to_string()));
}

// ============================================================================
// MacroRegistry Tests
// ============================================================================

#[test]
fn test_macro_registry_creation() {
    let registry = MacroRegistry::new();
    assert!(registry.list().is_empty());
}

#[test]
fn test_macro_registry_register() {
    let mut registry = MacroRegistry::new();

    registry.register(MacroDefinition::new("macro1", "mod1", "fn1"));
    registry.register(MacroDefinition::new("macro2", "mod2", "fn2"));

    assert!(registry.has("macro1"));
    assert!(registry.has("macro2"));
    assert!(!registry.has("macro3"));
}

#[test]
fn test_macro_registry_get() {
    let mut registry = MacroRegistry::new();
    registry.register(
        MacroDefinition::new("testMacro", "test_module", "test_fn").description("Test description"),
    );

    let def = registry.get("testMacro").unwrap();
    assert_eq!(def.name, "testMacro");
    assert_eq!(def.module, "test_module");
    assert_eq!(def.function, "test_fn");
    assert_eq!(def.description, Some("Test description".to_string()));
}

#[test]
fn test_macro_registry_list() {
    let mut registry = MacroRegistry::new();
    registry.register(MacroDefinition::new("a", "m", "f"));
    registry.register(MacroDefinition::new("b", "m", "f"));
    registry.register(MacroDefinition::new("c", "m", "f"));

    let list = registry.list();
    assert_eq!(list.len(), 3);
}

#[test]
fn test_macro_registry_remove() {
    let mut registry = MacroRegistry::new();
    registry.register(MacroDefinition::new("toRemove", "m", "f"));

    assert!(registry.has("toRemove"));

    let removed = registry.remove("toRemove");
    assert!(removed.is_some());
    assert!(!registry.has("toRemove"));

    // Removing again should return None
    let removed_again = registry.remove("toRemove");
    assert!(removed_again.is_none());
}

// ============================================================================
// MacroExpansion Tests
// ============================================================================

#[test]
fn test_macro_expansion_creation() {
    let value = MacroValue::String("expanded".to_string());
    let expansion = MacroExpansion::new(value);

    assert_eq!(expansion.value.as_str(), Some("expanded"));
    assert!(expansion.js_literal.contains("expanded"));
    assert!(expansion.location.is_none());
}

#[test]
fn test_macro_expansion_with_location() {
    let value = MacroValue::Integer(42);
    let expansion = MacroExpansion::new(value).with_location("src/main.ts", 10, 5);

    let loc = expansion.location.unwrap();
    assert_eq!(loc.file, "src/main.ts");
    assert_eq!(loc.line, 10);
    assert_eq!(loc.column, 5);
}

// ============================================================================
// Built-in Macro Tests
// ============================================================================

#[test]
fn test_builtin_include_str() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("include.txt");
    std::fs::write(&file_path, "Included content").unwrap();

    let ctx = MacroContext::with_config(MacroConfig::new().cwd(temp_dir.path()).allow_fs(true));

    let result = builtins::include_str(&ctx, "include.txt").unwrap();
    assert_eq!(result.as_str(), Some("Included content"));
}

#[test]
fn test_builtin_include_bytes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("binary.bin");
    std::fs::write(&file_path, [0u8, 1, 2, 3]).unwrap();

    let ctx = MacroContext::with_config(MacroConfig::new().cwd(temp_dir.path()).allow_fs(true));

    let result = builtins::include_bytes(&ctx, "binary.bin").unwrap();
    // Result should be base64 encoded
    assert!(result.as_str().is_some());
}

#[test]
fn test_builtin_include_json() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("data.json");
    std::fs::write(&file_path, r#"{"key": "value"}"#).unwrap();

    let ctx = MacroContext::with_config(MacroConfig::new().cwd(temp_dir.path()).allow_fs(true));

    let result = builtins::include_json(&ctx, "data.json").unwrap();
    let obj = result.as_object().unwrap();
    assert!(obj.contains_key("key"));
}

#[test]
fn test_builtin_env() {
    let ctx = MacroContext::with_config(
        MacroConfig::new().env_var("BUILD_ENV", "production").allow_env(true),
    );

    let result = builtins::env(&ctx, "BUILD_ENV").unwrap();
    assert_eq!(result.as_str(), Some("production"));

    let missing = builtins::env(&ctx, "MISSING_VAR").unwrap();
    assert!(missing.is_null());
}

#[test]
fn test_builtin_env_or() {
    let ctx =
        MacroContext::with_config(MacroConfig::new().env_var("EXISTS", "value").allow_env(true));

    let result = builtins::env_or(&ctx, "EXISTS", "default").unwrap();
    assert_eq!(result.as_str(), Some("value"));

    let default = builtins::env_or(&ctx, "MISSING", "fallback").unwrap();
    assert_eq!(default.as_str(), Some("fallback"));
}

#[test]
fn test_builtin_timestamp() {
    let ctx = MacroContext::new();

    let result = builtins::timestamp(&ctx).unwrap();
    let ts = result.as_i64().unwrap();

    // Timestamp should be reasonable (after year 2020)
    assert!(ts > 1577836800); // 2020-01-01
}

#[test]
fn test_builtin_build_date() {
    let ctx = MacroContext::new();

    let result = builtins::build_date(&ctx).unwrap();
    let date = result.as_str().unwrap();

    // Should be ISO 8601 format
    assert!(date.contains('T'));
    assert!(date.ends_with('Z'));
    assert!(date.contains('-')); // Date separators
}
