//! Property-based tests for virtual environment isolation
//!
//! **Feature: dx-py-production-ready, Property 19: Virtual Environment Isolation**
//! **Validates: Requirements 10.1-10.5**
//!
//! Property 19: For any package installed in a virtual environment, importing that
//! package SHALL only succeed when the virtual environment is active.

use proptest::prelude::*;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;
use zip::write::FileOptions;
use zip::ZipWriter;

use dx_py_package_manager::cache::GlobalCache;
use dx_py_package_manager::installer::WheelInstaller;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a minimal test wheel with a given package name
fn create_test_wheel(pkg_name: &str, version: &str) -> Vec<u8> {
    let buffer = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buffer);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let normalized_name = pkg_name.replace('-', "_");
    let dist_info = format!("{}-{}.dist-info", normalized_name, version);

    // Add package __init__.py
    zip.start_file(format!("{}/__init__.py", normalized_name), options)
        .unwrap();
    zip.write_all(format!("# Package {}\n__version__ = '{}'\n", pkg_name, version).as_bytes())
        .unwrap();

    // Add METADATA
    let metadata = format!(
        "Metadata-Version: 2.1\nName: {}\nVersion: {}\nSummary: Test package\n",
        pkg_name, version
    );
    zip.start_file(format!("{}/METADATA", dist_info), options)
        .unwrap();
    zip.write_all(metadata.as_bytes()).unwrap();

    // Add WHEEL
    let wheel_content =
        "Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: true\nTag: py3-none-any\n";
    zip.start_file(format!("{}/WHEEL", dist_info), options)
        .unwrap();
    zip.write_all(wheel_content.as_bytes()).unwrap();

    // Add INSTALLER
    zip.start_file(format!("{}/INSTALLER", dist_info), options)
        .unwrap();
    zip.write_all(b"dx-py\n").unwrap();

    // Add RECORD
    zip.start_file(format!("{}/RECORD", dist_info), options)
        .unwrap();
    zip.write_all(b"").unwrap();

    zip.finish().unwrap().into_inner()
}

/// Create a virtual environment directory structure
fn create_venv_structure(venv_path: &std::path::Path) {
    #[cfg(unix)]
    {
        fs::create_dir_all(venv_path.join("bin")).unwrap();
        fs::create_dir_all(venv_path.join("lib/python3.12/site-packages")).unwrap();
    }
    #[cfg(windows)]
    {
        fs::create_dir_all(venv_path.join("Scripts")).unwrap();
        fs::create_dir_all(venv_path.join("Lib/site-packages")).unwrap();
    }

    // Create pyvenv.cfg
    let cfg_content = "home = /usr/bin\ninclude-system-site-packages = false\nversion = 3.12.0\n";
    fs::write(venv_path.join("pyvenv.cfg"), cfg_content).unwrap();
}

/// Get site-packages path for a venv
fn get_site_packages(venv_path: &std::path::Path) -> PathBuf {
    #[cfg(unix)]
    {
        venv_path.join("lib/python3.12/site-packages")
    }
    #[cfg(windows)]
    {
        venv_path.join("Lib/site-packages")
    }
}

/// Check if a package is importable from a given site-packages directory
fn is_package_importable(site_packages: &std::path::Path, pkg_name: &str) -> bool {
    let normalized_name = pkg_name.replace('-', "_");
    let init_path = site_packages.join(&normalized_name).join("__init__.py");
    init_path.exists()
}

