//! Property-based tests for capability manifest.
//!
//! Feature: dcp-protocol, Property 12: Capability Intersection Correctness

use dcp::CapabilityManifest;
use proptest::prelude::*;

/// Strategy to generate a random CapabilityManifest
fn arb_manifest() -> impl Strategy<Value = CapabilityManifest> {
    (
        any::<u16>(),                                // version
        prop::collection::vec(any::<u16>(), 0..100), // tool_ids
        prop::collection::vec(any::<u16>(), 0..50),  // resource_ids
        prop::collection::vec(any::<u16>(), 0..30),  // prompt_ids
        any::<u64>(),                                // extensions
    )
        .prop_map(|(version, tools, resources, prompts, extensions)| {
            let mut manifest = CapabilityManifest::new(version);
            for tool_id in tools {
                if (tool_id as usize) < CapabilityManifest::MAX_TOOLS {
                    manifest.set_tool(tool_id);
                }
            }
            for resource_id in resources {
                if (resource_id as usize) < CapabilityManifest::MAX_RESOURCES {
                    manifest.set_resource(resource_id);
                }
            }
            for prompt_id in prompts {
                if (prompt_id as usize) < CapabilityManifest::MAX_PROMPTS {
                    manifest.set_prompt(prompt_id);
                }
            }
            manifest.extensions = extensions;
            manifest
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// For any two CapabilityManifests, the intersection SHALL contain exactly
    /// the capabilities present in both manifests (bitwise AND).
    /// **Validates: Requirements 9.3, 9.4**
    #[test]
    fn prop_intersection_contains_only_common_tools(
        m1 in arb_manifest(),
        m2 in arb_manifest(),
    ) {
        let result = m1.intersect(&m2);

        // Check all possible tool IDs
        for tool_id in 0..CapabilityManifest::MAX_TOOLS as u16 {
            let in_m1 = m1.has_tool(tool_id);
            let in_m2 = m2.has_tool(tool_id);
            let in_result = result.has_tool(tool_id);

            // Result should have tool iff both m1 and m2 have it
            prop_assert_eq!(
                in_result,
                in_m1 && in_m2,
                "Tool {} intersection mismatch: m1={}, m2={}, result={}",
                tool_id, in_m1, in_m2, in_result
            );
        }
    }

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// For any two CapabilityManifests, the intersection SHALL contain exactly
    /// the resources present in both manifests.
    /// **Validates: Requirements 9.3, 9.4**
    #[test]
    fn prop_intersection_contains_only_common_resources(
        m1 in arb_manifest(),
        m2 in arb_manifest(),
    ) {
        let result = m1.intersect(&m2);

        // Check all possible resource IDs
        for resource_id in 0..CapabilityManifest::MAX_RESOURCES as u16 {
            let in_m1 = m1.has_resource(resource_id);
            let in_m2 = m2.has_resource(resource_id);
            let in_result = result.has_resource(resource_id);

            prop_assert_eq!(
                in_result,
                in_m1 && in_m2,
                "Resource {} intersection mismatch: m1={}, m2={}, result={}",
                resource_id, in_m1, in_m2, in_result
            );
        }
    }

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// For any two CapabilityManifests, the intersection SHALL contain exactly
    /// the prompts present in both manifests.
    /// **Validates: Requirements 9.3, 9.4**
    #[test]
    fn prop_intersection_contains_only_common_prompts(
        m1 in arb_manifest(),
        m2 in arb_manifest(),
    ) {
        let result = m1.intersect(&m2);

        // Check all possible prompt IDs
        for prompt_id in 0..CapabilityManifest::MAX_PROMPTS as u16 {
            let in_m1 = m1.has_prompt(prompt_id);
            let in_m2 = m2.has_prompt(prompt_id);
            let in_result = result.has_prompt(prompt_id);

            prop_assert_eq!(
                in_result,
                in_m1 && in_m2,
                "Prompt {} intersection mismatch: m1={}, m2={}, result={}",
                prompt_id, in_m1, in_m2, in_result
            );
        }
    }

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// For any two CapabilityManifests, the intersection SHALL contain exactly
    /// the extensions present in both manifests.
    /// **Validates: Requirements 9.3, 9.4**
    #[test]
    fn prop_intersection_contains_only_common_extensions(
        m1 in arb_manifest(),
        m2 in arb_manifest(),
    ) {
        let result = m1.intersect(&m2);

        // Check all 64 extension bits
        for bit in 0..64u8 {
            let in_m1 = m1.has_extension(bit);
            let in_m2 = m2.has_extension(bit);
            let in_result = result.has_extension(bit);

            prop_assert_eq!(
                in_result,
                in_m1 && in_m2,
                "Extension {} intersection mismatch: m1={}, m2={}, result={}",
                bit, in_m1, in_m2, in_result
            );
        }
    }

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// Intersection SHALL be commutative: A ∩ B = B ∩ A
    /// **Validates: Requirements 9.3, 9.4**
    #[test]
    fn prop_intersection_is_commutative(
        m1 in arb_manifest(),
        m2 in arb_manifest(),
    ) {
        let result1 = m1.intersect(&m2);
        let result2 = m2.intersect(&m1);

        // Tools should be identical
        prop_assert_eq!(result1.tools, result2.tools, "Tool intersection not commutative");
        // Resources should be identical
        prop_assert_eq!(result1.resources, result2.resources, "Resource intersection not commutative");
        // Prompts should be identical
        prop_assert_eq!(result1.prompts, result2.prompts, "Prompt intersection not commutative");
        // Extensions should be identical
        prop_assert_eq!(result1.extensions, result2.extensions, "Extension intersection not commutative");
    }

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// Intersection SHALL be associative: (A ∩ B) ∩ C = A ∩ (B ∩ C)
    /// **Validates: Requirements 9.3, 9.4**
    #[test]
    fn prop_intersection_is_associative(
        m1 in arb_manifest(),
        m2 in arb_manifest(),
        m3 in arb_manifest(),
    ) {
        let result1 = m1.intersect(&m2).intersect(&m3);
        let result2 = m1.intersect(&m2.intersect(&m3));

        prop_assert_eq!(result1.tools, result2.tools, "Tool intersection not associative");
        prop_assert_eq!(result1.resources, result2.resources, "Resource intersection not associative");
        prop_assert_eq!(result1.prompts, result2.prompts, "Prompt intersection not associative");
        prop_assert_eq!(result1.extensions, result2.extensions, "Extension intersection not associative");
    }

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// Intersection with self SHALL be idempotent: A ∩ A = A
    /// **Validates: Requirements 9.3, 9.4**
    #[test]
    fn prop_intersection_is_idempotent(
        m in arb_manifest(),
    ) {
        let result = m.intersect(&m);

        prop_assert_eq!(result.tools, m.tools, "Tool intersection not idempotent");
        prop_assert_eq!(result.resources, m.resources, "Resource intersection not idempotent");
        prop_assert_eq!(result.prompts, m.prompts, "Prompt intersection not idempotent");
        prop_assert_eq!(result.extensions, m.extensions, "Extension intersection not idempotent");
    }

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// Intersection with empty manifest SHALL produce empty result.
    /// **Validates: Requirements 9.3, 9.4**
    #[test]
    fn prop_intersection_with_empty_is_empty(
        m in arb_manifest(),
    ) {
        let empty = CapabilityManifest::new(1);
        let result = m.intersect(&empty);

        prop_assert_eq!(result.tool_count(), 0, "Intersection with empty should have no tools");
        prop_assert_eq!(result.resource_count(), 0, "Intersection with empty should have no resources");
        prop_assert_eq!(result.prompt_count(), 0, "Intersection with empty should have no prompts");
        prop_assert_eq!(result.extensions, 0, "Intersection with empty should have no extensions");
    }

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// Version in intersection SHALL be the minimum of both versions.
    /// **Validates: Requirements 9.3**
    #[test]
    fn prop_intersection_version_is_minimum(
        v1 in any::<u16>(),
        v2 in any::<u16>(),
    ) {
        let m1 = CapabilityManifest::new(v1);
        let m2 = CapabilityManifest::new(v2);
        let result = m1.intersect(&m2);

        prop_assert_eq!(result.version, v1.min(v2), "Version should be minimum");
    }

    /// Feature: dcp-protocol, Property 1: Binary Struct Round-Trip (partial)
    /// For any CapabilityManifest, serializing to bytes and deserializing back
    /// SHALL produce an equivalent struct.
    /// **Validates: Requirements 9.2**
    #[test]
    fn prop_manifest_round_trip(
        m in arb_manifest(),
    ) {
        let bytes = m.as_bytes();
        let parsed = CapabilityManifest::from_bytes(bytes).unwrap();

        prop_assert_eq!(parsed.version, m.version);
        prop_assert_eq!(parsed.tools, m.tools);
        prop_assert_eq!(parsed.resources, m.resources);
        prop_assert_eq!(parsed.prompts, m.prompts);
        prop_assert_eq!(parsed.extensions, m.extensions);
    }

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// Set/has operations SHALL be consistent for all valid IDs.
    /// **Validates: Requirements 9.3, 9.4**
    #[test]
    fn prop_set_has_consistency(
        tool_ids in prop::collection::vec(0u16..8192, 0..50),
        resource_ids in prop::collection::vec(0u16..1024, 0..30),
        prompt_ids in prop::collection::vec(0u16..512, 0..20),
    ) {
        let mut manifest = CapabilityManifest::new(1);

        // Set all IDs
        for &id in &tool_ids {
            manifest.set_tool(id);
        }
        for &id in &resource_ids {
            manifest.set_resource(id);
        }
        for &id in &prompt_ids {
            manifest.set_prompt(id);
        }

        // Verify all set IDs are present
        for &id in &tool_ids {
            prop_assert!(manifest.has_tool(id), "Tool {} should be set", id);
        }
        for &id in &resource_ids {
            prop_assert!(manifest.has_resource(id), "Resource {} should be set", id);
        }
        for &id in &prompt_ids {
            prop_assert!(manifest.has_prompt(id), "Prompt {} should be set", id);
        }
    }

    /// Feature: dcp-protocol, Property 12: Capability Intersection Correctness
    /// Clear operations SHALL remove capabilities.
    /// **Validates: Requirements 9.3, 9.4**
    #[test]
    fn prop_clear_removes_capability(
        tool_ids in prop::collection::hash_set(0u16..8192, 1..20),
    ) {
        let tool_ids: Vec<_> = tool_ids.into_iter().collect();
        let mut manifest = CapabilityManifest::new(1);

        // Set all IDs
        for &id in &tool_ids {
            manifest.set_tool(id);
        }

        // Clear half of them (using indices to avoid duplicates)
        let to_clear: Vec<_> = tool_ids.iter().enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, &id)| id)
            .collect();

        for &id in &to_clear {
            manifest.clear_tool(id);
        }

        // Verify cleared IDs are gone
        for &id in &to_clear {
            prop_assert!(!manifest.has_tool(id), "Tool {} should be cleared", id);
        }

        // Verify non-cleared IDs are still present
        let kept: Vec<_> = tool_ids.iter().enumerate()
            .filter(|(i, _)| i % 2 != 0)
            .map(|(_, &id)| id)
            .collect();

        for &id in &kept {
            prop_assert!(manifest.has_tool(id), "Tool {} should still be set", id);
        }
    }
}
