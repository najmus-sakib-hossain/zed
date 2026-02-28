//! Integration test for yield-based fixture teardown
//!
//! This test demonstrates the complete fixture teardown flow:
//! 1. Register yield-based fixtures (fixtures with teardown)
//! 2. Execute fixture setup
//! 3. Run test
//! 4. Execute fixture teardown in reverse order
//! 5. Verify teardown executes even on test failure
//!
//! **Validates: Requirement 11.4**
//! WHEN a fixture uses `yield`, THE Test_Runner SHALL execute teardown code after the test

use dx_py_core::FixtureId;
use dx_py_fixture::{
    FixtureDefinition, FixtureManager, FixtureScope, TeardownCodeType, TeardownManager,
};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

/// Simulates a fixture execution with setup and teardown tracking
#[derive(Debug, Clone)]
struct FixtureExecution {
    name: String,
    setup_order: usize,
    teardown_order: Option<usize>,
}

/// Test helper to track fixture execution order
#[derive(Debug, Clone)]
struct ExecutionTracker {
    executions: Arc<Mutex<Vec<FixtureExecution>>>,
    setup_counter: Arc<Mutex<usize>>,
    teardown_counter: Arc<Mutex<usize>>,
}

impl ExecutionTracker {
    fn new() -> Self {
        Self {
            executions: Arc::new(Mutex::new(Vec::new())),
            setup_counter: Arc::new(Mutex::new(0)),
            teardown_counter: Arc::new(Mutex::new(0)),
        }
    }

    fn record_setup(&self, name: &str) -> usize {
        let mut counter = self.setup_counter.lock().unwrap();
        let order = *counter;
        *counter += 1;

        let mut executions = self.executions.lock().unwrap();
        executions.push(FixtureExecution {
            name: name.to_string(),
            setup_order: order,
            teardown_order: None,
        });

        order
    }

    fn record_teardown(&self, name: &str) {
        let mut counter = self.teardown_counter.lock().unwrap();
        let order = *counter;
        *counter += 1;

        let mut executions = self.executions.lock().unwrap();
        if let Some(exec) = executions.iter_mut().find(|e| e.name == name) {
            exec.teardown_order = Some(order);
        }
    }

    fn get_executions(&self) -> Vec<FixtureExecution> {
        self.executions.lock().unwrap().clone()
    }

    fn verify_teardown_reverse_order(&self) -> bool {
        let executions = self.get_executions();
        let mut setup_order: Vec<_> = executions
            .iter()
            .filter(|e| e.teardown_order.is_some())
            .map(|e| (e.setup_order, e.teardown_order.unwrap()))
            .collect();

        setup_order.sort_by_key(|(setup, _)| *setup);

        // Teardown order should be reverse of setup order
        for i in 0..setup_order.len() {
            let expected_teardown = setup_order.len() - 1 - i;
            if setup_order[i].1 != expected_teardown {
                return false;
            }
        }

        true
    }
}

#[test]
fn test_simple_yield_fixture_teardown() {
    // Test: Simple fixture with yield executes teardown after test
    // Validates: Requirement 11.4

    let mut teardown_manager = TeardownManager::new();
    let tracker = ExecutionTracker::new();
    let tracker_clone = tracker.clone();

    // Register a yield-based fixture
    let fixture_id = FixtureId::new(1);
    let fixture_name = "temp_resource";

    // Simulate fixture setup
    tracker.record_setup(fixture_name);

    // Register teardown
    teardown_manager.register(
        fixture_id,
        fixture_name.to_string(),
        FixtureScope::Function,
        TeardownCodeType::Inline("cleanup_resource()".to_string()),
    );

    // Verify teardown is pending
    assert!(teardown_manager.has_pending());
    assert_eq!(teardown_manager.pending_count(FixtureScope::Function), 1);

    // Execute test (simulated)
    // ... test runs ...

    // Execute teardown after test
    let summary = teardown_manager.on_test_end(|code| {
        tracker_clone.record_teardown(&code.fixture_name);
        Ok(())
    });

    // Verify teardown executed successfully
    assert_eq!(summary.total, 1);
    assert_eq!(summary.succeeded, 1);
    assert_eq!(summary.failed, 0);
    assert!(summary.all_succeeded());

    // Verify teardown was recorded
    let executions = tracker.get_executions();
    assert_eq!(executions.len(), 1);
    assert_eq!(executions[0].name, fixture_name);
    assert!(executions[0].teardown_order.is_some());
}