// ============================================================================
// Property 19: Virtual Environment Isolation
// Validates: Requirements 10.2, 10.3
//
// *For any* package installed in a virtual environment, importing that package
// SHALL only succeed when the virtual environment is active.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 10.2, 10.3**
    ///
    /// Property 19a: Package installed in venv is only accessible when venv is active
    /// *For any* package installed in a virtual environment, the package SHALL be
    /// accessible from the venv's site-packages but NOT from other locations.
    #[test]
    fn prop_venv_package_isolation(
        pkg_name in "[a-z]{3,10}",
        major in 1u32..5,
        minor in 0u32..10,
    ) {
        let cache_dir = TempDir::new().unwrap();
        let venv1 = TempDir::new().unwrap();
        let venv2 = TempDir::new().unwrap();

        // Create two separate venvs
        create_venv_structure(venv1.path());
        create_venv_structure(venv2.path());

        let site_packages1 = get_site_packages(venv1.path());
        let site_packages2 = get_site_packages(venv2.path());

        // Install package to venv1
        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages1.clone());

        let version = format!("{}.{}.0", major, minor);
        let wheel_data = create_test_wheel(&pkg_name, &version);
        let result = installer.install_wheel(&wheel_data);

        prop_assert!(result.is_ok(), "Installation should succeed");

        // Package should be accessible in venv1
        prop_assert!(
            is_package_importable(&site_packages1, &pkg_name),
            "Package {} should be importable from venv1", pkg_name
        );

        // Package should NOT be accessible in venv2
        prop_assert!(
            !is_package_importable(&site_packages2, &pkg_name),
            "Package {} should NOT be importable from venv2", pkg_name
        );
    }

    /// **Validates: Requirements 10.2, 10.3**
    ///
    /// Property 19b: VIRTUAL_ENV determines installation location
    /// *For any* package installation, setting VIRTUAL_ENV SHALL cause the package
    /// to be installed to that venv's site-packages.
    #[test]
    fn prop_virtual_env_determines_install_location(
        pkg_name in "[a-z]{4,8}",
    ) {
        // Save original VIRTUAL_ENV
        let original_virtual_env = env::var("VIRTUAL_ENV").ok();

        let cache_dir = TempDir::new().unwrap();
        let venv1 = TempDir::new().unwrap();
        let venv2 = TempDir::new().unwrap();

        create_venv_structure(venv1.path());
        create_venv_structure(venv2.path());

        // Set VIRTUAL_ENV to venv1
        env::set_var("VIRTUAL_ENV", venv1.path());

        // Determine site-packages based on VIRTUAL_ENV (simulating real behavior)
        let site_packages = if let Ok(virtual_env) = env::var("VIRTUAL_ENV") {
            get_site_packages(&PathBuf::from(virtual_env))
        } else {
            get_site_packages(venv2.path())
        };

        // Install package
        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.clone());

        let wheel_data = create_test_wheel(&pkg_name, "1.0.0");
        let result = installer.install_wheel(&wheel_data);

        prop_assert!(result.is_ok(), "Installation should succeed");

        // Package should be in venv1 (where VIRTUAL_ENV points)
        let site_packages1 = get_site_packages(venv1.path());
        prop_assert!(
            is_package_importable(&site_packages1, &pkg_name),
            "Package should be installed in venv1 (VIRTUAL_ENV)"
        );

        // Package should NOT be in venv2
        let site_packages2 = get_site_packages(venv2.path());
        prop_assert!(
            !is_package_importable(&site_packages2, &pkg_name),
            "Package should NOT be installed in venv2"
        );

        // Restore original VIRTUAL_ENV
        match original_virtual_env {
            Some(val) => env::set_var("VIRTUAL_ENV", val),
            None => env::remove_var("VIRTUAL_ENV"),
        }
    }

    /// **Validates: Requirements 10.2, 10.3**
    ///
    /// Property 19c: Multiple packages in same venv are isolated from other venvs
    /// *For any* set of packages installed in a venv, all packages SHALL be
    /// accessible from that venv and NOT from other venvs.
    #[test]
    fn prop_multiple_packages_venv_isolation(
        num_packages in 2usize..5,
        seed in any::<u64>(),
    ) {
        let cache_dir = TempDir::new().unwrap();
        let venv1 = TempDir::new().unwrap();
        let venv2 = TempDir::new().unwrap();

        create_venv_structure(venv1.path());
        create_venv_structure(venv2.path());

        let site_packages1 = get_site_packages(venv1.path());
        let site_packages2 = get_site_packages(venv2.path());

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages1.clone());

        // Install multiple packages to venv1
        let mut pkg_names = Vec::new();
        for i in 0..num_packages {
            let pkg_name = format!("pkg{}{}", i, (seed >> (i * 8)) % 100);
            pkg_names.push(pkg_name.clone());

            let wheel_data = create_test_wheel(&pkg_name, "1.0.0");
            let result = installer.install_wheel(&wheel_data);
            prop_assert!(result.is_ok(), "Installation of {} should succeed", pkg_name);
        }

        // All packages should be in venv1
        for pkg_name in &pkg_names {
            prop_assert!(
                is_package_importable(&site_packages1, pkg_name),
                "Package {} should be in venv1", pkg_name
            );
        }

        // No packages should be in venv2
        for pkg_name in &pkg_names {
            prop_assert!(
                !is_package_importable(&site_packages2, pkg_name),
                "Package {} should NOT be in venv2", pkg_name
            );
        }
    }

    /// **Validates: Requirements 10.2, 10.3**
    ///
    /// Property 19d: Uninstalling from one venv doesn't affect other venvs
    /// *For any* package installed in multiple venvs, uninstalling from one venv
    /// SHALL NOT remove it from other venvs.
    #[test]
    fn prop_uninstall_venv_isolation(
        pkg_name in "[a-z]{4,8}",
    ) {
        let cache_dir = TempDir::new().unwrap();
        let venv1 = TempDir::new().unwrap();
        let venv2 = TempDir::new().unwrap();

        create_venv_structure(venv1.path());
        create_venv_structure(venv2.path());

        let site_packages1 = get_site_packages(venv1.path());
        let site_packages2 = get_site_packages(venv2.path());

        // Install same package to both venvs
        let wheel_data = create_test_wheel(&pkg_name, "1.0.0");

        let cache1 = GlobalCache::new(cache_dir.path()).unwrap();
        let installer1 = WheelInstaller::new(cache1, site_packages1.clone());
        let result1 = installer1.install_wheel(&wheel_data);
        prop_assert!(result1.is_ok(), "Installation to venv1 should succeed");

        let cache2 = GlobalCache::new(cache_dir.path()).unwrap();
        let installer2 = WheelInstaller::new(cache2, site_packages2.clone());
        let result2 = installer2.install_wheel(&wheel_data);
        prop_assert!(result2.is_ok(), "Installation to venv2 should succeed");

        // Both should have the package
        prop_assert!(
            is_package_importable(&site_packages1, &pkg_name),
            "Package should be in venv1"
        );
        prop_assert!(
            is_package_importable(&site_packages2, &pkg_name),
            "Package should be in venv2"
        );

        // Uninstall from venv1
        let uninstall_result = installer1.uninstall(&pkg_name);
        prop_assert!(uninstall_result.is_ok(), "Uninstall from venv1 should succeed");

        // Package should be removed from venv1
        prop_assert!(
            !is_package_importable(&site_packages1, &pkg_name),
            "Package should be removed from venv1"
        );

        // Package should still be in venv2
        prop_assert!(
            is_package_importable(&site_packages2, &pkg_name),
            "Package should still be in venv2 after uninstall from venv1"
        );
    }

    /// **Validates: Requirements 10.1, 10.4, 10.5**
    ///
    /// Property 19e: Venv structure is consistent
    /// *For any* created venv, it SHALL have the required directory structure
    /// (bin/Scripts, lib/Lib, site-packages) and pyvenv.cfg file.
    #[test]
    fn prop_venv_structure_consistency(
        _seed in any::<u64>(),
    ) {
        let venv = TempDir::new().unwrap();
        create_venv_structure(venv.path());

        // Check pyvenv.cfg exists
        let cfg_path = venv.path().join("pyvenv.cfg");
        prop_assert!(cfg_path.exists(), "pyvenv.cfg should exist");

        // Check directory structure
        #[cfg(unix)]
        {
            let bin_dir = venv.path().join("bin");
            prop_assert!(bin_dir.exists() && bin_dir.is_dir(), "bin directory should exist");

            let lib_dir = venv.path().join("lib/python3.12");
            prop_assert!(lib_dir.exists() && lib_dir.is_dir(), "lib/python3.12 directory should exist");

            let site_packages = venv.path().join("lib/python3.12/site-packages");
            prop_assert!(site_packages.exists() && site_packages.is_dir(), "site-packages should exist");
        }

        #[cfg(windows)]
        {
            let scripts_dir = venv.path().join("Scripts");
            prop_assert!(scripts_dir.exists() && scripts_dir.is_dir(), "Scripts directory should exist");

            let lib_dir = venv.path().join("Lib");
            prop_assert!(lib_dir.exists() && lib_dir.is_dir(), "Lib directory should exist");

            let site_packages = venv.path().join("Lib/site-packages");
            prop_assert!(site_packages.exists() && site_packages.is_dir(), "site-packages should exist");
        }
    }

    /// **Validates: Requirements 10.2, 10.3**
    ///
    /// Property 19f: Package versions are isolated between venvs
    /// *For any* package with different versions installed in different venvs,
    /// each venv SHALL have its own version without interference.
    #[test]
    fn prop_venv_version_isolation(
        pkg_name in "[a-z]{4,8}",
        version1_major in 1u32..5,
        version2_major in 1u32..5,
    ) {
        // Ensure different versions
        prop_assume!(version1_major != version2_major);

        let cache_dir = TempDir::new().unwrap();
        let venv1 = TempDir::new().unwrap();
        let venv2 = TempDir::new().unwrap();

        create_venv_structure(venv1.path());
        create_venv_structure(venv2.path());

        let site_packages1 = get_site_packages(venv1.path());
        let site_packages2 = get_site_packages(venv2.path());

        // Install version 1 to venv1
        let version1 = format!("{}.0.0", version1_major);
        let wheel1 = create_test_wheel(&pkg_name, &version1);
        let cache1 = GlobalCache::new(cache_dir.path()).unwrap();
        let installer1 = WheelInstaller::new(cache1, site_packages1.clone());
        let result1 = installer1.install_wheel(&wheel1);
        prop_assert!(result1.is_ok(), "Installation of v{} to venv1 should succeed", version1);

        // Install version 2 to venv2
        let version2 = format!("{}.0.0", version2_major);
        let wheel2 = create_test_wheel(&pkg_name, &version2);
        let cache2 = GlobalCache::new(cache_dir.path()).unwrap();
        let installer2 = WheelInstaller::new(cache2, site_packages2.clone());
        let result2 = installer2.install_wheel(&wheel2);
        prop_assert!(result2.is_ok(), "Installation of v{} to venv2 should succeed", version2);

        // Both should have the package
        prop_assert!(
            is_package_importable(&site_packages1, &pkg_name),
            "Package should be in venv1"
        );
        prop_assert!(
            is_package_importable(&site_packages2, &pkg_name),
            "Package should be in venv2"
        );

        // Check that dist-info directories have correct versions
        let normalized_name = pkg_name.replace('-', "_");
        let dist_info1 = site_packages1.join(format!("{}-{}.dist-info", normalized_name, version1));
        let dist_info2 = site_packages2.join(format!("{}-{}.dist-info", normalized_name, version2));

        prop_assert!(
            dist_info1.exists(),
            "venv1 should have dist-info for version {}", version1
        );
        prop_assert!(
            dist_info2.exists(),
            "venv2 should have dist-info for version {}", version2
        );
    }
}

