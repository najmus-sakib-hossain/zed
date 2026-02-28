//! Property-based tests for fixture resolution order and teardown
//!
//! **Property 9: Fixture Resolution Order**
//! **Validates: Requirements 3.2.1-3.2.4, 3.2.7**
//!
//! For any test with fixture dependencies, fixtures SHALL be set up in
//! dependency order and torn down in reverse order.
//!
//! **Property 8: Fixture Teardown Correctness**
//! **Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.5**
//!
//! For any set of fixtures with dependencies, teardown SHALL execute in
//! reverse order of setup, and all teardowns SHALL execute even if some fail.

use dx_py_core::FixtureId;
use dx_py_fixture::{
    FixtureCache, FixtureDefinition, FixtureManager, FixtureScope, TeardownCodeType,
    TeardownManager,
};
use proptest::prelude::*;
use tempfile::tempdir;

/// Generate arbitrary fixture names
fn arb_fixture_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("fixture_[a-z][a-z0-9_]{0,10}")
        .unwrap()
        .prop_filter("non-empty", |s| s.len() > 8)
}

/// Generate arbitrary fixture scopes
fn arb_fixture_scope() -> impl Strategy<Value = FixtureScope> {
    prop_oneof![
        Just(FixtureScope::Function),
        Just(FixtureScope::Class),
        Just(FixtureScope::Module),
        Just(FixtureScope::Session),
    ]
}

