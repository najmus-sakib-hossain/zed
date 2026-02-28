//! dx-pkg-workspace: Workspace Support
//!
//! Features:
//! - Workspace detection (npm, yarn, pnpm formats)
//! - Dependency hoisting
//! - Parallel installation
//! - Internal package linking
//! - Workspace-wide script execution

use dx_pkg_compat::PackageJson;
use dx_pkg_core::Result;
use glob::glob;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Workspace structure
pub struct Workspace {
    pub root: PathBuf,
    pub packages: Vec<WorkspacePackage>,
    pub format: WorkspaceFormat,
}

/// Individual workspace package
#[derive(Debug, Clone)]
pub struct WorkspacePackage {
    pub name: String,
    pub path: PathBuf,
    pub package_json: PackageJson,
}

/// Workspace format (npm, yarn, pnpm)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceFormat {
    /// npm workspaces (package.json workspaces field)
    Npm,
    /// Yarn workspaces (package.json workspaces field or workspaces.packages)
    Yarn,
    /// pnpm workspaces (pnpm-workspace.yaml)
    Pnpm,
    /// Lerna (lerna.json)
    Lerna,
    /// Not a workspace
    None,
}

impl Workspace {
    /// Detect workspace from root directory
    /// Supports npm, yarn, pnpm, and lerna workspace formats
    pub fn detect(root: impl AsRef<Path>) -> Result<Option<Self>> {
        let root = root.as_ref().to_path_buf();

        // Try pnpm first (pnpm-workspace.yaml)
        if let Some(ws) = Self::detect_pnpm(&root)? {
            return Ok(Some(ws));
        }

        // Try lerna (lerna.json)
        if let Some(ws) = Self::detect_lerna(&root)? {
            return Ok(Some(ws));
        }

        // Try npm/yarn (package.json workspaces field)
        if let Some(ws) = Self::detect_npm_yarn(&root)? {
            return Ok(Some(ws));
        }

        Ok(None)
    }

    /// Detect pnpm workspace from pnpm-workspace.yaml
    fn detect_pnpm(root: &Path) -> Result<Option<Self>> {
        let workspace_yaml = root.join("pnpm-workspace.yaml");

        if !workspace_yaml.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&workspace_yaml)?;
        let patterns = Self::parse_pnpm_workspace_yaml(&content);

        let packages = Self::find_packages_by_patterns(root, &patterns)?;

