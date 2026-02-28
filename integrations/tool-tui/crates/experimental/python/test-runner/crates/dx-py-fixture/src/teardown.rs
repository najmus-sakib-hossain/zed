//! Fixture teardown management
//!
//! This module implements the TeardownManager which handles:
//! - Tracking pending teardowns for yield-based fixtures
//! - Executing teardowns in reverse setup order
//! - Error-resilient teardown execution
//! - Scope-aware teardown timing

use std::collections::HashMap;

use crate::{FixtureId, FixtureScope};

/// Represents teardown code to be executed after a fixture yields
#[derive(Debug, Clone)]
pub struct TeardownCode {
    /// The fixture this teardown belongs to
    pub fixture_id: FixtureId,
    /// Name of the fixture for error reporting
    pub fixture_name: String,
    /// The scope of the fixture
    pub scope: FixtureScope,
    /// Bytecode or code reference for teardown execution
    pub code: TeardownCodeType,
    /// Order in which this fixture was set up (for reverse teardown)
    pub setup_order: usize,
}

/// Type of teardown code
#[derive(Debug, Clone)]
pub enum TeardownCodeType {
    /// Python bytecode to execute
    Bytecode(Vec<u8>),
    /// Reference to a code object by ID
    CodeRef(u64),
    /// Inline Python code string (for testing)
    Inline(String),
}

/// Result of a single teardown execution
#[derive(Debug, Clone)]
pub struct TeardownResult {
    /// The fixture that was torn down
    pub fixture_id: FixtureId,
    /// Name of the fixture
    pub fixture_name: String,
    /// Whether teardown succeeded
    pub success: bool,
    /// Error message if teardown failed
    pub error: Option<String>,
}

impl TeardownResult {
    /// Create a successful teardown result
    pub fn success(fixture_id: FixtureId, fixture_name: String) -> Self {
        Self {
            fixture_id,
            fixture_name,
            success: true,
            error: None,
        }
    }

    /// Create a failed teardown result
    pub fn failure(fixture_id: FixtureId, fixture_name: String, error: String) -> Self {
        Self {
            fixture_id,
            fixture_name,
            success: false,
            error: Some(error),
        }
    }

    /// Check if this teardown was successful
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Check if this teardown failed
    pub fn is_failure(&self) -> bool {
        !self.success
    }
}

/// Summary of teardown execution
#[derive(Debug, Clone, Default)]
pub struct TeardownSummary {
    /// Total number of teardowns executed
    pub total: usize,
    /// Number of successful teardowns
    pub succeeded: usize,
    /// Number of failed teardowns
    pub failed: usize,
    /// All error messages from failed teardowns
    pub errors: Vec<String>,
}

impl TeardownSummary {
    /// Create a summary from teardown results
    pub fn from_results(results: &[TeardownResult]) -> Self {
        let mut summary = Self {
            total: results.len(),
            ..Default::default()
        };

        for result in results {
            if result.success {
                summary.succeeded += 1;
            } else {
                summary.failed += 1;
                if let Some(ref error) = result.error {
                    summary.errors.push(format!(
                        "Fixture '{}' teardown failed: {}",
                        result.fixture_name, error
                    ));
                }
            }
        }

        summary
    }

    /// Check if all teardowns succeeded
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0
    }

    /// Get a formatted error report
    pub fn error_report(&self) -> Option<String> {
        if self.errors.is_empty() {
            None
        } else {
            Some(format!(
                "Teardown errors ({} of {} failed):\n{}",
                self.failed,
                self.total,
                self.errors.join("\n")
            ))
        }
    }
}

/// Pending teardown entry
#[derive(Debug, Clone)]
struct PendingTeardown {
    /// The teardown code to execute
    code: TeardownCode,
    /// Context identifier (test, class, module, or session)
    #[allow(dead_code)]
    context_id: u64,
}