/// Generate a fixture definition
fn arb_fixture_definition() -> impl Strategy<Value = FixtureDefinition> {
    (
        arb_fixture_name(),
        arb_fixture_scope(),
        any::<bool>(),
        any::<bool>(),
        1u32..1000u32,
    )
        .prop_map(|(name, scope, autouse, is_generator, line)| {
            FixtureDefinition::new(&name, "tests/conftest.py", line)
                .with_scope(scope)
                .with_autouse(autouse)
                .with_generator(is_generator)
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 9: Fixture Resolution Order
    /// Validates: Requirements 3.2.1, 3.2.4
    ///
    /// Property: Fixture scope priority is consistent
    #[test]
    fn fixture_scope_priority_ordering(scope1 in arb_fixture_scope(), scope2 in arb_fixture_scope()) {
        // Scope priority should be a total order
        let p1 = scope1.priority();
        let p2 = scope2.priority();

        // If scopes are equal, priorities should be equal
        if scope1 == scope2 {
            prop_assert_eq!(p1, p2);
        }

        // Function < Class < Module < Session
        match (scope1, scope2) {
            (FixtureScope::Function, FixtureScope::Class) => prop_assert!(p1 < p2),
            (FixtureScope::Function, FixtureScope::Module) => prop_assert!(p1 < p2),
            (FixtureScope::Function, FixtureScope::Session) => prop_assert!(p1 < p2),
            (FixtureScope::Class, FixtureScope::Module) => prop_assert!(p1 < p2),
            (FixtureScope::Class, FixtureScope::Session) => prop_assert!(p1 < p2),
            (FixtureScope::Module, FixtureScope::Session) => prop_assert!(p1 < p2),
            _ => {}
        }
    }

    /// Feature: dx-py-production-ready, Property 9: Fixture Resolution Order
    /// Validates: Requirements 3.2.1, 3.2.2
    ///
    /// Property: Fixture IDs are deterministic based on name
    #[test]
    fn fixture_id_determinism(name in arb_fixture_name(), line in 1u32..1000u32) {
        let fixture1 = FixtureDefinition::new(&name, "tests/conftest.py", line);
        let fixture2 = FixtureDefinition::new(&name, "tests/conftest.py", line);

        // Same name should produce same ID
        prop_assert_eq!(fixture1.id, fixture2.id);
    }

    /// Feature: dx-py-production-ready, Property 9: Fixture Resolution Order
    /// Validates: Requirements 3.2.3, 3.2.4
    ///
    /// Property: Resolved fixtures maintain dependency order
    #[test]
    fn fixture_dependency_order(
        base_name in arb_fixture_name(),
        dep_name in arb_fixture_name(),
    ) {
        prop_assume!(base_name != dep_name);

        let temp_dir = tempdir().unwrap();
        let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

        // Create a fixture that depends on another
        let dep_fixture = FixtureDefinition::new(&dep_name, "tests/conftest.py", 10);
        let base_fixture = FixtureDefinition::new(&base_name, "tests/conftest.py", 20)
            .with_dependencies(vec![dep_name.clone()]);

        manager.register(dep_fixture);
        manager.register(base_fixture);

        let resolved = manager.resolve_fixtures(std::slice::from_ref(&base_name)).unwrap();

        // Both fixtures should be resolved
        prop_assert_eq!(resolved.len(), 2);

        // Dependency should come before dependent
        let dep_idx = resolved.iter().position(|f| f.definition.name == dep_name).unwrap();
        let base_idx = resolved.iter().position(|f| f.definition.name == base_name).unwrap();

        prop_assert!(dep_idx < base_idx, "Dependency {} should come before {}", dep_name, base_name);
    }

    /// Feature: dx-py-production-ready, Property 9: Fixture Resolution Order
    /// Validates: Requirements 3.2.3, 3.2.7
    ///
    /// Property: Teardown order is reverse of setup order
    #[test]
    fn fixture_teardown_reverse_order(fixtures in prop::collection::vec(arb_fixture_definition(), 1..5)) {
        let temp_dir = tempdir().unwrap();
        let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

        // Register all fixtures
        let mut names = Vec::new();
        for fixture in &fixtures {
            // Ensure unique names
            let unique_name = format!("{}_{}", fixture.name, names.len());
            let mut f = fixture.clone();
            f.name = unique_name.clone();
            manager.register(f);
            names.push(unique_name);
        }

        let resolved = manager.resolve_fixtures(&names).unwrap();
        let teardown = manager.get_teardown_order(&resolved);

        // Teardown should only include generator fixtures
        let generator_count = resolved.iter().filter(|f| f.definition.is_generator).count();
        prop_assert_eq!(teardown.len(), generator_count);

        // Teardown order should be reverse of setup order for generators
        let setup_generators: Vec<_> = resolved.iter()
            .filter(|f| f.definition.is_generator)
            .map(|f| &f.definition.name)
            .collect();

        let teardown_names: Vec<_> = teardown.iter()
            .map(|f| &f.definition.name)
            .collect();

        let reversed_setup: Vec<_> = setup_generators.iter().rev().cloned().collect();
        prop_assert_eq!(teardown_names, reversed_setup, "Teardown should be reverse of setup");
    }

    /// Feature: dx-py-production-ready, Property 9: Fixture Resolution Order
    /// Validates: Requirements 3.2.2
    ///
    /// Property: Fixture cache hash is deterministic
    #[test]
    fn fixture_cache_hash_determinism(source in "def [a-z_]+\\(\\): (return [0-9]+|pass)") {
        let hash1 = FixtureCache::hash_source(&source);
        let hash2 = FixtureCache::hash_source(&source);

        prop_assert_eq!(hash1, hash2, "Same source should produce same hash");
    }

    /// Feature: dx-py-production-ready, Property 9: Fixture Resolution Order
    /// Validates: Requirements 3.2.2
    ///
    /// Property: Different sources produce different hashes (with high probability)
    #[test]
    fn fixture_cache_hash_uniqueness(
        source1 in "def fixture_[a-z]+\\(\\): return [0-9]+",
        source2 in "def fixture_[a-z]+\\(\\): return [0-9]+",
    ) {
        prop_assume!(source1 != source2);

        let hash1 = FixtureCache::hash_source(&source1);
        let hash2 = FixtureCache::hash_source(&source2);

        // Different sources should (almost always) produce different hashes
        // Note: This could theoretically fail due to hash collision, but it's extremely unlikely
        prop_assert_ne!(hash1, hash2, "Different sources should produce different hashes");
    }

    /// Feature: dx-py-production-ready, Property 9: Fixture Resolution Order
    /// Validates: Requirements 3.2.1
    ///
    /// Property: Fixture manager correctly tracks active fixtures
    #[test]
    fn fixture_activation_tracking(
        name in arb_fixture_name(),
        scope in arb_fixture_scope(),
    ) {
        let temp_dir = tempdir().unwrap();
        let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

        let fixture = FixtureDefinition::new(&name, "tests/conftest.py", 10)
            .with_scope(scope);
        manager.register(fixture);

        // Initially not active
        prop_assert!(!manager.is_active(&name));

        // Activate
        manager.activate_fixture(&name, scope);
        prop_assert!(manager.is_active(&name));

        // Deactivate scope
        manager.deactivate_scope(scope);
        prop_assert!(!manager.is_active(&name));
    }
}

/// Feature: dx-py-production-ready, Property 9: Fixture Resolution Order
/// Validates: Requirements 3.2.4
///
/// Property: Chain of dependencies is resolved correctly
#[test]
fn fixture_chain_dependency_resolution() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Create a chain: a -> b -> c -> d
    let d = FixtureDefinition::new("fixture_d", "tests/conftest.py", 10);
    let c = FixtureDefinition::new("fixture_c", "tests/conftest.py", 20)
        .with_dependencies(vec!["fixture_d".to_string()]);
    let b = FixtureDefinition::new("fixture_b", "tests/conftest.py", 30)
        .with_dependencies(vec!["fixture_c".to_string()]);
    let a = FixtureDefinition::new("fixture_a", "tests/conftest.py", 40)
        .with_dependencies(vec!["fixture_b".to_string()]);

    manager.register(d);
    manager.register(c);
    manager.register(b);
    manager.register(a);

    let resolved = manager.resolve_fixtures(&["fixture_a".to_string()]).unwrap();

    assert_eq!(resolved.len(), 4);

    // Verify order: d, c, b, a
    let names: Vec<_> = resolved.iter().map(|f| f.definition.name.as_str()).collect();
    let d_idx = names.iter().position(|&n| n == "fixture_d").unwrap();
    let c_idx = names.iter().position(|&n| n == "fixture_c").unwrap();
    let b_idx = names.iter().position(|&n| n == "fixture_b").unwrap();
    let a_idx = names.iter().position(|&n| n == "fixture_a").unwrap();

    assert!(d_idx < c_idx, "d should come before c");
    assert!(c_idx < b_idx, "c should come before b");
    assert!(b_idx < a_idx, "b should come before a");
}

/// Feature: dx-py-production-ready, Property 9: Fixture Resolution Order
/// Validates: Requirements 3.2.4
///
/// Property: Diamond dependency is resolved correctly
#[test]
fn fixture_diamond_dependency_resolution() {
    let temp_dir = tempdir().unwrap();
    let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

    // Create a diamond: a -> b, c; b -> d; c -> d
    let d = FixtureDefinition::new("fixture_d", "tests/conftest.py", 10);
    let b = FixtureDefinition::new("fixture_b", "tests/conftest.py", 20)
        .with_dependencies(vec!["fixture_d".to_string()]);
    let c = FixtureDefinition::new("fixture_c", "tests/conftest.py", 30)
        .with_dependencies(vec!["fixture_d".to_string()]);
    let a = FixtureDefinition::new("fixture_a", "tests/conftest.py", 40)
        .with_dependencies(vec!["fixture_b".to_string(), "fixture_c".to_string()]);

    manager.register(d);
    manager.register(b);
    manager.register(c);
    manager.register(a);

    let resolved = manager.resolve_fixtures(&["fixture_a".to_string()]).unwrap();

    assert_eq!(resolved.len(), 4);

    // Verify d comes before b and c, and b and c come before a
    let names: Vec<_> = resolved.iter().map(|f| f.definition.name.as_str()).collect();
    let d_idx = names.iter().position(|&n| n == "fixture_d").unwrap();
    let b_idx = names.iter().position(|&n| n == "fixture_b").unwrap();
    let c_idx = names.iter().position(|&n| n == "fixture_c").unwrap();
    let a_idx = names.iter().position(|&n| n == "fixture_a").unwrap();

    assert!(d_idx < b_idx, "d should come before b");
    assert!(d_idx < c_idx, "d should come before c");
    assert!(b_idx < a_idx, "b should come before a");
    assert!(c_idx < a_idx, "c should come before a");
}

// ============================================================================
// Property 8: Fixture Teardown Correctness
// Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.5
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 8: Fixture Teardown Correctness
    /// Validates: Requirements 9.1, 9.3
    ///
    /// Property: Teardown executes in reverse order of setup
    /// For any sequence of fixture registrations, teardown SHALL execute
    /// in the exact reverse order of registration (setup).
    #[test]
    fn teardown_executes_in_reverse_order(
        fixture_count in 1usize..10,
    ) {
        let mut manager = TeardownManager::new();

        // Register fixtures in order 1, 2, 3, ...
        for i in 0..fixture_count {
            manager.register(
                FixtureId::new(i as u64),
                format!("fixture_{}", i),
                FixtureScope::Function,
                TeardownCodeType::Inline(format!("cleanup_{}", i)),
            );
        }

        let mut execution_order = Vec::new();
        let results = manager.execute_scope(FixtureScope::Function, |code| {
            execution_order.push(code.setup_order);
            Ok(())
        });

        // All fixtures should be torn down
        prop_assert_eq!(results.len(), fixture_count);

        // Execution order should be reverse of setup order
        // Setup order: 0, 1, 2, ... (fixture_count - 1)
        // Teardown order: (fixture_count - 1), ..., 2, 1, 0
        let expected_order: Vec<usize> = (0..fixture_count).rev().collect();
        prop_assert_eq!(execution_order, expected_order,
            "Teardown should execute in reverse order of setup");
    }

    /// Feature: dx-py-production-ready, Property 8: Fixture Teardown Correctness
    /// Validates: Requirements 9.2, 9.4
    ///
    /// Property: All teardowns execute even when some fail
    /// For any set of fixtures where some teardowns fail, ALL teardowns
    /// SHALL still be attempted and all errors SHALL be reported.
    #[test]
    fn all_teardowns_execute_despite_failures(
        fixture_count in 2usize..8,
        fail_indices in prop::collection::vec(0usize..8, 0..4),
    ) {
        let mut manager = TeardownManager::new();

        for i in 0..fixture_count {
            manager.register(
                FixtureId::new(i as u64),
                format!("fixture_{}", i),
                FixtureScope::Function,
                TeardownCodeType::Inline(format!("cleanup_{}", i)),
            );
        }

        // Determine which fixtures should fail (within valid range)
        let fail_set: std::collections::HashSet<usize> = fail_indices
            .into_iter()
            .filter(|&i| i < fixture_count)
            .collect();

        let mut execution_count = 0;
        let summary = manager.execute_after_test_failure(FixtureScope::Function, |code| {
            execution_count += 1;
            if fail_set.contains(&code.setup_order) {
                Err(format!("Simulated failure for fixture {}", code.setup_order))
            } else {
                Ok(())
            }
        });

        // ALL fixtures should have been attempted
        prop_assert_eq!(execution_count, fixture_count,
            "All {} fixtures should be attempted, but only {} were", fixture_count, execution_count);

        // Summary should reflect correct counts
        prop_assert_eq!(summary.total, fixture_count);
        prop_assert_eq!(summary.failed, fail_set.len());
        prop_assert_eq!(summary.succeeded, fixture_count - fail_set.len());

        // All failures should be reported
        prop_assert_eq!(summary.errors.len(), fail_set.len(),
            "All {} failures should be reported", fail_set.len());
    }

    /// Feature: dx-py-production-ready, Property 8: Fixture Teardown Correctness
    /// Validates: Requirements 9.5
    ///
    /// Property: Scope isolation is maintained
    /// For any fixtures registered in different scopes, executing teardown
    /// for one scope SHALL NOT affect fixtures in other scopes.
    #[test]
    fn scope_isolation_maintained(
        func_count in 0usize..5,
        class_count in 0usize..5,
        module_count in 0usize..5,
        session_count in 0usize..5,
    ) {
        let mut manager = TeardownManager::new();
        let mut id_counter = 0u64;

        // Register fixtures in each scope
        for i in 0..func_count {
            manager.register(
                FixtureId::new(id_counter),
                format!("func_fixture_{}", i),
                FixtureScope::Function,
                TeardownCodeType::Inline("cleanup".to_string()),
            );
            id_counter += 1;
        }
        for i in 0..class_count {
            manager.register(
                FixtureId::new(id_counter),
                format!("class_fixture_{}", i),
                FixtureScope::Class,
                TeardownCodeType::Inline("cleanup".to_string()),
            );
            id_counter += 1;
        }
        for i in 0..module_count {
            manager.register(
                FixtureId::new(id_counter),
                format!("module_fixture_{}", i),
                FixtureScope::Module,
                TeardownCodeType::Inline("cleanup".to_string()),
            );
            id_counter += 1;
        }
        for i in 0..session_count {
            manager.register(
                FixtureId::new(id_counter),
                format!("session_fixture_{}", i),
                FixtureScope::Session,
                TeardownCodeType::Inline("cleanup".to_string()),
            );
            id_counter += 1;
        }

        // Verify initial counts
        prop_assert_eq!(manager.pending_count(FixtureScope::Function), func_count);
        prop_assert_eq!(manager.pending_count(FixtureScope::Class), class_count);
        prop_assert_eq!(manager.pending_count(FixtureScope::Module), module_count);
        prop_assert_eq!(manager.pending_count(FixtureScope::Session), session_count);

        // Execute function scope teardown
        let summary = manager.on_test_end(|_| Ok(()));
        prop_assert_eq!(summary.total, func_count);

        // Other scopes should be unaffected
        prop_assert_eq!(manager.pending_count(FixtureScope::Function), 0);
        prop_assert_eq!(manager.pending_count(FixtureScope::Class), class_count);
        prop_assert_eq!(manager.pending_count(FixtureScope::Module), module_count);
        prop_assert_eq!(manager.pending_count(FixtureScope::Session), session_count);

        // Execute class scope teardown
        let summary = manager.on_class_end(|_| Ok(()));
        prop_assert_eq!(summary.total, class_count);

        // Remaining scopes should be unaffected
        prop_assert_eq!(manager.pending_count(FixtureScope::Class), 0);
        prop_assert_eq!(manager.pending_count(FixtureScope::Module), module_count);
        prop_assert_eq!(manager.pending_count(FixtureScope::Session), session_count);
    }

    /// Feature: dx-py-production-ready, Property 8: Fixture Teardown Correctness
    /// Validates: Requirements 9.1
    ///
    /// Property: Yield fixture teardown code is stored and executed
    /// For any yield-based fixture, the teardown code registered after yield
    /// SHALL be stored and executed during teardown.
    #[test]
    fn yield_fixture_teardown_stored_and_executed(
        fixture_name in arb_fixture_name(),
        teardown_code in "[a-z_]+\\(\\)",
    ) {
        let mut manager = TeardownManager::new();

        manager.register(
            FixtureId::new(1),
            fixture_name.clone(),
            FixtureScope::Function,
            TeardownCodeType::Inline(teardown_code.clone()),
        );

        prop_assert!(manager.has_pending());
        prop_assert_eq!(manager.pending_count(FixtureScope::Function), 1);

        let mut executed_code = None;
        let results = manager.execute_scope(FixtureScope::Function, |code| {
            if let TeardownCodeType::Inline(ref s) = code.code {
                executed_code = Some(s.clone());
            }
            Ok(())
        });

        prop_assert_eq!(results.len(), 1);
        prop_assert!(results[0].success);
        prop_assert_eq!(executed_code, Some(teardown_code),
            "Teardown code should be executed");
    }
}

