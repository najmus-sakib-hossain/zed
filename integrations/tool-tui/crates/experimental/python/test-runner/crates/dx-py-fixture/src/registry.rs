//! Fixture registry for managing discovered fixtures
//!
//! The registry stores all discovered fixtures and provides lookup by name.
//! It integrates with FixtureManager for fixture resolution and execution.

use crate::{FixtureDefinition, FixtureScope};
use dx_py_core::FixtureError;
use std::collections::HashMap;
use std::path::PathBuf;

/// Registry of all discovered fixtures
///
/// The registry stores fixtures discovered from conftest.py files and test modules.
/// It provides efficient lookup by name and supports querying fixtures by scope.
pub struct FixtureRegistry {
    /// All fixtures indexed by name
    fixtures: HashMap<String, FixtureDefinition>,
    /// Fixtures indexed by module path for scope management
    by_module: HashMap<PathBuf, Vec<String>>,
}

impl FixtureRegistry {
    /// Create a new empty fixture registry
    pub fn new() -> Self {
        Self {
            fixtures: HashMap::new(),
            by_module: HashMap::new(),
        }
    }

    /// Register a fixture definition
    ///
    /// If a fixture with the same name already exists, it will be replaced.
    /// This allows test-local fixtures to override conftest fixtures.
    pub fn register(&mut self, fixture: FixtureDefinition) {
        let name = fixture.name.clone();
        let module_path = fixture.module_path.clone();

        // Add to by_module index
        self.by_module
            .entry(module_path)
            .or_default()
            .push(name.clone());

        // Add to main registry
        self.fixtures.insert(name, fixture);
    }

    /// Register multiple fixtures at once
    pub fn register_all(&mut self, fixtures: Vec<FixtureDefinition>) {
        for fixture in fixtures {
            self.register(fixture);
        }
    }

    /// Get a fixture by name
    pub fn get(&self, name: &str) -> Option<&FixtureDefinition> {
        self.fixtures.get(name)
    }

    /// Check if a fixture exists
    pub fn contains(&self, name: &str) -> bool {
        self.fixtures.contains_key(name)
    }