#[test]
fn test_multiple_yield_fixtures_teardown_order() {
    // Test: Multiple yield fixtures teardown in reverse setup order
    // Validates: Requirement 11.4

    let mut teardown_manager = TeardownManager::new();
    let tracker = ExecutionTracker::new();
    let tracker_clone = tracker.clone();

    // Register multiple yield-based fixtures simulating dependency chain
    // Setup order: config -> db -> api
    let fixtures = vec![
        ("config", FixtureId::new(1)),
        ("db", FixtureId::new(2)),
        ("api", FixtureId::new(3)),
    ];

    for (name, id) in &fixtures {
        tracker.record_setup(name);
        teardown_manager.register(
            *id,
            name.to_string(),
            FixtureScope::Function,
            TeardownCodeType::Inline(format!("cleanup_{}()", name)),
        );
    }

    // Execute teardown
    let summary = teardown_manager.on_test_end(|code| {
        tracker_clone.record_teardown(&code.fixture_name);
        Ok(())
    });

    // Verify all teardowns executed
    assert_eq!(summary.total, 3);
    assert!(summary.all_succeeded());

    // Verify teardown order is reverse of setup order
    assert!(tracker.verify_teardown_reverse_order());

    let executions = tracker.get_executions();
    let teardown_names: Vec<_> = executions
        .iter()
        .filter_map(|e| e.teardown_order.map(|order| (order, e.name.as_str())))
        .collect();

    // Sort by teardown order
    let mut sorted = teardown_names;
    sorted.sort_by_key(|(order, _)| *order);

    // Should be: api (2), db (1), config (0)
    assert_eq!(sorted[0].1, "api");
    assert_eq!(sorted[1].1, "db");
    assert_eq!(sorted[2].1, "config");
}

#[test]
fn test_teardown_executes_on_test_failure() {
    // Test: Teardown executes even when test fails
    // Validates: Requirement 11.4

    let mut teardown_manager = TeardownManager::new();
    let tracker = ExecutionTracker::new();
    let tracker_clone = tracker.clone();

    // Register fixtures
    for i in 1..=3 {
        let name = format!("fixture_{}", i);
        tracker.record_setup(&name);
        teardown_manager.register(
            FixtureId::new(i),
            name,
            FixtureScope::Function,
            TeardownCodeType::Inline(format!("cleanup_{}", i)),
        );
    }

    // Simulate test failure by using execute_after_test_failure
    let summary = teardown_manager.execute_after_test_failure(FixtureScope::Function, |code| {
        tracker_clone.record_teardown(&code.fixture_name);
        Ok(())
    });

    // Verify all teardowns executed despite test failure
    assert_eq!(summary.total, 3);
    assert!(summary.all_succeeded());

    // Verify all fixtures had teardown executed
    let executions = tracker.get_executions();
    assert_eq!(executions.len(), 3);
    assert!(executions.iter().all(|e| e.teardown_order.is_some()));
}

#[test]
fn test_teardown_continues_on_teardown_failure() {
    // Test: Teardown continues even when some teardowns fail
    // Validates: Requirement 11.4

    let mut teardown_manager = TeardownManager::new();
    let tracker = ExecutionTracker::new();
    let tracker_clone = tracker.clone();

    // Register fixtures
    for i in 1..=5 {
        let name = format!("fixture_{}", i);
        tracker.record_setup(&name);
        teardown_manager.register(
            FixtureId::new(i),
            name,
            FixtureScope::Function,
            TeardownCodeType::Inline(format!("cleanup_{}", i)),
        );
    }

    // Execute teardown with some failures
    let summary = teardown_manager.on_test_end(|code| {
        tracker_clone.record_teardown(&code.fixture_name);

        // Fail teardown for fixture_3 and fixture_5
        if code.fixture_name == "fixture_3" || code.fixture_name == "fixture_5" {
            Err(format!("Teardown failed for {}", code.fixture_name))
        } else {
            Ok(())
        }
    });

    // Verify all teardowns were attempted
    assert_eq!(summary.total, 5);
    assert_eq!(summary.succeeded, 3);
    assert_eq!(summary.failed, 2);

    // Verify all fixtures had teardown attempted
    let executions = tracker.get_executions();
    assert_eq!(executions.len(), 5);
    assert!(executions.iter().all(|e| e.teardown_order.is_some()));

    // Verify error reporting
    assert_eq!(summary.errors.len(), 2);
    let error_report = summary.error_report().unwrap();
    assert!(error_report.contains("fixture_3"));
    assert!(error_report.contains("fixture_5"));
}