/// Feature: dx-py-production-ready, Property 8: Fixture Teardown Correctness
/// Validates: Requirements 9.3
///
/// Property: Dependency chain teardown order
/// For a chain of dependent fixtures (a -> b -> c), teardown SHALL execute
/// in reverse dependency order (a first, then b, then c).
#[test]
fn dependency_chain_teardown_order() {
    let mut manager = TeardownManager::new();

    // Simulate setup order for dependency chain: c (base), b (depends on c), a (depends on b)
    // Setup order: c=0, b=1, a=2
    // Teardown order should be: a=2, b=1, c=0
    manager.register(
        FixtureId::new(3),
        "fixture_c".to_string(),
        FixtureScope::Function,
        TeardownCodeType::Inline("cleanup_c".to_string()),
    );
    manager.register(
        FixtureId::new(2),
        "fixture_b".to_string(),
        FixtureScope::Function,
        TeardownCodeType::Inline("cleanup_b".to_string()),
    );
    manager.register(
        FixtureId::new(1),
        "fixture_a".to_string(),
        FixtureScope::Function,
        TeardownCodeType::Inline("cleanup_a".to_string()),
    );

    let mut teardown_names = Vec::new();
    let results = manager.execute_scope(FixtureScope::Function, |code| {
        teardown_names.push(code.fixture_name.clone());
        Ok(())
    });

    assert_eq!(results.len(), 3);
    // Teardown should be reverse of setup: a, b, c
    assert_eq!(teardown_names, vec!["fixture_a", "fixture_b", "fixture_c"]);
}