/// Manages fixture teardown execution
///
/// The TeardownManager tracks pending teardowns for yield-based fixtures
/// and ensures they are executed in the correct order (reverse of setup)
/// even when tests or previous teardowns fail.
pub struct TeardownManager {
    /// Pending teardowns organized by scope
    pending_by_scope: HashMap<FixtureScope, Vec<PendingTeardown>>,
    /// Global counter for setup order
    setup_counter: usize,
    /// Current context IDs for each scope
    context_ids: HashMap<FixtureScope, u64>,
}

impl TeardownManager {
    /// Create a new teardown manager
    pub fn new() -> Self {
        Self {
            pending_by_scope: HashMap::new(),
            setup_counter: 0,
            context_ids: HashMap::new(),
        }
    }

    /// Set the context ID for a scope (e.g., test function ID, class ID, module ID)
    pub fn set_context(&mut self, scope: FixtureScope, context_id: u64) {
        self.context_ids.insert(scope, context_id);
    }

    /// Get the current context ID for a scope
    pub fn get_context(&self, scope: FixtureScope) -> Option<u64> {
        self.context_ids.get(&scope).copied()
    }

    /// Register a fixture for teardown
    ///
    /// This should be called after a yield-based fixture yields its value.
    /// The teardown code will be stored and executed later when the
    /// appropriate scope ends.
    pub fn register(
        &mut self,
        fixture_id: FixtureId,
        fixture_name: String,
        scope: FixtureScope,
        code: TeardownCodeType,
    ) {
        let setup_order = self.setup_counter;
        self.setup_counter += 1;

        let context_id = self.context_ids.get(&scope).copied().unwrap_or(0);

        let teardown_code = TeardownCode {
            fixture_id,
            fixture_name,
            scope,
            code,
            setup_order,
        };

        let pending = PendingTeardown {
            code: teardown_code,
            context_id,
        };

        self.pending_by_scope.entry(scope).or_default().push(pending);
    }

    /// Get the number of pending teardowns for a scope
    pub fn pending_count(&self, scope: FixtureScope) -> usize {
        self.pending_by_scope.get(&scope).map(|v| v.len()).unwrap_or(0)
    }

    /// Get total number of pending teardowns across all scopes
    pub fn total_pending(&self) -> usize {
        self.pending_by_scope.values().map(|v| v.len()).sum()
    }

    /// Check if there are any pending teardowns
    pub fn has_pending(&self) -> bool {
        self.pending_by_scope.values().any(|v| !v.is_empty())
    }

    /// Get pending teardowns for a scope in reverse setup order
    ///
    /// This ensures that fixtures are torn down in the reverse order they were set up,
    /// which is critical for proper dependency handling. If fixture B depends on fixture A,
    /// A is set up first, so B must be torn down first.
    fn get_pending_reversed(&mut self, scope: FixtureScope) -> Vec<TeardownCode> {
        let mut pending = self.pending_by_scope.remove(&scope).unwrap_or_default();

        // Sort by setup_order descending (reverse order)
        // This ensures dependent fixtures are torn down before their dependencies
        pending.sort_by(|a, b| b.code.setup_order.cmp(&a.code.setup_order));

        pending.into_iter().map(|p| p.code).collect()
    }

    /// Get the teardown order for fixtures with explicit dependencies
    ///
    /// This method takes into account fixture dependencies to ensure that
    /// dependent fixtures are always torn down before their dependencies.
    pub fn get_teardown_order(&self, scope: FixtureScope) -> Vec<FixtureId> {
        let pending = self.pending_by_scope.get(&scope);
        match pending {
            Some(teardowns) => {
                let mut sorted: Vec<_> = teardowns.iter().collect();
                // Sort by setup_order descending (reverse order)
                sorted.sort_by(|a, b| b.code.setup_order.cmp(&a.code.setup_order));
                sorted.into_iter().map(|p| p.code.fixture_id).collect()
            }
            None => Vec::new(),
        }
    }

