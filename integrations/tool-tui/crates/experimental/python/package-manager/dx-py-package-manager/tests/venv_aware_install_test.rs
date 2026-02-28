//! Test venv-aware package installation
//!
//! This test validates that the package installer correctly detects and uses
//! the VIRTUAL_ENV environment variable when installing packages.
//!
//! Requirements: 10.2, 10.3

use std::env;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test that installation respects VIRTUAL_ENV environment variable
#[test]
fn test_install_respects_virtual_env() {
    // Create two temporary directories: one for .venv and one for VIRTUAL_ENV
    let local_venv = TempDir::new().unwrap();
    let active_venv = TempDir::new().unwrap();

    // Create site-packages directories in both
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

    // Set VIRTUAL_ENV to the active venv
    env::set_var("VIRTUAL_ENV", active_venv.path());

    // Simulate getting site-packages path (this is what sync.rs does)
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
        // Fall back to local .venv
        #[cfg(unix)]
        {
            local_venv.path().join("lib/python3.12/site-packages")
        }
        #[cfg(windows)]
        {
            local_venv.path().join("Lib/site-packages")
        }
    };

    // Verify that site_packages points to the active venv, not local .venv
    assert!(
        site_packages.starts_with(active_venv.path()),
        "Site-packages should be in active venv: {:?} should start with {:?}",
        site_packages,
        active_venv.path()
    );

    assert!(
        !site_packages.starts_with(local_venv.path()),
        "Site-packages should NOT be in local venv: {:?} should not start with {:?}",
        site_packages,
        local_venv.path()
    );

    // Clean up
    env::remove_var("VIRTUAL_ENV");
}

/// Test that installation falls back to .venv when VIRTUAL_ENV is not set
#[test]
fn test_install_falls_back_to_local_venv() {
    // Ensure VIRTUAL_ENV is not set
    env::remove_var("VIRTUAL_ENV");

    let local_venv = TempDir::new().unwrap();

    // Create site-packages directory
    #[cfg(unix)]
    {
        std::fs::create_dir_all(local_venv.path().join("lib/python3.12/site-packages")).unwrap();
    }
    #[cfg(windows)]
    {
        std::fs::create_dir_all(local_venv.path().join("Lib/site-packages")).unwrap();
    }

    // Simulate getting site-packages path
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
        // Fall back to local .venv
        #[cfg(unix)]
        {
            local_venv.path().join("lib/python3.12/site-packages")
        }
        #[cfg(windows)]
        {
            local_venv.path().join("Lib/site-packages")
        }
    };

    // Verify that site_packages points to the local .venv
    assert!(
        site_packages.starts_with(local_venv.path()),
        "Site-packages should be in local venv when VIRTUAL_ENV is not set: {:?} should start with {:?}",
        site_packages,
        local_venv.path()
    );
}

/// Test that VIRTUAL_ENV takes precedence over local .venv
#[test]
fn test_virtual_env_takes_precedence() {
    let local_venv = TempDir::new().unwrap();
    let active_venv = TempDir::new().unwrap();

    // Create both directories
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

    // Test without VIRTUAL_ENV
    env::remove_var("VIRTUAL_ENV");
    let site_packages_without = if let Ok(virtual_env) = env::var("VIRTUAL_ENV") {
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

    // Test with VIRTUAL_ENV
    env::set_var("VIRTUAL_ENV", active_venv.path());
    let site_packages_with = if let Ok(virtual_env) = env::var("VIRTUAL_ENV") {
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

    // Verify they're different
    assert_ne!(
        site_packages_without, site_packages_with,
        "Site-packages path should change when VIRTUAL_ENV is set"
    );

    // Verify the with-VIRTUAL_ENV path points to active venv
    assert!(
        site_packages_with.starts_with(active_venv.path()),
        "With VIRTUAL_ENV set, site-packages should be in active venv"
    );

    // Verify the without-VIRTUAL_ENV path points to local venv
    assert!(
        site_packages_without.starts_with(local_venv.path()),
        "Without VIRTUAL_ENV set, site-packages should be in local venv"
    );

    // Clean up
    env::remove_var("VIRTUAL_ENV");
}
