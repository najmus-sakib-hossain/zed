//! Property-based tests for workspace management
//!
//! **Property 10: Workspace Member Enumeration**
//! **Property 13: Workspace Build Order**
//! **Validates: Requirements 6.3.1-6.3.7**

use proptest::prelude::*;
use std::collections::HashSet;
use tempfile::TempDir;

/// Generate valid package names (PEP 503 normalized)
fn package_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,15}"
}

/// Generate a list of package names
fn package_names_strategy(min: usize, max: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(package_name_strategy(), min..=max).prop_filter("unique names", |names| {
        let unique: HashSet<_> = names.iter().collect();
        unique.len() == names.len()
    })
}

/// Create a minimal pyproject.toml for a package
fn create_pyproject(name: &str, version: &str) -> String {
    format!(
        r#"[project]
name = "{}"
version = "{}"
"#,
        name, version
    )
}

proptest! {
    /// Property 10: Workspace Member Enumeration
    /// *For any* workspace configuration with glob patterns, the enumerated members
    /// SHALL match exactly the directories that match the patterns.
    #[test]
    fn prop_workspace_enumeration_matches_glob_patterns(
        package_names in package_names_strategy(1, 5)
    ) {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create workspace pyproject.toml with glob pattern
        let workspace_pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
"#;
        std::fs::write(root.join("pyproject.toml"), workspace_pyproject).unwrap();

        // Create packages directory
        let packages_dir = root.join("packages");
        std::fs::create_dir_all(&packages_dir).unwrap();

        // Create each package
        for name in &package_names {
            let pkg_dir = packages_dir.join(name);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            std::fs::write(
                pkg_dir.join("pyproject.toml"),
                create_pyproject(name, "0.1.0"),
            ).unwrap();
        }

        // Load workspace and enumerate members
        let manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
        let member_paths = manager.enumerate_members().unwrap();

        // Verify: enumerated members should match created packages
        prop_assert_eq!(
            member_paths.len(),
            package_names.len(),
            "Number of enumerated members should match number of created packages"
        );

        // Verify each package is found
        let enumerated_names: HashSet<String> = member_paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        for name in &package_names {
            prop_assert!(
                enumerated_names.contains(name),
                "Package '{}' should be enumerated", name
            );
        }
    }

    /// Property 10: Excluded patterns are respected
    #[test]
    fn prop_workspace_enumeration_respects_exclusions(
        included_names in package_names_strategy(1, 3),
        excluded_names in package_names_strategy(1, 2)
    ) {
        // Ensure no overlap between included and excluded
        let included_set: HashSet<_> = included_names.iter().collect();
        let excluded_set: HashSet<_> = excluded_names.iter().collect();
        prop_assume!(included_set.is_disjoint(&excluded_set));

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Build exclude patterns
        let exclude_patterns: Vec<String> = excluded_names
            .iter()
            .map(|n| format!("packages/{}", n))
            .collect();
        let exclude_toml = exclude_patterns
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(", ");

        // Create workspace pyproject.toml with exclusions
        let workspace_pyproject = format!(
            r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
exclude = [{}]
"#,
            exclude_toml
        );
        std::fs::write(root.join("pyproject.toml"), workspace_pyproject).unwrap();

        // Create packages directory
        let packages_dir = root.join("packages");
        std::fs::create_dir_all(&packages_dir).unwrap();

        // Create included packages
        for name in &included_names {
            let pkg_dir = packages_dir.join(name);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            std::fs::write(
                pkg_dir.join("pyproject.toml"),
                create_pyproject(name, "0.1.0"),
            ).unwrap();
        }

        // Create excluded packages
        for name in &excluded_names {
            let pkg_dir = packages_dir.join(name);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            std::fs::write(
                pkg_dir.join("pyproject.toml"),
                create_pyproject(name, "0.1.0"),
            ).unwrap();
        }

        // Load workspace and enumerate members
        let manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
        let member_paths = manager.enumerate_members().unwrap();

        // Verify: only included packages should be enumerated
        let enumerated_names: HashSet<String> = member_paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        for name in &included_names {
            prop_assert!(
                enumerated_names.contains(name),
                "Included package '{}' should be enumerated", name
            );
        }

        for name in &excluded_names {
            prop_assert!(
                !enumerated_names.contains(name),
                "Excluded package '{}' should NOT be enumerated", name
            );
        }
    }

    /// Property 10: Multiple glob patterns are combined correctly
    #[test]
    fn prop_workspace_multiple_patterns_combined(
        packages_names in package_names_strategy(1, 2),
        libs_names in package_names_strategy(1, 2)
    ) {
        // Ensure no overlap
        let packages_set: HashSet<_> = packages_names.iter().collect();
        let libs_set: HashSet<_> = libs_names.iter().collect();
        prop_assume!(packages_set.is_disjoint(&libs_set));

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create workspace with multiple patterns
        let workspace_pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*", "libs/*"]
"#;
        std::fs::write(root.join("pyproject.toml"), workspace_pyproject).unwrap();

        // Create packages
        let packages_dir = root.join("packages");
        std::fs::create_dir_all(&packages_dir).unwrap();
        for name in &packages_names {
            let pkg_dir = packages_dir.join(name);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            std::fs::write(
                pkg_dir.join("pyproject.toml"),
                create_pyproject(name, "0.1.0"),
            ).unwrap();
        }

        // Create libs
        let libs_dir = root.join("libs");
        std::fs::create_dir_all(&libs_dir).unwrap();
        for name in &libs_names {
            let lib_dir = libs_dir.join(name);
            std::fs::create_dir_all(&lib_dir).unwrap();
            std::fs::write(
                lib_dir.join("pyproject.toml"),
                create_pyproject(name, "0.1.0"),
            ).unwrap();
        }

        // Load workspace and enumerate members
        let manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
        let member_paths = manager.enumerate_members().unwrap();

        // Verify: all packages from both patterns should be enumerated
        let expected_count = packages_names.len() + libs_names.len();
        prop_assert_eq!(
            member_paths.len(),
            expected_count,
            "Should enumerate all packages from all patterns"
        );
    }

    /// Property 10: Directories without pyproject.toml are not enumerated
    #[test]
    fn prop_workspace_only_enumerates_projects(
        project_names in package_names_strategy(1, 2),
        non_project_names in package_names_strategy(1, 2)
    ) {
        // Ensure no overlap
        let project_set: HashSet<_> = project_names.iter().collect();
        let non_project_set: HashSet<_> = non_project_names.iter().collect();
        prop_assume!(project_set.is_disjoint(&non_project_set));

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create workspace
        let workspace_pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
"#;
        std::fs::write(root.join("pyproject.toml"), workspace_pyproject).unwrap();

        let packages_dir = root.join("packages");
        std::fs::create_dir_all(&packages_dir).unwrap();

        // Create valid projects
        for name in &project_names {
            let pkg_dir = packages_dir.join(name);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            std::fs::write(
                pkg_dir.join("pyproject.toml"),
                create_pyproject(name, "0.1.0"),
            ).unwrap();
        }

        // Create directories without pyproject.toml
        for name in &non_project_names {
            let dir = packages_dir.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            // No pyproject.toml!
        }

        // Load workspace and enumerate members
        let manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
        let member_paths = manager.enumerate_members().unwrap();

        // Verify: only directories with pyproject.toml should be enumerated
        prop_assert_eq!(
            member_paths.len(),
            project_names.len(),
            "Should only enumerate directories with pyproject.toml"
        );

        let enumerated_names: HashSet<String> = member_paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        for name in &non_project_names {
            prop_assert!(
                !enumerated_names.contains(name),
                "Directory '{}' without pyproject.toml should NOT be enumerated", name
            );
        }
    }
}

#[test]
fn test_empty_workspace_returns_root_if_project() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a simple project (not a workspace)
    let pyproject = r#"
[project]
name = "simple-project"
version = "1.0.0"
"#;
    std::fs::write(root.join("pyproject.toml"), pyproject).unwrap();

    let manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
    let member_paths = manager.enumerate_members().unwrap();

    assert_eq!(member_paths.len(), 1);
    assert_eq!(member_paths[0], root);
}

#[test]
fn test_workspace_is_workspace_flag() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create workspace
    let workspace_pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
"#;
    std::fs::write(root.join("pyproject.toml"), workspace_pyproject).unwrap();

    let manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
    assert!(manager.is_workspace());
}

#[test]
fn test_non_workspace_is_workspace_flag() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create simple project
    let pyproject = r#"
[project]
name = "simple-project"
version = "1.0.0"
"#;
    std::fs::write(root.join("pyproject.toml"), pyproject).unwrap();

    let manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
    assert!(!manager.is_workspace());
}

// ============================================================================
// Property 13: Workspace Build Order Tests
// Validates: Requirements 6.3.4, 6.3.5
// ============================================================================

/// Create a pyproject.toml with path dependencies
fn create_pyproject_with_deps(name: &str, version: &str, path_deps: &[&str]) -> String {
    let deps_str = if path_deps.is_empty() {
        String::new()
    } else {
        let deps: Vec<String> =
            path_deps.iter().map(|d| format!("\"{} @ file://../{}\"", d, d)).collect();
        format!("dependencies = [{}]", deps.join(", "))
    };

    format!(
        r#"[project]
name = "{}"
version = "{}"
{}
"#,
        name, version, deps_str
    )
}

proptest! {
    /// Property 13.1: Build order respects dependencies
    /// *For any* workspace with inter-member dependencies, the sorted build order
    /// SHALL place dependencies before dependents.
    #[test]
    fn prop_build_order_respects_dependencies(
        base_name in package_name_strategy()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create workspace with A -> B -> C dependency chain
        let pkg_a = format!("{}-a", base_name);
        let pkg_b = format!("{}-b", base_name);
        let pkg_c = format!("{}-c", base_name);

        let workspace_pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
"#;
        std::fs::write(root.join("pyproject.toml"), workspace_pyproject).unwrap();

        let packages_dir = root.join("packages");
        std::fs::create_dir_all(&packages_dir).unwrap();

        // C has no dependencies
        let c_dir = packages_dir.join(&pkg_c);
        std::fs::create_dir_all(&c_dir).unwrap();
        std::fs::write(
            c_dir.join("pyproject.toml"),
            create_pyproject_with_deps(&pkg_c, "0.1.0", &[]),
        ).unwrap();

        // B depends on C
        let b_dir = packages_dir.join(&pkg_b);
        std::fs::create_dir_all(&b_dir).unwrap();
        std::fs::write(
            b_dir.join("pyproject.toml"),
            create_pyproject_with_deps(&pkg_b, "0.1.0", &[&pkg_c]),
        ).unwrap();

        // A depends on B
        let a_dir = packages_dir.join(&pkg_a);
        std::fs::create_dir_all(&a_dir).unwrap();
        std::fs::write(
            a_dir.join("pyproject.toml"),
            create_pyproject_with_deps(&pkg_a, "0.1.0", &[&pkg_b]),
        ).unwrap();

        // Load workspace and get sorted members
        let mut manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
        let sorted = manager.sorted_members().unwrap();

        // Find positions in sorted order
        let pos_a = sorted.iter().position(|m| m.name == pkg_a);
        let pos_b = sorted.iter().position(|m| m.name == pkg_b);
        let pos_c = sorted.iter().position(|m| m.name == pkg_c);

        // Verify: C should come before B, B should come before A
        if let (Some(pa), Some(pb), Some(pc)) = (pos_a, pos_b, pos_c) {
            prop_assert!(pc < pb, "C should be built before B (C at {}, B at {})", pc, pb);
            prop_assert!(pb < pa, "B should be built before A (B at {}, A at {})", pb, pa);
        }
    }

    /// Property 13.2: Build order is deterministic
    /// *For any* workspace configuration, multiple calls to sorted_members
    /// SHALL return the same order.
    #[test]
    fn prop_build_order_is_deterministic(
        package_names in package_names_strategy(2, 4)
    ) {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let workspace_pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
"#;
        std::fs::write(root.join("pyproject.toml"), workspace_pyproject).unwrap();

        let packages_dir = root.join("packages");
        std::fs::create_dir_all(&packages_dir).unwrap();

        // Create packages without dependencies
        for name in &package_names {
            let pkg_dir = packages_dir.join(name);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            std::fs::write(
                pkg_dir.join("pyproject.toml"),
                create_pyproject(name, "0.1.0"),
            ).unwrap();
        }

        // Get sorted order multiple times
        let mut manager1 = dx_py_workspace::WorkspaceManager::load(root).unwrap();
        let sorted1: Vec<String> = manager1.sorted_members().unwrap()
            .iter()
            .map(|m| m.name.clone())
            .collect();

        let mut manager2 = dx_py_workspace::WorkspaceManager::load(root).unwrap();
        let sorted2: Vec<String> = manager2.sorted_members().unwrap()
            .iter()
            .map(|m| m.name.clone())
            .collect();

        // Verify: both calls should return the same order
        prop_assert_eq!(sorted1, sorted2, "Build order should be deterministic");
    }

    /// Property 13.3: All members are included in build order
    /// *For any* workspace, sorted_members SHALL include all workspace members.
    #[test]
    fn prop_build_order_includes_all_members(
        package_names in package_names_strategy(1, 5)
    ) {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let workspace_pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
"#;
        std::fs::write(root.join("pyproject.toml"), workspace_pyproject).unwrap();

        let packages_dir = root.join("packages");
        std::fs::create_dir_all(&packages_dir).unwrap();

        for name in &package_names {
            let pkg_dir = packages_dir.join(name);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            std::fs::write(
                pkg_dir.join("pyproject.toml"),
                create_pyproject(name, "0.1.0"),
            ).unwrap();
        }

        let mut manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
        let sorted = manager.sorted_members().unwrap();
        let sorted_names: HashSet<String> = sorted.iter().map(|m| m.name.clone()).collect();

        // Verify: all packages should be in the sorted list
        prop_assert_eq!(
            sorted_names.len(),
            package_names.len(),
            "All members should be in build order"
        );

        for name in &package_names {
            prop_assert!(
                sorted_names.contains(name),
                "Package '{}' should be in build order", name
            );
        }
    }

    /// Property 13.4: Diamond dependencies are handled correctly
    /// *For any* diamond dependency pattern (A -> B, A -> C, B -> D, C -> D),
    /// D SHALL be built before B and C, which SHALL be built before A.
    #[test]
    fn prop_build_order_handles_diamond_dependencies(
        base_name in package_name_strategy()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create diamond: A -> B -> D, A -> C -> D
        let pkg_a = format!("{}-a", base_name);
        let pkg_b = format!("{}-b", base_name);
        let pkg_c = format!("{}-c", base_name);
        let pkg_d = format!("{}-d", base_name);

        let workspace_pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
"#;
        std::fs::write(root.join("pyproject.toml"), workspace_pyproject).unwrap();

        let packages_dir = root.join("packages");
        std::fs::create_dir_all(&packages_dir).unwrap();

        // D has no dependencies (base of diamond)
        let d_dir = packages_dir.join(&pkg_d);
        std::fs::create_dir_all(&d_dir).unwrap();
        std::fs::write(
            d_dir.join("pyproject.toml"),
            create_pyproject_with_deps(&pkg_d, "0.1.0", &[]),
        ).unwrap();

        // B depends on D
        let b_dir = packages_dir.join(&pkg_b);
        std::fs::create_dir_all(&b_dir).unwrap();
        std::fs::write(
            b_dir.join("pyproject.toml"),
            create_pyproject_with_deps(&pkg_b, "0.1.0", &[&pkg_d]),
        ).unwrap();

        // C depends on D
        let c_dir = packages_dir.join(&pkg_c);
        std::fs::create_dir_all(&c_dir).unwrap();
        std::fs::write(
            c_dir.join("pyproject.toml"),
            create_pyproject_with_deps(&pkg_c, "0.1.0", &[&pkg_d]),
        ).unwrap();

        // A depends on B and C (top of diamond)
        let a_dir = packages_dir.join(&pkg_a);
        std::fs::create_dir_all(&a_dir).unwrap();
        std::fs::write(
            a_dir.join("pyproject.toml"),
            create_pyproject_with_deps(&pkg_a, "0.1.0", &[&pkg_b, &pkg_c]),
        ).unwrap();

        let mut manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
        let sorted = manager.sorted_members().unwrap();

        let pos_a = sorted.iter().position(|m| m.name == pkg_a);
        let pos_b = sorted.iter().position(|m| m.name == pkg_b);
        let pos_c = sorted.iter().position(|m| m.name == pkg_c);
        let pos_d = sorted.iter().position(|m| m.name == pkg_d);

        if let (Some(pa), Some(pb), Some(pc), Some(pd)) = (pos_a, pos_b, pos_c, pos_d) {
            // D should come before B and C
            prop_assert!(pd < pb, "D should be built before B");
            prop_assert!(pd < pc, "D should be built before C");
            // B and C should come before A
            prop_assert!(pb < pa, "B should be built before A");
            prop_assert!(pc < pa, "C should be built before A");
        }
    }
}

#[test]
fn test_sorted_members_simple_chain() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let workspace_pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
"#;
    std::fs::write(root.join("pyproject.toml"), workspace_pyproject).unwrap();

    let packages_dir = root.join("packages");
    std::fs::create_dir_all(&packages_dir).unwrap();

    // Create core (no deps)
    let core_dir = packages_dir.join("core");
    std::fs::create_dir_all(&core_dir).unwrap();
    std::fs::write(
        core_dir.join("pyproject.toml"),
        create_pyproject_with_deps("core", "0.1.0", &[]),
    )
    .unwrap();

    // Create utils (depends on core)
    let utils_dir = packages_dir.join("utils");
    std::fs::create_dir_all(&utils_dir).unwrap();
    std::fs::write(
        utils_dir.join("pyproject.toml"),
        create_pyproject_with_deps("utils", "0.1.0", &["core"]),
    )
    .unwrap();

    // Create app (depends on utils)
    let app_dir = packages_dir.join("app");
    std::fs::create_dir_all(&app_dir).unwrap();
    std::fs::write(
        app_dir.join("pyproject.toml"),
        create_pyproject_with_deps("app", "0.1.0", &["utils"]),
    )
    .unwrap();

    let mut manager = dx_py_workspace::WorkspaceManager::load(root).unwrap();
    let sorted = manager.sorted_members().unwrap();
    let names: Vec<&str> = sorted.iter().map(|m| m.name.as_str()).collect();

    // core should come first, then utils, then app
    let pos_core = names.iter().position(|&n| n == "core").unwrap();
    let pos_utils = names.iter().position(|&n| n == "utils").unwrap();
    let pos_app = names.iter().position(|&n| n == "app").unwrap();

    assert!(pos_core < pos_utils, "core should be before utils");
    assert!(pos_utils < pos_app, "utils should be before app");
}
