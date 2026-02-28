//! Property-based tests for the module system
//!
//! These tests validate the correctness properties defined in the design document.

#[cfg(test)]
mod tests {
    use super::super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    // Generators for property-based testing

    fn arb_module_id() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{2,20}".prop_map(|s| s.to_string())
    }

    fn arb_version() -> impl Strategy<Value = String> {
        (0u8..10, 0u8..20, 0u8..100)
            .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
    }

    fn arb_module() -> impl Strategy<Value = Module> {
        (arb_module_id(), "[A-Z][a-zA-Z ]{2,30}", arb_version(), "[a-zA-Z0-9 .,!]{0,100}").prop_map(
            |(id, name, version, description)| {
                Module::new(id, name, version).with_description(description)
            },
        )
    }

    fn arb_module_with_agents() -> impl Strategy<Value = Module> {
        (arb_module(), prop::collection::vec("[a-z-]{3,15}", 0..5)).prop_map(
            |(mut module, agents)| {
                for agent in agents {
                    module = module.with_agent(agent);
                }
                module
            },
        )
    }

    fn arb_module_with_workflows() -> impl Strategy<Value = Module> {
        (arb_module(), prop::collection::vec("[a-z-]{3,15}", 0..5)).prop_map(
            |(mut module, workflows)| {
                for workflow in workflows {
                    module = module.with_workflow(workflow);
                }
                module
            },
        )
    }

    fn arb_dependency() -> impl Strategy<Value = ModuleDependency> {
        (arb_module_id(), prop::bool::ANY).prop_map(|(id, optional)| {
            if optional {
                ModuleDependency::optional(id, "*")
            } else {
                ModuleDependency::new(id, "*")
            }
        })
    }

    // Property 26: Module Installation Isolation
    // For any installed module, its resources SHALL be isolated and not conflict
    // with other modules or core functionality.
    // **Validates: Requirements 9.7**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_module_isolation_namespacing(
            module1 in arb_module_with_agents(),
            module2 in arb_module_with_agents(),
        ) {
            // Skip if modules have the same ID (would be a conflict anyway)
            prop_assume!(module1.id != module2.id);

            let mut manager = ModuleManager::new("/tmp/test-isolation");
            manager.enable_isolation();

            // Simulate having both modules installed
            let agents1: Vec<String> = module1.agents.iter()
                .map(|a| module1.namespaced_agent(a))
                .collect();
            let agents2: Vec<String> = module2.agents.iter()
                .map(|a| module2.namespaced_agent(a))
                .collect();

            // With isolation enabled, namespaced agents should never conflict
            // even if they have the same base name
            let set1: HashSet<_> = agents1.iter().collect();
            let set2: HashSet<_> = agents2.iter().collect();

            // Intersection should be empty (no conflicts)
            let intersection: Vec<_> = set1.intersection(&set2).collect();
            prop_assert!(
                intersection.is_empty(),
                "Namespaced agents should not conflict: {:?}",
                intersection
            );
        }

        #[test]
        fn prop_module_isolation_workflows(
            module1 in arb_module_with_workflows(),
            module2 in arb_module_with_workflows(),
        ) {
            prop_assume!(module1.id != module2.id);

            // Namespaced workflows should never conflict
            let workflows1: Vec<String> = module1.workflows.iter()
                .map(|w| module1.namespaced_workflow(w))
                .collect();
            let workflows2: Vec<String> = module2.workflows.iter()
                .map(|w| module2.namespaced_workflow(w))
                .collect();

            let set1: HashSet<_> = workflows1.iter().collect();
            let set2: HashSet<_> = workflows2.iter().collect();

            let intersection: Vec<_> = set1.intersection(&set2).collect();
            prop_assert!(
                intersection.is_empty(),
                "Namespaced workflows should not conflict: {:?}",
                intersection
            );
        }

        #[test]
        fn prop_module_namespace_format(module in arb_module()) {
            // Namespaced IDs should always follow the format "module_id:resource_name"
            let agent_name = "test-agent";
            let namespaced = module.namespaced_agent(agent_name);

            prop_assert!(namespaced.contains(':'), "Namespaced ID should contain ':'");
            prop_assert!(
                namespaced.starts_with(&module.id),
                "Namespaced ID should start with module ID"
            );
            prop_assert!(
                namespaced.ends_with(agent_name),
                "Namespaced ID should end with resource name"
            );

            // Should be able to split back into module ID and resource name
            let parts: Vec<&str> = namespaced.split(':').collect();
            prop_assert_eq!(parts.len(), 2);
            prop_assert_eq!(parts[0], module.id);
            prop_assert_eq!(parts[1], agent_name);
        }
    }

    // Property 27: Module Dependency Resolution
    // For any module with dependencies, all dependencies SHALL be resolved
    // and installed before the module.
    // **Validates: Requirements 9.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_dependency_resolution_order(
            base_module in arb_module(),
            dep_module in arb_module(),
        ) {
            prop_assume!(base_module.id != dep_module.id);

            let mut manager = ModuleManager::new("/tmp/test-deps");

            // Create a module that depends on dep_module
            let dependent = Module::new(
                format!("{}-dependent", base_module.id),
                "Dependent Module",
                "1.0.0",
            )
            .with_dependency(ModuleDependency::new(&dep_module.id, "*"));

            // Without the dependency installed, resolution should fail
            let result = manager.resolve_dependencies(&dependent);
            prop_assert!(
                result.is_err(),
                "Should fail when required dependency is not installed"
            );

            // Install the dependency
            manager.installed.insert(dep_module.id.clone(), dep_module.clone());
            manager.status.insert(dep_module.id.clone(), ModuleStatus::Installed);

            // Now resolution should succeed
            let result = manager.resolve_dependencies(&dependent);
            prop_assert!(result.is_ok(), "Should succeed when dependency is installed");

            let resolved = result.unwrap();
            prop_assert!(
                resolved.iter().any(|m| m.id == dep_module.id),
                "Resolved dependencies should include the required module"
            );
        }

        #[test]
        fn prop_optional_dependency_resolution(
            base_module in arb_module(),
            opt_dep_id in arb_module_id(),
        ) {
            prop_assume!(base_module.id != opt_dep_id);

            let manager = ModuleManager::new("/tmp/test-opt-deps");

            // Create a module with an optional dependency
            let module_with_opt = Module::new(
                format!("{}-with-opt", base_module.id),
                "Module with Optional Dep",
                "1.0.0",
            )
            .with_dependency(ModuleDependency::optional(&opt_dep_id, "*"));

            // Optional dependency not installed should still resolve successfully
            let result = manager.resolve_dependencies(&module_with_opt);
            prop_assert!(
                result.is_ok(),
                "Should succeed even when optional dependency is not installed"
            );
        }

        #[test]
        fn prop_version_satisfaction(
            major in 0u8..10,
            minor in 0u8..20,
            patch in 0u8..100,
        ) {
            let version = format!("{}.{}.{}", major, minor, patch);
            let module = Module::new("test", "Test", &version);

            // Exact version should always match
            prop_assert!(module.satisfies_version(&version));

            // Wildcard should always match
            prop_assert!(module.satisfies_version("*"));

            // Caret (^) should match same major version
            let caret_req = format!("^{}", major);
            prop_assert!(module.satisfies_version(&caret_req));

            // Tilde (~) should match same major.minor
            let tilde_req = format!("~{}.{}", major, minor);
            prop_assert!(module.satisfies_version(&tilde_req));

            // Different major version should not match with caret
            if major < 9 {
                let different_major = format!("^{}", major + 1);
                prop_assert!(!module.satisfies_version(&different_major));
            }
        }

        #[test]
        fn prop_transitive_dependency_resolution(
            module_a in arb_module(),
            module_b in arb_module(),
            module_c in arb_module(),
        ) {
            // Ensure all modules have different IDs
            prop_assume!(module_a.id != module_b.id);
            prop_assume!(module_b.id != module_c.id);
            prop_assume!(module_a.id != module_c.id);

            let mut manager = ModuleManager::new("/tmp/test-transitive");

            // C depends on nothing
            let c = module_c.clone();

            // B depends on C
            let b = Module::new(&module_b.id, &module_b.name, &module_b.version)
                .with_dependency(ModuleDependency::new(&c.id, "*"));

            // A depends on B
            let a = Module::new(&module_a.id, &module_a.name, &module_a.version)
                .with_dependency(ModuleDependency::new(&b.id, "*"));

            // Install C and B
            manager.installed.insert(c.id.clone(), c.clone());
            manager.status.insert(c.id.clone(), ModuleStatus::Installed);
            manager.installed.insert(b.id.clone(), b.clone());
            manager.status.insert(b.id.clone(), ModuleStatus::Installed);

            // Resolve A's dependencies
            let result = manager.resolve_dependencies(&a);
            prop_assert!(result.is_ok(), "Transitive resolution should succeed");

            let resolved = result.unwrap();
            // Should include both B and C (transitive)
            prop_assert!(
                resolved.iter().any(|m| m.id == b.id),
                "Should include direct dependency B"
            );
            prop_assert!(
                resolved.iter().any(|m| m.id == c.id),
                "Should include transitive dependency C"
            );
        }
    }

    // Additional unit tests for edge cases

    #[test]
    fn test_circular_dependency_detection() {
        let mut manager = ModuleManager::new("/tmp/test-circular");

        // Create modules with circular dependency
        let module_a = Module::new("module-a", "Module A", "1.0.0")
            .with_dependency(ModuleDependency::new("module-b", "*"));

        let module_b = Module::new("module-b", "Module B", "1.0.0")
            .with_dependency(ModuleDependency::new("module-a", "*"));

        // Install both (simulating they were installed somehow)
        manager.installed.insert("module-a".to_string(), module_a.clone());
        manager.installed.insert("module-b".to_string(), module_b.clone());
        manager.status.insert("module-a".to_string(), ModuleStatus::Installed);
        manager.status.insert("module-b".to_string(), ModuleStatus::Installed);

        // Resolution should handle circular deps without infinite loop
        // (the visited set prevents this)
        let result = manager.resolve_dependencies(&module_a);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dx_manifest_parsing() {
        let manager = ModuleManager::new("/tmp/test-parse");

        let manifest = r#"
# Test Module Manifest
id|test-module
nm|Test Module
v|1.2.3
desc|A test module for testing
author|Test Author
license|MIT
dep.other-module|^1.0.0
agent.0|test-agent
workflow.0|test-workflow
template.0|test-template
resource.0|test-resource
"#;

        let module = manager.parse_dx_manifest(manifest).unwrap();

        assert_eq!(module.id, "test-module");
        assert_eq!(module.name, "Test Module");
        assert_eq!(module.version, "1.2.3");
        assert_eq!(module.description, "A test module for testing");
        assert_eq!(module.author, Some("Test Author".to_string()));
        assert_eq!(module.license, Some("MIT".to_string()));
        assert_eq!(module.dependencies.len(), 1);
        assert_eq!(module.dependencies[0].module_id, "other-module");
        assert_eq!(module.agents, vec!["test-agent"]);
        assert_eq!(module.workflows, vec!["test-workflow"]);
        assert_eq!(module.templates, vec!["test-template"]);
        assert_eq!(module.resources, vec!["test-resource"]);
    }

    #[test]
    fn test_manifest_missing_id_fails() {
        let manager = ModuleManager::new("/tmp/test-missing-id");

        let manifest = r#"
nm|Test Module
v|1.0.0
"#;

        let result = manager.parse_dx_manifest(manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_manifest_defaults() {
        let manager = ModuleManager::new("/tmp/test-defaults");

        let manifest = "id|minimal-module";

        let module = manager.parse_dx_manifest(manifest).unwrap();

        assert_eq!(module.id, "minimal-module");
        assert_eq!(module.name, "minimal-module"); // Defaults to ID
        assert_eq!(module.version, "0.0.0"); // Default version
    }
}
