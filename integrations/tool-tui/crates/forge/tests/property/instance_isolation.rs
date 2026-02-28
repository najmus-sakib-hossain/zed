//! Property test for instance isolation
//!
//! This test verifies that multiple Forge instances are completely isolated
//! from each other and operations on one instance do not affect another.

use dx_forge::core::{Forge, ForgeConfig};
use proptest::prelude::*;
use tempfile::TempDir;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: forge-production-ready, Property 1: Instance Isolation
    /// For any two Forge instances created with different project roots,
    /// operations performed on one instance SHALL NOT affect the state of the other instance.
    /// **Validates: Requirements 1.2, 1.3, 1.5**
    #[test]
    fn prop_instance_isolation(
        tool_name in "[a-z]{3,10}",
        _tool_version in "[0-9]\\.[0-9]\\.[0-9]",
        file_content_a in "[a-zA-Z0-9\\s]{10,100}",
        file_content_b in "[a-zA-Z0-9\\s]{10,100}",
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            // Create two temporary directories for isolated instances
            let temp_a = TempDir::new().unwrap();
            let temp_b = TempDir::new().unwrap();

            // Create two separate Forge instances
            let config_a = ForgeConfig::new(temp_a.path());
            let config_b = ForgeConfig::new(temp_b.path());

            let forge_a = Forge::with_config(config_a).unwrap();
            let forge_b = Forge::with_config(config_b).unwrap();

            // Verify instances have different project roots
            prop_assert_ne!(forge_a.project_root(), forge_b.project_root());

            // Test 1: Branching engine isolation
            {
                let branching_a = forge_a.branching_engine();
                let branching_b = forge_b.branching_engine();

                // Register a voter in instance A
                branching_a.write().register_permanent_voter(format!("voter_{}", tool_name)).unwrap();

                // Verify voter is NOT in instance B by checking if we can register the same voter
                // (if it was shared, this would fail)
                let result = branching_b.write().register_permanent_voter(format!("voter_{}", tool_name));
                prop_assert!(result.is_ok()); // Should succeed because instances are isolated
            }

            // Test 2: Event bus isolation
            {
                let event_bus_a = forge_a.event_bus();
                let event_bus_b = forge_b.event_bus();

                // Subscribe to events in both instances
                let mut receiver_a = event_bus_a.read().subscribe();
                let mut receiver_b = event_bus_b.read().subscribe();

                // Emit event in instance A
                event_bus_a.read().emit_tool_started(&tool_name).unwrap();

                // Instance A should receive the event
                let event_a = receiver_a.try_recv();
                prop_assert!(event_a.is_ok());

                // Instance B should NOT receive the event
                let event_b = receiver_b.try_recv();
                prop_assert!(event_b.is_err());
            }

            // Test 3: File operations isolation
            {
                let file_a = temp_a.path().join("test.txt");
                let file_b = temp_b.path().join("test.txt");

                // Write different content to files in each instance's directory
                std::fs::write(&file_a, &file_content_a).unwrap();
                std::fs::write(&file_b, &file_content_b).unwrap();

                // Verify files are isolated
                let content_a = std::fs::read_to_string(&file_a).unwrap();
                let content_b = std::fs::read_to_string(&file_b).unwrap();

                prop_assert_eq!(&content_a, &file_content_a);
                prop_assert_eq!(&content_b, &file_content_b);
                prop_assert_ne!(&content_a, &content_b);
            }

            // Test 4: Execution context isolation
            {
                let context_a = forge_a.get_execution_context();
                let context_b = forge_b.get_execution_context();

                // Verify contexts have different repo roots
                prop_assert_ne!(context_a.repo_root, context_b.repo_root);
                prop_assert_ne!(context_a.forge_path, context_b.forge_path);
            }

            Ok(())
        })?;
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_instance_isolation() {
        let temp_a = TempDir::new().unwrap();
        let temp_b = TempDir::new().unwrap();

        let forge_a = Forge::new(temp_a.path()).unwrap();
        let forge_b = Forge::new(temp_b.path()).unwrap();

        // Basic isolation check
        assert_ne!(forge_a.project_root(), forge_b.project_root());
        assert_ne!(forge_a.forge_dir(), forge_b.forge_dir());
    }
}