/// Feature: dx-py-production-ready, Property 8: Fixture Teardown Correctness
/// Validates: Requirements 9.2, 9.4
///
/// Property: Test failure does not prevent teardown
/// Even when a test fails, all fixture teardowns SHALL still execute.
#[test]
fn test_failure_does_not_prevent_teardown() {
    let mut manager = TeardownManager::new();

    // Register multiple fixtures
    for i in 1..=5 {
        manager.register(
            FixtureId::new(i),
            format!("fixture_{}", i),
            FixtureScope::Function,
            TeardownCodeType::Inline(format!("cleanup_{}", i)),
        );
    }

    // Simulate test failure by using execute_after_test_failure
    let mut execution_count = 0;
    let summary = manager.execute_after_test_failure(FixtureScope::Function, |_| {
        execution_count += 1;
        Ok(())
    });

    // All 5 fixtures should have been torn down
    assert_eq!(execution_count, 5);
    assert_eq!(summary.total, 5);
    assert!(summary.all_succeeded());
}

/// Feature: dx-py-production-ready, Property 8: Fixture Teardown Correctness
/// Validates: Requirements 9.4
///
/// Property: Teardown error reporting
/// When teardown fails, the error SHALL be reported with fixture name and error message.
#[test]
fn teardown_error_reporting() {
    let mut manager = TeardownManager::new();

    manager.register(
        FixtureId::new(1),
        "db_connection".to_string(),
        FixtureScope::Function,
        TeardownCodeType::Inline("close_db()".to_string()),
    );

    let summary = manager.execute_after_test_failure(FixtureScope::Function, |_| {
        Err("Connection already closed".to_string())
    });

    assert_eq!(summary.failed, 1);
    assert_eq!(summary.errors.len(), 1);

    let error_report = summary.error_report().unwrap();
    assert!(error_report.contains("db_connection"), "Error should mention fixture name");
    assert!(
        error_report.contains("Connection already closed"),
        "Error should include error message"
    );
}

