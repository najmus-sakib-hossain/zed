//! Editable Install Support (PEP 660)
//!
//! Implements editable installs that allow development packages to be
//! imported without reinstallation after source changes.

use std::fs;
use std::path::{Path, PathBuf};

use dx_py_compat::PyProjectToml;

use crate::{Error, Result};

/// Editable install information
#[derive(Debug, Clone)]
pub struct EditableInstall {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Project source directory
    pub source_dir: PathBuf,
    /// Path to .pth file
    pub pth_file: PathBuf,
    /// Path to .dist-info directory
    pub dist_info: PathBuf,
    /// Generated entry point scripts
    pub scripts: Vec<PathBuf>,
}

/// Editable installer for development packages
pub struct EditableInstaller {
    /// Site-packages directory
    site_packages: PathBuf,
}

impl EditableInstaller {
    /// Create a new editable installer
    pub fn new(site_packages: PathBuf) -> Self {
        Self { site_packages }
    }

    /// Get the site-packages directory
    pub fn site_packages(&self) -> &Path {
        &self.site_packages
    }

    /// Install a project in editable mode
    ///
    /// Creates a .pth file that adds the project directory to sys.path,
    /// generates entry point scripts, and creates a minimal .dist-info
    /// directory for package metadata.
    pub fn install(&self, project_dir: &Path) -> Result<EditableInstall> {
        // Load pyproject.toml
        let pyproject_path = project_dir.join("pyproject.toml");
        if !pyproject_path.exists() {
            return Err(Error::Cache(format!(
                "No pyproject.toml found in {}",
                project_dir.display()
            )));
        }

        let pyproject = PyProjectToml::load(&pyproject_path)?;

        let name = pyproject
            .name()
            .ok_or_else(|| Error::Cache("Package name not found in pyproject.toml".to_string()))?;
        let version = pyproject.version().unwrap_or("0.0.0");

        // Normalize package name for filesystem
        let normalized_name = name.replace('-', "_").to_lowercase();

        // Determine the source path to add to sys.path
        let source_path = self.find_source_path(project_dir, &normalized_name)?;

        // Create .pth file
        let pth_file = self.create_pth_file(&normalized_name, &source_path)?;

        // Create .dist-info directory
        let dist_info = self.create_dist_info(&normalized_name, version, project_dir)?;

        // Generate entry point scripts
        let scripts = self.generate_scripts(&pyproject, project_dir)?;

        Ok(EditableInstall {
            name: name.to_string(),
            version: version.to_string(),
            source_dir: project_dir.to_path_buf(),
            pth_file,
            dist_info,
            scripts,
        })
    }

    /// Find the source path to add to sys.path
    ///
    /// Supports both src-layout (src/package) and flat-layout (package/)
    fn find_source_path(&self, project_dir: &Path, normalized_name: &str) -> Result<PathBuf> {
        // Check for src-layout first
        let src_layout = project_dir.join("src");
        if src_layout.exists() && src_layout.join(normalized_name).exists() {
            return Ok(src_layout);
        }

        // Check for flat layout (package directory at root)
        let flat_layout = project_dir.join(normalized_name);
        if flat_layout.exists() {
            return Ok(project_dir.to_path_buf());
        }

        // Check for single-file module
        let single_file = project_dir.join(format!("{}.py", normalized_name));
        if single_file.exists() {
            return Ok(project_dir.to_path_buf());
        }

        // Default to project directory
        Ok(project_dir.to_path_buf())
    }

    /// Create a .pth file that adds the source directory to sys.path
    fn create_pth_file(&self, normalized_name: &str, source_path: &Path) -> Result<PathBuf> {
        let pth_filename = format!("_editable_{}.pth", normalized_name);
        let pth_path = self.site_packages.join(&pth_filename);

        // Write the source path to the .pth file
        let content = source_path.to_string_lossy().to_string();
        fs::write(&pth_path, content)?;

        Ok(pth_path)
    }