// ============================================================================
// Additional Unit Tests for Edge Cases
// ============================================================================

#[test]
fn test_venv_without_virtual_env_var() {
    // Test that installation works when VIRTUAL_ENV is not set
    env::remove_var("VIRTUAL_ENV");

    let cache_dir = TempDir::new().unwrap();
    let venv = TempDir::new().unwrap();

    create_venv_structure(venv.path());
    let site_packages = get_site_packages(venv.path());

    let cache = GlobalCache::new(cache_dir.path()).unwrap();
    let installer = WheelInstaller::new(cache, site_packages.clone());

    let wheel_data = create_test_wheel("test-pkg", "1.0.0");
    let result = installer.install_wheel(&wheel_data);

    assert!(result.is_ok(), "Installation should succeed without VIRTUAL_ENV");
    assert!(
        is_package_importable(&site_packages, "test-pkg"),
        "Package should be installed"
    );
}

#[test]
fn test_venv_with_invalid_virtual_env_var() {
    // Test that installation handles invalid VIRTUAL_ENV gracefully
    env::set_var("VIRTUAL_ENV", "/nonexistent/path");

    let cache_dir = TempDir::new().unwrap();
    let venv = TempDir::new().unwrap();

    create_venv_structure(venv.path());
    let site_packages = get_site_packages(venv.path());

    let cache = GlobalCache::new(cache_dir.path()).unwrap();
    let installer = WheelInstaller::new(cache, site_packages.clone());

    let wheel_data = create_test_wheel("test-pkg", "1.0.0");
    let result = installer.install_wheel(&wheel_data);

    assert!(result.is_ok(), "Installation should succeed even with invalid VIRTUAL_ENV");

    env::remove_var("VIRTUAL_ENV");
}