    /// Execute all pending teardowns for a specific scope
    ///
    /// Teardowns are executed in reverse order of setup.
    /// Even if a teardown fails, subsequent teardowns will still be executed.
    /// All errors are collected and returned.
    pub fn execute_scope<F>(&mut self, scope: FixtureScope, mut executor: F) -> Vec<TeardownResult>
    where
        F: FnMut(&TeardownCode) -> Result<(), String>,
    {
        let teardowns = self.get_pending_reversed(scope);
        let mut results = Vec::with_capacity(teardowns.len());

        for teardown in teardowns {
            let result = match executor(&teardown) {
                Ok(()) => TeardownResult::success(teardown.fixture_id, teardown.fixture_name),
                Err(e) => TeardownResult::failure(teardown.fixture_id, teardown.fixture_name, e),
            };
            results.push(result);
        }

        results
    }

    /// Execute all pending teardowns across all scopes
    ///
    /// Scopes are processed in order: Function, Class, Module, Session.
    /// Within each scope, teardowns are executed in reverse setup order.
    pub fn execute_all<F>(&mut self, mut executor: F) -> Vec<TeardownResult>
    where
        F: FnMut(&TeardownCode) -> Result<(), String>,
    {
        let mut all_results = Vec::new();

        // Execute in scope order (function first, session last)
        // We need to collect scopes first to avoid borrow issues
        let scopes = [
            FixtureScope::Function,
            FixtureScope::Class,
            FixtureScope::Module,
            FixtureScope::Session,
        ];

        for scope in scopes {
            let teardowns = self.get_pending_reversed(scope);
            for teardown in teardowns {
                let result = match executor(&teardown) {
                    Ok(()) => TeardownResult::success(teardown.fixture_id, teardown.fixture_name),
                    Err(e) => {
                        TeardownResult::failure(teardown.fixture_id, teardown.fixture_name, e)
                    }
                };
                all_results.push(result);
            }
        }

        all_results
    }

    /// Clear all pending teardowns without executing them
    ///
    /// This should only be used in exceptional circumstances (e.g., process shutdown).
    pub fn clear(&mut self) {
        self.pending_by_scope.clear();
    }

    /// Reset the setup counter (typically at session start)
    pub fn reset_counter(&mut self) {
        self.setup_counter = 0;
    }

