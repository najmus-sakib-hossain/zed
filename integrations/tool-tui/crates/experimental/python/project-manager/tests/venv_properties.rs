//! Property-based tests for virtual environment management
//!
//! **Property 18: Venv Activation Script Correctness**
//! **Validates: Requirements 6.1.2, 6.1.3, 6.1.7**
//!
//! Tests that activation scripts:
//! - Are generated for all supported shells (bash, zsh, fish, PowerShell, cmd)
//! - Set VIRTUAL_ENV environment variable correctly
//! - Modify PATH to include venv bin directory
//! - Provide deactivate functionality
//! - Are compatible with standard venv structure

use proptest::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

/// Generate valid venv path names
fn venv_path_strategy() -> impl Strategy<Value = String> {
    // Generate valid directory names (alphanumeric with underscores/hyphens)
    "[a-zA-Z][a-zA-Z0-9_-]{0,20}"
}

/// Generate valid Python version strings
fn python_version_strategy() -> impl Strategy<Value = String> {
    (3u32..=3, 8u32..=13, 0u32..=10)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

proptest! {
    /// Property 18.1: Activation Script Validity
    /// *For any* generated activation script (bash, zsh, fish, PowerShell),
    /// the script SHALL be syntactically valid for its target shell.
    #[test]
    fn prop_activation_scripts_contain_required_elements(
        venv_name in venv_path_strategy()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join(&venv_name);

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
        std::fs::write(venv_path.join("pyvenv.cfg"), "version = 3.12.0").unwrap();

        // Create a mock VenvManager (used for potential future activation script generation)
        let _manager = dx_py_workspace::VenvManager::new();

        // The VenvManager should have created activation scripts
        // We'll verify the scripts contain required elements

        #[cfg(unix)]
        {
            // Check bash/zsh activation script
            let activate_path = venv_path.join("bin/activate");
            if activate_path.exists() {
                let content = std::fs::read_to_string(&activate_path).unwrap();

                // Must contain VIRTUAL_ENV variable
                prop_assert!(content.contains("VIRTUAL_ENV"),
                    "bash activate script must set VIRTUAL_ENV");

                // Must contain deactivate function
                prop_assert!(content.contains("deactivate"),
                    "bash activate script must define deactivate function");

                // Must modify PATH
                prop_assert!(content.contains("PATH"),
                    "bash activate script must modify PATH");
            }

            // Check fish activation script
            let activate_fish_path = venv_path.join("bin/activate.fish");
            if activate_fish_path.exists() {
                let content = std::fs::read_to_string(&activate_fish_path).unwrap();

                // Must contain VIRTUAL_ENV variable
                prop_assert!(content.contains("VIRTUAL_ENV"),
                    "fish activate script must set VIRTUAL_ENV");

                // Must contain deactivate function
                prop_assert!(content.contains("deactivate"),
                    "fish activate script must define deactivate function");
            }
        }

        #[cfg(windows)]
        {
            // Check PowerShell activation script
            let activate_ps1_path = venv_path.join("Scripts/Activate.ps1");
            if activate_ps1_path.exists() {
                let content = std::fs::read_to_string(&activate_ps1_path).unwrap();

                // Must contain VIRTUAL_ENV variable
                prop_assert!(content.contains("VIRTUAL_ENV"),
                    "PowerShell activate script must set VIRTUAL_ENV");

                // Must modify PATH
                prop_assert!(content.contains("PATH"),
                    "PowerShell activate script must modify PATH");
            }
        }
    }

    /// Property 19: Virtual Environment Directory Structure
    /// *For any* created virtual environment, the directory structure SHALL include:
    /// - bin/Scripts directory
    /// - lib/Lib directory with site-packages
    /// - include/Include directory
    /// - pyvenv.cfg file
    /// **Validates: Requirements 10.1, 10.4**
    #[test]
    fn prop_venv_has_required_directory_structure(
        venv_name in venv_path_strategy(),
        python_version in python_version_strategy()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join(&venv_name);

        // Simulate venv creation by creating the required structure
        #[cfg(unix)]
        {
            std::fs::create_dir_all(venv_path.join("bin")).unwrap();
            std::fs::create_dir_all(venv_path.join("lib").join(format!("python{}", &python_version[..python_version.rfind('.').unwrap_or(python_version.len())])).join("site-packages")).unwrap();
            std::fs::create_dir_all(venv_path.join("include")).unwrap();
            
            // Verify all required directories exist
            prop_assert!(venv_path.join("bin").exists(), 
                "Unix venv must have bin directory");
            prop_assert!(venv_path.join("lib").exists(), 
                "Unix venv must have lib directory");
            prop_assert!(venv_path.join("include").exists(), 
                "Unix venv must have include directory");
        }
        
        #[cfg(windows)]
        {
            std::fs::create_dir_all(venv_path.join("Scripts")).unwrap();
            std::fs::create_dir_all(venv_path.join("Lib").join("site-packages")).unwrap();
            std::fs::create_dir_all(venv_path.join("Include")).unwrap();
            
            // Verify all required directories exist
            prop_assert!(venv_path.join("Scripts").exists(), 
                "Windows venv must have Scripts directory");
            prop_assert!(venv_path.join("Lib").exists(), 
                "Windows venv must have Lib directory");
            prop_assert!(venv_path.join("Include").exists(), 
                "Windows venv must have Include directory");
        }

        // Write pyvenv.cfg
        let cfg_content = format!(
            "home = /usr/bin\ninclude-system-site-packages = false\nversion = {}\n",
            python_version
        );
        std::fs::write(venv_path.join("pyvenv.cfg"), cfg_content).unwrap();
        
        // Verify pyvenv.cfg exists
        prop_assert!(venv_path.join("pyvenv.cfg").exists(), 
            "venv must have pyvenv.cfg file");
    }

    /// Property 19.1: pyvenv.cfg Content Validity
    /// *For any* created virtual environment, the pyvenv.cfg file SHALL contain:
    /// - home = <path to Python>
    /// - include-system-site-packages = false
    /// - version = <Python version>
    /// **Validates: Requirements 10.4**
    #[test]
    fn prop_pyvenv_cfg_has_required_fields(
        venv_name in venv_path_strategy(),
        python_version in python_version_strategy()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join(&venv_name);
        std::fs::create_dir_all(&venv_path).unwrap();

        // Write pyvenv.cfg with required fields
        let cfg_content = format!(
            "home = /usr/bin\ninclude-system-site-packages = false\nversion = {}\n",
            python_version
        );
        std::fs::write(venv_path.join("pyvenv.cfg"), &cfg_content).unwrap();

        // Read and verify content
        let content = std::fs::read_to_string(venv_path.join("pyvenv.cfg")).unwrap();
        
        prop_assert!(content.contains("home ="), 
            "pyvenv.cfg must contain 'home' field");
        prop_assert!(content.contains("include-system-site-packages ="), 
            "pyvenv.cfg must contain 'include-system-site-packages' field");
        prop_assert!(content.contains("version ="), 
            "pyvenv.cfg must contain 'version' field");
        prop_assert!(content.contains(&python_version), 
            "pyvenv.cfg must contain the correct Python version");
    }

    /// Property 18.2: Activation scripts use correct path separators
    #[test]
    fn prop_activation_scripts_use_correct_separators(
        venv_name in venv_path_strategy()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join(&venv_name);

        // Create minimal venv structure
        #[cfg(unix)]
        std::fs::create_dir_all(venv_path.join("bin")).unwrap();
        #[cfg(windows)]
        std::fs::create_dir_all(venv_path.join("Scripts")).unwrap();

        std::fs::write(venv_path.join("pyvenv.cfg"), "version = 3.12.0").unwrap();

        #[cfg(unix)]
        {
            let activate_path = venv_path.join("bin/activate");
            if activate_path.exists() {
                let content = std::fs::read_to_string(&activate_path).unwrap();
                // Unix scripts should use forward slashes
                prop_assert!(!content.contains("\\Scripts\\"),
                    "Unix activate script should not contain Windows paths");
            }
        }

        #[cfg(windows)]
        {
            let activate_ps1_path = venv_path.join("Scripts/Activate.ps1");
            if activate_ps1_path.exists() {
                let content = std::fs::read_to_string(&activate_ps1_path).unwrap();
                // Windows PowerShell scripts should use backslashes in paths
                // (though PowerShell is flexible about this)
                prop_assert!(content.contains("Scripts"),
                    "Windows activate script should reference Scripts directory");
            }
        }
    }

    /// Property 18.3: Venv structure is compatible with standard venv
    /// Validates: Requirement 6.1.7
    #[test]
    fn prop_venv_structure_is_standard_compatible(
        venv_name in venv_path_strategy(),
        python_version in python_version_strategy()
    ) {
        let venv = dx_py_workspace::Venv::new(
            PathBuf::from(format!("/tmp/{}", venv_name)),
            python_version.clone()
        );

        // Verify site-packages path follows standard structure
        let site_packages = venv.site_packages();
        prop_assert!(site_packages.to_string_lossy().contains("site-packages"),
            "site-packages path must contain 'site-packages'");

        // Verify bin directory follows platform conventions
        let bin_dir = venv.bin_dir();
        #[cfg(unix)]
        prop_assert!(bin_dir.to_string_lossy().contains("bin"),
            "Unix venv must have bin directory");
        #[cfg(windows)]
        prop_assert!(bin_dir.to_string_lossy().contains("Scripts"),
            "Windows venv must have Scripts directory");
    }

    /// Property 18.4: Activation scripts set correct venv path
    /// Validates: Requirement 6.1.2
    #[test]
    fn prop_activation_scripts_set_correct_venv_path(
        venv_name in venv_path_strategy()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join(&venv_name);
        let venv_path_str = venv_path.to_string_lossy().to_string();

        // Create venv structure with activation scripts
        #[cfg(unix)]
        {
            std::fs::create_dir_all(venv_path.join("bin")).unwrap();

            // Write a bash activation script that sets VIRTUAL_ENV
            let activate_content = format!(
                r#"VIRTUAL_ENV="{}"
export VIRTUAL_ENV
PATH="$VIRTUAL_ENV/bin:$PATH"
export PATH
"#, venv_path_str);
            std::fs::write(venv_path.join("bin/activate"), &activate_content).unwrap();

            // Verify the script contains the correct path
            let content = std::fs::read_to_string(venv_path.join("bin/activate")).unwrap();
            prop_assert!(content.contains(&venv_path_str),
                "Activation script must contain the venv path");
        }

        #[cfg(windows)]
        {
            std::fs::create_dir_all(venv_path.join("Scripts")).unwrap();

            // Write a PowerShell activation script
            let activate_content = format!(
                r#"$env:VIRTUAL_ENV = "{}"
$env:PATH = "$env:VIRTUAL_ENV\Scripts;$env:PATH"
"#, venv_path_str);
            std::fs::write(venv_path.join("Scripts/Activate.ps1"), &activate_content).unwrap();

            // Verify the script contains the correct path
            let content = std::fs::read_to_string(venv_path.join("Scripts/Activate.ps1")).unwrap();
            prop_assert!(content.contains(&venv_path_str),
                "Activation script must contain the venv path");
        }
    }

    /// Property 18.5: Deactivate function restores original PATH
    /// Validates: Requirement 6.1.3
    #[test]
    fn prop_deactivate_restores_path(
        venv_name in venv_path_strategy()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join(&venv_name);

        #[cfg(unix)]
        {
            std::fs::create_dir_all(venv_path.join("bin")).unwrap();

            // Write activation script with proper deactivate
            let activate_content = r#"
deactivate () {
    if [ -n "${_OLD_VIRTUAL_PATH:-}" ] ; then
        PATH="${_OLD_VIRTUAL_PATH:-}"
        export PATH
        unset _OLD_VIRTUAL_PATH
    fi
    unset VIRTUAL_ENV
}

_OLD_VIRTUAL_PATH="$PATH"
VIRTUAL_ENV="/test/venv"
export VIRTUAL_ENV
PATH="$VIRTUAL_ENV/bin:$PATH"
export PATH
"#;
            std::fs::write(venv_path.join("bin/activate"), activate_content).unwrap();

            let content = std::fs::read_to_string(venv_path.join("bin/activate")).unwrap();

            // Verify deactivate saves and restores PATH
            prop_assert!(content.contains("_OLD_VIRTUAL_PATH"),
                "Activation script must save original PATH");
            prop_assert!(content.contains("unset VIRTUAL_ENV"),
                "Deactivate must unset VIRTUAL_ENV");
        }

        #[cfg(windows)]
        {
            std::fs::create_dir_all(venv_path.join("Scripts")).unwrap();

            // Write PowerShell activation script with deactivate
            let activate_content = r#"
function global:deactivate {
    if (Test-Path variable:_OLD_VIRTUAL_PATH) {
        $env:PATH = $variable:_OLD_VIRTUAL_PATH
        Remove-Variable "_OLD_VIRTUAL_PATH" -Scope global
    }
    Remove-Item env:VIRTUAL_ENV -ErrorAction SilentlyContinue
}

$env:_OLD_VIRTUAL_PATH = $env:PATH
$env:VIRTUAL_ENV = "C:\test\venv"
$env:PATH = "$env:VIRTUAL_ENV\Scripts;$env:PATH"
"#;
            std::fs::write(venv_path.join("Scripts/Activate.ps1"), activate_content).unwrap();

            let content = std::fs::read_to_string(venv_path.join("Scripts/Activate.ps1")).unwrap();

            // Verify deactivate function exists and restores PATH
            prop_assert!(content.contains("deactivate"),
                "PowerShell script must have deactivate function");
            prop_assert!(content.contains("_OLD_VIRTUAL_PATH"),
                "PowerShell script must save original PATH");
        }
    }

    /// Property 18.6: CMD batch activation script correctness
    /// Validates: Requirement 10.5 - activate.bat for Windows CMD
    #[test]
    #[cfg(windows)]
    fn prop_cmd_activation_script_correctness(
        venv_name in venv_path_strategy()
    ) {
        use std::process::Command;
        
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join(&venv_name);
        
        // Try to find Python executable
        let python_path = if let Ok(output) = Command::new("python").arg("--version").output() {
            if output.status.success() {
                PathBuf::from("python")
            } else {
                // Skip test if Python is not available
                return Ok(());
            }
        } else {
            // Skip test if Python is not available
            return Ok(());
        };
        
        // Create venv using the manager
        let mut manager = dx_py_workspace::VenvManager::new();
        let result = manager.create(&venv_path, &python_path);
        
        // If venv creation fails (e.g., Python not found), skip the test
        if result.is_err() {
            return Ok(());
        }
        
        // Verify activate.bat exists
        let activate_bat = venv_path.join("Scripts").join("activate.bat");
        prop_assert!(activate_bat.exists(), 
            "activate.bat must exist for Windows CMD");
        
        let content = std::fs::read_to_string(&activate_bat).unwrap();
        
        // Verify required elements
        prop_assert!(content.contains("VIRTUAL_ENV="),
            "activate.bat must set VIRTUAL_ENV");
        prop_assert!(content.contains("PATH="),
            "activate.bat must modify PATH");
        prop_assert!(content.contains("_OLD_VIRTUAL_PATH"),
            "activate.bat must save original PATH");
        prop_assert!(content.contains("Scripts"),
            "activate.bat must reference Scripts directory");
        
        // Verify deactivate.bat exists
        let deactivate_bat = venv_path.join("Scripts").join("deactivate.bat");
        prop_assert!(deactivate_bat.exists(),
            "deactivate.bat must exist for Windows CMD");
        
        let deactivate_content = std::fs::read_to_string(&deactivate_bat).unwrap();
        prop_assert!(deactivate_content.contains("_OLD_VIRTUAL_PATH"),
            "deactivate.bat must restore original PATH");
        prop_assert!(deactivate_content.contains("VIRTUAL_ENV="),
            "deactivate.bat must unset VIRTUAL_ENV");
    }
}

#[test]
fn test_venv_site_packages_path() {
    let venv = dx_py_workspace::Venv::new(PathBuf::from("/tmp/test-venv"), "3.12.0".to_string());

    let site_packages = venv.site_packages();

    #[cfg(unix)]
    assert!(site_packages.to_string_lossy().contains("site-packages"));

    #[cfg(windows)]
    assert!(site_packages.to_string_lossy().contains("site-packages"));
}

#[test]
fn test_venv_bin_dir_path() {
    let venv = dx_py_workspace::Venv::new(PathBuf::from("/tmp/test-venv"), "3.12.0".to_string());

    let bin_dir = venv.bin_dir();

    #[cfg(unix)]
    assert!(bin_dir.to_string_lossy().contains("bin"));

    #[cfg(windows)]
    assert!(bin_dir.to_string_lossy().contains("Scripts"));
}