/// Feature: dx-py-production-ready, Property 8: Fixture Teardown Correctness
/// Validates: Requirements 9.5
///
/// Property: Full lifecycle scope transitions
/// Fixtures at each scope level SHALL only be torn down when that scope ends.
#[test]
fn full_lifecycle_scope_transitions() {
    let mut manager = TeardownManager::new();

    // Register fixtures at all scope levels
    manager.register(
        FixtureId::new(1),
        "session_db".to_string(),
        FixtureScope::Session,
        TeardownCodeType::Inline("close_db".to_string()),
    );
    manager.register(
        FixtureId::new(2),
        "module_config".to_string(),
        FixtureScope::Module,
        TeardownCodeType::Inline("cleanup_config".to_string()),
    );
    manager.register(
        FixtureId::new(3),
        "class_client".to_string(),
        FixtureScope::Class,
        TeardownCodeType::Inline("close_client".to_string()),
    );
    manager.register(
        FixtureId::new(4),
        "func_request".to_string(),
        FixtureScope::Function,
        TeardownCodeType::Inline("cleanup_request".to_string()),
    );

    // Verify all scopes have pending teardowns
    assert!(manager.has_pending_for_scope(FixtureScope::Function));
    assert!(manager.has_pending_for_scope(FixtureScope::Class));
    assert!(manager.has_pending_for_scope(FixtureScope::Module));
    assert!(manager.has_pending_for_scope(FixtureScope::Session));

    // Test ends - only function scope should be cleared
    manager.on_test_end(|_| Ok(()));
    assert!(!manager.has_pending_for_scope(FixtureScope::Function));
    assert!(manager.has_pending_for_scope(FixtureScope::Class));
    assert!(manager.has_pending_for_scope(FixtureScope::Module));
    assert!(manager.has_pending_for_scope(FixtureScope::Session));

    // Class ends - only class scope should be cleared
    manager.on_class_end(|_| Ok(()));
    assert!(!manager.has_pending_for_scope(FixtureScope::Class));
    assert!(manager.has_pending_for_scope(FixtureScope::Module));
    assert!(manager.has_pending_for_scope(FixtureScope::Session));

    // Module ends - only module scope should be cleared
    manager.on_module_end(|_| Ok(()));
    assert!(!manager.has_pending_for_scope(FixtureScope::Module));
    assert!(manager.has_pending_for_scope(FixtureScope::Session));

    // Session ends - session scope should be cleared
    manager.on_session_end(|_| Ok(()));
    assert!(!manager.has_pending_for_scope(FixtureScope::Session));
    assert!(!manager.has_pending());
}