    /// Execute teardowns for a scope after a test failure
    ///
    /// This method ensures teardowns are executed even when the test fails.
    /// It collects all errors and returns a summary.
    pub fn execute_after_test_failure<F>(
        &mut self,
        scope: FixtureScope,
        mut executor: F,
    ) -> TeardownSummary
    where
        F: FnMut(&TeardownCode) -> Result<(), String>,
    {
        let teardowns = self.get_pending_reversed(scope);
        let mut results = Vec::with_capacity(teardowns.len());

        for teardown in teardowns {
            // Always attempt teardown, even if previous ones failed
            let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                executor(&teardown)
            })) {
                Ok(Ok(())) => TeardownResult::success(teardown.fixture_id, teardown.fixture_name),
                Ok(Err(e)) => {
                    TeardownResult::failure(teardown.fixture_id, teardown.fixture_name, e)
                }
                Err(_) => TeardownResult::failure(
                    teardown.fixture_id,
                    teardown.fixture_name,
                    "Teardown panicked".to_string(),
                ),
            };
            results.push(result);
        }

        TeardownSummary::from_results(&results)
    }

    /// Execute all teardowns with full error collection
    ///
    /// This method executes all pending teardowns across all scopes,
    /// collecting all errors even if some teardowns fail or panic.
    pub fn execute_all_with_summary<F>(&mut self, mut executor: F) -> TeardownSummary
    where
        F: FnMut(&TeardownCode) -> Result<(), String>,
    {
        let mut all_results = Vec::new();

        let scopes = [
            FixtureScope::Function,
            FixtureScope::Class,
            FixtureScope::Module,
            FixtureScope::Session,
        ];

        for scope in scopes {
            let teardowns = self.get_pending_reversed(scope);
            for teardown in teardowns {
                let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    executor(&teardown)
                })) {
                    Ok(Ok(())) => {
                        TeardownResult::success(teardown.fixture_id, teardown.fixture_name)
                    }
                    Ok(Err(e)) => {
                        TeardownResult::failure(teardown.fixture_id, teardown.fixture_name, e)
                    }
                    Err(_) => TeardownResult::failure(
                        teardown.fixture_id,
                        teardown.fixture_name,
                        "Teardown panicked".to_string(),
                    ),
                };
                all_results.push(result);
            }
        }

        TeardownSummary::from_results(&all_results)
    }

    // ========== Scope-Aware Teardown Methods ==========

    /// Called when a test function ends
    ///
    /// Executes all function-scoped fixture teardowns.
    pub fn on_test_end<F>(&mut self, executor: F) -> TeardownSummary
    where
        F: FnMut(&TeardownCode) -> Result<(), String>,
    {
        self.execute_after_test_failure(FixtureScope::Function, executor)
    }

    /// Called when a test class ends
    ///
    /// Executes all class-scoped fixture teardowns.
    pub fn on_class_end<F>(&mut self, executor: F) -> TeardownSummary
    where
        F: FnMut(&TeardownCode) -> Result<(), String>,
    {
        self.execute_after_test_failure(FixtureScope::Class, executor)
    }

    /// Called when a test module ends
    ///
    /// Executes all module-scoped fixture teardowns.
    pub fn on_module_end<F>(&mut self, executor: F) -> TeardownSummary
    where
        F: FnMut(&TeardownCode) -> Result<(), String>,
    {
        self.execute_after_test_failure(FixtureScope::Module, executor)
    }

    /// Called when the test session ends
    ///
    /// Executes all session-scoped fixture teardowns.
    pub fn on_session_end<F>(&mut self, executor: F) -> TeardownSummary
    where
        F: FnMut(&TeardownCode) -> Result<(), String>,
    {
        self.execute_after_test_failure(FixtureScope::Session, executor)
    }

    /// Get scopes that have pending teardowns
    pub fn scopes_with_pending(&self) -> Vec<FixtureScope> {
        let mut scopes = Vec::new();
        for scope in [
            FixtureScope::Function,
            FixtureScope::Class,
            FixtureScope::Module,
            FixtureScope::Session,
        ] {
            if self.pending_count(scope) > 0 {
                scopes.push(scope);
            }
        }
        scopes
    }

    /// Check if a specific scope has pending teardowns
    pub fn has_pending_for_scope(&self, scope: FixtureScope) -> bool {
        self.pending_count(scope) > 0
    }
}

