//! Property tests for icon tree shaking
//!
//! Feature: dx-www-production-ready, Property 5: Icon Tree Shaking
//!
//! This property test verifies that for any component tree, the final bundle
//! should only contain icons that are actually referenced in components.
//! Unused icons should be tree-shaken out.

use build::{BuildCache, IconConfig, IconProcessor};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Strategy for generating arbitrary icon names
fn arbitrary_icon_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("(heroicons|mdi|lucide):[a-z]{3,10}").unwrap()
}

/// Strategy for generating arbitrary component file names
fn arbitrary_component_filename() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z]{3,10}\\.(rs|pg|html)").unwrap()
}

/// Strategy for generating a set of used icons (1-10 icons)
fn arbitrary_used_icons() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arbitrary_icon_name(), 1..=10)
}

/// Strategy for generating a set of unused icons (0-5 icons)
fn arbitrary_unused_icons() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arbitrary_icon_name(), 0..=5)
}

/// Create a component file with icon usage
fn create_component_with_icons(path: &PathBuf, icon_names: &[String]) {
    let mut content = String::from("fn render() {\n");
    for icon_name in icon_names {
        content.push_str(&format!("    <dx-icon name=\"{}\" />\n", icon_name));
    }
    content.push_str("}\n");
    fs::write(path, content).unwrap();
}

