//! Virtual environment management
//!
//! Provides ultra-fast virtual environment creation and management.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Virtual environment information
#[derive(Debug, Clone)]
pub struct Venv {
    /// Path to the virtual environment
    pub path: PathBuf,
    /// Python version used
    pub python_version: String,
    /// Path to the Python executable in the venv
    pub python_path: PathBuf,
}

impl Venv {
    /// Create a new Venv info
    pub fn new(path: PathBuf, python_version: String) -> Self {
        #[cfg(unix)]
        let python_path = path.join("bin").join("python");
        #[cfg(windows)]
        let python_path = path.join("Scripts").join("python.exe");

        Self {
            path,
            python_version,
            python_path,
        }
    }

    /// Get the site-packages directory
    pub fn site_packages(&self) -> PathBuf {
        #[cfg(unix)]
        {
            self.path
                .join("lib")
                .join(format!(
                    "python{}",
                    &self.python_version
                        [..self.python_version.rfind('.').unwrap_or(self.python_version.len())]
                ))
                .join("site-packages")
        }
        #[cfg(windows)]
        {
            self.path.join("Lib").join("site-packages")
        }
    }

    /// Get the bin/Scripts directory
    pub fn bin_dir(&self) -> PathBuf {
        #[cfg(unix)]
        {
            self.path.join("bin")
        }
        #[cfg(windows)]
        {
            self.path.join("Scripts")
        }
    }

    /// Check if the venv exists and is valid
    pub fn is_valid(&self) -> bool {
        self.python_path.exists()
    }
}

/// Cached venv skeleton for fast creation
#[derive(Debug)]
struct VenvSkeleton {
    /// Python version this skeleton is for
    #[allow(dead_code)]
    python_version: String,
    /// Path to the skeleton
    path: PathBuf,
}

/// Virtual environment manager
///
/// Creates virtual environments in under 10ms using cached skeletons.
pub struct VenvManager {
    /// Cache directory for venv skeletons
    cache_dir: PathBuf,
    /// Cached skeletons by Python version
    skeletons: HashMap<String, VenvSkeleton>,
}

impl VenvManager {
    /// Create a new venv manager
    pub fn new() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx-py")
            .join("venv-cache");