impl Default for TeardownManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_teardown_manager_new() {
        let manager = TeardownManager::new();
        assert!(!manager.has_pending());
        assert_eq!(manager.total_pending(), 0);
    }

    #[test]
    fn test_register_teardown() {
        let mut manager = TeardownManager::new();

        manager.register(
            FixtureId::new(1),
            "fixture1".to_string(),
            FixtureScope::Function,
            TeardownCodeType::Inline("cleanup()".to_string()),
        );

        assert!(manager.has_pending());
        assert_eq!(manager.pending_count(FixtureScope::Function), 1);
        assert_eq!(manager.pending_count(FixtureScope::Module), 0);
    }

    #[test]
    fn test_execute_scope_reverse_order() {
        let mut manager = TeardownManager::new();

        // Register in order: 1, 2, 3
        for i in 1..=3 {
            manager.register(
                FixtureId::new(i),
                format!("fixture{}", i),
                FixtureScope::Function,
                TeardownCodeType::Inline(format!("cleanup{}", i)),
            );
        }

        let mut execution_order = Vec::new();
        let results = manager.execute_scope(FixtureScope::Function, |code| {
            execution_order.push(code.fixture_id.0);
            Ok(())
        });

        // Should execute in reverse order: 3, 2, 1
        assert_eq!(execution_order, vec![3, 2, 1]);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn test_execute_continues_on_failure() {
        let mut manager = TeardownManager::new();

        for i in 1..=3 {
            manager.register(
                FixtureId::new(i),
                format!("fixture{}", i),
                FixtureScope::Function,
                TeardownCodeType::Inline(format!("cleanup{}", i)),
            );
        }

        let mut execution_order = Vec::new();
        let results = manager.execute_scope(FixtureScope::Function, |code| {
            execution_order.push(code.fixture_id.0);
            // Fail on fixture 2
            if code.fixture_id.0 == 2 {
                Err("Teardown failed".to_string())
            } else {
                Ok(())
            }
        });

        // All three should still execute
        assert_eq!(execution_order, vec![3, 2, 1]);
        assert_eq!(results.len(), 3);

        // Check results
        assert!(results[0].success); // fixture 3
        assert!(!results[1].success); // fixture 2 failed
        assert!(results[2].success); // fixture 1
    }

    #[test]
    fn test_scope_isolation() {
        let mut manager = TeardownManager::new();

        manager.register(
            FixtureId::new(1),
            "func_fixture".to_string(),
            FixtureScope::Function,
            TeardownCodeType::Inline("cleanup_func".to_string()),
        );

        manager.register(
            FixtureId::new(2),
            "module_fixture".to_string(),
            FixtureScope::Module,
            TeardownCodeType::Inline("cleanup_module".to_string()),
        );

        assert_eq!(manager.pending_count(FixtureScope::Function), 1);
        assert_eq!(manager.pending_count(FixtureScope::Module), 1);

        // Execute only function scope
        let results = manager.execute_scope(FixtureScope::Function, |_| Ok(()));
        assert_eq!(results.len(), 1);

        // Module scope should still have pending
        assert_eq!(manager.pending_count(FixtureScope::Function), 0);
        assert_eq!(manager.pending_count(FixtureScope::Module), 1);
    }

    #[test]
    fn test_context_tracking() {
        let mut manager = TeardownManager::new();

        manager.set_context(FixtureScope::Function, 12345);
        assert_eq!(manager.get_context(FixtureScope::Function), Some(12345));
        assert_eq!(manager.get_context(FixtureScope::Module), None);
    }

    #[test]
    fn test_get_teardown_order() {
        let mut manager = TeardownManager::new();

        // Register fixtures simulating dependency order: config -> db -> app
        // Setup order: config (0), db (1), app (2)
        manager.register(
            FixtureId::new(1),
            "config".to_string(),
            FixtureScope::Function,
            TeardownCodeType::Inline("cleanup_config".to_string()),
        );
        manager.register(
            FixtureId::new(2),
            "db".to_string(),
            FixtureScope::Function,
            TeardownCodeType::Inline("cleanup_db".to_string()),
        );
        manager.register(
            FixtureId::new(3),
            "app".to_string(),
            FixtureScope::Function,
            TeardownCodeType::Inline("cleanup_app".to_string()),
        );

        // Teardown order should be reverse: app (3), db (2), config (1)
        let order = manager.get_teardown_order(FixtureScope::Function);
        assert_eq!(order, vec![FixtureId::new(3), FixtureId::new(2), FixtureId::new(1)]);
    }

    #[test]
    fn test_dependency_chain_teardown() {
        let mut manager = TeardownManager::new();

        // Simulate a dependency chain: a -> b -> c -> d
        // Setup order: d, c, b, a (dependencies first)
        let fixtures = ["d", "c", "b", "a"];
        for (i, name) in fixtures.iter().enumerate() {
            manager.register(
                FixtureId::new(i as u64 + 1),
                name.to_string(),
                FixtureScope::Function,
                TeardownCodeType::Inline(format!("cleanup_{}", name)),
            );
        }

        let mut execution_order = Vec::new();
        let results = manager.execute_scope(FixtureScope::Function, |code| {
            execution_order.push(code.fixture_name.clone());
            Ok(())
        });

        // Teardown should be reverse of setup: a, b, c, d
        assert_eq!(execution_order, vec!["a", "b", "c", "d"]);
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn test_teardown_summary() {
        let results = vec![
            TeardownResult::success(FixtureId::new(1), "fixture1".to_string()),
            TeardownResult::failure(
                FixtureId::new(2),
                "fixture2".to_string(),
                "Error 1".to_string(),
            ),
            TeardownResult::success(FixtureId::new(3), "fixture3".to_string()),
            TeardownResult::failure(
                FixtureId::new(4),
                "fixture4".to_string(),
                "Error 2".to_string(),
            ),
        ];

        let summary = TeardownSummary::from_results(&results);

        assert_eq!(summary.total, 4);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.errors.len(), 2);
        assert!(!summary.all_succeeded());
        assert!(summary.error_report().is_some());
    }

    #[test]
    fn test_teardown_summary_all_success() {
        let results = vec![
            TeardownResult::success(FixtureId::new(1), "fixture1".to_string()),
            TeardownResult::success(FixtureId::new(2), "fixture2".to_string()),
        ];

        let summary = TeardownSummary::from_results(&results);

        assert!(summary.all_succeeded());
        assert!(summary.error_report().is_none());
    }

    #[test]
    fn test_execute_after_test_failure() {
        let mut manager = TeardownManager::new();

        for i in 1..=3 {
            manager.register(
                FixtureId::new(i),
                format!("fixture{}", i),
                FixtureScope::Function,
                TeardownCodeType::Inline(format!("cleanup{}", i)),
            );
        }

        let mut execution_order = Vec::new();
        let summary = manager.execute_after_test_failure(FixtureScope::Function, |code| {
            execution_order.push(code.fixture_id.0);
            // Fail on fixture 2
            if code.fixture_id.0 == 2 {
                Err("Teardown failed".to_string())
            } else {
                Ok(())
            }
        });

        // All three should still execute
        assert_eq!(execution_order, vec![3, 2, 1]);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn test_multiple_failures_all_reported() {
        let mut manager = TeardownManager::new();

        for i in 1..=4 {
            manager.register(
                FixtureId::new(i),
                format!("fixture{}", i),
                FixtureScope::Function,
                TeardownCodeType::Inline(format!("cleanup{}", i)),
            );
        }

        let summary = manager.execute_after_test_failure(FixtureScope::Function, |code| {
            // Fail on fixtures 2 and 3
            if code.fixture_id.0 == 2 || code.fixture_id.0 == 3 {
                Err(format!("Error in fixture {}", code.fixture_id.0))
            } else {
                Ok(())
            }
        });

        assert_eq!(summary.total, 4);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.errors.len(), 2);

        // Both errors should be reported
        let error_report = summary.error_report().unwrap();
        assert!(error_report.contains("fixture3"));
        assert!(error_report.contains("fixture2"));
    }

    #[test]
    fn test_scope_aware_teardown_function() {
        let mut manager = TeardownManager::new();

        // Register fixtures in different scopes
        manager.register(
            FixtureId::new(1),
            "func_fixture".to_string(),
            FixtureScope::Function,
            TeardownCodeType::Inline("cleanup_func".to_string()),
        );
        manager.register(
            FixtureId::new(2),
            "module_fixture".to_string(),
            FixtureScope::Module,
            TeardownCodeType::Inline("cleanup_module".to_string()),
        );

        // on_test_end should only teardown function-scoped fixtures
        let summary = manager.on_test_end(|_| Ok(()));

        assert_eq!(summary.total, 1);
        assert!(!manager.has_pending_for_scope(FixtureScope::Function));
        assert!(manager.has_pending_for_scope(FixtureScope::Module));
    }

    #[test]
    fn test_scope_aware_teardown_module() {
        let mut manager = TeardownManager::new();

        manager.register(
            FixtureId::new(1),
            "module_fixture".to_string(),
            FixtureScope::Module,
            TeardownCodeType::Inline("cleanup_module".to_string()),
        );
        manager.register(
            FixtureId::new(2),
            "session_fixture".to_string(),
            FixtureScope::Session,
            TeardownCodeType::Inline("cleanup_session".to_string()),
        );

        // on_module_end should only teardown module-scoped fixtures
        let summary = manager.on_module_end(|_| Ok(()));

        assert_eq!(summary.total, 1);
        assert!(!manager.has_pending_for_scope(FixtureScope::Module));
        assert!(manager.has_pending_for_scope(FixtureScope::Session));
    }

    #[test]
    fn test_scope_aware_teardown_session() {
        let mut manager = TeardownManager::new();

        manager.register(
            FixtureId::new(1),
            "session_fixture".to_string(),
            FixtureScope::Session,
            TeardownCodeType::Inline("cleanup_session".to_string()),
        );

        // on_session_end should teardown session-scoped fixtures
        let summary = manager.on_session_end(|_| Ok(()));

        assert_eq!(summary.total, 1);
        assert!(!manager.has_pending_for_scope(FixtureScope::Session));
    }

    #[test]
    fn test_scopes_with_pending() {
        let mut manager = TeardownManager::new();

        assert!(manager.scopes_with_pending().is_empty());

        manager.register(
            FixtureId::new(1),
            "func_fixture".to_string(),
            FixtureScope::Function,
            TeardownCodeType::Inline("cleanup".to_string()),
        );
        manager.register(
            FixtureId::new(2),
            "session_fixture".to_string(),
            FixtureScope::Session,
            TeardownCodeType::Inline("cleanup".to_string()),
        );

        let scopes = manager.scopes_with_pending();
        assert_eq!(scopes.len(), 2);
        assert!(scopes.contains(&FixtureScope::Function));
        assert!(scopes.contains(&FixtureScope::Session));
        assert!(!scopes.contains(&FixtureScope::Class));
        assert!(!scopes.contains(&FixtureScope::Module));
    }

    #[test]
    fn test_full_lifecycle_teardown() {
        let mut manager = TeardownManager::new();

        // Simulate a full test lifecycle with fixtures at all scopes
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

        let mut teardown_order = Vec::new();

        // Test ends - function scope teardown
        let summary = manager.on_test_end(|code| {
            teardown_order.push(code.fixture_name.clone());
            Ok(())
        });
        assert_eq!(summary.total, 1);
        assert_eq!(teardown_order, vec!["func_request"]);

        // Class ends - class scope teardown
        let summary = manager.on_class_end(|code| {
            teardown_order.push(code.fixture_name.clone());
            Ok(())
        });
        assert_eq!(summary.total, 1);
        assert_eq!(teardown_order, vec!["func_request", "class_client"]);

        // Module ends - module scope teardown
        let summary = manager.on_module_end(|code| {
            teardown_order.push(code.fixture_name.clone());
            Ok(())
        });
        assert_eq!(summary.total, 1);
        assert_eq!(teardown_order, vec!["func_request", "class_client", "module_config"]);

        // Session ends - session scope teardown
        let summary = manager.on_session_end(|code| {
            teardown_order.push(code.fixture_name.clone());
            Ok(())
        });
        assert_eq!(summary.total, 1);
        assert_eq!(
            teardown_order,
            vec![
                "func_request",
                "class_client",
                "module_config",
                "session_db"
            ]
        );

        // All teardowns complete
        assert!(!manager.has_pending());
    }
}
