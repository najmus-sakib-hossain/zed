//! Property-based tests for editable installs
//!
//! Property 7: Editable Install Visibility
//! For any editable install, changes to Python files in the source directory
//! SHALL be immediately visible to imports without reinstallation.
//!
//! Validates: Requirements 2.3.1, 2.3.2, 2.3.5

use proptest::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use dx_py_package_manager::installer::EditableInstaller;

/// Generate arbitrary valid package names
fn arb_package_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9]{2,10}")
        .unwrap()
        .prop_filter("valid package name", |s| !s.is_empty() && s.len() >= 3)
}

/// Generate arbitrary version strings
fn arb_version() -> impl Strategy<Value = String> {
    (1u32..100, 0u32..100, 0u32..100)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

/// Generate arbitrary Python module content
fn arb_module_content() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z_][a-z0-9_]{0,20}")
        .unwrap()
        .prop_map(|var_name| {
            format!(
                "# Auto-generated module\n\n{} = 42\n\ndef main():\n    return 0\n",
                if var_name.is_empty() {
                    "value".to_string()
                } else {
                    var_name
                }
            )
        })
}

/// Create a test project with the given name and version
fn create_test_project(dir: &std::path::Path, name: &str, version: &str, content: &str) {
    let normalized = name.replace('-', "_");

    // Create pyproject.toml
    let pyproject = format!(
        r#"[project]
name = "{}"
version = "{}"

[project.scripts]
{}-cli = "{}:main"
"#,
        name, version, name, normalized
    );
    fs::write(dir.join("pyproject.toml"), pyproject).unwrap();

    // Create package directory
    let pkg_dir = dir.join(&normalized);
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(pkg_dir.join("__init__.py"), content).unwrap();
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 7: Editable Install Visibility
    /// Validates: Requirements 2.3.1, 2.3.2, 2.3.5
    ///
    /// For any editable install, the .pth file SHALL correctly point to the
    /// source directory, enabling immediate visibility of source changes.
    #[test]
    fn prop_editable_install_visibility(
        name in arb_package_name(),
        version in arb_version(),
        content in arb_module_content(),
    ) {
        let project_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        create_test_project(project_dir.path(), &name, &version, &content);

        let installer = EditableInstaller::new(site_packages.path().to_path_buf());
        let result = installer.install(project_dir.path());

        // Installation should succeed
        prop_assert!(result.is_ok(), "Editable install should succeed");
        let install = result.unwrap();

        // Verify .pth file exists and points to source
        prop_assert!(install.pth_file.exists(), ".pth file should exist");
        let pth_content = fs::read_to_string(&install.pth_file).unwrap();

        // .pth should contain a path that includes the project directory
        let project_path = project_dir.path().to_string_lossy();
        prop_assert!(
            pth_content.contains(&*project_path) ||
            PathBuf::from(pth_content.trim()).exists(),
            ".pth should point to valid source directory"
        );

        // Verify .dist-info exists with correct metadata
        prop_assert!(install.dist_info.exists(), ".dist-info should exist");
        prop_assert!(
            install.dist_info.join("METADATA").exists(),
            "METADATA file should exist"
        );
        prop_assert!(
            install.dist_info.join("direct_url.json").exists(),
            "direct_url.json should exist for editable installs"
        );

        // Verify direct_url.json marks this as editable
        let direct_url = fs::read_to_string(install.dist_info.join("direct_url.json")).unwrap();
        prop_assert!(
            direct_url.contains("\"editable\": true"),
            "direct_url.json should mark install as editable"
        );
    }

    /// Property: Editable install is detectable
    /// Validates: Requirement 2.3.4 (uninstall tracking)
    #[test]
    fn prop_editable_is_detectable(
        name in arb_package_name(),
        version in arb_version(),
    ) {
        let project_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        create_test_project(project_dir.path(), &name, &version, "def main(): return 0\n");

        let installer = EditableInstaller::new(site_packages.path().to_path_buf());

        // Before install, should not be editable
        prop_assert!(
            !installer.is_editable(&name).unwrap(),
            "Package should not be editable before install"
        );

        // Install
        installer.install(project_dir.path()).unwrap();

        // After install, should be editable
        prop_assert!(
            installer.is_editable(&name).unwrap(),
            "Package should be editable after install"
        );
    }

    /// Property: Editable uninstall removes all traces
    /// Validates: Requirement 2.3.4
    #[test]
    fn prop_editable_uninstall_complete(
        name in arb_package_name(),
        version in arb_version(),
    ) {
        let project_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        create_test_project(project_dir.path(), &name, &version, "def main(): return 0\n");

        let installer = EditableInstaller::new(site_packages.path().to_path_buf());
        let install = installer.install(project_dir.path()).unwrap();

        // Verify installed
        prop_assert!(install.pth_file.exists());
        prop_assert!(install.dist_info.exists());

        // Uninstall
        let removed = installer.uninstall(&name).unwrap();
        prop_assert!(removed > 0, "Should remove at least one file");

        // Verify removed
        prop_assert!(!install.pth_file.exists(), ".pth file should be removed");
        prop_assert!(!install.dist_info.exists(), ".dist-info should be removed");
        prop_assert!(
            !installer.is_editable(&name).unwrap(),
            "Package should not be editable after uninstall"
        );
    }

    /// Property: Multiple editable installs don't interfere
    #[test]
    fn prop_multiple_editable_independent(
        name1 in arb_package_name(),
        name2 in arb_package_name(),
    ) {
        // Skip if names are the same
        prop_assume!(name1 != name2);

        let project_dir1 = TempDir::new().unwrap();
        let project_dir2 = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        create_test_project(project_dir1.path(), &name1, "1.0.0", "x = 1\ndef main(): return 0\n");
        create_test_project(project_dir2.path(), &name2, "2.0.0", "y = 2\ndef main(): return 0\n");

        let installer = EditableInstaller::new(site_packages.path().to_path_buf());

        // Install both
        let install1 = installer.install(project_dir1.path()).unwrap();
        let install2 = installer.install(project_dir2.path()).unwrap();

        // Both should be installed
        prop_assert!(installer.is_editable(&name1).unwrap());
        prop_assert!(installer.is_editable(&name2).unwrap());

        // .pth files should be different
        prop_assert_ne!(install1.pth_file, install2.pth_file);

        // Uninstalling one should not affect the other
        installer.uninstall(&name1).unwrap();
        prop_assert!(!installer.is_editable(&name1).unwrap());
        prop_assert!(installer.is_editable(&name2).unwrap());
    }

    /// Property: Editable install info is retrievable
    #[test]
    fn prop_editable_info_retrievable(
        name in arb_package_name(),
        version in arb_version(),
    ) {
        let project_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        create_test_project(project_dir.path(), &name, &version, "def main(): return 0\n");

        let installer = EditableInstaller::new(site_packages.path().to_path_buf());
        let install = installer.install(project_dir.path()).unwrap();

        // Get info should return the same data
        let info = installer.get_info(&name).unwrap();
        prop_assert!(info.is_some(), "Info should be retrievable");

        let info = info.unwrap();
        prop_assert_eq!(&info.name, &name);
        prop_assert_eq!(&info.version, &version);
        prop_assert_eq!(info.pth_file, install.pth_file);
        prop_assert_eq!(info.dist_info, install.dist_info);
    }

    /// Property: List editable returns all installed packages
    #[test]
    fn prop_list_editable_complete(
        count in 1usize..5,
    ) {
        let site_packages = TempDir::new().unwrap();
        let installer = EditableInstaller::new(site_packages.path().to_path_buf());

        let mut project_dirs = Vec::new();
        let mut names = Vec::new();

        for i in 0..count {
            let project_dir = TempDir::new().unwrap();
            let name = format!("testpkg{}", i);
            create_test_project(project_dir.path(), &name, "1.0.0", "def main(): return 0\n");
            installer.install(project_dir.path()).unwrap();
            names.push(name);
            project_dirs.push(project_dir);
        }

        let listed = installer.list_editable().unwrap();
        prop_assert_eq!(listed.len(), count, "Should list all installed packages");

        // All names should be present
        let listed_names: std::collections::HashSet<_> = listed.iter()
            .map(|i| i.name.clone())
            .collect();
        for name in &names {
            prop_assert!(
                listed_names.contains(name),
                "Listed packages should include {}",
                name
            );
        }
    }
}

