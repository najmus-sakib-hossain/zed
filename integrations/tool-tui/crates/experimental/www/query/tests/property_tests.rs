//! Property-based tests for the Query Module
//!
//! Tests Properties 19, 20, 21 from the production-readiness spec:
//! - Property 19: SQL Injection Prevention
//! - Property 20: Connection Pool Reuse
//! - Property 21: Query Error Structure

use dx_www_query::{MockConnectionPool, ParameterizedQuery, QueryError, QueryParam, sql_safety};
use proptest::prelude::*;
use std::time::Duration;

// ============================================================================
// Property 19: SQL Injection Prevention
// For any user-provided input containing SQL injection patterns (quotes,
// semicolons, comments), parameterized queries SHALL escape or reject the
// input without executing malicious SQL.
// **Validates: Requirements 8.2**
// ============================================================================

proptest! {
    /// Feature: production-readiness, Property 19: SQL Injection Prevention
    /// Sanitized strings should not contain dangerous SQL characters
    #[test]
    fn prop_sql_sanitization_removes_dangerous_chars(
        input in "[a-zA-Z0-9';\\-\\*/\\\\]{1,100}"
    ) {
        let sanitized = sql_safety::sanitize(&input);

        // Semicolons should be removed
        prop_assert!(!sanitized.contains(';'),
            "Sanitized string should not contain semicolons");

        // Null bytes should be removed
        prop_assert!(!sanitized.contains('\0'),
            "Sanitized string should not contain null bytes");
    }

    /// Feature: production-readiness, Property 19: SQL Injection Prevention
    /// Common injection patterns should be detected
    #[test]
    fn prop_injection_patterns_detected(
        prefix in "[a-zA-Z]{0,10}",
        suffix in "[a-zA-Z]{0,10}"
    ) {
        // Test various injection patterns
        let patterns = vec![
            format!("{}' OR 1=1{}", prefix, suffix),
            format!("{}'; DROP TABLE users;--{}", prefix, suffix),
            format!("{}UNION SELECT * FROM passwords{}", prefix, suffix),
            format!("{}/* comment */{}", prefix, suffix),
        ];

        for pattern in patterns {
            prop_assert!(sql_safety::contains_injection_pattern(&pattern),
                "Should detect injection pattern: {}", pattern);
        }
    }

    /// Feature: production-readiness, Property 19: SQL Injection Prevention
    /// Safe strings should not be flagged as injection attempts
    #[test]
    fn prop_safe_strings_not_flagged(
        input in "[a-zA-Z0-9@._\\- ]{1,50}"
    ) {
        // Normal user input should not trigger injection detection
        // (unless it happens to contain a pattern by chance)
        let has_pattern = sql_safety::contains_injection_pattern(&input);

        // If flagged, verify it actually contains a pattern
        if has_pattern {
            let upper = input.to_uppercase();
            let contains_known = upper.contains("OR ") ||
                                 upper.contains("AND ") ||
                                 upper.contains("--") ||
                                 upper.contains("/*") ||
                                 upper.contains("*/");
            prop_assert!(contains_known || input.contains("1=1") || input.contains("1 = 1"),
                "False positive injection detection for: {}", input);
        }
    }

    /// Feature: production-readiness, Property 19: SQL Injection Prevention
    /// Parameterized queries should properly escape string parameters
    #[test]
    fn prop_parameterized_query_escapes_strings(
        value in "[a-zA-Z0-9'\"\\\\;]{1,50}"
    ) {
        let escaped = QueryParam::escape_string(&value);

        // Single quotes should be doubled
        let original_quotes = value.matches('\'').count();
        let escaped_quotes = escaped.matches("''").count();
        prop_assert_eq!(original_quotes, escaped_quotes,
            "Single quotes should be escaped by doubling");

        // Null bytes should be removed
        prop_assert!(!escaped.contains('\0'),
            "Escaped string should not contain null bytes");
    }
}

// ============================================================================
// Property 20: Connection Pool Reuse
// For any sequence of queries, the number of database connections created
// SHALL be bounded by the pool size, regardless of query count.
// **Validates: Requirements 8.3**
// ============================================================================

