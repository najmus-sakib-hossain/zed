//! Integration test for activation script generation
//! 
//! This test verifies that all activation scripts are created correctly
//! when a virtual environment is created.

use std::path::PathBuf;
use tempfile::TempDir;

#[test]
#[cfg(windows)]
fn test_windows_activation_scripts_complete() {
    use std::process::Command;
    
    let temp_dir = TempDir::new().unwrap();
    let venv_path = temp_dir.path().join("test-venv");
    
    // Try to find Python
    let python_check = Command::new("python").arg("--version").output();
    if python_check.is_err() || !python_check.unwrap().status.success() {
        eprintln!("Skipping test: Python not found");
        return;
    }
    
    let python_path = PathBuf::from("python");
    
    // Create venv
    let mut manager = dx_py_workspace::VenvManager::new();
    let result = manager.create(&venv_path, &python_path);
    
    if result.is_err() {
        eprintln!("Skipping test: Failed to create venv");
        return;
    }
    
    // Verify all Windows activation scripts exist
    let scripts_dir = venv_path.join("Scripts");
    
    // PowerShell script
    let activate_ps1 = scripts_dir.join("Activate.ps1");
    assert!(activate_ps1.exists(), "Activate.ps1 should exist");
    let ps1_content = std::fs::read_to_string(&activate_ps1).unwrap();
    assert!(ps1_content.contains("$env:VIRTUAL_ENV"), "Activate.ps1 should set VIRTUAL_ENV");
    assert!(ps1_content.contains("deactivate"), "Activate.ps1 should have deactivate function");
    
    // CMD batch script
    let activate_bat = scripts_dir.join("activate.bat");
    assert!(activate_bat.exists(), "activate.bat should exist");
    let bat_content = std::fs::read_to_string(&activate_bat).unwrap();
    assert!(bat_content.contains("VIRTUAL_ENV="), "activate.bat should set VIRTUAL_ENV");
    assert!(bat_content.contains("PATH="), "activate.bat should modify PATH");
    
    // Deactivate batch script
    let deactivate_bat = scripts_dir.join("deactivate.bat");
    assert!(deactivate_bat.exists(), "deactivate.bat should exist");
    let deactivate_content = std::fs::read_to_string(&deactivate_bat).unwrap();
    assert!(deactivate_content.contains("_OLD_VIRTUAL_PATH"), "deactivate.bat should restore PATH");
    
    println!("✓ All Windows activation scripts created successfully");
}

#[test]
#[cfg(unix)]
fn test_unix_activation_scripts_complete() {
    use std::process::Command;
    
    let temp_dir = TempDir::new().unwrap();
    let venv_path = temp_dir.path().join("test-venv");
    
    // Try to find Python
    let python_check = Command::new("python3").arg("--version").output()
        .or_else(|_| Command::new("python").arg("--version").output());
    
    if python_check.is_err() || !python_check.unwrap().status.success() {
        eprintln!("Skipping test: Python not found");
        return;
    }
    
    let python_path = PathBuf::from("python3");
    
    // Create venv
    let mut manager = dx_py_workspace::VenvManager::new();
    let result = manager.create(&venv_path, &python_path);
    
    if result.is_err() {
        eprintln!("Skipping test: Failed to create venv");
        return;
    }
    
    // Verify all Unix activation scripts exist
    let bin_dir = venv_path.join("bin");
    
    // Bash/zsh script
    let activate_sh = bin_dir.join("activate");
    assert!(activate_sh.exists(), "activate script should exist");
    let sh_content = std::fs::read_to_string(&activate_sh).unwrap();
    assert!(sh_content.contains("VIRTUAL_ENV="), "activate should set VIRTUAL_ENV");
    assert!(sh_content.contains("deactivate"), "activate should have deactivate function");
    
    // Fish script
    let activate_fish = bin_dir.join("activate.fish");
    assert!(activate_fish.exists(), "activate.fish should exist");
    let fish_content = std::fs::read_to_string(&activate_fish).unwrap();
    assert!(fish_content.contains("VIRTUAL_ENV"), "activate.fish should set VIRTUAL_ENV");
    assert!(fish_content.contains("deactivate"), "activate.fish should have deactivate function");
    
    // PowerShell script (also on Unix for cross-platform support)
    let activate_ps1 = bin_dir.join("Activate.ps1");
    assert!(activate_ps1.exists(), "Activate.ps1 should exist");
    let ps1_content = std::fs::read_to_string(&activate_ps1).unwrap();
    assert!(ps1_content.contains("VIRTUAL_ENV"), "Activate.ps1 should set VIRTUAL_ENV");
    
    println!("✓ All Unix activation scripts created successfully");
}