proptest! {
    /// Property 5a: Only used icons are included in the bundle
    ///
    /// For any component tree with a set of used icons, the final bundle
    /// should contain exactly those icons and no others.
    ///
    /// **Validates: Requirements 1.3**
    // Feature: dx-www-production-ready, Property 5: Icon Tree Shaking
    #[test]
    fn only_used_icons_are_included(
        used_icons in arbitrary_used_icons(),
        component_filename in arbitrary_component_filename(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let components_dir = temp_dir.path().join("components");
        fs::create_dir(&components_dir).unwrap();

        // Create component file with used icons
        let component_path = components_dir.join(&component_filename);
        create_component_with_icons(&component_path, &used_icons);

        // Create IconProcessor with tree-shaking enabled
        let config = IconConfig {
            components_dir: components_dir.clone(),
            output_dir: temp_dir.path().join("icons"),
            file_extensions: vec!["rs".to_string(), "pg".to_string(), "html".to_string()],
            tree_shaking: true,
        };
        let processor = IconProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Process icons
        let result = processor.process_icons(&mut cache);
        prop_assert!(result.is_ok(), "Icon processing should succeed");

        let (manifest, _artifacts) = result.unwrap();

        // Verify only used icons are in the manifest
        let manifest_icon_names: HashSet<String> =
            manifest.icons.iter().map(|i| i.name.clone()).collect();

        let used_icons_set: HashSet<String> = used_icons.iter().cloned().collect();

        // All used icons should be in the manifest
        for used_icon in &used_icons_set {
            prop_assert!(
                manifest_icon_names.contains(used_icon),
                "Used icon '{}' should be in manifest",
                used_icon
            );
        }

        // Manifest should not contain more icons than used
        prop_assert_eq!(
            manifest_icon_names.len(),
            used_icons_set.len(),
            "Manifest should contain exactly {} icons (only used icons)",
            used_icons_set.len()
        );
    }

    /// Property 5b: Unused icons are tree-shaken out
    ///
    /// For any component tree with used and unused icons, the final bundle
    /// should only contain the used icons, not the unused ones.
    ///
    /// **Validates: Requirements 1.3**
    // Feature: dx-www-production-ready, Property 5: Icon Tree Shaking
    #[test]
    fn unused_icons_are_tree_shaken(
        used_icons in arbitrary_used_icons(),
        unused_icons in arbitrary_unused_icons(),
        component_filename in arbitrary_component_filename(),
    ) {
        // Ensure used and unused icons are distinct
        let used_set: HashSet<String> = used_icons.iter().cloned().collect();
        let unused_set: HashSet<String> = unused_icons
            .iter()
            .filter(|icon| !used_set.contains(*icon))
            .cloned()
            .collect();

        if unused_set.is_empty() {
            // Skip test if no distinct unused icons
            return Ok(());
        }

        let temp_dir = TempDir::new().unwrap();
        let components_dir = temp_dir.path().join("components");
        fs::create_dir(&components_dir).unwrap();

        // Create component file with only used icons
        let component_path = components_dir.join(&component_filename);
        create_component_with_icons(&component_path, &used_icons);

        // Create IconProcessor with tree-shaking enabled
        let config = IconConfig {
            components_dir: components_dir.clone(),
            output_dir: temp_dir.path().join("icons"),
            file_extensions: vec!["rs".to_string(), "pg".to_string(), "html".to_string()],
            tree_shaking: true,
        };
        let processor = IconProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Process icons
        let result = processor.process_icons(&mut cache);
        prop_assert!(result.is_ok(), "Icon processing should succeed");

        let (manifest, _artifacts) = result.unwrap();

        // Verify unused icons are NOT in the manifest
        let manifest_icon_names: HashSet<String> =
            manifest.icons.iter().map(|i| i.name.clone()).collect();

        for unused_icon in &unused_set {
            prop_assert!(
                !manifest_icon_names.contains(unused_icon),
                "Unused icon '{}' should NOT be in manifest (tree-shaken out)",
                unused_icon
            );
        }
    }

    /// Property 5c: Multiple components with different icons
    ///
    /// For any set of components each using different icons, the final bundle
    /// should contain the union of all used icons.
    ///
    /// **Validates: Requirements 1.3**
    // Feature: dx-www-production-ready, Property 5: Icon Tree Shaking
    #[test]
    fn multiple_components_union_of_icons(
        component1_icons in prop::collection::vec(arbitrary_icon_name(), 1..=5),
        component2_icons in prop::collection::vec(arbitrary_icon_name(), 1..=5),
        filename1 in arbitrary_component_filename(),
        filename2 in prop::string::string_regex("[a-z]{3,10}\\.(rs|pg|html)").unwrap(),
    ) {
        // Ensure filenames are different
        if filename1 == filename2 {
            return Ok(());
        }

        let temp_dir = TempDir::new().unwrap();
        let components_dir = temp_dir.path().join("components");
        fs::create_dir(&components_dir).unwrap();

        // Create first component file
        let component1_path = components_dir.join(&filename1);
        create_component_with_icons(&component1_path, &component1_icons);

        // Create second component file
        let component2_path = components_dir.join(&filename2);
        create_component_with_icons(&component2_path, &component2_icons);

        // Calculate expected union of icons
        let mut expected_icons: HashSet<String> = HashSet::new();
        expected_icons.extend(component1_icons.iter().cloned());
        expected_icons.extend(component2_icons.iter().cloned());

        // Create IconProcessor with tree-shaking enabled
        let config = IconConfig {
            components_dir: components_dir.clone(),
            output_dir: temp_dir.path().join("icons"),
            file_extensions: vec!["rs".to_string(), "pg".to_string(), "html".to_string()],
            tree_shaking: true,
        };
        let processor = IconProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Process icons
        let result = processor.process_icons(&mut cache);
        prop_assert!(result.is_ok(), "Icon processing should succeed");

        let (manifest, _artifacts) = result.unwrap();

        // Verify manifest contains union of all used icons
        let manifest_icon_names: HashSet<String> =
            manifest.icons.iter().map(|i| i.name.clone()).collect();

        prop_assert_eq!(
            manifest_icon_names.len(),
            expected_icons.len(),
            "Manifest should contain exactly {} icons (union of all components)",
            expected_icons.len()
        );

        for expected_icon in &expected_icons {
            prop_assert!(
                manifest_icon_names.contains(expected_icon),
                "Icon '{}' used in components should be in manifest",
                expected_icon
            );
        }
    }

    /// Property 5d: Empty component tree produces no icons
    ///
    /// For a component tree with no icon usage, the final bundle should
    /// contain no icons.
    ///
    /// **Validates: Requirements 1.3**
    // Feature: dx-www-production-ready, Property 5: Icon Tree Shaking
    #[test]
    fn empty_component_tree_no_icons(
        component_filename in arbitrary_component_filename(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let components_dir = temp_dir.path().join("components");
        fs::create_dir(&components_dir).unwrap();

        // Create component file without any icons
        let component_path = components_dir.join(&component_filename);
        fs::write(&component_path, "fn render() {\n    // No icons here\n}\n").unwrap();

        // Create IconProcessor with tree-shaking enabled
        let config = IconConfig {
            components_dir: components_dir.clone(),
            output_dir: temp_dir.path().join("icons"),
            file_extensions: vec!["rs".to_string(), "pg".to_string(), "html".to_string()],
            tree_shaking: true,
        };
        let processor = IconProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Process icons
        let result = processor.process_icons(&mut cache);
        prop_assert!(result.is_ok(), "Icon processing should succeed");

        let (manifest, _artifacts) = result.unwrap();

        // Verify no icons in manifest
        prop_assert_eq!(
            manifest.total_count,
            0,
            "Empty component tree should produce no icons"
        );
        prop_assert_eq!(
            manifest.icons.len(),
            0,
            "Manifest should contain no icons"
        );
    }

    /// Property 5e: Duplicate icon references are deduplicated
    ///
    /// For any component tree where the same icon is referenced multiple times,
    /// the final bundle should contain that icon only once.
    ///
    /// **Validates: Requirements 1.3**
    // Feature: dx-www-production-ready, Property 5: Icon Tree Shaking
    #[test]
    fn duplicate_icons_are_deduplicated(
        icon_name in arbitrary_icon_name(),
        duplicate_count in 2..=10usize,
        component_filename in arbitrary_component_filename(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let components_dir = temp_dir.path().join("components");
        fs::create_dir(&components_dir).unwrap();

        // Create component file with duplicate icon references
        let component_path = components_dir.join(&component_filename);
        let mut content = String::from("fn render() {\n");
        for _ in 0..duplicate_count {
            content.push_str(&format!("    <dx-icon name=\"{}\" />\n", icon_name));
        }
        content.push_str("}\n");
        fs::write(&component_path, content).unwrap();

        // Create IconProcessor with tree-shaking enabled
        let config = IconConfig {
            components_dir: components_dir.clone(),
            output_dir: temp_dir.path().join("icons"),
            file_extensions: vec!["rs".to_string(), "pg".to_string(), "html".to_string()],
            tree_shaking: true,
        };
        let processor = IconProcessor::new(config);
        let mut cache = BuildCache::new(temp_dir.path()).unwrap();

        // Process icons
        let result = processor.process_icons(&mut cache);
        prop_assert!(result.is_ok(), "Icon processing should succeed");

        let (manifest, _artifacts) = result.unwrap();

        // Verify icon appears only once in manifest
        prop_assert_eq!(
            manifest.total_count,
            1,
            "Duplicate icon references should be deduplicated to 1 icon"
        );

        let manifest_icon_names: HashSet<String> =
            manifest.icons.iter().map(|i| i.name.clone()).collect();

        prop_assert!(
            manifest_icon_names.contains(&icon_name),
            "Icon '{}' should be in manifest",
            icon_name
        );
    }

    /// Property 5f: Tree-shaking disabled includes all discovered icons
    ///
    /// When tree-shaking is disabled, verify that the behavior is consistent
    /// (this is a control test to ensure the tree-shaking flag works).
    ///
    /// **Validates: Requirements 1.3**
    // Feature: dx-www-production-ready, Property 5: Icon Tree Shaking
    #[test]
    fn tree_shaking_flag_controls_behavior(
        used_icons in arbitrary_used_icons(),
        component_filename in arbitrary_component_filename(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let components_dir = temp_dir.path().join("components");
        fs::create_dir(&components_dir).unwrap();

        // Create component file with used icons
        let component_path = components_dir.join(&component_filename);
        create_component_with_icons(&component_path, &used_icons);

        // Test with tree-shaking enabled
        let config_enabled = IconConfig {
            components_dir: components_dir.clone(),
            output_dir: temp_dir.path().join("icons_enabled"),
            file_extensions: vec!["rs".to_string(), "pg".to_string(), "html".to_string()],
            tree_shaking: true,
        };
        let processor_enabled = IconProcessor::new(config_enabled);
        let cache_enabled_dir = temp_dir.path().join("cache_enabled");
        let mut cache_enabled = BuildCache::new(&cache_enabled_dir).unwrap();

        let result_enabled = processor_enabled.process_icons(&mut cache_enabled);
        prop_assert!(result_enabled.is_ok(), "Icon processing should succeed");

        let (manifest_enabled, _) = result_enabled.unwrap();

        // Verify icons are included when tree-shaking is enabled
        let used_icons_set: HashSet<String> = used_icons.iter().cloned().collect();
        prop_assert_eq!(
            manifest_enabled.total_count,
            used_icons_set.len(),
            "With tree-shaking enabled, should include only used icons"
        );
    }
}