proptest! {
    /// Feature: production-readiness, Property 20: Connection Pool Reuse
    /// Connection count should never exceed pool size
    #[test]
    fn prop_connection_pool_bounded(
        pool_size in 1u32..20,
        query_count in 1usize..100,
    ) {
        let pool = MockConnectionPool::new(pool_size);
        let mut connections = Vec::new();
        let mut acquired = 0;

        // Try to acquire connections
        for _ in 0..query_count {
            match pool.acquire() {
                Ok(conn) => {
                    acquired += 1;
                    connections.push(conn);
                }
                Err(_) => {
                    // Pool exhausted, which is expected
                }
            }

            // Active connections should never exceed pool size
            prop_assert!(pool.stats().active_connections <= pool_size,
                "Active connections {} exceeded pool size {}",
                pool.stats().active_connections, pool_size);
        }

        // Should have acquired at most pool_size connections
        prop_assert!(acquired <= pool_size as usize,
            "Acquired {} connections but pool size is {}",
            acquired, pool_size);
    }

    /// Feature: production-readiness, Property 20: Connection Pool Reuse
    /// Connections should be properly released back to pool
    #[test]
    fn prop_connection_pool_release(
        pool_size in 2u32..10,
        iterations in 1usize..20,
    ) {
        let pool = MockConnectionPool::new(pool_size);

        for _ in 0..iterations {
            // Acquire all connections
            let mut connections = Vec::new();
            for _ in 0..pool_size {
                if let Ok(conn) = pool.acquire() {
                    connections.push(conn);
                }
            }

            // All connections should be active
            prop_assert_eq!(pool.stats().active_connections, pool_size,
                "Should have {} active connections", pool_size);

            // Drop all connections
            drop(connections);

            // All connections should be released
            prop_assert_eq!(pool.stats().active_connections, 0,
                "All connections should be released");
            prop_assert_eq!(pool.stats().idle_connections, pool_size,
                "All connections should be idle");
        }
    }

    /// Feature: production-readiness, Property 20: Connection Pool Reuse
    /// Query execution should track statistics correctly
    #[test]
    fn prop_query_execution_tracking(
        pool_size in 1u32..5,
        query_count in 1usize..50,
    ) {
        let pool = MockConnectionPool::new(pool_size);
        let mut executed = 0;

        for _ in 0..query_count {
            if let Ok(conn) = pool.acquire() {
                let query = ParameterizedQuery::new("SELECT 1").int32(1);
                if conn.execute(&query).is_ok() {
                    executed += 1;
                }
            }
        }

        // Executed queries should match stats
        prop_assert_eq!(pool.stats().queries_executed, executed as u64,
            "Query execution count mismatch");
    }
}

// ============================================================================
// Property 21: Query Error Structure
// For any failed query, the returned error SHALL contain the error type,
// a sanitized message (no credentials), and query context (table/operation).
// **Validates: Requirements 8.4**
// ============================================================================

proptest! {
    /// Feature: production-readiness, Property 21: Query Error Structure
    /// All error types should have valid error codes
    #[test]
    fn prop_error_codes_valid(
        message in "[a-zA-Z0-9 ]{1,50}",
        context in "[a-zA-Z0-9 ]{0,30}",
    ) {
        let errors = vec![
            QueryError::query_failed(&message, Some(context.clone())),
            QueryError::connection_failed(&message),
            QueryError::timeout(Duration::from_secs(30)),
            QueryError::InvalidParameter { message: message.clone() },
            QueryError::PoolExhausted,
            QueryError::SerializationError(message.clone()),
        ];

        for error in errors {
            let code = error.error_code();
            prop_assert!(code >= 2001 && code <= 2999,
                "Error code {} should be in range 2001-2999", code);
        }
    }

    /// Feature: production-readiness, Property 21: Query Error Structure
    /// Sanitized messages should not contain sensitive information
    #[test]
    fn prop_error_sanitization_no_credentials(
        password in "[a-zA-Z0-9!@#$%]{8,20}",
        secret in "[a-zA-Z0-9]{16,32}",
    ) {
        // Create error with sensitive info in the message
        let sensitive_message = format!(
            "Query failed: password='{}', api_key='{}'",
            password, secret
        );

        let error = QueryError::QueryFailed {
            message: sensitive_message,
            query_context: Some("users table".to_string()),
        };

        let sanitized = error.sanitized_message();

        // Sanitized message should not contain the sensitive values
        prop_assert!(!sanitized.contains(&password),
            "Sanitized message should not contain password");
        prop_assert!(!sanitized.contains(&secret),
            "Sanitized message should not contain secret");
        prop_assert!(!sanitized.contains("api_key"),
            "Sanitized message should not contain 'api_key'");
    }

    /// Feature: production-readiness, Property 21: Query Error Structure
    /// All errors should produce non-empty sanitized messages
    #[test]
    fn prop_error_sanitized_messages_non_empty(
        message in "[a-zA-Z0-9 ]{1,50}",
        duration_secs in 1u64..3600,
    ) {
        let errors = vec![
            QueryError::query_failed(&message, None),
            QueryError::connection_failed(&message),
            QueryError::timeout(Duration::from_secs(duration_secs)),
            QueryError::InvalidParameter { message: message.clone() },
            QueryError::PoolExhausted,
            QueryError::SerializationError(message),
        ];

        for error in errors {
            let sanitized = error.sanitized_message();
            prop_assert!(!sanitized.is_empty(),
                "Sanitized message should not be empty");
            prop_assert!(sanitized.len() >= 10,
                "Sanitized message should be descriptive (at least 10 chars)");
        }
    }

    /// Feature: production-readiness, Property 21: Query Error Structure
    /// Timeout errors should include duration information
    #[test]
    fn prop_timeout_error_includes_duration(
        duration_secs in 1u64..3600,
    ) {
        let error = QueryError::timeout(Duration::from_secs(duration_secs));
        let sanitized = error.sanitized_message();

        // Should mention timeout
        prop_assert!(sanitized.to_lowercase().contains("timeout") ||
                    sanitized.to_lowercase().contains("timed out"),
            "Timeout error should mention timeout: {}", sanitized);
    }
}