    /// Get all fixtures in a module
    pub fn get_module_fixtures(&self, module_path: &PathBuf) -> Vec<&FixtureDefinition> {
        self.by_module
            .get(module_path)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.fixtures.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all autouse fixtures for a given scope
    ///
    /// Returns fixtures that have autouse=True and a scope at or above the given scope.
    pub fn get_autouse_fixtures(&self, scope: FixtureScope) -> Vec<&FixtureDefinition> {
        self.fixtures
            .values()
            .filter(|f| f.autouse && f.scope.priority() >= scope.priority())
            .collect()
    }

    /// Get all fixtures with a specific scope
    pub fn get_fixtures_by_scope(&self, scope: FixtureScope) -> Vec<&FixtureDefinition> {
        self.fixtures
            .values()
            .filter(|f| f.scope == scope)
            .collect()
    }

    /// Get all fixture names
    pub fn fixture_names(&self) -> Vec<&str> {
        self.fixtures.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of registered fixtures
    pub fn len(&self) -> usize {
        self.fixtures.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.fixtures.is_empty()
    }

    /// Clear all fixtures
    pub fn clear(&mut self) {
        self.fixtures.clear();
        self.by_module.clear();
    }

    /// Remove a fixture by name
    pub fn remove(&mut self, name: &str) -> Option<FixtureDefinition> {
        if let Some(fixture) = self.fixtures.remove(name) {
            // Remove from by_module index
            if let Some(names) = self.by_module.get_mut(&fixture.module_path) {
                names.retain(|n| n != name);
            }
            Some(fixture)
        } else {
            None
        }
    }

    /// Validate that all fixture dependencies exist
    ///
    /// Returns an error if any fixture depends on a non-existent fixture.
    pub fn validate_dependencies(&self) -> Result<(), FixtureError> {
        for fixture in self.fixtures.values() {
            for dep in &fixture.dependencies {
                if !self.fixtures.contains_key(dep) {
                    return Err(FixtureError::NotFound(format!(
                        "Fixture '{}' depends on non-existent fixture '{}'",
                        fixture.name, dep
                    )));
                }
            }
        }
        Ok(())
    }

    /// Detect circular dependencies in fixtures
    ///
    /// Returns an error if a circular dependency is detected.
    pub fn detect_circular_dependencies(&self) -> Result<(), FixtureError> {
        for fixture in self.fixtures.values() {
            let mut visited = std::collections::HashSet::new();
            let mut stack = vec![fixture.name.as_str()];

            while let Some(current) = stack.pop() {
                if !visited.insert(current) {
                    return Err(FixtureError::NotFound(format!(
                        "Circular dependency detected involving fixture '{}'",
                        current
                    )));
                }

                if let Some(current_fixture) = self.fixtures.get(current) {
                    for dep in &current_fixture.dependencies {
                        if dep == &fixture.name {
                            return Err(FixtureError::NotFound(format!(
                                "Circular dependency: '{}' -> '{}' -> '{}'",
                                fixture.name, current, dep
                            )));
                        }
                        stack.push(dep.as_str());
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for FixtureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_fixture(name: &str, module: &str) -> FixtureDefinition {
        FixtureDefinition::new(name, PathBuf::from(module), 1)
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = FixtureRegistry::new();
        let fixture = create_test_fixture("test_fixture", "test.py");

        registry.register(fixture.clone());

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test_fixture"));
        assert_eq!(registry.get("test_fixture").unwrap().name, "test_fixture");
    }

    #[test]
    fn test_register_multiple() {
        let mut registry = FixtureRegistry::new();
        let fixtures = vec![
            create_test_fixture("fixture1", "test.py"),
            create_test_fixture("fixture2", "test.py"),
            create_test_fixture("fixture3", "test.py"),
        ];

        registry.register_all(fixtures);

        assert_eq!(registry.len(), 3);
        assert!(registry.contains("fixture1"));
        assert!(registry.contains("fixture2"));
        assert!(registry.contains("fixture3"));
    }

    #[test]
    fn test_get_module_fixtures() {
        let mut registry = FixtureRegistry::new();
        let module1 = PathBuf::from("test1.py");
        let module2 = PathBuf::from("test2.py");

        registry.register(create_test_fixture("fixture1", "test1.py"));
        registry.register(create_test_fixture("fixture2", "test1.py"));
        registry.register(create_test_fixture("fixture3", "test2.py"));

        let module1_fixtures = registry.get_module_fixtures(&module1);
        assert_eq!(module1_fixtures.len(), 2);

        let module2_fixtures = registry.get_module_fixtures(&module2);
        assert_eq!(module2_fixtures.len(), 1);
    }

    #[test]
    fn test_get_autouse_fixtures() {
        let mut registry = FixtureRegistry::new();

        registry.register(
            create_test_fixture("auto1", "test.py")
                .with_autouse(true)
                .with_scope(FixtureScope::Function),
        );
        registry.register(
            create_test_fixture("auto2", "test.py")
                .with_autouse(true)
                .with_scope(FixtureScope::Module),
        );
        registry.register(create_test_fixture("normal", "test.py"));

        let autouse = registry.get_autouse_fixtures(FixtureScope::Function);
        assert_eq!(autouse.len(), 2);
    }

    #[test]
    fn test_get_fixtures_by_scope() {
        let mut registry = FixtureRegistry::new();

        registry.register(
            create_test_fixture("func1", "test.py").with_scope(FixtureScope::Function),
        );
        registry.register(
            create_test_fixture("func2", "test.py").with_scope(FixtureScope::Function),
        );
        registry.register(
            create_test_fixture("mod1", "test.py").with_scope(FixtureScope::Module),
        );

        let function_fixtures = registry.get_fixtures_by_scope(FixtureScope::Function);
        assert_eq!(function_fixtures.len(), 2);

        let module_fixtures = registry.get_fixtures_by_scope(FixtureScope::Module);
        assert_eq!(module_fixtures.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut registry = FixtureRegistry::new();
        registry.register(create_test_fixture("test_fixture", "test.py"));

        assert_eq!(registry.len(), 1);

        let removed = registry.remove("test_fixture");
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);
        assert!(!registry.contains("test_fixture"));
    }

    #[test]
    fn test_clear() {
        let mut registry = FixtureRegistry::new();
        registry.register(create_test_fixture("fixture1", "test.py"));
        registry.register(create_test_fixture("fixture2", "test.py"));

        assert_eq!(registry.len(), 2);

        registry.clear();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_validate_dependencies_success() {
        let mut registry = FixtureRegistry::new();

        registry.register(create_test_fixture("base", "test.py"));
        registry.register(
            create_test_fixture("dependent", "test.py").with_dependencies(vec!["base".to_string()]),
        );

        assert!(registry.validate_dependencies().is_ok());
    }

    #[test]
    fn test_validate_dependencies_failure() {
        let mut registry = FixtureRegistry::new();

        registry.register(
            create_test_fixture("dependent", "test.py")
                .with_dependencies(vec!["nonexistent".to_string()]),
        );

        assert!(registry.validate_dependencies().is_err());
    }

    #[test]
    fn test_detect_circular_dependencies() {
        let mut registry = FixtureRegistry::new();

        // Create a circular dependency: A -> B -> A
        registry.register(
            create_test_fixture("fixture_a", "test.py")
                .with_dependencies(vec!["fixture_b".to_string()]),
        );
        registry.register(
            create_test_fixture("fixture_b", "test.py")
                .with_dependencies(vec!["fixture_a".to_string()]),
        );

        assert!(registry.detect_circular_dependencies().is_err());
    }

    #[test]
    fn test_fixture_override() {
        let mut registry = FixtureRegistry::new();

        // Register a fixture from conftest
        registry.register(
            create_test_fixture("shared", "conftest.py").with_scope(FixtureScope::Session),
        );

        // Override with test-local fixture
        registry.register(
            create_test_fixture("shared", "test.py").with_scope(FixtureScope::Function),
        );

        // Should have the test-local version
        let fixture = registry.get("shared").unwrap();
        assert_eq!(fixture.module_path, PathBuf::from("test.py"));
        assert_eq!(fixture.scope, FixtureScope::Function);
    }
}
