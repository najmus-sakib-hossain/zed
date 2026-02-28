//! Integration test for venv-aware package installation
//!
//! This test validates that packages are installed to the correct location
//! based on the VIRTUAL_ENV environment variable.
//!
//! Requirements: 10.2, 10.3

use dx_py_package_manager::{GlobalCache, WheelInstaller};
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a minimal test wheel
fn create_test_wheel() -> Vec<u8> {
    use std::io::Write;
    use zip::write::{FileOptions, ZipWriter};

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));

        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);

        // Add package file
        zip.start_file("test_pkg/__init__.py", options).unwrap();
        zip.write_all(b"# Test package\n__version__ = '1.0.0'\n")
            .unwrap();

        // Add dist-info directory
        zip.start_file("test_pkg-1.0.0.dist-info/METADATA", options)
            .unwrap();
        zip.write_all(
            b"Metadata-Version: 2.1\nName: test-pkg\nVersion: 1.0.0\nSummary: Test package\n",
        )
        .unwrap();

        zip.start_file("test_pkg-1.0.0.dist-info/WHEEL", options)
            .unwrap();
        zip.write_all(b"Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: true\nTag: py3-none-any\n")
            .unwrap();

        zip.start_file("test_pkg-1.0.0.dist-info/RECORD", options)
            .unwrap();
        zip.write_all(b"test_pkg/__init__.py,,\ntest_pkg-1.0.0.dist-info/METADATA,,\ntest_pkg-1.0.0.dist-info/WHEEL,,\ntest_pkg-1.0.0.dist-info/RECORD,,\n")
            .unwrap();

        zip.finish().unwrap();
    }
    buffer
}

/// Test that packages are installed to VIRTUAL_ENV when set
#[test]
fn test_install_to_virtual_env_when_set() {
    let cache_dir = TempDir::new().unwrap();
    let local_venv = TempDir::new().unwrap();
    let active_venv = TempDir::new().unwrap();

    // Create site-packages in both venvs
    #[cfg(unix)]
    {
        std::fs::create_dir_all(local_venv.path().join("lib/python3.12/site-packages")).unwrap();
        std::fs::create_dir_all(active_venv.path().join("lib/python3.12/site-packages")).unwrap();
    }
    #[cfg(windows)]
    {
        std::fs::create_dir_all(local_venv.path().join("Lib/site-packages")).unwrap();
        std::fs::create_dir_all(active_venv.path().join("Lib/site-packages")).unwrap();
    }

    // Set VIRTUAL_ENV to active venv
    env::set_var("VIRTUAL_ENV", active_venv.path());

    // Determine site-packages path (simulating what sync.rs does)
    let site_packages = if let Ok(virtual_env) = env::var("VIRTUAL_ENV") {
        let venv_path = PathBuf::from(virtual_env);
        #[cfg(unix)]
        {
            venv_path.join("lib/python3.12/site-packages")
        }
        #[cfg(windows)]
        {
            venv_path.join("Lib/site-packages")
        }
    } else {
        #[cfg(unix)]
        {
            local_venv.path().join("lib/python3.12/site-packages")
        }
        #[cfg(windows)]
        {
            local_venv.path().join("Lib/site-packages")
        }
    };

    // Create installer with the determined site-packages
    let cache = GlobalCache::new(cache_dir.path()).unwrap();
    let installer = WheelInstaller::new(cache, site_packages.clone());

    // Install a test wheel
    let wheel_data = create_test_wheel();
    let result = installer.install_wheel(&wheel_data);

    assert!(result.is_ok(), "Installation should succeed");

    // Verify package was installed to active venv, not local venv
    #[cfg(unix)]
    {
        let active_pkg = active_venv
            .path()
            .join("lib/python3.12/site-packages/test_pkg/__init__.py");
        let local_pkg = local_venv
            .path()
            .join("lib/python3.12/site-packages/test_pkg/__init__.py");

        assert!(
            active_pkg.exists(),
            "Package should be installed in active venv: {:?}",
            active_pkg
        );
        assert!(
            !local_pkg.exists(),
            "Package should NOT be installed in local venv: {:?}",
            local_pkg
        );
    }
    #[cfg(windows)]
    {
        let active_pkg = active_venv
            .path()
            .join("Lib/site-packages/test_pkg/__init__.py");
        let local_pkg = local_venv
            .path()
            .join("Lib/site-packages/test_pkg/__init__.py");

        assert!(
            active_pkg.exists(),
            "Package should be installed in active venv: {:?}",
            active_pkg
        );
        assert!(
            !local_pkg.exists(),
            "Package should NOT be installed in local venv: {:?}",
            local_pkg
        );
    }

    // Clean up
    env::remove_var("VIRTUAL_ENV");
}

/// Test that packages are installed to local .venv when VIRTUAL_ENV is not set
#[test]
fn test_install_to_local_venv_when_virtual_env_not_set() {
    let cache_dir = TempDir::new().unwrap();
    let local_venv = TempDir::new().unwrap();

    // Ensure VIRTUAL_ENV is not set
    env::remove_var("VIRTUAL_ENV");

    // Create site-packages
    #[cfg(unix)]
    {
        std::fs::create_dir_all(local_venv.path().join("lib/python3.12/site-packages")).unwrap();
    }
    #[cfg(windows)]
    {
        std::fs::create_dir_all(local_venv.path().join("Lib/site-packages")).unwrap();
    }

    // Determine site-packages path
    let site_packages = if let Ok(virtual_env) = env::var("VIRTUAL_ENV") {
        let venv_path = PathBuf::from(virtual_env);
        #[cfg(unix)]
        {
            venv_path.join("lib/python3.12/site-packages")
        }
        #[cfg(windows)]
        {
            venv_path.join("Lib/site-packages")
        }
    } else {
        #[cfg(unix)]
        {
            local_venv.path().join("lib/python3.12/site-packages")
        }
        #[cfg(windows)]
        {
            local_venv.path().join("Lib/site-packages")
        }
    };

    // Create installer
    let cache = GlobalCache::new(cache_dir.path()).unwrap();
    let installer = WheelInstaller::new(cache, site_packages.clone());

    // Install a test wheel
    let wheel_data = create_test_wheel();
    let result = installer.install_wheel(&wheel_data);

    assert!(result.is_ok(), "Installation should succeed");

    // Verify package was installed to local venv
    #[cfg(unix)]
    {
        let local_pkg = local_venv
            .path()
            .join("lib/python3.12/site-packages/test_pkg/__init__.py");
        assert!(
            local_pkg.exists(),
            "Package should be installed in local venv: {:?}",
            local_pkg
        );
    }
    #[cfg(windows)]
    {
        let local_pkg = local_venv
            .path()
            .join("Lib/site-packages/test_pkg/__init__.py");
        assert!(
            local_pkg.exists(),
            "Package should be installed in local venv: {:?}",
            local_pkg
        );
    }
}
