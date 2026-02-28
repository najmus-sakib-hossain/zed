//! Integration tests for dx-py-package-manager
//!
//! These tests require network access and hit real PyPI.
//! Run with: cargo test --test integration_tests -- --ignored

use dx_py_package_manager::{AsyncPyPiClient, DependencySpec, PyPiResolver};

/// Test resolving a real package from PyPI
#[tokio::test]
#[ignore = "requires network access"]
async fn test_resolve_requests_package() {
    let client = AsyncPyPiClient::new();
    let mut resolver = PyPiResolver::new(client);

    let deps = vec![DependencySpec::parse("requests>=2.28").unwrap()];

    let resolution = resolver.resolve(&deps).await.unwrap();

    // Should have resolved requests and its dependencies
    assert!(!resolution.packages.is_empty(), "Should resolve at least one package");

    // Find requests in the resolution
    let requests = resolution.packages.iter().find(|p| p.name == "requests");
    assert!(requests.is_some(), "Should have resolved requests package");

    let requests = requests.unwrap();
    assert!(requests.version_string.starts_with("2."), "Should be version 2.x");

    // Should have transitive dependencies
    let dep_names: Vec<_> = resolution.packages.iter().map(|p| p.name.as_str()).collect();

    // requests typically depends on urllib3, certifi, charset-normalizer, idna
    println!("Resolved packages: {:?}", dep_names);
    assert!(dep_names.len() >= 2, "Should have transitive dependencies");
}

/// Test fetching package metadata from PyPI
#[tokio::test]
#[ignore = "requires network access"]
async fn test_fetch_package_metadata() {
    let client = AsyncPyPiClient::new();

    let info = client.get_package("requests").await.unwrap();

    assert_eq!(info.info.name.to_lowercase(), "requests");
    assert!(!info.releases.is_empty(), "Should have releases");

    // Check that we have version info
    assert!(!info.info.version.is_empty(), "Should have a version");
}

/// Test fetching package versions
#[tokio::test]
#[ignore = "requires network access"]
async fn test_fetch_package_versions() {
    let client = AsyncPyPiClient::new();

    let versions = client.get_versions("flask").await.unwrap();

    assert!(!versions.is_empty(), "Should have versions");

    // Flask has been around for a while, should have many versions
    assert!(versions.len() > 10, "Flask should have many versions");
}

/// Test fetching package dependencies
#[tokio::test]
#[ignore = "requires network access"]
async fn test_fetch_package_dependencies() {
    let client = AsyncPyPiClient::new();

    let deps = client.get_dependencies("flask", "3.0.0").await.unwrap();

    // Flask 3.0.0 has dependencies like Werkzeug, Jinja2, etc.
    assert!(!deps.is_empty(), "Flask should have dependencies");

    let dep_names: Vec<_> = deps.iter().map(|d| d.name.as_str()).collect();
    println!("Flask dependencies: {:?}", dep_names);
}

/// Test finding best wheel for platform
#[tokio::test]
#[ignore = "requires network access"]
async fn test_find_best_wheel() {
    let client = AsyncPyPiClient::new();
    let platform_env = dx_py_core::wheel::PlatformEnvironment::detect();

    // numpy has platform-specific wheels
    let wheel = client.find_best_wheel("numpy", "1.26.0", &platform_env).await.unwrap();

    if let Some(wheel) = wheel {
        println!("Found wheel: {}", wheel.filename);
        assert!(wheel.filename.ends_with(".whl"), "Should be a wheel file");
    } else {
        // May not have a wheel for all platforms
        println!("No compatible wheel found for this platform");
    }
}

/// Test downloading a package
#[tokio::test]
#[ignore = "requires network access"]
async fn test_download_package() {
    let client = AsyncPyPiClient::new();
    let platform_env = dx_py_core::wheel::PlatformEnvironment::detect();

    // Find a distribution for a pure Python package (should work on all platforms)
    let dist = client.find_distribution("six", "1.16.0", &platform_env).await.unwrap();

    assert!(dist.is_some(), "Should find a distribution for six");

    let dist = dist.unwrap();
    println!("Downloading: {}", dist.filename);

    // Download and verify
    let data = client.download(&dist.url, &dist.digests.sha256).await.unwrap();

    assert!(!data.is_empty(), "Downloaded data should not be empty");
    println!("Downloaded {} bytes", data.len());
}

/// Test resolution with markers
#[tokio::test]
#[ignore = "requires network access"]
async fn test_resolve_with_markers() {
    let client = AsyncPyPiClient::new();
    let mut resolver = PyPiResolver::new(client);

    // colorama is typically only needed on Windows
    let deps = vec![DependencySpec::parse("colorama; sys_platform == 'win32'").unwrap()];

    let resolution = resolver.resolve(&deps).await.unwrap();

    // On Windows, should resolve colorama
    // On other platforms, should be empty (marker evaluates to false)
    #[cfg(windows)]
    {
        assert!(!resolution.packages.is_empty(), "Should resolve colorama on Windows");
    }

    #[cfg(not(windows))]
    {
        assert!(resolution.packages.is_empty(), "Should not resolve colorama on non-Windows");
    }
}