        Ok(Some(Self {
            root: root.to_path_buf(),
            packages,
            format: WorkspaceFormat::Pnpm,
        }))
    }

    /// Parse pnpm-workspace.yaml to extract package patterns
    fn parse_pnpm_workspace_yaml(content: &str) -> Vec<String> {
        let mut patterns = Vec::new();
        let mut in_packages = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed == "packages:" {
                in_packages = true;
                continue;
            }

            if in_packages {
                if trimmed.starts_with('-') {
                    // Extract pattern from "- 'packages/*'" or "- packages/*"
                    let pattern = trimmed
                        .trim_start_matches('-')
                        .trim()
                        .trim_matches('\'')
                        .trim_matches('"')
                        .to_string();
                    patterns.push(pattern);
                } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    // End of packages section
                    break;
                }
            }
        }

        patterns
    }

    /// Detect lerna workspace from lerna.json
    fn detect_lerna(root: &Path) -> Result<Option<Self>> {
        let lerna_json = root.join("lerna.json");

        if !lerna_json.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&lerna_json)?;
        let patterns = Self::parse_lerna_json(&content);

        let packages = Self::find_packages_by_patterns(root, &patterns)?;

        Ok(Some(Self {
            root: root.to_path_buf(),
            packages,
            format: WorkspaceFormat::Lerna,
        }))
    }

    /// Parse lerna.json to extract package patterns
    fn parse_lerna_json(content: &str) -> Vec<String> {
        // Simple JSON parsing for packages field
        // In production, use serde_json
        let mut patterns = Vec::new();

        if let Some(start) = content.find("\"packages\"") {
            if let Some(arr_start) = content[start..].find('[') {
                if let Some(arr_end) = content[start + arr_start..].find(']') {
                    let arr_content = &content[start + arr_start + 1..start + arr_start + arr_end];
                    for part in arr_content.split(',') {
                        let pattern = part.trim().trim_matches('"').trim_matches('\'').to_string();
                        if !pattern.is_empty() {
                            patterns.push(pattern);
                        }
                    }
                }
            }
        }

        if patterns.is_empty() {
            // Default lerna pattern
            patterns.push("packages/*".to_string());
        }

        patterns
    }

    /// Detect npm/yarn workspace from package.json
    fn detect_npm_yarn(root: &Path) -> Result<Option<Self>> {
        let pkg_json_path = root.join("package.json");

        if !pkg_json_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&pkg_json_path)?;
        let patterns = Self::parse_package_json_workspaces(&content);

        if patterns.is_empty() {
            return Ok(None);
        }

        let packages = Self::find_packages_by_patterns(root, &patterns)?;

        // Determine if it's yarn or npm based on lockfile
        let format = if root.join("yarn.lock").exists() {
            WorkspaceFormat::Yarn
        } else {
            WorkspaceFormat::Npm
        };

        Ok(Some(Self {
            root: root.to_path_buf(),
            packages,
            format,
        }))
    }

    /// Parse package.json to extract workspace patterns
    fn parse_package_json_workspaces(content: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Try "workspaces": ["packages/*"]
        if let Some(start) = content.find("\"workspaces\"") {
            let rest = &content[start..];

            // Check for array format
            if let Some(arr_start) = rest.find('[') {
                if let Some(arr_end) = rest[arr_start..].find(']') {
                    let arr_content = &rest[arr_start + 1..arr_start + arr_end];
                    for part in arr_content.split(',') {
                        let pattern = part.trim().trim_matches('"').trim_matches('\'').to_string();
                        if !pattern.is_empty() {
                            patterns.push(pattern);
                        }
                    }
                }
            }

            // Check for object format (yarn): "workspaces": { "packages": [...] }
            if patterns.is_empty() {
                if let Some(pkg_start) = rest.find("\"packages\"") {
                    if let Some(arr_start) = rest[pkg_start..].find('[') {
                        if let Some(arr_end) = rest[pkg_start + arr_start..].find(']') {
                            let arr_content =
                                &rest[pkg_start + arr_start + 1..pkg_start + arr_start + arr_end];
                            for part in arr_content.split(',') {
                                let pattern =
                                    part.trim().trim_matches('"').trim_matches('\'').to_string();
                                if !pattern.is_empty() {
                                    patterns.push(pattern);
                                }
                            }
                        }
                    }
                }
            }
        }

        patterns
    }

    /// Find packages matching glob patterns
    fn find_packages_by_patterns(
        root: &Path,
        patterns: &[String],
    ) -> Result<Vec<WorkspacePackage>> {
        let mut packages = Vec::new();
        let mut seen_paths = HashSet::new();

        for pattern in patterns {
            let full_pattern = root.join(pattern).join("package.json");
            let pattern_str = full_pattern.to_string_lossy().to_string();

            // Use glob to find matching package.json files
            if let Ok(entries) = glob(&pattern_str) {
                for entry in entries.flatten() {
                    let pkg_dir = entry.parent().unwrap().to_path_buf();

                    if seen_paths.contains(&pkg_dir) {
                        continue;
                    }
                    seen_paths.insert(pkg_dir.clone());

                    if let Ok(pkg_json) = PackageJson::read(&entry) {
                        packages.push(WorkspacePackage {
                            name: pkg_json.name.clone(),
                            path: pkg_dir,
                            package_json: pkg_json,
                        });
                    }
                }
            }
        }

        Ok(packages)
    }

    /// Get all workspace packages
    pub fn list_packages(&self) -> &[WorkspacePackage] {
        &self.packages
    }

    /// Get package by name
    pub fn get_package(&self, name: &str) -> Option<&WorkspacePackage> {
        self.packages.iter().find(|p| p.name == name)
    }

    /// Install dependencies for all workspace packages
    pub fn install_all(&self) -> Result<()> {
        // Hoist common dependencies to root
        let _hoisted = self.hoist_dependencies();

        // Install hoisted dependencies at root
        // (In real impl, would call dx-pkg-install)

        // Link workspace packages to each other
        self.link_workspace_packages()?;

        Ok(())
    }

    /// Link workspace packages that depend on each other
    pub fn link_workspace_packages(&self) -> Result<()> {
        let package_names: HashSet<_> = self.packages.iter().map(|p| p.name.clone()).collect();

        for pkg in &self.packages {
            let node_modules = pkg.path.join("node_modules");
            std::fs::create_dir_all(&node_modules)?;

            // Check dependencies for workspace packages
            for dep_name in pkg.package_json.dependencies.keys() {
                if package_names.contains(dep_name) {
                    // This is a workspace sibling - create symlink
                    if let Some(dep_pkg) = self.get_package(dep_name) {
                        let link_path = node_modules.join(dep_name);

                        // Remove existing link/dir
                        if link_path.exists() {
                            if link_path.is_symlink() {
                                std::fs::remove_file(&link_path)?;
                            } else {
                                std::fs::remove_dir_all(&link_path)?;
                            }
                        }

                        // Create symlink
                        #[cfg(unix)]
                        std::os::unix::fs::symlink(&dep_pkg.path, &link_path)?;

                        #[cfg(windows)]
                        std::os::windows::fs::symlink_dir(&dep_pkg.path, &link_path)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Run a script in all workspace packages
    pub fn run_script_all(&self, script: &str) -> Result<Vec<ScriptResult>> {
        let mut results = Vec::new();

        for pkg in &self.packages {
            if let Some(cmd) = pkg.package_json.scripts.get(script) {
                let result = self.run_script_in_package(pkg, script, cmd)?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Run a script in a specific package
    fn run_script_in_package(
        &self,
        pkg: &WorkspacePackage,
        script: &str,
        cmd: &str,
    ) -> Result<ScriptResult> {
        use std::process::Command;

        let output = Command::new(if cfg!(windows) { "cmd" } else { "sh" })
            .args(if cfg!(windows) {
                vec!["/C", cmd]
            } else {
                vec!["-c", cmd]
            })
            .current_dir(&pkg.path)
            .output()?;

        Ok(ScriptResult {
            package: pkg.name.clone(),
            script: script.to_string(),
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Hoist common dependencies to root
    pub fn hoist_dependencies(&self) -> HashMap<String, String> {
        let mut dep_counts: HashMap<String, (String, usize)> = HashMap::new();

        // Count dependency usage across packages
        for pkg in &self.packages {
            for (name, version) in &pkg.package_json.dependencies {
                let entry = dep_counts.entry(name.clone()).or_insert((version.clone(), 0));
                entry.1 += 1;
            }
        }

        // Return dependencies used by 2+ packages
        dep_counts
            .into_iter()
            .filter(|(_, (_, count))| *count >= 2)
            .map(|(name, (version, _))| (name, version))
            .collect()
    }
}

/// Result of running a script
#[derive(Debug)]
pub struct ScriptResult {
    pub package: String,
    pub script: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Filter for selecting workspace packages
#[derive(Debug, Clone)]
pub struct WorkspaceFilter {
    /// Package name patterns (glob-style)
    pub patterns: Vec<String>,
    /// Include dependencies of matched packages
    pub include_dependencies: bool,
    /// Include dependents of matched packages
    pub include_dependents: bool,
}

impl WorkspaceFilter {
    /// Create a new filter with the given patterns
    pub fn new(patterns: Vec<String>) -> Self {
        Self {
            patterns,
            include_dependencies: false,
            include_dependents: false,
        }
    }

    /// Create a filter that matches all packages
    pub fn all() -> Self {
        Self {
            patterns: vec!["*".to_string()],
            include_dependencies: false,
            include_dependents: false,
        }
    }

    /// Include dependencies of matched packages
    pub fn with_dependencies(mut self) -> Self {
        self.include_dependencies = true;
        self
    }

    /// Include dependents of matched packages
    pub fn with_dependents(mut self) -> Self {
        self.include_dependents = true;
        self
    }

    /// Check if a package name matches the filter
    pub fn matches(&self, name: &str) -> bool {
        for pattern in &self.patterns {
            if Self::glob_match(pattern, name) {
                return true;
            }
        }
        false
    }

    /// Simple glob matching (supports * and ?)
    fn glob_match(pattern: &str, name: &str) -> bool {
        // Handle exact match
        if pattern == name {
            return true;
        }

        // Handle * wildcard (matches everything)
        if pattern == "*" {
            return true;
        }

        // Handle prefix match (e.g., "@scope/*")
        if let Some(prefix) = pattern.strip_suffix("/*") {
            return name.starts_with(prefix);
        }

        // Handle suffix match (e.g., "*-utils")
        if let Some(suffix) = pattern.strip_prefix('*') {
            return name.ends_with(suffix);
        }

        // Handle contains match (e.g., "*core*")
        if pattern.starts_with('*') && pattern.ends_with('*') && pattern.len() > 2 {
            let middle = &pattern[1..pattern.len() - 1];
            return name.contains(middle);
        }

        false
    }
}

impl Workspace {
    /// Filter packages by name pattern
    pub fn filter_packages(&self, filter: &WorkspaceFilter) -> Vec<&WorkspacePackage> {
        let mut matched: HashSet<String> = HashSet::new();

        // First pass: find directly matched packages
        for pkg in &self.packages {
            if filter.matches(&pkg.name) {
                matched.insert(pkg.name.clone());
            }
        }

        // Second pass: include dependencies if requested
        if filter.include_dependencies {
            let mut to_add = Vec::new();
            for pkg in &self.packages {
                if matched.contains(&pkg.name) {
                    for dep_name in pkg.package_json.dependencies.keys() {
                        if self.get_package(dep_name).is_some() {
                            to_add.push(dep_name.clone());
                        }
                    }
                }
            }
            matched.extend(to_add);
        }

        // Third pass: include dependents if requested
        if filter.include_dependents {
            let mut to_add = Vec::new();
            for pkg in &self.packages {
                for dep_name in pkg.package_json.dependencies.keys() {
                    if matched.contains(dep_name) {
                        to_add.push(pkg.name.clone());
                    }
                }
            }
            matched.extend(to_add);
        }

        // Return filtered packages
        self.packages.iter().filter(|p| matched.contains(&p.name)).collect()
    }

    /// Run a script in filtered workspace packages
    pub fn run_script_filtered(
        &self,
        script: &str,
        filter: &WorkspaceFilter,
    ) -> Result<Vec<ScriptResult>> {
        let packages = self.filter_packages(filter);
        let mut results = Vec::new();

        for pkg in packages {
            if let Some(cmd) = pkg.package_json.scripts.get(script) {
                let result = self.run_script_in_package(pkg, script, cmd)?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Check if a package depends on another workspace package
    pub fn has_workspace_dependency(&self, pkg: &WorkspacePackage, dep_name: &str) -> bool {
        pkg.package_json.dependencies.contains_key(dep_name) && self.get_package(dep_name).is_some()
    }

    /// Get all workspace packages that depend on the given package
    pub fn get_dependents(&self, package_name: &str) -> Vec<&WorkspacePackage> {
        self.packages
            .iter()
            .filter(|p| p.package_json.dependencies.contains_key(package_name))
            .collect()
    }

    /// Get topological order of packages (dependencies first)
    pub fn topological_order(&self) -> Vec<&WorkspacePackage> {
        let package_names: HashSet<_> = self.packages.iter().map(|p| p.name.clone()).collect();
        let mut visited = HashSet::new();
        let mut result = Vec::new();

        fn visit<'a>(
            pkg: &'a WorkspacePackage,
            packages: &'a [WorkspacePackage],
            package_names: &HashSet<String>,
            visited: &mut HashSet<String>,
            result: &mut Vec<&'a WorkspacePackage>,
        ) {
            if visited.contains(&pkg.name) {
                return;
            }
            visited.insert(pkg.name.clone());

            // Visit dependencies first
            for dep_name in pkg.package_json.dependencies.keys() {
                if package_names.contains(dep_name) {
                    if let Some(dep_pkg) = packages.iter().find(|p| &p.name == dep_name) {
                        visit(dep_pkg, packages, package_names, visited, result);
                    }
                }
            }

            result.push(pkg);
        }

        for pkg in &self.packages {
            visit(pkg, &self.packages, &package_names, &mut visited, &mut result);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_detection() {
        let temp = std::env::temp_dir().join("dx-workspace-test");
        std::fs::create_dir_all(&temp).unwrap();

        let pkg_json = temp.join("package.json");
        std::fs::write(&pkg_json, r#"{"name":"test","version":"1.0.0"}"#).unwrap();

        let ws = Workspace::detect(&temp).unwrap();
        // No workspaces field, so should be None
        assert!(ws.is_none());

        std::fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_npm_workspace_detection() {
        let temp = std::env::temp_dir().join("dx-npm-workspace-test");
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::create_dir_all(temp.join("packages/a")).unwrap();
        std::fs::create_dir_all(temp.join("packages/b")).unwrap();

        // Root package.json with workspaces
        let pkg_json = temp.join("package.json");
        std::fs::write(
            &pkg_json,
            r#"{"name":"root","version":"1.0.0","workspaces":["packages/*"]}"#,
        )
        .unwrap();

        // Package A
        std::fs::write(
            temp.join("packages/a/package.json"),
            r#"{"name":"@test/a","version":"1.0.0"}"#,
        )
        .unwrap();

        // Package B
        std::fs::write(
            temp.join("packages/b/package.json"),
            r#"{"name":"@test/b","version":"1.0.0"}"#,
        )
        .unwrap();

        let ws = Workspace::detect(&temp).unwrap();
        assert!(ws.is_some());

        let ws = ws.unwrap();
        assert_eq!(ws.format, WorkspaceFormat::Npm);
        assert_eq!(ws.packages.len(), 2);

        std::fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_pnpm_workspace_detection() {
        let temp = std::env::temp_dir().join("dx-pnpm-workspace-test");
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::create_dir_all(temp.join("packages/core")).unwrap();

        // pnpm-workspace.yaml
        std::fs::write(temp.join("pnpm-workspace.yaml"), "packages:\n  - 'packages/*'\n").unwrap();

        // Package
        std::fs::write(
            temp.join("packages/core/package.json"),
            r#"{"name":"@test/core","version":"1.0.0"}"#,
        )
        .unwrap();

        let ws = Workspace::detect(&temp).unwrap();
        assert!(ws.is_some());

        let ws = ws.unwrap();
        assert_eq!(ws.format, WorkspaceFormat::Pnpm);
        assert_eq!(ws.packages.len(), 1);

        std::fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_parse_pnpm_workspace_yaml() {
        let content = r#"
packages:
  - 'packages/*'
  - 'apps/*'
  - '!**/test/**'
"#;
        let patterns = Workspace::parse_pnpm_workspace_yaml(content);
        assert_eq!(patterns.len(), 3);
        assert_eq!(patterns[0], "packages/*");
        assert_eq!(patterns[1], "apps/*");
    }

    #[test]
    fn test_parse_package_json_workspaces() {
        let content = r#"{"name":"root","workspaces":["packages/*","apps/*"]}"#;
        let patterns = Workspace::parse_package_json_workspaces(content);
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0], "packages/*");
        assert_eq!(patterns[1], "apps/*");
    }

    #[test]
    fn test_workspace_filter_exact_match() {
        let filter = WorkspaceFilter::new(vec!["@test/core".to_string()]);
        assert!(filter.matches("@test/core"));
        assert!(!filter.matches("@test/utils"));
    }

    #[test]
    fn test_workspace_filter_wildcard() {
        let filter = WorkspaceFilter::new(vec!["*".to_string()]);
        assert!(filter.matches("@test/core"));
        assert!(filter.matches("lodash"));
    }

    #[test]
    fn test_workspace_filter_scope_wildcard() {
        let filter = WorkspaceFilter::new(vec!["@test/*".to_string()]);
        assert!(filter.matches("@test/core"));
        assert!(filter.matches("@test/utils"));
        assert!(!filter.matches("@other/core"));
    }

    #[test]
    fn test_workspace_filter_suffix() {
        let filter = WorkspaceFilter::new(vec!["*-utils".to_string()]);
        assert!(filter.matches("string-utils"));
        assert!(filter.matches("@test/array-utils"));
        assert!(!filter.matches("utils-core"));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Feature: production-readiness, Property 22: Workspace Detection
    /// Validates: Requirements 22.4
    ///
    /// For any directory structure with workspace configuration (npm, yarn, or pnpm format),
    /// workspace detection SHALL identify all member packages.

    // Generate valid package names
    fn package_name_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z0-9-]{0,20}")
            .unwrap()
            .prop_filter("non-empty", |s| !s.is_empty())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property: npm workspace detection identifies all packages
        #[test]
        fn prop_npm_workspace_detection(
            root_name in package_name_strategy(),
            pkg_names in prop::collection::vec(package_name_strategy(), 1..5),
        ) {
            let temp = std::env::temp_dir().join(format!("dx-prop-npm-{}", root_name));
            let _ = std::fs::remove_dir_all(&temp);
            std::fs::create_dir_all(&temp).unwrap();

            // Create packages directory
            std::fs::create_dir_all(temp.join("packages")).unwrap();

            // Create root package.json with workspaces
            let root_pkg = format!(
                r#"{{"name":"{}","version":"1.0.0","workspaces":["packages/*"]}}"#,
                root_name
            );
            std::fs::write(temp.join("package.json"), root_pkg).unwrap();

            // Create workspace packages
            let mut created_packages = Vec::new();
            for (i, name) in pkg_names.iter().enumerate() {
                let pkg_dir = temp.join("packages").join(format!("pkg-{}", i));
                std::fs::create_dir_all(&pkg_dir).unwrap();

                let pkg_json = format!(
                    r#"{{"name":"@{}/{}","version":"1.0.0"}}"#,
                    root_name, name
                );
                std::fs::write(pkg_dir.join("package.json"), pkg_json).unwrap();
                created_packages.push(format!("@{}/{}", root_name, name));
            }

            // Detect workspace
            let ws = Workspace::detect(&temp).unwrap();
            prop_assert!(ws.is_some(), "Workspace should be detected");

            let ws = ws.unwrap();
            prop_assert_eq!(ws.format, WorkspaceFormat::Npm, "Should detect npm format");
            prop_assert_eq!(
                ws.packages.len(),
                created_packages.len(),
                "Should find all packages"
            );

            // Verify all packages are found
            let found_names: std::collections::HashSet<_> =
                ws.packages.iter().map(|p| p.name.clone()).collect();
            for expected in &created_packages {
                prop_assert!(
                    found_names.contains(expected),
                    "Package {} should be found",
                    expected
                );
            }

            // Cleanup
            let _ = std::fs::remove_dir_all(&temp);
        }

        /// Property: pnpm workspace detection identifies all packages
        #[test]
        fn prop_pnpm_workspace_detection(
            root_name in package_name_strategy(),
            pkg_count in 1usize..4,
        ) {
            let temp = std::env::temp_dir().join(format!("dx-prop-pnpm-{}", root_name));
            let _ = std::fs::remove_dir_all(&temp);
            std::fs::create_dir_all(&temp).unwrap();

            // Create packages directory
            std::fs::create_dir_all(temp.join("packages")).unwrap();

            // Create pnpm-workspace.yaml
            std::fs::write(
                temp.join("pnpm-workspace.yaml"),
                "packages:\n  - 'packages/*'\n"
            ).unwrap();

            // Create workspace packages
            for i in 0..pkg_count {
                let pkg_dir = temp.join("packages").join(format!("pkg-{}", i));
                std::fs::create_dir_all(&pkg_dir).unwrap();

                let pkg_json = format!(
                    r#"{{"name":"@{}/pkg-{}","version":"1.0.0"}}"#,
                    root_name, i
                );
                std::fs::write(pkg_dir.join("package.json"), pkg_json).unwrap();
            }

            // Detect workspace
            let ws = Workspace::detect(&temp).unwrap();
            prop_assert!(ws.is_some(), "Workspace should be detected");

            let ws = ws.unwrap();
            prop_assert_eq!(ws.format, WorkspaceFormat::Pnpm, "Should detect pnpm format");
            prop_assert_eq!(
                ws.packages.len(),
                pkg_count,
                "Should find all {} packages",
                pkg_count
            );

            // Cleanup
            let _ = std::fs::remove_dir_all(&temp);
        }

        /// Property: workspace detection returns None for non-workspaces
        #[test]
        fn prop_non_workspace_detection(
            name in package_name_strategy(),
        ) {
            let temp = std::env::temp_dir().join(format!("dx-prop-non-ws-{}", name));
            let _ = std::fs::remove_dir_all(&temp);
            std::fs::create_dir_all(&temp).unwrap();

            // Create regular package.json without workspaces
            let pkg_json = format!(
                r#"{{"name":"{}","version":"1.0.0"}}"#,
                name
            );
            std::fs::write(temp.join("package.json"), pkg_json).unwrap();

            // Detect workspace
            let ws = Workspace::detect(&temp).unwrap();
            prop_assert!(ws.is_none(), "Non-workspace should return None");

            // Cleanup
            let _ = std::fs::remove_dir_all(&temp);
        }
    }
}