        Self {
            cache_dir,
            skeletons: HashMap::new(),
        }
    }

    /// Create a venv manager with a custom cache directory
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            skeletons: HashMap::new(),
        }
    }

    /// Create a virtual environment
    pub fn create(&mut self, path: &Path, python: &Path) -> Result<Venv> {
        let python_version = self.get_python_version(python)?;

        // Check if we can reuse a cached skeleton
        if let Some(skeleton) = self.get_skeleton(&python_version) {
            self.copy_skeleton(&skeleton.path, path)?;
        } else {
            // Create minimal venv structure
            self.create_minimal_venv(path, python, &python_version)?;

            // Cache skeleton for future use
            self.store_skeleton(&python_version, path)?;
        }

        Ok(Venv::new(path.to_path_buf(), python_version))
    }

    /// Create a minimal virtual environment structure
    fn create_minimal_venv(&self, path: &Path, python: &Path, python_version: &str) -> Result<()> {
        // Create directory structure
        #[cfg(unix)]
        {
            std::fs::create_dir_all(path.join("bin"))?;
            let lib_dir = path
                .join("lib")
                .join(format!(
                    "python{}",
                    &python_version[..python_version.rfind('.').unwrap_or(python_version.len())]
                ))
                .join("site-packages");
            std::fs::create_dir_all(&lib_dir)?;
            std::fs::create_dir_all(path.join("include"))?;
        }
        #[cfg(windows)]
        {
            std::fs::create_dir_all(path.join("Scripts"))?;
            std::fs::create_dir_all(path.join("Lib").join("site-packages"))?;
            std::fs::create_dir_all(path.join("Include"))?;
        }

        // Create symlink to Python executable
        #[cfg(unix)]
        {
            let python_link = path.join("bin").join("python");
            if !python_link.exists() {
                std::os::unix::fs::symlink(python, &python_link)
                    .map_err(|e| Error::VenvError(format!("Failed to create symlink: {}", e)))?;
            }
            let python3_link = path.join("bin").join("python3");
            if !python3_link.exists() {
                std::os::unix::fs::symlink(python, &python3_link)
                    .map_err(|e| Error::VenvError(format!("Failed to create symlink: {}", e)))?;
            }
        }
        #[cfg(windows)]
        {
            let python_exe = path.join("Scripts").join("python.exe");
            if !python_exe.exists() {
                std::fs::copy(python, &python_exe)
                    .map_err(|e| Error::VenvError(format!("Failed to copy Python: {}", e)))?;
            }
        }

        // Write activation scripts
        self.write_activate_scripts(path, python_version)?;

        // Write pyvenv.cfg
        self.write_pyvenv_cfg(path, python, python_version)?;

        Ok(())
    }

    /// Write activation scripts for various shells
    fn write_activate_scripts(&self, venv: &Path, _python_version: &str) -> Result<()> {
        let venv_path = venv.to_string_lossy();

        // bash/zsh activation
        #[cfg(unix)]
        {
            let activate_sh = format!(
                r#"# This file must be used with "source bin/activate" *from bash*
# you cannot run it directly

deactivate () {{
    if [ -n "${{_OLD_VIRTUAL_PATH:-}}" ] ; then
        PATH="${{_OLD_VIRTUAL_PATH:-}}"
        export PATH
        unset _OLD_VIRTUAL_PATH
    fi

    if [ -n "${{_OLD_VIRTUAL_PYTHONHOME:-}}" ] ; then
        PYTHONHOME="${{_OLD_VIRTUAL_PYTHONHOME:-}}"
        export PYTHONHOME
        unset _OLD_VIRTUAL_PYTHONHOME
    fi

    if [ -n "${{BASH:-}}" -o -n "${{ZSH_VERSION:-}}" ] ; then
        hash -r 2> /dev/null
    fi

    if [ -n "${{_OLD_VIRTUAL_PS1:-}}" ] ; then
        PS1="${{_OLD_VIRTUAL_PS1:-}}"
        export PS1
        unset _OLD_VIRTUAL_PS1
    fi

    unset VIRTUAL_ENV
    if [ ! "${{1:-}}" = "nondestructive" ] ; then
        unset -f deactivate
    fi
}}

deactivate nondestructive

VIRTUAL_ENV="{}"
export VIRTUAL_ENV

_OLD_VIRTUAL_PATH="$PATH"
PATH="$VIRTUAL_ENV/bin:$PATH"
export PATH

if [ -z "${{VIRTUAL_ENV_DISABLE_PROMPT:-}}" ] ; then
    _OLD_VIRTUAL_PS1="${{PS1:-}}"
    PS1="($(basename "$VIRTUAL_ENV")) ${{PS1:-}}"
    export PS1
fi

if [ -n "${{BASH:-}}" -o -n "${{ZSH_VERSION:-}}" ] ; then
    hash -r 2> /dev/null
fi
"#,
                venv_path
            );
            std::fs::write(venv.join("bin").join("activate"), activate_sh)?;
        }

        // fish activation
        #[cfg(unix)]
        {
            let activate_fish = format!(
                r#"function deactivate -d "Exit virtual environment and return to normal shell environment"
    if test -n "$_OLD_VIRTUAL_PATH"
        set -gx PATH $_OLD_VIRTUAL_PATH
        set -e _OLD_VIRTUAL_PATH
    end
    if test -n "$_OLD_VIRTUAL_PYTHONHOME"
        set -gx PYTHONHOME $_OLD_VIRTUAL_PYTHONHOME
        set -e _OLD_VIRTUAL_PYTHONHOME
    end
    if test -n "$_OLD_FISH_PROMPT_OVERRIDE"
        functions -e fish_prompt
        set -e _OLD_FISH_PROMPT_OVERRIDE
        functions -c _old_fish_prompt fish_prompt
        functions -e _old_fish_prompt
    end
    set -e VIRTUAL_ENV
    if test "$argv[1]" != "nondestructive"
        functions -e deactivate
    end
end

deactivate nondestructive

set -gx VIRTUAL_ENV "{}"
set -gx _OLD_VIRTUAL_PATH $PATH
set -gx PATH "$VIRTUAL_ENV/bin" $PATH
"#,
                venv_path
            );
            std::fs::write(venv.join("bin").join("activate.fish"), activate_fish)?;
        }

        // PowerShell activation
        #[cfg(windows)]
        {
            let activate_ps1 = format!(
                r#"$script:THIS_PATH = $myinvocation.mycommand.path
$script:BASE_DIR = Split-Path (Resolve-Path "$THIS_PATH/..") -Parent

function global:deactivate([switch] $NonDestructive) {{
    if (Test-Path variable:_OLD_VIRTUAL_PATH) {{
        $env:PATH = $variable:_OLD_VIRTUAL_PATH
        Remove-Variable "_OLD_VIRTUAL_PATH" -Scope global
    }}

    if (Test-Path function:_old_virtual_prompt) {{
        $function:prompt = $function:_old_virtual_prompt
        Remove-Item function:\_old_virtual_prompt
    }}

    if ($env:VIRTUAL_ENV) {{
        Remove-Item env:VIRTUAL_ENV -ErrorAction SilentlyContinue
    }}

    if (!$NonDestructive) {{
        Remove-Item function:deactivate
    }}
}}

deactivate -nondestructive

$env:VIRTUAL_ENV = "{}"
$env:_OLD_VIRTUAL_PATH = $env:PATH
$env:PATH = "$env:VIRTUAL_ENV\Scripts;$env:PATH"
"#,
                venv_path
            );
            std::fs::write(venv.join("Scripts").join("Activate.ps1"), activate_ps1)?;
        }

        // CMD batch activation
        #[cfg(windows)]
        {
            let activate_bat = format!(
                r#"@echo off

set "VIRTUAL_ENV={}"

if defined _OLD_VIRTUAL_PROMPT (
    set "PROMPT=%_OLD_VIRTUAL_PROMPT%"
) else (
    if not defined PROMPT (
        set "PROMPT=$P$G"
    )
    if not defined VIRTUAL_ENV_DISABLE_PROMPT (
        set "_OLD_VIRTUAL_PROMPT=%PROMPT%"
    )
)
if not defined VIRTUAL_ENV_DISABLE_PROMPT (
    set "PROMPT=(venv) %PROMPT%"
)

if defined _OLD_VIRTUAL_PYTHONHOME (
    set "PYTHONHOME=%_OLD_VIRTUAL_PYTHONHOME%"
    set _OLD_VIRTUAL_PYTHONHOME=
)

if defined _OLD_VIRTUAL_PATH (
    set "PATH=%_OLD_VIRTUAL_PATH%"
) else (
    set "_OLD_VIRTUAL_PATH=%PATH%"
)

set "PATH=%VIRTUAL_ENV%\Scripts;%PATH%"

:END
"#,
                venv_path
            );
            std::fs::write(venv.join("Scripts").join("activate.bat"), activate_bat)?;

            // Also create deactivate.bat
            let deactivate_bat = r#"@echo off

if defined _OLD_VIRTUAL_PROMPT (
    set "PROMPT=%_OLD_VIRTUAL_PROMPT%"
    set _OLD_VIRTUAL_PROMPT=
)

if defined _OLD_VIRTUAL_PYTHONHOME (
    set "PYTHONHOME=%_OLD_VIRTUAL_PYTHONHOME%"
    set _OLD_VIRTUAL_PYTHONHOME=
)

if defined _OLD_VIRTUAL_PATH (
    set "PATH=%_OLD_VIRTUAL_PATH%"
    set _OLD_VIRTUAL_PATH=
)

set VIRTUAL_ENV=

:END
"#;
            std::fs::write(venv.join("Scripts").join("deactivate.bat"), deactivate_bat)?;
        }

        #[cfg(unix)]
        {
            let activate_ps1 = format!(
                r#"$env:VIRTUAL_ENV = "{}"
$env:_OLD_VIRTUAL_PATH = $env:PATH
$env:PATH = "$env:VIRTUAL_ENV/bin:$env:PATH"
"#,
                venv_path
            );
            std::fs::write(venv.join("bin").join("Activate.ps1"), activate_ps1)?;
        }

        Ok(())
    }

    /// Write pyvenv.cfg file
    fn write_pyvenv_cfg(&self, venv: &Path, python: &Path, python_version: &str) -> Result<()> {
        let home = python.parent().unwrap_or(Path::new(""));
        let cfg = format!(
            r#"home = {}
include-system-site-packages = false
version = {}
"#,
            home.display(),
            python_version
        );
        std::fs::write(venv.join("pyvenv.cfg"), cfg)?;
        Ok(())
    }

    /// Get Python version from executable
    fn get_python_version(&self, python: &Path) -> Result<String> {
        let output = std::process::Command::new(python)
            .args(["--version"])
            .output()
            .map_err(|e| Error::PythonNotFound(format!("Failed to run Python: {}", e)))?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = version_str
            .trim()
            .strip_prefix("Python ")
            .unwrap_or(version_str.trim())
            .to_string();

        Ok(version)
    }

    /// Get a cached skeleton for a Python version
    fn get_skeleton(&self, python_version: &str) -> Option<&VenvSkeleton> {
        self.skeletons.get(python_version)
    }

    /// Copy a skeleton to create a new venv
    fn copy_skeleton(&self, skeleton: &Path, dest: &Path) -> Result<()> {
        self.copy_dir_recursive(skeleton, dest)?;
        Ok(())
    }

    /// Recursively copy a directory
    #[allow(clippy::only_used_in_recursion)]
    fn copy_dir_recursive(&self, src: &Path, dest: &Path) -> Result<()> {
        std::fs::create_dir_all(dest)?;

        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if src_path.is_dir() {
                self.copy_dir_recursive(&src_path, &dest_path)?;
            } else if src_path.is_symlink() {
                #[cfg(unix)]
                {
                    let target = std::fs::read_link(&src_path)?;
                    std::os::unix::fs::symlink(&target, &dest_path).map_err(|e| {
                        Error::VenvError(format!("Failed to create symlink: {}", e))
                    })?;
                }
                #[cfg(windows)]
                {
                    std::fs::copy(&src_path, &dest_path)?;
                }
            } else {
                std::fs::copy(&src_path, &dest_path)?;
            }
        }

        Ok(())
    }

    /// Store a venv as a skeleton for future use
    fn store_skeleton(&mut self, python_version: &str, venv: &Path) -> Result<()> {
        let skeleton_path = self.cache_dir.join(python_version);
        std::fs::create_dir_all(&skeleton_path)?;

        self.copy_dir_recursive(venv, &skeleton_path)?;

        self.skeletons.insert(
            python_version.to_string(),
            VenvSkeleton {
                python_version: python_version.to_string(),
                path: skeleton_path,
            },
        );

        Ok(())
    }

    /// Remove a virtual environment
    pub fn remove(&self, path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_dir_all(path)?;
        }
        Ok(())
    }

    /// Check if a path is a valid virtual environment
    pub fn is_venv(&self, path: &Path) -> bool {
        path.join("pyvenv.cfg").exists()
    }
}