    /// Create a minimal .dist-info directory for the editable install
    fn create_dist_info(
        &self,
        normalized_name: &str,
        version: &str,
        project_dir: &Path,
    ) -> Result<PathBuf> {
        let dist_info_name = format!("{}-{}.dist-info", normalized_name, version);
        let dist_info_path = self.site_packages.join(&dist_info_name);

        fs::create_dir_all(&dist_info_path)?;

        // Create METADATA file
        let metadata = format!(
            "Metadata-Version: 2.1\nName: {}\nVersion: {}\n",
            normalized_name.replace('_', "-"),
            version
        );
        fs::write(dist_info_path.join("METADATA"), metadata)?;

        // Create INSTALLER file
        fs::write(dist_info_path.join("INSTALLER"), "dx-py\n")?;

        // Create direct_url.json for PEP 610 compliance
        let direct_url = format!(
            r#"{{"url": "file://{}", "dir_info": {{"editable": true}}}}"#,
            project_dir.to_string_lossy().replace('\\', "/")
        );
        fs::write(dist_info_path.join("direct_url.json"), direct_url)?;

        // Create RECORD file (empty for editable installs)
        fs::write(dist_info_path.join("RECORD"), "")?;

        Ok(dist_info_path)
    }

    /// Generate entry point scripts from pyproject.toml
    fn generate_scripts(
        &self,
        pyproject: &PyProjectToml,
        project_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        let mut scripts = Vec::new();

        // Get scripts from [project.scripts]
        let project_scripts = pyproject.project.as_ref().and_then(|p| p.scripts.as_ref());

        if let Some(script_map) = project_scripts {
            for (name, entry_point) in script_map {
                if let Some(script_path) = self.create_script(name, entry_point, project_dir)? {
                    scripts.push(script_path);
                }
            }
        }

        // Get gui-scripts from [project.gui-scripts]
        let gui_scripts = pyproject.project.as_ref().and_then(|p| p.gui_scripts.as_ref());

        if let Some(script_map) = gui_scripts {
            for (name, entry_point) in script_map {
                if let Some(script_path) = self.create_script(name, entry_point, project_dir)? {
                    scripts.push(script_path);
                }
            }
        }

        Ok(scripts)
    }

    /// Create a single entry point script
    fn create_script(
        &self,
        name: &str,
        entry_point: &str,
        project_dir: &Path,
    ) -> Result<Option<PathBuf>> {
        // Parse entry point: module:function or module:object.method
        let parts: Vec<&str> = entry_point.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Ok(None);
        }

        let module = parts[0].trim();
        let function = parts[1].trim();

        // Get scripts directory
        let scripts_dir = self
            .site_packages
            .parent()
            .map(|p| {
                if cfg!(windows) {
                    p.join("Scripts")
                } else {
                    p.join("bin")
                }
            })
            .unwrap_or_else(|| self.site_packages.join("Scripts"));
        fs::create_dir_all(&scripts_dir)?;

        // Generate wrapper script that imports from the editable source
        let wrapper = format!(
            r#"#!python
# -*- coding: utf-8 -*-
# Editable install entry point for {}
import sys
import os

# Ensure the project directory is in sys.path
project_dir = r'{}'
if project_dir not in sys.path:
    sys.path.insert(0, project_dir)

from {} import {}

if __name__ == '__main__':
    sys.exit({}())
"#,
            name,
            project_dir.to_string_lossy(),
            module,
            function,
            function
        );

        #[cfg(windows)]
        {
            // On Windows, create both .py script and a .cmd wrapper
            let py_path = scripts_dir.join(format!("{}-script.py", name));
            fs::write(&py_path, &wrapper)?;

            // Create .cmd wrapper
            let cmd_path = scripts_dir.join(format!("{}.cmd", name));
            let cmd_content =
                format!("@echo off\r\npython \"{}\" %*\r\n", py_path.to_string_lossy());
            fs::write(&cmd_path, cmd_content)?;

            Ok(Some(cmd_path))
        }

