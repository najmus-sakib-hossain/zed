//! Python version management commands
//!
//! Commands for installing, listing, and managing Python versions
//! using python-build-standalone releases.

use std::path::Path;

use dx_py_core::Result;
use dx_py_workspace::{PythonManager, RealPythonManager};

/// Install a Python version
pub fn install(version: &str) -> Result<()> {
    println!("Installing Python {}...", version);

    let manager = PythonManager::new();
    let install_path = manager.version_path(version);

    if manager.is_installed(version) {
        println!("Python {} is already installed at {}", version, install_path.display());
        return Ok(());
    }

    // Use RealPythonManager for actual download
    let real_manager = RealPythonManager::new();

    println!("Downloading Python {} from python-build-standalone...", version);

    match real_manager.install(version) {
        Ok(installation) => {
            println!("\n✓ Python {} installed successfully!", version);
            println!("  Location: {}", installation.path.display());

            // Verify installation
            if installation.path.exists() {
                println!("\nTo use this Python version:");
                println!("  dx-py python pin {}", version);
            }
        }
        Err(e) => {
            eprintln!("\n✗ Failed to install Python {}: {}", version, e);
            eprintln!("\nYou can also install Python manually:");
            eprintln!(
                "  1. Download from https://github.com/indygreg/python-build-standalone/releases"
            );
            eprintln!("  2. Extract to {}", install_path.display());
            return Err(e);
        }
    }

    Ok(())
}

/// List installed Python versions
pub fn list() -> Result<()> {
    let mut manager = PythonManager::new();
    let installations = manager.discover();

    // Also try to list available versions from python-build-standalone
    let real_manager = RealPythonManager::new();

    if installations.is_empty() {
        println!("No Python installations found.");
    } else {
        println!("Installed Python versions:\n");

        for install in &installations {
            let marker = if install.is_managed {
                " (managed by dx-py)"
            } else if install.is_system {
                " (system)"
            } else {
                ""
            };

            println!("  {} @ {}{}", install.version, install.path.display(), marker);
        }
    }

    // Show available versions if we can fetch them
    if let Ok(available) = real_manager.list_available() {
        let installed_versions: std::collections::HashSet<_> =
            installations.iter().map(|i| &i.version).collect();

        let not_installed: Vec<_> = available
            .iter()
            .filter(|r| !installed_versions.contains(&r.version))
            .take(5)
            .collect();

        if !not_installed.is_empty() {
            println!("\nAvailable for installation:");
            for release in not_installed {
                println!("  {} ({})", release.version, release.platform);
            }
            println!("\nInstall with: dx-py python install <version>");
        }
    }

    if installations.is_empty() {
        println!("\nTo install Python:");
        println!("  dx-py python install 3.12.0");
    }

    Ok(())
}

/// Pin Python version for the current project
pub fn pin(version: &str) -> Result<()> {
    let project_dir = Path::new(".");

    let manager = PythonManager::new();

    // Check if version is installed
    if !manager.is_installed(version) {
        println!("Warning: Python {} is not installed.", version);
        println!("Run 'dx-py python install {}' to install it.", version);
    }

    manager.pin(project_dir, version)?;

    println!("Pinned Python version to {} in .python-version", version);

    Ok(())
}

/// Show which Python would be used
pub fn which() -> Result<()> {
    let project_dir = Path::new(".");

    let mut manager = PythonManager::new();

    // Check for pinned version
    if let Some(pinned) = manager.read_pin(project_dir)? {
        println!("Pinned version: {}", pinned);

        manager.discover();
        if let Some(install) = manager.find(&pinned) {
            println!("Python path: {}", install.path.display());

            // Verify it exists
            if install.path.exists() {
                println!("Status: ✓ Available");
            } else {
                println!("Status: ✗ Path does not exist");
            }
        } else {
            println!("Status: ✗ Not installed");
            println!("\nRun 'dx-py python install {}' to install it", pinned);
        }
    } else {
        // Use first available
        let installations = manager.discover();
        if let Some(install) = installations.first() {
            println!("Using: {} @ {}", install.version, install.path.display());
            println!("(No .python-version file found, using first available)");
            println!("\nTo pin a specific version:");
            println!("  dx-py python pin {}", install.version);
        } else {
            println!("No Python installation found.");
            println!("\nRun 'dx-py python install <version>' to install Python.");
            println!("Example: dx-py python install 3.12.0");
        }
    }

    Ok(())
}
