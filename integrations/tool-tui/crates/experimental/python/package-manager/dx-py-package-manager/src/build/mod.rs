//! PEP 517 Build System
//!
//! Implements a PEP 517 build frontend that can build wheels and sdists
//! using any PEP 517 compliant build backend (setuptools, hatchling, flit, etc.)

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use dx_py_compat::PyProjectToml;

use crate::{Error, Result};

/// Default build backend if none specified
pub const DEFAULT_BUILD_BACKEND: &str = "setuptools.build_meta";

/// Default build requirements if none specified
pub const DEFAULT_BUILD_REQUIRES: &[&str] = &["setuptools>=61.0", "wheel"];

/// PEP 517 Build System
///
/// Provides a build frontend that can invoke any PEP 517 compliant build backend
/// to build wheels and source distributions.
pub struct BuildFrontend {
    /// Project directory containing pyproject.toml
    project_dir: PathBuf,
    /// Parsed pyproject.toml
    pyproject: PyProjectToml,
    /// Build backend module path
    build_backend: String,
    /// Build requirements
    build_requires: Vec<String>,
    /// Python interpreter to use
    python: PathBuf,
}

impl BuildFrontend {
    /// Create a new build frontend for the given project directory
    pub fn new(project_dir: &Path) -> Result<Self> {
        let pyproject_path = project_dir.join("pyproject.toml");
        if !pyproject_path.exists() {
            return Err(Error::Cache(format!(
                "No pyproject.toml found in {}",
                project_dir.display()
            )));
        }

        let pyproject = PyProjectToml::load(&pyproject_path)?;

        let (build_backend, build_requires) = match &pyproject.build_system {
            Some(bs) => (
                bs.build_backend.clone().unwrap_or_else(|| DEFAULT_BUILD_BACKEND.to_string()),
                if bs.requires.is_empty() {
                    DEFAULT_BUILD_REQUIRES.iter().map(|s| s.to_string()).collect()
                } else {
                    bs.requires.clone()
                },
            ),
            None => (
                DEFAULT_BUILD_BACKEND.to_string(),
                DEFAULT_BUILD_REQUIRES.iter().map(|s| s.to_string()).collect(),
            ),
        };

        // Find Python interpreter
        let python = Self::find_python()?;

        Ok(Self {
            project_dir: project_dir.to_path_buf(),
            pyproject,
            build_backend,
            build_requires,
            python,
        })
    }

    /// Find a Python interpreter
    fn find_python() -> Result<PathBuf> {
        // Try common Python executable names
        let candidates = if cfg!(windows) {
            vec!["python.exe", "python3.exe", "py.exe"]
        } else {
            vec!["python3", "python"]
        };

        for candidate in candidates {
            if let Ok(output) = Command::new(candidate)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                if output.success() {
                    return Ok(PathBuf::from(candidate));
                }
            }
        }

        Err(Error::Cache(
            "No Python interpreter found. Please install Python 3.8+".to_string(),
        ))
    }

    /// Get the project name
    pub fn name(&self) -> Option<&str> {
        self.pyproject.name()
    }

    /// Get the project version
    pub fn version(&self) -> Option<&str> {
        self.pyproject.version()
    }

    /// Get the build backend
    pub fn build_backend(&self) -> &str {
        &self.build_backend
    }

    /// Get the build requirements
    pub fn build_requires(&self) -> &[String] {
        &self.build_requires
    }

    /// Create an isolated build environment
    pub fn create_build_env(&self, env_dir: &Path) -> Result<BuildEnvironment> {
        BuildEnvironment::create(env_dir, &self.python, &self.build_requires)
    }

    /// Build a wheel using the build backend
    pub fn build_wheel(&self, output_dir: &Path) -> Result<PathBuf> {
        // Create temporary build environment
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::Cache(format!("Failed to create temp dir: {}", e)))?;
        let build_env = self.create_build_env(temp_dir.path())?;

        // Ensure output directory exists
        std::fs::create_dir_all(output_dir)?;

        // Build the wheel using the backend
        let wheel_path =
            build_env.build_wheel(&self.project_dir, output_dir, &self.build_backend)?;

        Ok(wheel_path)
    }

    /// Build a source distribution using the build backend
    pub fn build_sdist(&self, output_dir: &Path) -> Result<PathBuf> {
        // Create temporary build environment
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::Cache(format!("Failed to create temp dir: {}", e)))?;
        let build_env = self.create_build_env(temp_dir.path())?;

        // Ensure output directory exists
        std::fs::create_dir_all(output_dir)?;

        // Build the sdist using the backend
        let sdist_path =
            build_env.build_sdist(&self.project_dir, output_dir, &self.build_backend)?;

        Ok(sdist_path)
    }
}