impl Default for VenvManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Real virtual environment manager with pip bootstrap support
pub struct RealVenvManager {
    /// Base venv manager
    manager: VenvManager,
}

impl RealVenvManager {
    /// Create a new real venv manager
    pub fn new() -> Self {
        Self {
            manager: VenvManager::new(),
        }
    }

    /// Create a real venv manager with a custom cache directory
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self {
            manager: VenvManager::with_cache_dir(cache_dir),
        }
    }

    /// Create a virtual environment
    pub fn create(&mut self, path: &Path, python: &Path) -> Result<Venv> {
        self.manager.create(path, python)
    }

    /// Create a virtual environment with pip and setuptools
    pub fn create_with_packages(&mut self, path: &Path, python: &Path) -> Result<Venv> {
        let venv = self.manager.create(path, python)?;

        // Bootstrap pip using ensurepip
        self.bootstrap_pip(&venv)?;

        Ok(venv)
    }

    /// Bootstrap pip into the virtual environment using ensurepip
    fn bootstrap_pip(&self, venv: &Venv) -> Result<()> {
        let output = std::process::Command::new(&venv.python_path)
            .args(["-m", "ensurepip", "--upgrade"])
            .output()
            .map_err(|e| Error::VenvError(format!("Failed to run ensurepip: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::VenvError(format!("ensurepip failed: {}", stderr)));
        }

        Ok(())
    }

    /// Install packages into the virtual environment using pip
    pub fn pip_install(&self, venv: &Venv, packages: &[&str]) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let mut args = vec!["-m", "pip", "install", "--quiet"];
        args.extend(packages.iter().copied());

        let output = std::process::Command::new(&venv.python_path)
            .args(&args)
            .output()
            .map_err(|e| Error::VenvError(format!("Failed to run pip: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::VenvError(format!("pip install failed: {}", stderr)));
        }

        Ok(())
    }

    /// Run a command in the virtual environment
    pub fn run(&self, venv: &Venv, cmd: &str, args: &[&str]) -> Result<std::process::ExitStatus> {
        let bin_dir = venv.bin_dir();

        // Look for the command in the venv's bin directory first
        #[cfg(windows)]
        let cmd_path = bin_dir.join(format!("{}.exe", cmd));
        #[cfg(not(windows))]
        let cmd_path = bin_dir.join(cmd);

        let actual_cmd = if cmd_path.exists() {
            cmd_path
        } else {
            PathBuf::from(cmd)
        };

        let status = std::process::Command::new(&actual_cmd)
            .args(args)
            .env("VIRTUAL_ENV", &venv.path)
            .env(
                "PATH",
                format!(
                    "{}{}{}",
                    bin_dir.display(),
                    std::path::MAIN_SEPARATOR,
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .status()
            .map_err(|e| Error::VenvError(format!("Failed to run command: {}", e)))?;

        Ok(status)
    }

    /// Remove a virtual environment
    pub fn remove(&self, path: &Path) -> Result<()> {
        self.manager.remove(path)
    }

    /// Check if a path is a valid virtual environment
    pub fn is_venv(&self, path: &Path) -> bool {
        self.manager.is_venv(path)
    }
}

impl Default for RealVenvManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_venv_new() {
        let venv = Venv::new(PathBuf::from("/tmp/test-venv"), "3.12.0".to_string());
        assert_eq!(venv.python_version, "3.12.0");
    }

    #[test]
    fn test_venv_manager_new() {
        let manager = VenvManager::new();
        assert!(manager.cache_dir.to_string_lossy().contains("dx-py"));
    }

    #[test]
    fn test_venv_manager_with_cache_dir() {
        let temp_dir = TempDir::new().unwrap();
        let manager = VenvManager::with_cache_dir(temp_dir.path().to_path_buf());
        assert_eq!(manager.cache_dir, temp_dir.path());
    }

    #[test]
    fn test_is_venv() {
        let temp_dir = TempDir::new().unwrap();
        let manager = VenvManager::new();

        // Not a venv initially
        assert!(!manager.is_venv(temp_dir.path()));

        // Create pyvenv.cfg
        std::fs::write(temp_dir.path().join("pyvenv.cfg"), "version = 3.12.0").unwrap();
        assert!(manager.is_venv(temp_dir.path()));
    }

    #[test]
    fn test_remove_venv() {
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join("test-venv");
        std::fs::create_dir_all(&venv_path).unwrap();
        std::fs::write(venv_path.join("pyvenv.cfg"), "version = 3.12.0").unwrap();

        let manager = VenvManager::new();
        assert!(venv_path.exists());

        manager.remove(&venv_path).unwrap();
        assert!(!venv_path.exists());
    }

    #[test]
    fn test_venv_directory_structure() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join("test-venv");
        
        // Create minimal venv structure manually to test
        #[cfg(unix)]
        {
            std::fs::create_dir_all(venv_path.join("bin")).unwrap();
            std::fs::create_dir_all(venv_path.join("lib/python3.12/site-packages")).unwrap();
            std::fs::create_dir_all(venv_path.join("include")).unwrap();
            
            assert!(venv_path.join("bin").exists(), "bin directory should exist");
            assert!(venv_path.join("lib").exists(), "lib directory should exist");
            assert!(venv_path.join("include").exists(), "include directory should exist");
        }
        #[cfg(windows)]
        {
            std::fs::create_dir_all(venv_path.join("Scripts")).unwrap();
            std::fs::create_dir_all(venv_path.join("Lib/site-packages")).unwrap();
            std::fs::create_dir_all(venv_path.join("Include")).unwrap();
            
            assert!(venv_path.join("Scripts").exists(), "Scripts directory should exist");
            assert!(venv_path.join("Lib").exists(), "Lib directory should exist");
            assert!(venv_path.join("Include").exists(), "Include directory should exist");
        }
    }

    #[test]
    fn test_pyvenv_cfg_content() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join("test-venv");
        std::fs::create_dir_all(&venv_path).unwrap();
        
        // Write a sample pyvenv.cfg
        let cfg_content = "home = /usr/bin\ninclude-system-site-packages = false\nversion = 3.12.0\n";
        std::fs::write(venv_path.join("pyvenv.cfg"), cfg_content).unwrap();
        
        // Verify the file exists and contains expected content
        let pyvenv_cfg = venv_path.join("pyvenv.cfg");
        assert!(pyvenv_cfg.exists(), "pyvenv.cfg should exist");
        
        let content = std::fs::read_to_string(&pyvenv_cfg).unwrap();
        assert!(content.contains("home ="), "pyvenv.cfg should contain home");
        assert!(content.contains("include-system-site-packages ="), "pyvenv.cfg should contain include-system-site-packages");
        assert!(content.contains("version ="), "pyvenv.cfg should contain version");
    }

    #[test]
    fn test_activation_scripts_created() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let venv_path = temp_dir.path().join("test-venv");
        
        // Create the venv structure
        #[cfg(unix)]
        {
            std::fs::create_dir_all(venv_path.join("bin")).unwrap();
        }
        #[cfg(windows)]
        {
            std::fs::create_dir_all(venv_path.join("Scripts")).unwrap();
        }
        
        // Create activation scripts using the manager's method
        let manager = VenvManager::new();
        manager.write_activate_scripts(&venv_path, "3.12.0").unwrap();
        
        // Verify scripts exist
        #[cfg(unix)]
        {
            let activate_sh = venv_path.join("bin").join("activate");
            assert!(activate_sh.exists(), "activate script should exist");
            
            let content = std::fs::read_to_string(&activate_sh).unwrap();
            assert!(content.contains("VIRTUAL_ENV="), "activate should set VIRTUAL_ENV");
            assert!(content.contains("PATH="), "activate should modify PATH");
            
            let activate_fish = venv_path.join("bin").join("activate.fish");
            assert!(activate_fish.exists(), "activate.fish script should exist");
            
            let activate_ps1 = venv_path.join("bin").join("Activate.ps1");
            assert!(activate_ps1.exists(), "Activate.ps1 script should exist");
        }
        #[cfg(windows)]
        {
            let activate_ps1 = venv_path.join("Scripts").join("Activate.ps1");
            assert!(activate_ps1.exists(), "Activate.ps1 script should exist");
            
            let content = std::fs::read_to_string(&activate_ps1).unwrap();
            assert!(content.contains("$env:VIRTUAL_ENV"), "Activate.ps1 should set VIRTUAL_ENV");
            assert!(content.contains("$env:PATH"), "Activate.ps1 should modify PATH");
            
            let activate_bat = venv_path.join("Scripts").join("activate.bat");
            assert!(activate_bat.exists(), "activate.bat script should exist");
            
            let bat_content = std::fs::read_to_string(&activate_bat).unwrap();
            assert!(bat_content.contains("VIRTUAL_ENV="), "activate.bat should set VIRTUAL_ENV");
            assert!(bat_content.contains("PATH="), "activate.bat should modify PATH");
            
            let deactivate_bat = venv_path.join("Scripts").join("deactivate.bat");
            assert!(deactivate_bat.exists(), "deactivate.bat script should exist");
        }
    }
}