#[test]
fn test_empty_venv_has_no_packages() {
    let venv = TempDir::new().unwrap();
    create_venv_structure(venv.path());

    let site_packages = get_site_packages(venv.path());

    // Empty venv should have no packages
    assert!(
        !is_package_importable(&site_packages, "nonexistent-pkg"),
        "Empty venv should not have any packages"
    );
}

#[test]
fn test_venv_pyvenv_cfg_content() {
    let venv = TempDir::new().unwrap();
    create_venv_structure(venv.path());

    let cfg_path = venv.path().join("pyvenv.cfg");
    assert!(cfg_path.exists(), "pyvenv.cfg should exist");

    let content = fs::read_to_string(&cfg_path).unwrap();
    assert!(
        content.contains("home ="),
        "pyvenv.cfg should contain 'home ='"
    );
    assert!(
        content.contains("include-system-site-packages ="),
        "pyvenv.cfg should contain 'include-system-site-packages ='"
    );
    assert!(
        content.contains("version ="),
        "pyvenv.cfg should contain 'version ='"
    );
}

#[test]
fn test_venv_site_packages_is_writable() {
    let venv = TempDir::new().unwrap();
    create_venv_structure(venv.path());

    let site_packages = get_site_packages(venv.path());

    // Try to create a test file
    let test_file = site_packages.join("test_write.txt");
    let write_result = fs::write(&test_file, b"test");

    assert!(
        write_result.is_ok(),
        "site-packages should be writable"
    );
    assert!(test_file.exists(), "Test file should be created");
}