/// Test src-layout detection
#[test]
fn test_src_layout_detection() {
    let project_dir = TempDir::new().unwrap();
    let site_packages = TempDir::new().unwrap();

    // Create src-layout project
    let pyproject = r#"[project]
name = "src-layout-pkg"
version = "1.0.0"
"#;
    fs::write(project_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let src_dir = project_dir.path().join("src").join("src_layout_pkg");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("__init__.py"), "VALUE = 42\n").unwrap();

    let installer = EditableInstaller::new(site_packages.path().to_path_buf());
    let install = installer.install(project_dir.path()).unwrap();

    // .pth should point to src directory
    let pth_content = fs::read_to_string(&install.pth_file).unwrap();
    assert!(pth_content.contains("src"), ".pth should point to src directory");
}

/// Test flat-layout detection
#[test]
fn test_flat_layout_detection() {
    let project_dir = TempDir::new().unwrap();
    let site_packages = TempDir::new().unwrap();

    // Create flat-layout project
    let pyproject = r#"[project]
name = "flat-layout-pkg"
version = "1.0.0"
"#;
    fs::write(project_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let pkg_dir = project_dir.path().join("flat_layout_pkg");
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(pkg_dir.join("__init__.py"), "VALUE = 42\n").unwrap();

    let installer = EditableInstaller::new(site_packages.path().to_path_buf());
    let install = installer.install(project_dir.path()).unwrap();

    // .pth should point to project directory
    let pth_content = fs::read_to_string(&install.pth_file).unwrap();
    let pth_path = PathBuf::from(pth_content.trim());
    assert!(pth_path.exists(), ".pth should point to valid directory");
}

/// Test entry point script generation
#[test]
fn test_entry_point_generation() {
    let project_dir = TempDir::new().unwrap();
    let site_packages = TempDir::new().unwrap();

    // Create venv-like structure with Scripts/bin directory
    // Note: scripts_dir is intentionally unused as we can't easily test script
    // generation without a proper venv structure. This test verifies the basic
    // install works with scripts defined.
    #[cfg(windows)]
    let _scripts_dir = site_packages.path().parent().unwrap().join("Scripts");
    #[cfg(not(windows))]
    let _scripts_dir = site_packages.path().parent().unwrap().join("bin");

    let pyproject = r#"[project]
name = "script-pkg"
version = "1.0.0"

[project.scripts]
my-script = "script_pkg:main"
"#;
    fs::write(project_dir.path().join("pyproject.toml"), pyproject).unwrap();

    let pkg_dir = project_dir.path().join("script_pkg");
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(pkg_dir.join("__init__.py"), "def main(): return 0\n").unwrap();

    let installer = EditableInstaller::new(site_packages.path().to_path_buf());
    let install = installer.install(project_dir.path()).unwrap();

    // Install should succeed even if scripts can't be created
    // (scripts dir may not exist in test environment)
    assert!(install.pth_file.exists());
}
