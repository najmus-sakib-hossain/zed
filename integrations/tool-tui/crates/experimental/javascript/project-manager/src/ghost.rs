//! Ghost Detector
//!
//! Detects undeclared dependencies in the workspace.

use crate::change::ChangeDetector;
use crate::error::ScanError;
use crate::types::GhostDependency;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Ghost dependency report
#[derive(Debug, Clone, Default)]
pub struct GhostReport {
    /// Undeclared dependencies found
    pub ghosts: Vec<GhostDependency>,
    /// Hoisting accidents (works due to hoisting, not declaration)
    pub hoisting_accidents: Vec<HoistingAccident>,
    /// Known vulnerabilities in ghost deps
    pub vulnerabilities: Vec<Vulnerability>,
}

/// Hoisting accident
#[derive(Debug, Clone)]
pub struct HoistingAccident {
    /// Package that has the accident
    pub package_idx: u32,
    /// Dependency that works due to hoisting
    pub dependency: String,
    /// Package that actually declares it
    pub declared_by: String,
}

/// Vulnerability in a ghost dependency
#[derive(Debug, Clone)]
pub struct Vulnerability {
    /// Package name
    pub package: String,
    /// Severity (low, medium, high, critical)
    pub severity: String,
    /// CVE identifier
    pub cve: Option<String>,
    /// Description
    pub description: String,
}

/// Ghost Detector for finding undeclared dependencies
pub struct GhostDetector {
    /// Change detector for import scanning
    change_detector: ChangeDetector,
    /// Package dependencies: package_idx -> set of declared deps
    declared_deps: HashMap<u32, HashSet<String>>,
    /// Package paths: package_idx -> path
    package_paths: HashMap<u32, PathBuf>,
    /// All packages in workspace (for hoisting detection)
    workspace_packages: HashSet<String>,
}

impl GhostDetector {
    /// Create a new ghost detector
    pub fn new() -> Self {
        Self {
            change_detector: ChangeDetector::new(),
            declared_deps: HashMap::new(),
            package_paths: HashMap::new(),
            workspace_packages: HashSet::new(),
        }
    }

    /// Set declared dependencies for a package
    pub fn set_declared_deps(&mut self, package_idx: u32, deps: HashSet<String>) {
        self.declared_deps.insert(package_idx, deps);
    }

    /// Set package path
    pub fn set_package_path(&mut self, package_idx: u32, path: PathBuf) {
        self.package_paths.insert(package_idx, path);
    }

    /// Add workspace package name
    pub fn add_workspace_package(&mut self, name: String) {
        self.workspace_packages.insert(name);
    }

    /// Scan workspace for ghost dependencies
    pub fn scan(&self) -> Result<GhostReport, ScanError> {
        let mut report = GhostReport::default();

        for &package_idx in self.package_paths.keys() {
            let ghosts = self.scan_package(package_idx)?;
            report.ghosts.extend(ghosts);
        }

        // Detect hoisting accidents
        report.hoisting_accidents = self.detect_hoisting_accidents(&report.ghosts);

        Ok(report)
    }

    /// Scan single package for ghost dependencies
    pub fn scan_package(&self, package_idx: u32) -> Result<Vec<GhostDependency>, ScanError> {
        let path = self.package_paths.get(&package_idx).ok_or_else(|| ScanError::ReadFailed {
            path: PathBuf::from(format!("package_{}", package_idx)),
            reason: "package path not set".to_string(),
        })?;

        let declared = self.declared_deps.get(&package_idx).cloned().unwrap_or_default();

        let mut ghosts = Vec::new();

        // Scan source files
        let src_dir = path.join("src");
        if src_dir.exists() {
            self.scan_directory(&src_dir, package_idx, &declared, &mut ghosts)?;
        }

        // Also scan root level files
        for entry in std::fs::read_dir(path).map_err(|e| ScanError::ReadFailed {
            path: path.clone(),
            reason: e.to_string(),
        })? {
            let entry = entry.map_err(|e| ScanError::ReadFailed {
                path: path.clone(),
                reason: e.to_string(),
            })?;

            let file_path = entry.path();
            if self.is_source_file(&file_path) {
                self.scan_file(&file_path, package_idx, &declared, &mut ghosts)?;
            }
        }

        Ok(ghosts)
    }

    /// Check if import is declared
    pub fn is_declared(&self, package_idx: u32, import: &str) -> bool {
        let declared = match self.declared_deps.get(&package_idx) {
            Some(d) => d,
            None => return false,
        };

        // Extract package name from import
        let package_name = self.extract_package_name(import);

        // Check if declared
        declared.contains(&package_name)
            || self.is_builtin(&package_name)
            || self.is_relative(import)
            || self.workspace_packages.contains(&package_name)
    }

    // Private helpers

    fn scan_directory(
        &self,
        dir: &Path,
        package_idx: u32,
        declared: &HashSet<String>,
        ghosts: &mut Vec<GhostDependency>,
    ) -> Result<(), ScanError> {
        for entry in std::fs::read_dir(dir).map_err(|e| ScanError::ReadFailed {
            path: dir.to_path_buf(),
            reason: e.to_string(),
        })? {
            let entry = entry.map_err(|e| ScanError::ReadFailed {
                path: dir.to_path_buf(),
                reason: e.to_string(),
            })?;

            let path = entry.path();

            if path.is_dir() {
                self.scan_directory(&path, package_idx, declared, ghosts)?;
            } else if self.is_source_file(&path) {
                self.scan_file(&path, package_idx, declared, ghosts)?;
            }
        }

        Ok(())
    }

