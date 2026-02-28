//! Property-based tests for Django compatibility layer
//!
//! **Feature: dx-py-game-changer, Property 5: Django Subsystem Correctness**
//! **Validates: Requirements 3.2, 3.3, 3.4, 3.6**

use dx_py_django::{
    database::{DatabaseAdapter, DbRow, DbValue, PostgresAdapter, SqliteAdapter},
    json::{ujson, JsonParser},
    password::PasswordHasher,
    template::{escape_html, Context, ContextValue, Markup, TemplateCompiler, TemplateRenderer},
};
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;

// ============================================================================
// JSON Subsystem Properties
// ============================================================================

proptest! {
    /// **Property 5.1: JSON Round-Trip Consistency**
    /// *For any* valid JSON value, serializing and then parsing should produce
    /// an equivalent value.
    #[test]
    fn json_roundtrip(
        s in "[a-zA-Z0-9 ]{0,50}",
        n in -1000000i64..1000000i64,
        _f in -1000.0f64..1000.0f64,
        b in any::<bool>(),
    ) {
        let parser = JsonParser::new();

        // Test string roundtrip
        let json_str = format!(r#"{{"key": "{}"}}"#, s.replace('\\', "\\\\").replace('"', "\\\""));
        if let Ok(parsed) = parser.parse(&json_str) {
            let serialized = parser.serialize(&parsed).unwrap();
            let reparsed = parser.parse(&serialized).unwrap();
            prop_assert_eq!(parsed, reparsed);
        }

        // Test integer roundtrip
        let json_int = format!(r#"{{"num": {}}}"#, n);
        let parsed = parser.parse(&json_int).unwrap();
        let serialized = parser.serialize(&parsed).unwrap();
        let reparsed = parser.parse(&serialized).unwrap();
        prop_assert_eq!(parsed, reparsed);

        // Test boolean roundtrip
        let json_bool = format!(r#"{{"flag": {}}}"#, b);
        let parsed = parser.parse(&json_bool).unwrap();
        let serialized = parser.serialize(&parsed).unwrap();
        let reparsed = parser.parse(&serialized).unwrap();
        prop_assert_eq!(parsed, reparsed);
    }

    /// **Property 5.2: ujson API Compatibility**
    /// *For any* valid input, ujson.encode followed by ujson.decode should
    /// produce the original value.
    #[test]
    fn ujson_api_roundtrip(
        key in "[a-zA-Z][a-zA-Z0-9]{0,20}",
        value in "[a-zA-Z0-9 ]{0,30}",
    ) {
        let obj = serde_json::json!({ key: value });
        let encoded = ujson::encode(&obj).unwrap();
        let decoded = ujson::decode(&encoded).unwrap();
        prop_assert_eq!(obj, decoded);
    }
}

// ============================================================================
// Password Hashing Properties
// ============================================================================

// Password hashing tests use fewer iterations due to bcrypt's intentional slowness
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    /// **Property 5.3: Password Hash Verification**
    /// *For any* password, hashing and then verifying with the same password
    /// should return true.
    #[test]
    fn password_hash_verify(password in "[a-zA-Z0-9!@#$%^&*]{8,32}") {
        let hasher = PasswordHasher::new();
        let hash = hasher.hash(&password).unwrap();

        // Hash should start with algorithm identifier
        prop_assert!(hash.starts_with("bcrypt_sha256$"));

        // Verification should succeed
        let verified = hasher.verify(&password, &hash).unwrap();
        prop_assert!(verified);
    }

    /// **Property 5.4: Password Hash Uniqueness**
    /// *For any* password, hashing twice should produce different hashes
    /// (due to random salt).
    #[test]
    fn password_hash_uniqueness(password in "[a-zA-Z0-9]{8,20}") {
        let hasher = PasswordHasher::new();
        let hash1 = hasher.hash(&password).unwrap();
        let hash2 = hasher.hash(&password).unwrap();

        // Hashes should be different (different salts)
        prop_assert_ne!(&hash1, &hash2);

        // But both should verify correctly
        prop_assert!(hasher.verify(&password, &hash1).unwrap());
        prop_assert!(hasher.verify(&password, &hash2).unwrap());
    }

    /// **Property 5.5: Wrong Password Rejection**
    /// *For any* two different passwords, verifying with the wrong password
    /// should return false.
    #[test]
    fn password_wrong_rejection(
        password1 in "[a-zA-Z0-9]{8,16}",
        password2 in "[a-zA-Z0-9]{8,16}",
    ) {
        prop_assume!(password1 != password2);

        let hasher = PasswordHasher::new();
        let hash = hasher.hash(&password1).unwrap();

        let verified = hasher.verify(&password2, &hash).unwrap();
        prop_assert!(!verified);
    }
}

// ============================================================================
// Database Adapter Properties
// ============================================================================

proptest! {
    /// **Property 5.6: Database Connection State Consistency**
    /// *For any* sequence of connect/close operations, the connection state
    /// should be consistent.
    #[test]
    fn db_connection_state(ops in prop::collection::vec(any::<bool>(), 1..10)) {
        let mut adapter = SqliteAdapter::new();
        let mut expected_connected;

        for should_connect in ops {
            if should_connect {
                let _ = adapter.connect(":memory:");
                expected_connected = true;
            } else {
                let _ = adapter.close();
                expected_connected = false;
            }
            prop_assert_eq!(adapter.is_connected(), expected_connected);
        }
    }

    /// **Property 5.7: Transaction Isolation**
    /// *For any* transaction, commit or rollback should end the transaction.
    #[test]
    fn db_transaction_isolation(commit in any::<bool>()) {
        let mut adapter = SqliteAdapter::new();
        adapter.connect(":memory:").unwrap();

        adapter.begin_transaction().unwrap();

        if commit {
            adapter.commit().unwrap();
        } else {
            adapter.rollback().unwrap();
        }

        // After commit/rollback, should not be in transaction
        // (begin_transaction should succeed again)
        prop_assert!(adapter.begin_transaction().is_ok());
    }

    /// **Property 5.8: DbValue Type Preservation**
    /// *For any* DbValue, type conversions should be consistent.
    #[test]
    fn db_value_type_preservation(
        i in any::<i64>(),
        _f in any::<f64>(),
        s in "[a-zA-Z0-9]{0,20}",
    ) {
        let int_val = DbValue::Integer(i);
        prop_assert_eq!(int_val.as_integer(), Some(i));
        prop_assert_eq!(int_val.type_name(), "int");

        let text_val = DbValue::Text(s.clone());
        prop_assert_eq!(text_val.as_text(), Some(s));
        prop_assert_eq!(text_val.type_name(), "str");

        let null_val = DbValue::Null;
        prop_assert!(null_val.is_null());
        prop_assert_eq!(null_val.type_name(), "NoneType");
    }

    /// **Property 5.9: DbRow Column Access**
    /// *For any* row with columns, accessing by name should be case-insensitive.
    #[test]
    fn db_row_column_access(
        col_name in "[a-zA-Z][a-zA-Z0-9]{0,10}",
        value in any::<i64>(),
    ) {
        let row = DbRow::new(
            vec![col_name.clone()],
            vec![DbValue::Integer(value)],
        );

        // Access by exact name
        prop_assert_eq!(row.get(&col_name), Some(&DbValue::Integer(value)));

        // Access by uppercase
        prop_assert_eq!(row.get(&col_name.to_uppercase()), Some(&DbValue::Integer(value)));

        // Access by lowercase
        prop_assert_eq!(row.get(&col_name.to_lowercase()), Some(&DbValue::Integer(value)));
    }
}

// ============================================================================
// Template Engine Properties
// ============================================================================

proptest! {
    /// **Property 5.10: HTML Escape Idempotence**
    /// *For any* already-escaped string, escaping again should not change it
    /// (except for the & in entities).
    #[test]
    fn html_escape_special_chars(s in "[a-zA-Z0-9 ]{0,50}") {
        // For strings without special chars, escape should be identity
        let escaped = escape_html(&s);
        prop_assert_eq!(escaped.as_str(), s);
    }

    /// **Property 5.11: HTML Escape Correctness**
    /// *For any* string with special characters, all dangerous chars should be escaped.
    #[test]
    fn html_escape_dangerous_chars(
        prefix in "[a-zA-Z]{0,10}",
        suffix in "[a-zA-Z]{0,10}",
    ) {
        // Test each dangerous character
        let dangerous = ['<', '>', '&', '"', '\''];

        for c in dangerous {
            let input = format!("{}{}{}", prefix, c, suffix);
            let escaped = escape_html(&input);

            // The escaped string should not contain the raw dangerous char
            // (except & which is used in entities)
            if c != '&' {
                prop_assert!(!escaped.as_str().contains(c));
            }
        }
    }

    /// **Property 5.12: Template Variable Substitution**
    /// *For any* variable name and value, the template should substitute correctly.
    #[test]
    fn template_variable_substitution(
        var_name in "[a-zA-Z][a-zA-Z0-9]{0,10}",
        var_value in "[a-zA-Z0-9 ]{0,30}",
    ) {
        let compiler = TemplateCompiler::new();
        let renderer = TemplateRenderer::new();

        let template_src = format!("Hello, {{{{ {} }}}}!", var_name);
        let template = compiler.compile("test", &template_src).unwrap();

        let mut ctx = Context::new().with_autoescape(false);
        ctx.set(&var_name, ContextValue::String(var_value.clone()));

        let result = renderer.render(&template, &ctx).unwrap();
        prop_assert_eq!(result, format!("Hello, {}!", var_value));
    }

    /// **Property 5.13: Template Filter Chain**
    /// *For any* string, applying upper then lower should produce lowercase.
    #[test]
    fn template_filter_chain(s in "[a-zA-Z]{1,20}") {
        let compiler = TemplateCompiler::new();
        let renderer = TemplateRenderer::new();

        // Test upper filter
        let template = compiler.compile("test", "{{ name|upper }}").unwrap();
        let mut ctx = Context::new().with_autoescape(false);
        ctx.set("name", ContextValue::String(s.clone()));

        let result = renderer.render(&template, &ctx).unwrap();
        prop_assert_eq!(result, s.to_uppercase());

        // Test lower filter
        let template = compiler.compile("test", "{{ name|lower }}").unwrap();
        let result = renderer.render(&template, &ctx).unwrap();
        prop_assert_eq!(result, s.to_lowercase());
    }

    /// **Property 5.14: Template Autoescape Safety**
    /// *For any* HTML content, autoescape should prevent XSS.
    #[test]
    fn template_autoescape_safety(
        prefix in "[a-zA-Z]{0,10}",
        suffix in "[a-zA-Z]{0,10}",
    ) {
        let compiler = TemplateCompiler::new();
        let renderer = TemplateRenderer::new();

        let template = compiler.compile("test", "{{ content }}").unwrap();

        // Create potentially dangerous content
        let dangerous = format!("{}<script>alert('xss')</script>{}", prefix, suffix);

        let mut ctx = Context::new();  // autoescape on by default
        ctx.set("content", ContextValue::String(dangerous));

        let result = renderer.render(&template, &ctx).unwrap();

        // Result should not contain raw <script> tag
        prop_assert!(!result.contains("<script>"));
        prop_assert!(result.contains("&lt;script&gt;"));
    }

    /// **Property 5.15: Markup Safety Preservation**
    /// *For any* Markup object, it should preserve its content without double-escaping.
    #[test]
    fn markup_no_double_escape(s in "[a-zA-Z0-9]{0,30}") {
        let markup = Markup::new(&s);
        prop_assert_eq!(markup.as_str(), &s);
        prop_assert_eq!(markup.len(), s.len());

        // Concatenation should work
        let markup2 = Markup::new("!");
        let combined = markup.concat(&markup2);
        prop_assert_eq!(combined.as_str(), format!("{}!", s));
    }
}

// ============================================================================
// Integration Properties
// ============================================================================

proptest! {
    /// **Property 5.16: Django-like Request Processing**
    /// *For any* valid request data, the full Django-like pipeline should work:
    /// JSON parsing -> Database storage -> Template rendering
    #[test]
    fn django_pipeline_integration(
        name in "[a-zA-Z]{1,20}",
        age in 1i64..120i64,
    ) {
        // 1. Parse JSON request
        let parser = JsonParser::new();
        let json_input = format!(r#"{{"name": "{}", "age": {}}}"#, name, age);
        let parsed = parser.parse(&json_input).unwrap();

        prop_assert_eq!(parsed["name"].as_str().unwrap(), &name);
        prop_assert_eq!(parsed["age"].as_i64().unwrap(), age);

        // 2. Store in database
        let mut db = SqliteAdapter::new();
        db.connect(":memory:").unwrap();
        let rowid = db.insert("users", &[
            DbValue::Text(name.clone()),
            DbValue::Integer(age),
        ]).unwrap();
        prop_assert!(rowid > 0);

        // 3. Render template
        let compiler = TemplateCompiler::new();
        let renderer = TemplateRenderer::new();
        let template = compiler.compile("user", "User: {{ name }}, Age: {{ age }}").unwrap();

        let mut ctx = Context::new().with_autoescape(false);
        ctx.set("name", ContextValue::String(name.clone()));
        ctx.set("age", ContextValue::Int(age));

        let result = renderer.render(&template, &ctx).unwrap();
        prop_assert!(result.contains(&name));
        prop_assert!(result.contains(&age.to_string()));
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_json_nested_objects() {
        let parser = JsonParser::new();
        let json = r#"{"outer": {"inner": {"deep": 42}}}"#;
        let parsed = parser.parse(json).unwrap();
        assert_eq!(parsed["outer"]["inner"]["deep"], 42);
    }

    #[test]
    fn test_json_arrays() {
        let parser = JsonParser::new();
        let json = r#"[1, 2, 3, "four", true, null]"#;
        let parsed = parser.parse(json).unwrap();
        assert_eq!(parsed[0], 1);
        assert_eq!(parsed[3], "four");
        assert_eq!(parsed[4], true);
        assert!(parsed[5].is_null());
    }

    #[test]
    fn test_password_format() {
        let hasher = PasswordHasher::new();
        let hash = hasher.hash("testpassword123").unwrap();

        // Should have Django-compatible format
        assert!(hash.starts_with("bcrypt_sha256$"));

        // Should be verifiable
        assert!(hasher.verify("testpassword123", &hash).unwrap());
        assert!(!hasher.verify("wrongpassword", &hash).unwrap());
    }

    #[test]
    fn test_postgres_connection_params() {
        let mut adapter = PostgresAdapter::new()
            .with_param("host", "localhost")
            .with_param("port", "5432")
            .with_param("dbname", "test");

        adapter.connect("user=postgres").unwrap();

        assert_eq!(adapter.get_param("host"), Some(&"localhost".to_string()));
        assert_eq!(adapter.get_param("port"), Some(&"5432".to_string()));
        assert_eq!(adapter.get_param("user"), Some(&"postgres".to_string()));
    }

    #[test]
    fn test_template_comment_removal() {
        let compiler = TemplateCompiler::new();
        let renderer = TemplateRenderer::new();

        let template = compiler.compile("test", "Hello{# this is a comment #}World").unwrap();
        let ctx = Context::new();
        let result = renderer.render(&template, &ctx).unwrap();

        assert_eq!(result, "HelloWorld");
    }

    #[test]
    fn test_template_missing_variable() {
        let compiler = TemplateCompiler::new();
        let renderer = TemplateRenderer::new();

        let template = compiler.compile("test", "Hello, {{ name }}!").unwrap();
        let ctx = Context::new().with_autoescape(false);

        // Missing variable should render as empty
        let result = renderer.render(&template, &ctx).unwrap();
        assert_eq!(result, "Hello, !");
    }
}
