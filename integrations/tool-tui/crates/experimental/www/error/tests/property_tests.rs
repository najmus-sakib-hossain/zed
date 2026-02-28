//! Property-based tests for dx-www-error crate.
//!
//! These tests verify universal properties that should hold across all inputs.

use dx_www_error::{
    safe_sync::{SafeMutex, SafeRwLock},
    structured_errors::{
        AuthErrorCode, DatabaseErrorCode, DxError, InternalErrorCode, RecoveryConfig, SyncErrorCode,
    },
};
use proptest::prelude::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ============================================================================
// Property 6: Mutex Poisoning Recovery
// **Validates: Requirements 2.1**
//
// *For any* SafeMutex that has been poisoned by a panicking thread,
// subsequent lock() calls SHALL return Ok with the recovered value
// instead of panicking.
// ============================================================================

/// Feature: production-readiness, Property 6: Mutex Poisoning Recovery
///
/// This test verifies that SafeMutex recovers gracefully from poisoning.
/// We simulate poisoning by having a thread panic while holding the lock,
/// then verify that subsequent lock attempts succeed.
#[test]
fn property_6_mutex_poisoning_recovery() {
    // Create a SafeMutex with an initial value
    let mutex = Arc::new(SafeMutex::new(42i32));
    let mutex_clone = Arc::clone(&mutex);

    // Spawn a thread that will panic while holding the lock
    let handle = thread::spawn(move || {
        let _guard = mutex_clone.lock().unwrap();
        // Panic while holding the lock - this poisons a standard Mutex
        panic!("Intentional panic to poison the mutex");
    });

    // Wait for the thread to finish (it will panic)
    let _ = handle.join();

    // Now try to lock the mutex - SafeMutex should recover
    let result = mutex.lock();

    // The lock should succeed (not panic or return Err)
    assert!(result.is_ok(), "SafeMutex should recover from poisoning");

    // The value should still be accessible
    let guard = result.unwrap();
    assert_eq!(*guard, 42, "Value should be preserved after poisoning recovery");

    // The guard should indicate it was poisoned
    assert!(guard.was_poisoned(), "Guard should indicate mutex was poisoned");
}

/// Feature: production-readiness, Property 6: Mutex Poisoning Recovery (RwLock variant)
///
/// Same property but for SafeRwLock.
#[test]
fn property_6_rwlock_poisoning_recovery() {
    let lock = Arc::new(SafeRwLock::new(42i32));
    let lock_clone = Arc::clone(&lock);

    // Spawn a thread that will panic while holding a write lock
    let handle = thread::spawn(move || {
        let _guard = lock_clone.write().unwrap();
        panic!("Intentional panic to poison the rwlock");
    });

    let _ = handle.join();

    // Read lock should recover
    let read_result = lock.read();
    assert!(read_result.is_ok(), "SafeRwLock read should recover from poisoning");
    assert_eq!(*read_result.unwrap(), 42);

    // Write lock should also recover
    let write_result = lock.write();
    assert!(write_result.is_ok(), "SafeRwLock write should recover from poisoning");
}

// ============================================================================
// Property 8: Structured Error Completeness
// **Validates: Requirements 2.5**
//
// *For any* DxError instance, it SHALL contain a non-empty error code,
// a non-empty message, and optionally a source error and recovery suggestion.
// ============================================================================