#[test]
fn test_scope_aware_teardown() {
    // Test: Teardown respects fixture scopes
    // Validates: Requirement 11.4

    let mut teardown_manager = TeardownManager::new();
    let tracker = ExecutionTracker::new();

    // Register fixtures at different scopes
    let fixtures = vec![
        ("func_fixture", FixtureScope::Function, FixtureId::new(1)),
        ("class_fixture", FixtureScope::Class, FixtureId::new(2)),
        ("module_fixture", FixtureScope::Module, FixtureId::new(3)),
        ("session_fixture", FixtureScope::Session, FixtureId::new(4)),
    ];

    for (name, scope, id) in &fixtures {
        tracker.record_setup(name);
        teardown_manager.register(
            *id,
            name.to_string(),
            *scope,
            TeardownCodeType::Inline(format!("cleanup_{}()", name)),
        );
    }

    // Verify all scopes have pending teardowns
    assert_eq!(teardown_manager.pending_count(FixtureScope::Function), 1);
    assert_eq!(teardown_manager.pending_count(FixtureScope::Class), 1);
    assert_eq!(teardown_manager.pending_count(FixtureScope::Module), 1);
    assert_eq!(teardown_manager.pending_count(FixtureScope::Session), 1);

    // Test ends - only function scope should be cleared
    let tracker_clone = tracker.clone();
    let summary = teardown_manager.on_test_end(|code| {
        tracker_clone.record_teardown(&code.fixture_name);
        Ok(())
    });
    assert_eq!(summary.total, 1);
    assert_eq!(teardown_manager.pending_count(FixtureScope::Function), 0);
    assert_eq!(teardown_manager.pending_count(FixtureScope::Class), 1);

    // Class ends - only class scope should be cleared
    let tracker_clone = tracker.clone();
    let summary = teardown_manager.on_class_end(|code| {
        tracker_clone.record_teardown(&code.fixture_name);
        Ok(())
    });
    assert_eq!(summary.total, 1);
    assert_eq!(teardown_manager.pending_count(FixtureScope::Class), 0);
    assert_eq!(teardown_manager.pending_count(FixtureScope::Module), 1);

    // Module ends - only module scope should be cleared
    let tracker_clone = tracker.clone();
    let summary = teardown_manager.on_module_end(|code| {
        tracker_clone.record_teardown(&code.fixture_name);
        Ok(())
    });
    assert_eq!(summary.total, 1);
    assert_eq!(teardown_manager.pending_count(FixtureScope::Module), 0);
    assert_eq!(teardown_manager.pending_count(FixtureScope::Session), 1);

    // Session ends - session scope should be cleared
    let tracker_clone = tracker.clone();
    let summary = teardown_manager.on_session_end(|code| {
        tracker_clone.record_teardown(&code.fixture_name);
        Ok(())
    });
    assert_eq!(summary.total, 1);
    assert_eq!(teardown_manager.pending_count(FixtureScope::Session), 0);

    // Verify all fixtures had teardown executed
    let executions = tracker.get_executions();
    assert_eq!(executions.len(), 4);
    assert!(executions.iter().all(|e| e.teardown_order.is_some()));
}

#[test]
fn test_fixture_manager_integration_with_teardown() {
    // Test: Complete integration of FixtureManager with TeardownManager
    // Validates: Requirement 11.4

    let temp_dir = tempdir().unwrap();
    let mut fixture_manager = FixtureManager::new(temp_dir.path()).unwrap();
    let mut teardown_manager = TeardownManager::new();

    // Register yield-based fixtures with dependencies
    let config = FixtureDefinition::new("config", "tests/conftest.py", 10)
        .with_generator(true); // Marks as yield-based

    let db = FixtureDefinition::new("db", "tests/conftest.py", 20)
        .with_dependencies(vec!["config".to_string()])
        .with_generator(true);

    let api = FixtureDefinition::new("api", "tests/conftest.py", 30)
        .with_dependencies(vec!["db".to_string()])
        .with_generator(true);

    fixture_manager.register(config);
    fixture_manager.register(db);
    fixture_manager.register(api);

    // Resolve fixtures for a test
    let resolved = fixture_manager
        .resolve_fixtures(&["api".to_string()])
        .unwrap();

    // Verify dependency order
    assert_eq!(resolved.len(), 3);
    let names: Vec<_> = resolved.iter().map(|f| f.definition.name.as_str()).collect();
    assert_eq!(names, vec!["config", "db", "api"]);

    // Simulate fixture setup and register teardowns
    for fixture in &resolved {
        if fixture.definition.is_generator {
            teardown_manager.register(
                fixture.definition.id,
                fixture.definition.name.clone(),
                fixture.definition.scope,
                TeardownCodeType::Inline(format!("cleanup_{}()", fixture.definition.name)),
            );
        }
    }

    // Get teardown order
    let teardown_fixtures = fixture_manager.get_teardown_order(&resolved);
    assert_eq!(teardown_fixtures.len(), 3);

    // Verify teardown order is reverse of setup
    let teardown_order = teardown_manager.get_teardown_order(FixtureScope::Function);
    assert_eq!(teardown_order.len(), 3);

    // Execute teardown
    let mut teardown_names = Vec::new();
    let summary = teardown_manager.on_test_end(|code| {
        teardown_names.push(code.fixture_name.clone());
        Ok(())
    });

    // Verify teardown executed in reverse order
    assert_eq!(summary.total, 3);
    assert!(summary.all_succeeded());
    assert_eq!(teardown_names, vec!["api", "db", "config"]);
}