/// Isolated build environment for PEP 517 builds
pub struct BuildEnvironment {
    /// Path to the virtual environment
    #[allow(dead_code)]
    env_dir: PathBuf,
    /// Path to the Python interpreter in the venv
    python: PathBuf,
}

impl BuildEnvironment {
    /// Create a new isolated build environment
    pub fn create(env_dir: &Path, python: &Path, requirements: &[String]) -> Result<Self> {
        // Create virtual environment
        let status = Command::new(python)
            .args(["-m", "venv", &env_dir.to_string_lossy()])
            .status()
            .map_err(|e| Error::Cache(format!("Failed to create venv: {}", e)))?;

        if !status.success() {
            return Err(Error::Cache("Failed to create build environment".to_string()));
        }

        // Get path to Python in venv
        let venv_python = if cfg!(windows) {
            env_dir.join("Scripts").join("python.exe")
        } else {
            env_dir.join("bin").join("python")
        };

        let build_env = Self {
            env_dir: env_dir.to_path_buf(),
            python: venv_python,
        };

        // Install build requirements
        if !requirements.is_empty() {
            build_env.install_requirements(requirements)?;
        }

        Ok(build_env)
    }

    /// Install requirements into the build environment
    fn install_requirements(&self, requirements: &[String]) -> Result<()> {
        let mut cmd = Command::new(&self.python);
        cmd.args(["-m", "pip", "install", "--quiet"]);
        for req in requirements {
            cmd.arg(req);
        }

        let status = cmd
            .status()
            .map_err(|e| Error::Cache(format!("Failed to install build requirements: {}", e)))?;

        if !status.success() {
            return Err(Error::Cache("Failed to install build requirements".to_string()));
        }

        Ok(())
    }

    /// Build a wheel using the specified backend
    pub fn build_wheel(
        &self,
        project_dir: &Path,
        output_dir: &Path,
        build_backend: &str,
    ) -> Result<PathBuf> {
        // PEP 517 build_wheel hook invocation
        let project_dir_str = project_dir.to_string_lossy().replace('\\', "\\\\");
        let output_dir_str = output_dir.to_string_lossy().replace('\\', "\\\\");
        let script = format!(
            r#"
import sys
import os
sys.path.insert(0, '.')
os.chdir(r'{project_dir}')

# Import the build backend
backend_path = '{build_backend}'
parts = backend_path.split(':')
module_path = parts[0]
obj_path = parts[1] if len(parts) > 1 else None

import importlib
backend = importlib.import_module(module_path)
if obj_path:
    for attr in obj_path.split('.'):
        backend = getattr(backend, attr)

# Call build_wheel
wheel_name = backend.build_wheel(r'{output_dir}')
print(wheel_name)
"#,
            project_dir = project_dir_str,
            build_backend = build_backend,
            output_dir = output_dir_str,
        );

        let output = Command::new(&self.python)
            .args(["-c", &script])
            .current_dir(project_dir)
            .output()
            .map_err(|e| Error::Cache(format!("Failed to run build_wheel: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Cache(format!("build_wheel failed: {}", stderr)));
        }

        let wheel_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(output_dir.join(wheel_name))
    }

    /// Build a source distribution using the specified backend
    pub fn build_sdist(
        &self,
        project_dir: &Path,
        output_dir: &Path,
        build_backend: &str,
    ) -> Result<PathBuf> {
        // PEP 517 build_sdist hook invocation
        let project_dir_str = project_dir.to_string_lossy().replace('\\', "\\\\");
        let output_dir_str = output_dir.to_string_lossy().replace('\\', "\\\\");
        let script = format!(
            r#"
import sys
import os
sys.path.insert(0, '.')
os.chdir(r'{project_dir}')

# Import the build backend
backend_path = '{build_backend}'
parts = backend_path.split(':')
module_path = parts[0]
obj_path = parts[1] if len(parts) > 1 else None

import importlib
backend = importlib.import_module(module_path)
if obj_path:
    for attr in obj_path.split('.'):
        backend = getattr(backend, attr)

# Call build_sdist
sdist_name = backend.build_sdist(r'{output_dir}')
print(sdist_name)
"#,
            project_dir = project_dir_str,
            build_backend = build_backend,
            output_dir = output_dir_str,
        );

        let output = Command::new(&self.python)
            .args(["-c", &script])
            .current_dir(project_dir)
            .output()
            .map_err(|e| Error::Cache(format!("Failed to run build_sdist: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Cache(format!("build_sdist failed: {}", stderr)));
        }

        let sdist_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(output_dir.join(sdist_name))
    }
}