proptest! {
    /// Feature: production-readiness, Property 8: Structured Error Completeness
    ///
    /// For any error message string, creating a DxError should always produce
    /// an error with a valid code and message.
    #[test]
    fn property_8_auth_error_completeness(message in "\\PC+") {
        let codes = [
            AuthErrorCode::InvalidCredentials,
            AuthErrorCode::TokenExpired,
            AuthErrorCode::TokenInvalid,
            AuthErrorCode::TokenRevoked,
            AuthErrorCode::RateLimited,
            AuthErrorCode::CsrfInvalid,
            AuthErrorCode::MissingAuth,
            AuthErrorCode::Forbidden,
        ];

        for code in codes {
            let err = DxError::auth(code, message.clone());

            // Must have a non-empty code
            prop_assert!(!err.error_code().is_empty(), "Error code should not be empty");

            // Must have a non-empty message
            prop_assert!(err.has_message(), "Error should have a message");

            // Must have a recovery suggestion
            prop_assert!(err.recovery_suggestion().is_some(), "Auth errors should have recovery suggestions");
        }
    }

    /// Feature: production-readiness, Property 8: Structured Error Completeness (Database)
    #[test]
    fn property_8_database_error_completeness(message in "\\PC+", context in "\\PC*") {
        let codes = [
            DatabaseErrorCode::ConnectionFailed,
            DatabaseErrorCode::QueryTimeout,
            DatabaseErrorCode::ConstraintViolation,
            DatabaseErrorCode::TransactionFailed,
            DatabaseErrorCode::PoolExhausted,
            DatabaseErrorCode::QuerySyntax,
        ];

        for code in codes {
            let err = if context.is_empty() {
                DxError::database(code, message.clone())
            } else {
                DxError::database_with_context(code, message.clone(), context.clone())
            };

            prop_assert!(!err.error_code().is_empty());
            prop_assert!(err.has_message());
            prop_assert!(err.recovery_suggestion().is_some());
        }
    }

    /// Feature: production-readiness, Property 8: Structured Error Completeness (Sync)
    #[test]
    fn property_8_sync_error_completeness(message in "\\PC+") {
        let codes = [
            SyncErrorCode::ConnectionLost,
            SyncErrorCode::ChannelNotFound,
            SyncErrorCode::DeliveryFailed,
            SyncErrorCode::BufferOverflow,
            SyncErrorCode::InvalidMessage,
            SyncErrorCode::SubscriptionFailed,
        ];

        for code in codes {
            let err = DxError::sync(code, message.clone());

            prop_assert!(!err.error_code().is_empty());
            prop_assert!(err.has_message());
            prop_assert!(err.recovery_suggestion().is_some());
        }
    }

    /// Feature: production-readiness, Property 8: Structured Error Completeness (Internal)
    #[test]
    fn property_8_internal_error_completeness(message in "\\PC+") {
        let codes = [
            InternalErrorCode::ConfigError,
            InternalErrorCode::LockFailed,
            InternalErrorCode::ResourceExhausted,
            InternalErrorCode::UnexpectedState,
            InternalErrorCode::IoError,
            InternalErrorCode::SerializationError,
        ];

        for code in codes {
            let err = DxError::internal(code, message.clone());

            prop_assert!(!err.error_code().is_empty());
            prop_assert!(err.has_message());
            prop_assert!(err.recovery_suggestion().is_some());
        }
    }

    /// Feature: production-readiness, Property 8: Config and Lock errors
    #[test]
    fn property_8_config_lock_error_completeness(
        key in "\\PC+",
        message in "\\PC+",
        resource in "\\PC+"
    ) {
        // Config error
        let config_err = DxError::config(key.clone(), message.clone());
        prop_assert!(!config_err.error_code().is_empty());
        prop_assert!(config_err.has_message());

        // Lock error
        let lock_err = DxError::lock(resource, message);
        prop_assert!(!lock_err.error_code().is_empty());
        prop_assert!(lock_err.has_message());
        prop_assert!(lock_err.recovery_suggestion().is_some());
    }

    /// Feature: production-readiness, Property 8: Validation error
    #[test]
    fn property_8_validation_error_completeness(
        field in "\\PC+",
        message in "\\PC+"
    ) {
        let err = DxError::validation_field(field, message);
        prop_assert!(!err.error_code().is_empty());
        prop_assert!(err.has_message());
    }
}

// ============================================================================
// Property 7: Error Isolation
// **Validates: Requirements 2.4**
//
// *For any* component failure within an ErrorBoundary, other components
// outside that boundary SHALL continue to operate normally.
// ============================================================================