#[test]
fn test_mixed_generator_and_regular_fixtures() {
    // Test: Only generator fixtures (with yield) have teardown
    // Validates: Requirement 11.4

    let temp_dir = tempdir().unwrap();
    let mut fixture_manager = FixtureManager::new(temp_dir.path()).unwrap();
    let mut teardown_manager = TeardownManager::new();

    // Register mix of generator and regular fixtures
    let regular = FixtureDefinition::new("regular", "tests/conftest.py", 10)
        .with_generator(false);

    let generator = FixtureDefinition::new("generator", "tests/conftest.py", 20)
        .with_generator(true);

    let another_regular = FixtureDefinition::new("another_regular", "tests/conftest.py", 30)
        .with_generator(false);

    fixture_manager.register(regular);
    fixture_manager.register(generator);
    fixture_manager.register(another_regular);

    // Resolve all fixtures
    let resolved = fixture_manager
        .resolve_fixtures(&["regular".to_string(), "generator".to_string(), "another_regular".to_string()])
        .unwrap();

    assert_eq!(resolved.len(), 3);

    // Register teardowns only for generator fixtures
    for fixture in &resolved {
        if fixture.definition.is_generator {
            teardown_manager.register(
                fixture.definition.id,
                fixture.definition.name.clone(),
                fixture.definition.scope,
                TeardownCodeType::Inline(format!("cleanup_{}()", fixture.definition.name)),
            );
        }
    }

    // Only generator fixture should have teardown
    assert_eq!(teardown_manager.pending_count(FixtureScope::Function), 1);

    // Get teardown order - should only include generator fixtures
    let teardown_fixtures = fixture_manager.get_teardown_order(&resolved);
    assert_eq!(teardown_fixtures.len(), 1);
    assert_eq!(teardown_fixtures[0].definition.name, "generator");

    // Execute teardown
    let mut teardown_names = Vec::new();
    let summary = teardown_manager.on_test_end(|code| {
        teardown_names.push(code.fixture_name.clone());
        Ok(())
    });

    assert_eq!(summary.total, 1);
    assert_eq!(teardown_names, vec!["generator"]);
}

#[test]
fn test_teardown_with_real_python_example() {
    // Test: Simulate a real pytest fixture with yield
    // Example:
    //   @pytest.fixture
    //   def temp_file():
    //       file = open("test.txt", "w")
    //       yield file
    //       file.close()  # This is the teardown code
    //
    // Validates: Requirement 11.4

    let mut teardown_manager = TeardownManager::new();
    let tracker = ExecutionTracker::new();
    let tracker_clone = tracker.clone();

    // Simulate fixture setup
    let fixture_name = "temp_file";
    tracker.record_setup(fixture_name);

    // Register teardown (code after yield)
    teardown_manager.register(
        FixtureId::new(1),
        fixture_name.to_string(),
        FixtureScope::Function,
        TeardownCodeType::Inline("file.close()".to_string()),
    );

    // Simulate test execution
    // ... test uses the file ...

    // Execute teardown after test completes
    let summary = teardown_manager.on_test_end(|code| {
        tracker_clone.record_teardown(&code.fixture_name);
        // Simulate executing the teardown code
        if let TeardownCodeType::Inline(ref code_str) = code.code {
            assert_eq!(code_str, "file.close()");
        }
        Ok(())
    });

    // Verify teardown executed
    assert!(summary.all_succeeded());

    let executions = tracker.get_executions();
    assert_eq!(executions.len(), 1);
    assert_eq!(executions[0].name, fixture_name);
    assert!(executions[0].teardown_order.is_some());
}