/// Test downloading and installing a wheel
#[tokio::test]
#[ignore = "requires network access"]
async fn test_download_and_install_wheel() {
    use dx_py_package_manager::{GlobalCache, WheelInstaller};
    use tempfile::TempDir;

    let client = AsyncPyPiClient::new();
    let platform_env = dx_py_core::wheel::PlatformEnvironment::detect();

    // Create temp directories for cache and site-packages
    let cache_dir = TempDir::new().unwrap();
    let site_packages_dir = TempDir::new().unwrap();

    let cache = GlobalCache::new(cache_dir.path()).unwrap();
    let installer = WheelInstaller::new(
        GlobalCache::new(cache_dir.path()).unwrap(),
        site_packages_dir.path().to_path_buf(),
    );

    // Download a pure Python package
    let dist = client.find_distribution("six", "1.16.0", &platform_env).await.unwrap();
    assert!(dist.is_some(), "Should find six distribution");

    let dist = dist.unwrap();
    let data = client.download(&dist.url, &dist.digests.sha256).await.unwrap();

    // Store in cache
    let hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    };
    cache.store(&hash, &data).unwrap();

    // Install the wheel
    let installed = installer.install_wheel(&data).unwrap();

    assert_eq!(installed.name.to_lowercase(), "six");
    assert_eq!(installed.version, "1.16.0");
    assert!(!installed.files.is_empty(), "Should have installed files");

    // Verify files exist in site-packages
    let six_py = site_packages_dir.path().join("six.py");
    assert!(
        six_py.exists() || site_packages_dir.path().join("six/__init__.py").exists(),
        "six.py or six/__init__.py should exist"
    );

    // Verify dist-info exists
    assert!(installed.dist_info.exists(), "dist-info should exist");
}

/// Test uninstalling a package
#[tokio::test]
#[ignore = "requires network access"]
async fn test_uninstall_package() {
    use dx_py_package_manager::{GlobalCache, WheelInstaller};
    use tempfile::TempDir;

    let client = AsyncPyPiClient::new();
    let platform_env = dx_py_core::wheel::PlatformEnvironment::detect();

    let cache_dir = TempDir::new().unwrap();
    let site_packages_dir = TempDir::new().unwrap();

    let installer = WheelInstaller::new(
        GlobalCache::new(cache_dir.path()).unwrap(),
        site_packages_dir.path().to_path_buf(),
    );

    // Download and install
    let dist = client.find_distribution("six", "1.16.0", &platform_env).await.unwrap().unwrap();
    let data = client.download(&dist.url, &dist.digests.sha256).await.unwrap();
    let installed = installer.install_wheel(&data).unwrap();

    // Verify installed
    assert!(installed.dist_info.exists());

    // Uninstall
    let removed = installer.uninstall("six").unwrap();
    assert!(removed > 0, "Should have removed files");

    // Verify uninstalled
    let six_py = site_packages_dir.path().join("six.py");
    assert!(!six_py.exists(), "six.py should be removed");
}

// NOTE: The following tests are disabled because they reference dx_py_workspace
// which is not yet implemented. They will be re-enabled when the workspace
// management crate is created.

/*
/// Test virtual environment creation
#[test]
#[ignore = "requires Python to be installed"]
fn test_venv_creation() {
    use dx_py_workspace::{RealVenvManager, PythonManager};
    use tempfile::TempDir;
    use std::process::Command;

    let temp_dir = TempDir::new().unwrap();
    let venv_path = temp_dir.path().join("test-venv");

    // Find a Python installation
    let mut python_manager = PythonManager::new();
    let installations = python_manager.discover();

    if installations.is_empty() {
        println!("Skipping test: no Python installation found");
        return;
    }

    let python = &installations[0];
    println!("Using Python: {} @ {}", python.version, python.path.display());

    // Create venv using RealVenvManager
    let mut venv_manager = RealVenvManager::new();
    let venv = venv_manager.create(&venv_path, &python.path).unwrap();

    // Verify venv structure
    assert!(venv.path.exists(), "Venv path should exist");
    assert!(venv.site_packages().exists(), "Site-packages should exist");
    assert!(venv.bin_dir().exists(), "Bin directory should exist");

    // Verify pyvenv.cfg exists
    let pyvenv_cfg = venv_path.join("pyvenv.cfg");
    assert!(pyvenv_cfg.exists(), "pyvenv.cfg should exist");

    // Verify Python executable exists in venv
    #[cfg(unix)]
    let python_exe = venv.bin_dir().join("python");
    #[cfg(windows)]
    let python_exe = venv.bin_dir().join("python.exe");

    assert!(python_exe.exists(), "Python executable should exist in venv");

    // Try running Python in the venv
    let output = Command::new(&python_exe)
        .arg("--version")
        .output();

    if let Ok(output) = output {
        let version = String::from_utf8_lossy(&output.stdout);
        println!("Venv Python version: {}", version.trim());
        assert!(output.status.success(), "Python should run successfully");
    }
}

/// Test activation script generation
#[test]
fn test_activation_scripts_generated() {
    use dx_py_workspace::Venv;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let venv_path = temp_dir.path().join("test-venv");

    // Create minimal venv structure
    #[cfg(unix)]
    {
        std::fs::create_dir_all(venv_path.join("bin")).unwrap();
        std::fs::create_dir_all(venv_path.join("lib/python3.12/site-packages")).unwrap();
    }
    #[cfg(windows)]
    {
        std::fs::create_dir_all(venv_path.join("Scripts")).unwrap();
        std::fs::create_dir_all(venv_path.join("Lib/site-packages")).unwrap();
    }

    // Write pyvenv.cfg
    std::fs::write(venv_path.join("pyvenv.cfg"), "version = 3.12.0\n").unwrap();

    // Create a Venv instance and verify structure
    let venv = Venv::new(venv_path.clone(), "3.12.0".to_string());

    // Verify venv structure is correct
    assert!(venv.path.exists(), "Venv path should exist");
    assert!(venv.site_packages().exists(), "Site-packages should exist");
    assert!(venv.bin_dir().exists(), "Bin directory should exist");

    // Note: Activation scripts are generated by VenvManager::create_minimal_venv
    // which is called internally during venv creation. This test verifies the
    // basic venv structure that would be created.
}
*/