/// Feature: production-readiness, Property 7: Error Isolation
///
/// This test verifies that errors in one SafeMutex don't affect others.
#[test]
fn property_7_error_isolation_between_mutexes() {
    let mutex1 = Arc::new(SafeMutex::new(1i32));
    let mutex2 = Arc::new(SafeMutex::new(2i32));

    let mutex1_clone = Arc::clone(&mutex1);

    // Poison mutex1
    let handle = thread::spawn(move || {
        let _guard = mutex1_clone.lock().unwrap();
        panic!("Poison mutex1");
    });
    let _ = handle.join();

    // mutex2 should be completely unaffected
    {
        let guard2 = mutex2.lock().unwrap();
        assert_eq!(*guard2, 2);
        assert!(!guard2.was_poisoned(), "mutex2 should not be poisoned");
    }

    // mutex1 should recover
    {
        let guard1 = mutex1.lock().unwrap();
        assert_eq!(*guard1, 1);
        assert!(guard1.was_poisoned(), "mutex1 should indicate it was poisoned");
    }
}

/// Feature: production-readiness, Property 7: Error Isolation (concurrent access)
#[test]
fn property_7_error_isolation_concurrent() {
    let mutex = Arc::new(SafeMutex::new(0i32));
    let mut handles = vec![];

    // Spawn multiple threads, some will panic
    for i in 0..10 {
        let mutex_clone = Arc::clone(&mutex);
        handles.push(thread::spawn(move || {
            let result = catch_unwind(AssertUnwindSafe(|| {
                let mut guard = mutex_clone.lock().unwrap();
                *guard += 1;
                if i == 5 {
                    panic!("Thread 5 panics");
                }
            }));
            result.is_ok()
        }));
    }

    // Wait for all threads
    let results: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // At least some threads should have succeeded
    let successes = results.iter().filter(|&&r| r).count();
    assert!(successes > 0, "Some threads should succeed despite one panicking");

    // The mutex should still be usable
    let guard = mutex.lock();
    assert!(guard.is_ok(), "Mutex should still be usable after thread panic");
}

/// Feature: production-readiness, Property 7: Error Isolation (ErrorBoundary components)
///
/// *For any* component failure within an ErrorBoundary, other components
/// outside that boundary SHALL continue to operate normally.
#[test]
fn property_7_error_boundary_isolation() {
    use dx_www_error::{BoundaryState, ComponentError, ErrorBoundaryRegistry, ErrorSeverity};

    // Create a registry with multiple components
    let registry = ErrorBoundaryRegistry::new();

    // Register multiple independent components
    registry.register(1, 3).expect("register component 1");
    registry.register(2, 3).expect("register component 2");
    registry.register(3, 3).expect("register component 3");

    // Simulate failure in component 1
    let error =
        ComponentError::new(1, 500, ErrorSeverity::Critical, "Component 1 failed catastrophically")
            .with_timestamp(12345);

    registry.report_error(error);

    // Component 1 should be in failed state
    let boundary1 = registry.get(1).expect("component 1 should exist");
    assert!(boundary1.has_failed(), "Component 1 should be in failed state");
    assert_eq!(boundary1.get_state(), BoundaryState::Failed);

    // Components 2 and 3 should be completely unaffected
    let boundary2 = registry.get(2).expect("component 2 should exist");
    let boundary3 = registry.get(3).expect("component 3 should exist");

    assert!(
        !boundary2.has_failed(),
        "Component 2 should NOT be affected by component 1's failure"
    );
    assert!(
        !boundary3.has_failed(),
        "Component 3 should NOT be affected by component 1's failure"
    );
    assert_eq!(boundary2.get_state(), BoundaryState::Normal);
    assert_eq!(boundary3.get_state(), BoundaryState::Normal);

    // Component 2 should still be able to catch its own errors independently
    let error2 =
        ComponentError::new(2, 404, ErrorSeverity::Warning, "Component 2 had a minor issue")
            .with_timestamp(12346);

    registry.report_error(error2);

    // Now component 2 is failed, but component 3 is still normal
    let boundary2_after = registry.get(2).expect("component 2 should exist");
    let boundary3_after = registry.get(3).expect("component 3 should exist");

    assert!(boundary2_after.has_failed(), "Component 2 should now be failed");
    assert!(!boundary3_after.has_failed(), "Component 3 should still be normal");

    // Component 1 can recover independently
    assert!(boundary1.recover(), "Component 1 should be able to recover");
    assert_eq!(boundary1.get_state(), BoundaryState::Recovering);

    // Reset component 1
    boundary1.reset().expect("reset should succeed");
    assert_eq!(boundary1.get_state(), BoundaryState::Normal);
    assert!(!boundary1.has_failed(), "Component 1 should be back to normal");

    // Component 2 is still failed (independent state)
    let boundary2_final = registry.get(2).expect("component 2 should exist");
    assert!(boundary2_final.has_failed(), "Component 2 should still be failed");
}