    fn scan_file(
        &self,
        path: &Path,
        _package_idx: u32,
        declared: &HashSet<String>,
        ghosts: &mut Vec<GhostDependency>,
    ) -> Result<(), ScanError> {
        let imports =
            self.change_detector
                .detect_imports_file(path)
                .map_err(|e| ScanError::ReadFailed {
                    path: path.to_path_buf(),
                    reason: e.to_string(),
                })?;

        for import in imports {
            let package_name = self.extract_package_name(&import.specifier);

            if !self.is_relative(&import.specifier)
                && !self.is_builtin(&package_name)
                && !declared.contains(&package_name)
                && !self.workspace_packages.contains(&package_name)
            {
                ghosts.push(GhostDependency {
                    package_name,
                    importing_file: path.to_path_buf(),
                    line: import.line,
                    column: import.column,
                });
            }
        }

        Ok(())
    }

    fn is_source_file(&self, path: &Path) -> bool {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        matches!(ext, "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs")
    }

    fn extract_package_name(&self, specifier: &str) -> String {
        // Handle scoped packages (@org/package)
        if specifier.starts_with('@') {
            let parts: Vec<&str> = specifier.splitn(3, '/').collect();
            if parts.len() >= 2 {
                return format!("{}/{}", parts[0], parts[1]);
            }
        }

        // Regular package
        specifier.split('/').next().unwrap_or(specifier).to_string()
    }

    fn is_relative(&self, specifier: &str) -> bool {
        specifier.starts_with('.') || specifier.starts_with('/')
    }

    fn is_builtin(&self, package: &str) -> bool {
        const BUILTINS: &[&str] = &[
            "fs",
            "path",
            "os",
            "util",
            "events",
            "stream",
            "http",
            "https",
            "url",
            "querystring",
            "crypto",
            "buffer",
            "child_process",
            "cluster",
            "dgram",
            "dns",
            "net",
            "readline",
            "repl",
            "tls",
            "tty",
            "v8",
            "vm",
            "zlib",
            "assert",
            "async_hooks",
            "console",
            "constants",
            "domain",
            "inspector",
            "module",
            "perf_hooks",
            "process",
            "punycode",
            "string_decoder",
            "timers",
            "trace_events",
            "worker_threads",
            "node:fs",
            "node:path",
            "node:os", // Node.js prefixed
        ];
        BUILTINS.contains(&package)
    }

    fn detect_hoisting_accidents(&self, _ghosts: &[GhostDependency]) -> Vec<HoistingAccident> {
        // In a real implementation, this would check if the ghost dependency
        // is declared by another package in the workspace and works due to hoisting
        Vec::new()
    }
}

impl Default for GhostDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_package_name() {
        let detector = GhostDetector::new();

        assert_eq!(detector.extract_package_name("lodash"), "lodash");
        assert_eq!(detector.extract_package_name("lodash/get"), "lodash");
        assert_eq!(detector.extract_package_name("@types/node"), "@types/node");
        assert_eq!(detector.extract_package_name("@org/pkg/sub"), "@org/pkg");
    }

    #[test]
    fn test_is_declared() {
        let mut detector = GhostDetector::new();

        let mut deps = HashSet::new();
        deps.insert("lodash".to_string());
        deps.insert("react".to_string());
        detector.set_declared_deps(0, deps);

        assert!(detector.is_declared(0, "lodash"));
        assert!(detector.is_declared(0, "lodash/get"));
        assert!(detector.is_declared(0, "react"));
        assert!(!detector.is_declared(0, "express"));

        // Relative imports are always "declared"
        assert!(detector.is_declared(0, "./utils"));
        assert!(detector.is_declared(0, "../shared"));

        // Builtins are always "declared"
        assert!(detector.is_declared(0, "fs"));
        assert!(detector.is_declared(0, "path"));
    }

    #[test]
    fn test_scan_package() {
        let temp = TempDir::new().unwrap();
        let pkg_path = temp.path().to_path_buf();

        // Create source file with imports
        fs::create_dir(pkg_path.join("src")).unwrap();
        fs::write(
            pkg_path.join("src/index.ts"),
            r#"
import lodash from 'lodash';
import express from 'express';
import { foo } from './utils';
import fs from 'fs';
"#,
        )
        .unwrap();

        let mut detector = GhostDetector::new();

        // Only lodash is declared
        let mut deps = HashSet::new();
        deps.insert("lodash".to_string());
        detector.set_declared_deps(0, deps);
        detector.set_package_path(0, pkg_path);

        let ghosts = detector.scan_package(0).unwrap();

        // express should be detected as ghost
        assert!(ghosts.iter().any(|g| g.package_name == "express"));

        // lodash should NOT be detected (declared)
        assert!(!ghosts.iter().any(|g| g.package_name == "lodash"));

        // fs should NOT be detected (builtin)
        assert!(!ghosts.iter().any(|g| g.package_name == "fs"));

        // ./utils should NOT be detected (relative)
        assert!(!ghosts.iter().any(|g| g.package_name.starts_with('.')));
    }

    #[test]
    fn test_ghost_report_completeness() {
        let ghost = GhostDependency {
            package_name: "express".to_string(),
            importing_file: PathBuf::from("packages/api/src/server.ts"),
            line: 5,
            column: 22,
        };

        // Verify all required fields are present
        assert!(!ghost.package_name.is_empty());
        assert!(!ghost.importing_file.as_os_str().is_empty());
        assert!(ghost.line > 0);
        assert!(ghost.column > 0);
    }
}