        #[cfg(not(windows))]
        {
            let script_path = scripts_dir.join(name);
            fs::write(&script_path, &wrapper)?;

            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&script_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&script_path, perms)?;
            }

            Ok(Some(script_path))
        }
    }

    /// Uninstall an editable package
    pub fn uninstall(&self, package_name: &str) -> Result<u64> {
        let normalized = package_name.replace('-', "_").to_lowercase();
        let mut removed = 0;

        // Remove .pth file
        let pth_filename = format!("_editable_{}.pth", normalized);
        let pth_path = self.site_packages.join(&pth_filename);
        if pth_path.exists() {
            fs::remove_file(&pth_path)?;
            removed += 1;
        }

        // Find and remove .dist-info directory
        for entry in fs::read_dir(&self.site_packages)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_lowercase();

            if name_str.starts_with(&format!("{}-", normalized)) && name_str.ends_with(".dist-info")
            {
                let dist_info_path = entry.path();

                // Check if this is an editable install
                let direct_url_path = dist_info_path.join("direct_url.json");
                if direct_url_path.exists() {
                    let content = fs::read_to_string(&direct_url_path)?;
                    if content.contains("\"editable\": true") {
                        removed += self.count_files(&dist_info_path)?;
                        fs::remove_dir_all(&dist_info_path)?;
                    }
                }
            }
        }

        // Remove entry point scripts
        let scripts_dir = self
            .site_packages
            .parent()
            .map(|p| {
                if cfg!(windows) {
                    p.join("Scripts")
                } else {
                    p.join("bin")
                }
            })
            .unwrap_or_else(|| self.site_packages.join("Scripts"));

        if scripts_dir.exists() {
            // We need to check each script to see if it belongs to this package
            // For now, we rely on the .dist-info RECORD file which we don't populate
            // In a full implementation, we'd track scripts in RECORD
        }

        Ok(removed)
    }

    /// Check if a package is installed in editable mode
    pub fn is_editable(&self, package_name: &str) -> Result<bool> {
        let normalized = package_name.replace('-', "_").to_lowercase();

        // Check for .pth file
        let pth_filename = format!("_editable_{}.pth", normalized);
        let pth_path = self.site_packages.join(&pth_filename);
        if pth_path.exists() {
            return Ok(true);
        }

        // Check for direct_url.json with editable flag
        for entry in fs::read_dir(&self.site_packages)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_lowercase();

            if name_str.starts_with(&format!("{}-", normalized)) && name_str.ends_with(".dist-info")
            {
                let direct_url_path = entry.path().join("direct_url.json");
                if direct_url_path.exists() {
                    let content = fs::read_to_string(&direct_url_path)?;
                    if content.contains("\"editable\": true") {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Get information about an editable install
    pub fn get_info(&self, package_name: &str) -> Result<Option<EditableInstall>> {
        let normalized = package_name.replace('-', "_").to_lowercase();

        // Find .pth file
        let pth_filename = format!("_editable_{}.pth", normalized);
        let pth_path = self.site_packages.join(&pth_filename);
        if !pth_path.exists() {
            return Ok(None);
        }

        // Read source directory from .pth file
        let source_dir = PathBuf::from(fs::read_to_string(&pth_path)?.trim());

        // Find .dist-info directory
        let mut dist_info = None;
        let mut version = String::from("0.0.0");

        for entry in fs::read_dir(&self.site_packages)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_lowercase();

            if name_str.starts_with(&format!("{}-", normalized)) && name_str.ends_with(".dist-info")
            {
                dist_info = Some(entry.path());
                // Extract version from directory name
                if let Some(v) = name_str
                    .strip_prefix(&format!("{}-", normalized))
                    .and_then(|s| s.strip_suffix(".dist-info"))
                {
                    version = v.to_string();
                }
                break;
            }
        }

        let dist_info = dist_info
            .ok_or_else(|| Error::Cache(format!("No .dist-info found for {}", package_name)))?;

        Ok(Some(EditableInstall {
            name: package_name.to_string(),
            version,
            source_dir,
            pth_file: pth_path,
            dist_info,
            scripts: Vec::new(), // Would need to scan scripts dir
        }))
    }

    /// Count files in a directory
    #[allow(clippy::only_used_in_recursion)]
    fn count_files(&self, dir: &Path) -> Result<u64> {
        let mut count = 0;
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().is_file() {
                count += 1;
            } else if entry.path().is_dir() {
                count += self.count_files(&entry.path())?;
            }
        }
        Ok(count)
    }

    /// List all editable installs
    pub fn list_editable(&self) -> Result<Vec<EditableInstall>> {
        let mut installs = Vec::new();

        for entry in fs::read_dir(&self.site_packages)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with("_editable_") && name_str.ends_with(".pth") {
                // Extract package name from .pth filename
                if let Some(pkg_name) =
                    name_str.strip_prefix("_editable_").and_then(|s| s.strip_suffix(".pth"))
                {
                    if let Ok(Some(info)) = self.get_info(pkg_name) {
                        installs.push(info);
                    }
                }
            }
        }

        Ok(installs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_project(dir: &Path, name: &str, version: &str) {
        // Create pyproject.toml
        let pyproject = format!(
            r#"[project]
name = "{}"
version = "{}"

[project.scripts]
{}-cli = "{}:main"
"#,
            name,
            version,
            name,
            name.replace('-', "_")
        );
        fs::write(dir.join("pyproject.toml"), pyproject).unwrap();

        // Create package directory
        let pkg_dir = dir.join(name.replace('-', "_"));
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("__init__.py"), "def main(): return 0\n").unwrap();
    }

    #[test]
    fn test_editable_install() {
        let project_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        create_test_project(project_dir.path(), "test-pkg", "1.0.0");

        let installer = EditableInstaller::new(site_packages.path().to_path_buf());
        let result = installer.install(project_dir.path()).unwrap();

        assert_eq!(result.name, "test-pkg");
        assert_eq!(result.version, "1.0.0");
        assert!(result.pth_file.exists());
        assert!(result.dist_info.exists());
    }

    #[test]
    fn test_editable_uninstall() {
        let project_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        create_test_project(project_dir.path(), "test-pkg", "1.0.0");

        let installer = EditableInstaller::new(site_packages.path().to_path_buf());
        let result = installer.install(project_dir.path()).unwrap();

        assert!(result.pth_file.exists());

        let removed = installer.uninstall("test-pkg").unwrap();
        assert!(removed > 0);
        assert!(!result.pth_file.exists());
    }

    #[test]
    fn test_is_editable() {
        let project_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        create_test_project(project_dir.path(), "test-pkg", "1.0.0");

        let installer = EditableInstaller::new(site_packages.path().to_path_buf());

        assert!(!installer.is_editable("test-pkg").unwrap());

        installer.install(project_dir.path()).unwrap();

        assert!(installer.is_editable("test-pkg").unwrap());
    }

    #[test]
    fn test_src_layout() {
        let project_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        // Create src-layout project
        let pyproject = r#"[project]
name = "src-pkg"
version = "1.0.0"
"#;
        fs::write(project_dir.path().join("pyproject.toml"), pyproject).unwrap();

        let src_dir = project_dir.path().join("src").join("src_pkg");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("__init__.py"), "").unwrap();

        let installer = EditableInstaller::new(site_packages.path().to_path_buf());
        let result = installer.install(project_dir.path()).unwrap();

        // Check that .pth points to src directory
        let pth_content = fs::read_to_string(&result.pth_file).unwrap();
        assert!(pth_content.contains("src"));
    }

    #[test]
    fn test_list_editable() {
        let project_dir1 = TempDir::new().unwrap();
        let project_dir2 = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        create_test_project(project_dir1.path(), "pkg-a", "1.0.0");
        create_test_project(project_dir2.path(), "pkg-b", "2.0.0");

        let installer = EditableInstaller::new(site_packages.path().to_path_buf());
        installer.install(project_dir1.path()).unwrap();
        installer.install(project_dir2.path()).unwrap();

        let installs = installer.list_editable().unwrap();
        assert_eq!(installs.len(), 2);
    }
}