proptest! {
    /// Feature: production-readiness, Property 7: Error Isolation (property-based)
    ///
    /// For any set of component IDs and any subset that fails, the non-failing
    /// components should remain in Normal state.
    #[test]
    fn property_7_error_isolation_property_based(
        component_count in 2usize..10,
        failing_indices in proptest::collection::vec(0usize..10, 1..5)
    ) {
        use dx_www_error::{ErrorBoundaryRegistry, ComponentError, ErrorSeverity, BoundaryState};

        let registry = ErrorBoundaryRegistry::new();

        // Register all components
        for i in 0..component_count {
            registry.register(i as u16, 3).expect("register should succeed");
        }

        // Determine which components will fail (normalize indices to valid range)
        let failing_set: std::collections::HashSet<usize> = failing_indices
            .iter()
            .map(|&i| i % component_count)
            .collect();

        // Report errors for failing components
        for &failing_idx in &failing_set {
            let error = ComponentError::new(
                failing_idx as u16,
                500,
                ErrorSeverity::Error,
                format!("Component {} failed", failing_idx),
            ).with_timestamp(failing_idx as i64);

            registry.report_error(error);
        }

        // Verify isolation: each component's state should be independent
        for i in 0..component_count {
            let boundary = registry.get(i as u16);
            prop_assert!(boundary.is_some(), "Component {} should exist", i);

            let boundary = boundary.unwrap();
            let should_be_failed = failing_set.contains(&i);

            prop_assert_eq!(
                boundary.has_failed(),
                should_be_failed,
                "Component {} isolation violated: expected failed={}, got failed={}",
                i,
                should_be_failed,
                boundary.has_failed()
            );

            if should_be_failed {
                prop_assert_eq!(boundary.get_state(), BoundaryState::Failed);
            } else {
                prop_assert_eq!(boundary.get_state(), BoundaryState::Normal);
            }
        }
    }
}

// ============================================================================
// Recovery Config Properties
// ============================================================================

proptest! {
    /// Verify exponential backoff calculation is bounded
    #[test]
    fn property_recovery_delay_bounded(
        attempt in 0u32..20,
        base_ms in 10u64..1000,
        max_ms in 1000u64..60000
    ) {
        let config = RecoveryConfig::new(
            20,
            Duration::from_millis(base_ms),
            Duration::from_millis(max_ms),
            0.0, // No jitter for deterministic testing
        );

        let delay = config.delay_for_attempt(attempt);

        // Delay should never exceed max_delay
        prop_assert!(delay <= Duration::from_millis(max_ms));

        // Delay should be positive
        prop_assert!(delay.as_millis() > 0 || attempt == 0 && base_ms == 0);
    }

    /// Verify retry logic is consistent
    #[test]
    fn property_retry_logic_consistent(
        max_retries in 1u32..10,
        attempt in 0u32..20
    ) {
        let config = RecoveryConfig::new(
            max_retries,
            Duration::from_millis(100),
            Duration::from_secs(10),
            0.1,
        );

        let should_retry = config.should_retry(attempt);

        // Should retry if and only if attempt < max_retries
        prop_assert_eq!(should_retry, attempt < max_retries);
    }
}
