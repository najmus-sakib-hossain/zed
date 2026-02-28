//! Property-based test for fixture scope semantics
//!
//! Feature: dx-py-production-ready, Property 21: Fixture Scope Semantics
//! Validates: Requirements 11.2, 11.3
//!
//! For any fixture with a specified scope, the fixture function SHALL be called
//! at most once per scope instance (once per module for module scope, once per
//! session for session scope).

use dx_py_fixture::{FixtureDefinition, FixtureManager, FixtureScope, ScopeInstance};
use proptest::prelude::*;
use std::path::PathBuf;
use tempfile::tempdir;

/// Generate arbitrary fixture scopes
fn arb_fixture_scope() -> impl Strategy<Value = FixtureScope> {
    prop_oneof![
        Just(FixtureScope::Function),
        Just(FixtureScope::Class),
        Just(FixtureScope::Module),
        Just(FixtureScope::Session),
    ]
}

/// Generate arbitrary module paths
fn arb_module_path() -> impl Strategy<Value = PathBuf> {
    prop::string::string_regex("tests/test_[a-z]+\\.py")
        .unwrap()
        .prop_map(PathBuf::from)
}

/// Generate arbitrary class names
fn arb_class_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("Test[A-Z][a-z]+").unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 21: Fixture Scope Semantics
    /// Validates: Requirements 11.2, 11.3
    ///
    /// Property: Function-scoped fixtures are never cached
    /// For any function-scoped fixture, each test SHALL receive a fresh setup.
    #[test]
    fn function_scope_never_caches(
        module_path in arb_module_path(),
        class_name in prop::option::of(arb_class_name()),
        test_count in 1usize..10,
    ) {
        let temp_dir = tempdir().unwrap();
        let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

        let fixture = FixtureDefinition::new("data", "tests/conftest.py", 10)
            .with_scope(FixtureScope::Function);
        manager.register(fixture);

        // Run multiple tests
        for _ in 0..test_count {
            let resolved = manager
                .resolve_fixtures_for_test_with_context(
                    &["data".to_string()],
                    &module_path,
                    class_name.as_ref(),
                )
                .unwrap();

            prop_assert_eq!(resolved.len(), 1);
            prop_assert!(resolved[0].needs_setup, "Function scope should always need setup");
            prop_assert!(resolved[0].cached_value.is_none(), "Function scope should not cache");

            // Simulate setup by caching (should not actually cache)
            let scope_instance = ScopeInstance::from_test_context(
                FixtureScope::Function,
                &module_path,
                class_name.as_ref(),
            );
            manager.cache_for_scope("data", scope_instance, vec![1, 2, 3]);
        }
    }

    /// Feature: dx-py-production-ready, Property 21: Fixture Scope Semantics
    /// Validates: Requirements 11.2
    ///
    /// Property: Module-scoped fixtures are cached per module
    /// For any module-scoped fixture, all tests in the same module SHALL
    /// receive the same cached value after the first setup.
    #[test]
    fn module_scope_caches_per_module(
        module1 in arb_module_path(),
        module2 in arb_module_path(),
        test_count in 2usize..5,
    ) {
        prop_assume!(module1 != module2);

        let temp_dir = tempdir().unwrap();
        let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

        let fixture = FixtureDefinition::new("db", "tests/conftest.py", 10)
            .with_scope(FixtureScope::Module);
        manager.register(fixture);

        // First test in module1 - should need setup
        let resolved1 = manager
            .resolve_fixtures_for_test_with_context(&["db".to_string()], &module1, None)
            .unwrap();
        prop_assert!(resolved1[0].needs_setup);

        // Cache the value
        let scope_instance1 = ScopeInstance::from_test_context(FixtureScope::Module, &module1, None);
        manager.cache_for_scope("db", scope_instance1, vec![10, 20, 30]);

        // Subsequent tests in module1 - should use cached value
        for _ in 1..test_count {
            let resolved = manager
                .resolve_fixtures_for_test_with_context(&["db".to_string()], &module1, None)
                .unwrap();
            prop_assert!(!resolved[0].needs_setup, "Should use cached value in same module");
            prop_assert_eq!(&resolved[0].cached_value, &Some(vec![10, 20, 30]));
        }

        // First test in module2 - should need setup again
        let resolved2 = manager
            .resolve_fixtures_for_test_with_context(&["db".to_string()], &module2, None)
            .unwrap();
        prop_assert!(resolved2[0].needs_setup, "Different module should need setup");
        prop_assert!(resolved2[0].cached_value.is_none());
    }

    /// Feature: dx-py-production-ready, Property 21: Fixture Scope Semantics
    /// Validates: Requirements 11.2
    ///
    /// Property: Class-scoped fixtures are cached per class
    /// For any class-scoped fixture, all tests in the same class SHALL
    /// receive the same cached value after the first setup.
    #[test]
    fn class_scope_caches_per_class(
        module in arb_module_path(),
        class1 in arb_class_name(),
        class2 in arb_class_name(),
        test_count in 2usize..5,
    ) {
        prop_assume!(class1 != class2);

        let temp_dir = tempdir().unwrap();
        let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

        let fixture = FixtureDefinition::new("client", "tests/conftest.py", 10)
            .with_scope(FixtureScope::Class);
        manager.register(fixture);

        // First test in class1 - should need setup
        let resolved1 = manager
            .resolve_fixtures_for_test_with_context(&["client".to_string()], &module, Some(&class1))
            .unwrap();
        prop_assert!(resolved1[0].needs_setup);

        // Cache the value
        let scope_instance1 = ScopeInstance::from_test_context(FixtureScope::Class, &module, Some(&class1));
        manager.cache_for_scope("client", scope_instance1, vec![1, 2, 3, 4]);

        // Subsequent tests in class1 - should use cached value
        for _ in 1..test_count {
            let resolved = manager
                .resolve_fixtures_for_test_with_context(&["client".to_string()], &module, Some(&class1))
                .unwrap();
            prop_assert!(!resolved[0].needs_setup, "Should use cached value in same class");
            prop_assert_eq!(&resolved[0].cached_value, &Some(vec![1, 2, 3, 4]));
        }

        // First test in class2 - should need setup again
        let resolved2 = manager
            .resolve_fixtures_for_test_with_context(&["client".to_string()], &module, Some(&class2))
            .unwrap();
        prop_assert!(resolved2[0].needs_setup, "Different class should need setup");
        prop_assert!(resolved2[0].cached_value.is_none());
    }

    /// Feature: dx-py-production-ready, Property 21: Fixture Scope Semantics
    /// Validates: Requirements 11.3
    ///
    /// Property: Session-scoped fixtures are cached globally
    /// For any session-scoped fixture, all tests SHALL receive the same
    /// cached value after the first setup, regardless of module or class.
    #[test]
    fn session_scope_caches_globally(
        module1 in arb_module_path(),
        module2 in arb_module_path(),
        class1 in arb_class_name(),
        class2 in arb_class_name(),
        test_count in 2usize..5,
    ) {
        let temp_dir = tempdir().unwrap();
        let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

        let fixture = FixtureDefinition::new("config", "tests/conftest.py", 10)
            .with_scope(FixtureScope::Session);
        manager.register(fixture);

        // First test - should need setup
        let resolved1 = manager
            .resolve_fixtures_for_test_with_context(&["config".to_string()], &module1, Some(&class1))
            .unwrap();
        prop_assert!(resolved1[0].needs_setup);

        // Cache the value
        let scope_instance = ScopeInstance::from_test_context(FixtureScope::Session, &module1, Some(&class1));
        manager.cache_for_scope("config", scope_instance, vec![99, 88, 77]);

        // All subsequent tests should use cached value, regardless of context
        let contexts = vec![
            (module1.clone(), Some(class1.clone())),
            (module1.clone(), Some(class2.clone())),
            (module1.clone(), None),
            (module2.clone(), Some(class1.clone())),
            (module2.clone(), Some(class2.clone())),
            (module2.clone(), None),
        ];

        for (module, class) in contexts.iter().take(test_count.min(6)) {
            let resolved = manager
                .resolve_fixtures_for_test_with_context(
                    &["config".to_string()],
                    module,
                    class.as_ref(),
                )
                .unwrap();
            prop_assert!(!resolved[0].needs_setup, "Session scope should be cached globally");
            prop_assert_eq!(&resolved[0].cached_value, &Some(vec![99, 88, 77]));
        }
    }

    /// Feature: dx-py-production-ready, Property 21: Fixture Scope Semantics
    /// Validates: Requirements 11.2, 11.3
    ///
    /// Property: Scope instances are correctly identified
    /// For any two tests with the same scope context, they SHALL share
    /// the same scope instance and thus the same cached fixture.
    #[test]
    fn scope_instance_identity(
        scope in arb_fixture_scope(),
        module in arb_module_path(),
        class in prop::option::of(arb_class_name()),
    ) {
        let scope_instance1 = ScopeInstance::from_test_context(scope, &module, class.as_ref());
        let scope_instance2 = ScopeInstance::from_test_context(scope, &module, class.as_ref());

        // Same context should produce equal scope instances
        prop_assert_eq!(&scope_instance1, &scope_instance2);

        // Session scope should always be equal regardless of context
        if scope == FixtureScope::Session {
            let other_module = PathBuf::from("tests/other.py");
            let other_class = Some("OtherClass".to_string());
            let scope_instance3 = ScopeInstance::from_test_context(
                FixtureScope::Session,
                &other_module,
                other_class.as_ref(),
            );
            prop_assert_eq!(&scope_instance1, &scope_instance3, "Session scope should be global");
        }
    }

    /// Feature: dx-py-production-ready, Property 21: Fixture Scope Semantics
    /// Validates: Requirements 11.2, 11.3
    ///
    /// Property: Clearing scope cache removes only that scope
    /// When a scope cache is cleared, only fixtures in that specific scope
    /// instance SHALL be removed, not fixtures in other scopes.
    #[test]
    fn clear_scope_cache_isolation(
        module in arb_module_path(),
        class in arb_class_name(),
    ) {
        let temp_dir = tempdir().unwrap();
        let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

        // Register fixtures at different scopes
        let module_fixture = FixtureDefinition::new("db", "tests/conftest.py", 10)
            .with_scope(FixtureScope::Module);
        let class_fixture = FixtureDefinition::new("client", "tests/conftest.py", 20)
            .with_scope(FixtureScope::Class);
        let session_fixture = FixtureDefinition::new("config", "tests/conftest.py", 30)
            .with_scope(FixtureScope::Session);

        manager.register(module_fixture);
        manager.register(class_fixture);
        manager.register(session_fixture);

        // Cache values for all scopes
        let module_scope = ScopeInstance::from_test_context(FixtureScope::Module, &module, None);
        let class_scope = ScopeInstance::from_test_context(FixtureScope::Class, &module, Some(&class));
        let session_scope = ScopeInstance::from_test_context(FixtureScope::Session, &module, Some(&class));

        manager.cache_for_scope("db", module_scope.clone(), vec![1]);
        manager.cache_for_scope("client", class_scope.clone(), vec![2]);
        manager.cache_for_scope("config", session_scope.clone(), vec![3]);

        // Clear class scope
        manager.clear_scope_cache(FixtureScope::Class, &module, Some(&class));

        // Only class cache should be cleared
        prop_assert_eq!(manager.get_cached_for_scope("db", &module_scope), Some(vec![1]));
        prop_assert_eq!(manager.get_cached_for_scope("client", &class_scope), None);
        prop_assert_eq!(manager.get_cached_for_scope("config", &session_scope), Some(vec![3]));
    }

    /// Feature: dx-py-production-ready, Property 21: Fixture Scope Semantics
    /// Validates: Requirements 11.2, 11.3
    ///
    /// Property: Multiple fixtures with different scopes cache independently
    /// When multiple fixtures with different scopes are used in a test,
    /// each SHALL be cached according to its own scope rules.
    #[test]
    fn multiple_scopes_cache_independently(
        module in arb_module_path(),
        class in arb_class_name(),
        test_count in 2usize..5,
    ) {
        let temp_dir = tempdir().unwrap();
        let mut manager = FixtureManager::new(temp_dir.path()).unwrap();

        // Register fixtures with different scopes
        let session = FixtureDefinition::new("config", "tests/conftest.py", 10)
            .with_scope(FixtureScope::Session);
        let module_fix = FixtureDefinition::new("db", "tests/conftest.py", 20)
            .with_scope(FixtureScope::Module);
        let class_fix = FixtureDefinition::new("client", "tests/conftest.py", 30)
            .with_scope(FixtureScope::Class);
        let function = FixtureDefinition::new("request", "tests/conftest.py", 40)
            .with_scope(FixtureScope::Function);

        manager.register(session);
        manager.register(module_fix);
        manager.register(class_fix);
        manager.register(function);

        // First test - all need setup
        let resolved1 = manager
            .resolve_fixtures_for_test_with_context(
                &["config".to_string(), "db".to_string(), "client".to_string(), "request".to_string()],
                &module,
                Some(&class),
            )
            .unwrap();

        for fixture in &resolved1 {
            prop_assert!(fixture.needs_setup);
        }

        // Cache all values
        let session_scope = ScopeInstance::from_test_context(FixtureScope::Session, &module, Some(&class));
        let module_scope = ScopeInstance::from_test_context(FixtureScope::Module, &module, Some(&class));
        let class_scope = ScopeInstance::from_test_context(FixtureScope::Class, &module, Some(&class));

        manager.cache_for_scope("config", session_scope, vec![1]);
        manager.cache_for_scope("db", module_scope, vec![2]);
        manager.cache_for_scope("client", class_scope, vec![3]);

        // Subsequent tests in same context
        for _ in 1..test_count {
            let resolved = manager
                .resolve_fixtures_for_test_with_context(
                    &["config".to_string(), "db".to_string(), "client".to_string(), "request".to_string()],
                    &module,
                    Some(&class),
                )
                .unwrap();

            let config = resolved.iter().find(|f| f.definition.name == "config").unwrap();
            let db = resolved.iter().find(|f| f.definition.name == "db").unwrap();
            let client = resolved.iter().find(|f| f.definition.name == "client").unwrap();
            let request = resolved.iter().find(|f| f.definition.name == "request").unwrap();

            // Session, module, and class should be cached
            prop_assert!(!config.needs_setup);
            prop_assert_eq!(&config.cached_value, &Some(vec![1]));

            prop_assert!(!db.needs_setup);
            prop_assert_eq!(&db.cached_value, &Some(vec![2]));

            prop_assert!(!client.needs_setup);
            prop_assert_eq!(&client.cached_value, &Some(vec![3]));

            // Function should never be cached
            prop_assert!(request.needs_setup);
            prop_assert_eq!(&request.cached_value, &None);
        }
    }
}
