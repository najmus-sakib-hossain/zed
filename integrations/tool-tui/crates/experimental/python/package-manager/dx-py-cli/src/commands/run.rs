//! Run a command in the virtual environment
//!
//! This command properly activates the virtual environment and runs
//! the specified command with the correct PATH and environment variables.

use std::path::Path;
use std::process::Command;

use dx_py_core::Result;

/// Run the run command
pub fn run(command: &[String]) -> Result<()> {
    if command.is_empty() {
        return Err(dx_py_core::Error::Cache(
            "No command specified. Usage: dx-py run <command> [args...]".to_string(),
        ));
    }

    let venv_path = Path::new(".venv");
    if !venv_path.exists() {
        return Err(dx_py_core::Error::Cache(
            "No virtual environment found. Run 'dx-py init' first.".to_string(),
        ));
    }

    // Get the bin/Scripts directory
    #[cfg(unix)]
    let bin_dir = venv_path.join("bin");
    #[cfg(windows)]
    let bin_dir = venv_path.join("Scripts");

    if !bin_dir.exists() {
        return Err(dx_py_core::Error::Cache(format!(
            "Virtual environment bin directory not found: {}",
            bin_dir.display()
        )));
    }

    // Build the PATH with venv bin directory first
    let path_var = std::env::var("PATH").unwrap_or_default();
    #[cfg(unix)]
    let new_path = format!("{}:{}", bin_dir.display(), path_var);
    #[cfg(windows)]
    let new_path = format!("{};{}", bin_dir.display(), path_var);

    // Get the absolute path to the venv
    let venv_abs = std::fs::canonicalize(venv_path).unwrap_or_else(|_| venv_path.to_path_buf());

    // Check if the command exists in the venv
    let cmd_name = &command[0];

    #[cfg(unix)]
    let possible_paths = vec![bin_dir.join(cmd_name)];

    #[cfg(windows)]
    let possible_paths = [
        bin_dir.join(format!("{}.exe", cmd_name)),
        bin_dir.join(format!("{}.bat", cmd_name)),
        bin_dir.join(format!("{}.cmd", cmd_name)),
        bin_dir.join(format!("{}-script.py", cmd_name)),
        bin_dir.join(cmd_name),
    ];

    let actual_cmd = possible_paths
        .iter()
        .find(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| cmd_name.clone());

    // Set up environment variables for proper venv activation
    let mut cmd = Command::new(&actual_cmd);
    cmd.args(&command[1..]).env("PATH", &new_path).env("VIRTUAL_ENV", &venv_abs);

    // Remove PYTHONHOME if set (can interfere with venv)
    cmd.env_remove("PYTHONHOME");

    // Set PYTHONPATH to include site-packages
    #[cfg(unix)]
    {
        // Find the Python version directory
        let lib_dir = venv_path.join("lib");
        if lib_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&lib_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir()
                        && path
                            .file_name()
                            .map(|n| n.to_string_lossy().starts_with("python"))
                            .unwrap_or(false)
                    {
                        let site_packages = path.join("site-packages");
                        if site_packages.exists() {
                            cmd.env("PYTHONPATH", site_packages);
                            break;
                        }
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        let site_packages = venv_path.join("Lib").join("site-packages");
        if site_packages.exists() {
            cmd.env("PYTHONPATH", site_packages);
        }
    }

    // Run the command
    let status = cmd.status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            dx_py_core::Error::Cache(format!(
                "Command '{}' not found. Is it installed in the virtual environment?",
                cmd_name
            ))
        } else {
            dx_py_core::Error::Cache(format!("Failed to run command: {}", e))
        }
    })?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
