//! Workspace/monorepo support
//!
//! Provides Cargo-style workspace management for Python projects.
//!
//! # Features
//!
//! - Glob pattern matching for workspace members
//! - Path dependency resolution between workspace members
//! - Shared dependency management
//! - Editable/development mode installation support
//!
//! # Example
//!
//! ```toml
//! [tool.dx-py.workspace]
//! members = ["packages/*", "libs/*"]
//! exclude = ["packages/deprecated"]
//! shared_dependencies = { requests = ">=2.28" }
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Workspace configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WorkspaceConfig {
    /// Glob patterns for workspace members
    #[serde(default)]
    pub members: Vec<String>,
    /// Glob patterns for excluded paths
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Shared dependencies across workspace
    #[serde(default)]
    pub shared_dependencies: HashMap<String, String>,
}

/// Path dependency information
#[derive(Debug, Clone)]
pub struct PathDependency {
    /// Package name
    pub name: String,
    /// Path to the dependency (relative to workspace root)
    pub path: PathBuf,
    /// Whether to install in editable mode
    pub editable: bool,
    /// Version constraint (if any)
    pub version: Option<String>,
}

/// A workspace member (individual project)
#[derive(Debug, Clone)]
pub struct WorkspaceMember {
    /// Path to the member project
    pub path: PathBuf,
    /// Project name
    pub name: String,
    /// Project version
    pub version: String,
    /// Dependencies
    pub dependencies: HashMap<String, String>,
    /// Dev dependencies
    pub dev_dependencies: HashMap<String, String>,
    /// Path dependencies (references to other workspace members or local packages)
    pub path_dependencies: Vec<PathDependency>,
}

impl WorkspaceMember {
    /// Load a workspace member from a directory
    pub fn load(path: &Path) -> Result<Self> {
        let pyproject_path = path.join("pyproject.toml");
        let pyproject_dx_path = path.join("pyproject.dx");

        if pyproject_dx_path.exists() {
            // Binary format - not implemented yet
            return Err(Error::Cache("Binary pyproject.dx not yet supported".to_string()));
        }

        if !pyproject_path.exists() {
            return Err(Error::Cache(format!("No pyproject.toml found in {}", path.display())));
        }

        let content = std::fs::read_to_string(&pyproject_path)?;
        let toml: toml::Value = toml::from_str(&content)
            .map_err(|e| Error::Cache(format!("Failed to parse pyproject.toml: {}", e)))?;

        let project = toml.get("project").ok_or_else(|| {
            Error::Cache("Missing [project] section in pyproject.toml".to_string())
        })?;

        let name = project
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Cache("Missing project.name".to_string()))?
            .to_string();

        let version =
            project.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0").to_string();

        let dependencies = Self::parse_dependencies(project.get("dependencies"));
        let dev_dependencies =
            Self::parse_optional_dependencies(project.get("optional-dependencies"), "dev");

        // Parse path dependencies from tool.dx-py.dependencies or project.dependencies
        let path_dependencies = Self::parse_path_dependencies(&toml, path);

        Ok(Self {
            path: path.to_path_buf(),
            name,
            version,
            dependencies,
            dev_dependencies,
            path_dependencies,
        })
    }

    /// Load a workspace member from a directory with workspace root context
    pub fn load_with_root(path: &Path, workspace_root: &Path) -> Result<Self> {
        let mut member = Self::load(path)?;

        // Resolve path dependencies relative to workspace root
        for path_dep in &mut member.path_dependencies {
            if path_dep.path.is_relative() {
                // First try relative to member path
                let member_relative = path.join(&path_dep.path);
                if member_relative.exists() {
                    path_dep.path = member_relative;
                } else {
                    // Then try relative to workspace root
                    let root_relative = workspace_root.join(&path_dep.path);
                    if root_relative.exists() {
                        path_dep.path = root_relative;
                    }
                }
            }
        }

        Ok(member)
    }

    /// Parse path dependencies from pyproject.toml
    fn parse_path_dependencies(toml: &toml::Value, member_path: &Path) -> Vec<PathDependency> {
        let mut path_deps = Vec::new();

        // Check tool.dx-py.dependencies for path dependencies
        if let Some(tool) = toml.get("tool") {
            if let Some(dx_py) = tool.get("dx-py") {
                if let Some(deps) = dx_py.get("dependencies") {
                    if let Some(table) = deps.as_table() {
                        for (name, value) in table {
                            if let Some(path_dep) =
                                Self::parse_path_dep_entry(name, value, member_path)
                            {
                                path_deps.push(path_dep);
                            }
                        }
                    }
                }
            }
        }

        // Also check project.dependencies for path syntax
        if let Some(project) = toml.get("project") {
            if let Some(deps) = project.get("dependencies") {
                if let Some(arr) = deps.as_array() {
                    for dep in arr {
                        if let Some(dep_str) = dep.as_str() {
                            // Check for @ file:// syntax
                            if dep_str.contains(" @ file://") {
                                if let Some(path_dep) =
                                    Self::parse_pep508_path_dep(dep_str, member_path)
                                {
                                    path_deps.push(path_dep);
                                }
                            }
                        }
                    }
                }
            }
        }

        path_deps
    }

    /// Parse a path dependency entry from tool.dx-py.dependencies
    fn parse_path_dep_entry(
        name: &str,
        value: &toml::Value,
        member_path: &Path,
    ) -> Option<PathDependency> {
        if let Some(table) = value.as_table() {
            if let Some(path_value) = table.get("path") {
                if let Some(path_str) = path_value.as_str() {
                    let path = PathBuf::from(path_str);
                    let editable = table.get("editable").and_then(|v| v.as_bool()).unwrap_or(true);
                    let version =
                        table.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());

                    return Some(PathDependency {
                        name: name.to_string(),
                        path: if path.is_relative() {
                            member_path.join(&path)
                        } else {
                            path
                        },
                        editable,
                        version,
                    });
                }
            }
        }
        None
    }

    /// Parse a PEP 508 path dependency (package @ file://path)
    fn parse_pep508_path_dep(dep_str: &str, member_path: &Path) -> Option<PathDependency> {
        let parts: Vec<&str> = dep_str.splitn(2, " @ file://").collect();
        if parts.len() != 2 {
            return None;
        }

        // Keep original name (don't normalize for path dependencies)
        let name = parts[0].trim().to_string();
        let path_str = parts[1].trim();
        let path = PathBuf::from(path_str);

        Some(PathDependency {
            name,
            path: if path.is_relative() {
                member_path.join(&path)
            } else {
                path
            },
            editable: true,
            version: None,
        })
    }

    /// Parse dependencies from TOML array
    fn parse_dependencies(deps: Option<&toml::Value>) -> HashMap<String, String> {
        let mut result = HashMap::new();

        if let Some(toml::Value::Array(arr)) = deps {
            for dep in arr {
                if let Some(dep_str) = dep.as_str() {
                    // Parse "package>=1.0" format
                    let (name, version) = Self::parse_dep_string(dep_str);
                    result.insert(name, version);
                }
            }
        }

        result
    }

    /// Parse optional dependencies group
    fn parse_optional_dependencies(
        optional: Option<&toml::Value>,
        group: &str,
    ) -> HashMap<String, String> {
        let mut result = HashMap::new();

        if let Some(toml::Value::Table(table)) = optional {
            if let Some(toml::Value::Array(arr)) = table.get(group) {
                for dep in arr {
                    if let Some(dep_str) = dep.as_str() {
                        let (name, version) = Self::parse_dep_string(dep_str);
                        result.insert(name, version);
                    }
                }
            }
        }

        result
    }

    /// Parse a dependency string like "requests>=2.0"
    fn parse_dep_string(s: &str) -> (String, String) {
        // Find version constraint operator
        let operators = [">=", "<=", "==", "!=", "~=", ">", "<"];
        for op in operators {
            if let Some(idx) = s.find(op) {
                return (s[..idx].trim().to_string(), s[idx..].trim().to_string());
            }
        }
        (s.trim().to_string(), "*".to_string())
    }
}

/// Workspace manager
///
/// Manages Cargo-style workspaces for Python projects.
pub struct WorkspaceManager {
    /// Workspace root directory
    root: PathBuf,
    /// Workspace configuration
    config: WorkspaceConfig,
    /// Cached workspace members
    members_cache: Option<Vec<WorkspaceMember>>,
}

impl WorkspaceManager {
    /// Load a workspace from a directory
    pub fn load(root: &Path) -> Result<Self> {
        let config = Self::load_config(root)?;

        Ok(Self {
            root: root.to_path_buf(),
            config,
            members_cache: None,
        })
    }

    /// Load workspace configuration
    fn load_config(root: &Path) -> Result<WorkspaceConfig> {
        let pyproject_dx = root.join("pyproject.dx");
        let pyproject_toml = root.join("pyproject.toml");

        if pyproject_dx.exists() {
            // Binary format - not implemented yet
            return Err(Error::Cache("Binary pyproject.dx not yet supported".to_string()));
        }

        if !pyproject_toml.exists() {
            // No workspace config, return empty
            return Ok(WorkspaceConfig::default());
        }

        let content = std::fs::read_to_string(&pyproject_toml)?;
        let toml: toml::Value = toml::from_str(&content)
            .map_err(|e| Error::Cache(format!("Failed to parse pyproject.toml: {}", e)))?;

        // Check for [tool.dx-py.workspace] section
        let workspace_config =
            toml.get("tool").and_then(|t| t.get("dx-py")).and_then(|d| d.get("workspace"));

        if let Some(ws) = workspace_config {
            let config: WorkspaceConfig = ws
                .clone()
                .try_into()
                .map_err(|e| Error::Cache(format!("Failed to parse workspace config: {}", e)))?;
            return Ok(config);
        }

        Ok(WorkspaceConfig::default())
    }

    /// Get the workspace root
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the workspace configuration
    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }

    /// Get all workspace members
    pub fn members(&mut self) -> Result<&[WorkspaceMember]> {
        if self.members_cache.is_none() {
            self.members_cache = Some(self.discover_members()?);
        }
        Ok(self.members_cache.as_ref().unwrap())
    }

    /// Enumerate workspace members based on glob patterns
    ///
    /// This method parses the workspace.members glob patterns and returns
    /// all directories that match the patterns and contain a pyproject.toml.
    pub fn enumerate_members(&self) -> Result<Vec<PathBuf>> {
        let mut member_paths = Vec::new();

        if self.config.members.is_empty() {
            // No workspace members defined, check if root is a project
            if self.root.join("pyproject.toml").exists() {
                member_paths.push(self.root.clone());
            }
            return Ok(member_paths);
        }

        for pattern in &self.config.members {
            let full_pattern = self.root.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            for entry in glob::glob(&pattern_str)
                .map_err(|e| Error::Cache(format!("Invalid glob pattern '{}': {}", pattern, e)))?
            {
                let path = entry.map_err(|e| Error::Cache(format!("Glob error: {}", e)))?;

                // Check if excluded
                if self.is_excluded(&path) {
                    continue;
                }

                // Check if it's a project directory
                if path.join("pyproject.toml").exists() || path.join("pyproject.dx").exists() {
                    member_paths.push(path);
                }
            }
        }

        // Sort for deterministic ordering
        member_paths.sort();

        Ok(member_paths)
    }

    /// Discover workspace members based on glob patterns
    fn discover_members(&self) -> Result<Vec<WorkspaceMember>> {
        let member_paths = self.enumerate_members()?;
        let mut members = Vec::new();

        for path in member_paths {
            match WorkspaceMember::load_with_root(&path, &self.root) {
                Ok(member) => members.push(member),
                Err(e) => {
                    // Log warning but continue with other members
                    eprintln!(
                        "Warning: Failed to load workspace member at {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        Ok(members)
    }

    /// Check if a path is excluded
    fn is_excluded(&self, path: &Path) -> bool {
        for pattern in &self.config.exclude {
            let full_pattern = self.root.join(pattern);
            if let Ok(glob_pattern) = glob::Pattern::new(&full_pattern.to_string_lossy()) {
                if glob_pattern.matches_path(path) {
                    return true;
                }
            }
        }
        false
    }

    /// Get all dependencies across the workspace
    pub fn all_dependencies(&mut self) -> Result<HashMap<String, String>> {
        let mut all_deps = HashMap::new();

        // Add shared dependencies first
        for (name, version) in &self.config.shared_dependencies {
            all_deps.insert(name.clone(), version.clone());
        }

        // Collect member dependencies
        let member_deps: Vec<(String, String)> = self
            .members()?
            .iter()
            .flat_map(|m| m.dependencies.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Add member dependencies (shared takes precedence)
        for (name, version) in member_deps {
            if !self.config.shared_dependencies.contains_key(&name) {
                all_deps.insert(name, version);
            }
        }

        Ok(all_deps)
    }

    /// Get all path dependencies across the workspace
    pub fn all_path_dependencies(&mut self) -> Result<Vec<PathDependency>> {
        let mut path_deps = Vec::new();

        for member in self.members()? {
            for dep in &member.path_dependencies {
                // Avoid duplicates
                if !path_deps.iter().any(|d: &PathDependency| d.name == dep.name) {
                    path_deps.push(dep.clone());
                }
            }
        }

        Ok(path_deps)
    }

    /// Resolve a path dependency to a workspace member
    pub fn resolve_path_dependency(
        &mut self,
        dep: &PathDependency,
    ) -> Result<Option<&WorkspaceMember>> {
        let members = self.members()?;

        // First try to find by path
        for member in members {
            if member.path == dep.path {
                return Ok(Some(member));
            }
        }

        // Then try to find by name
        for member in members {
            if member.name == dep.name {
                return Ok(Some(member));
            }
        }

        Ok(None)
    }

    /// Get inter-workspace dependencies (dependencies between workspace members)
    pub fn inter_workspace_dependencies(&mut self) -> Result<HashMap<String, Vec<String>>> {
        let mut deps_map: HashMap<String, Vec<String>> = HashMap::new();

        // Get all member names
        let member_names: std::collections::HashSet<String> =
            self.members()?.iter().map(|m| m.name.clone()).collect();

        // For each member, find dependencies that are other workspace members
        for member in self.members()? {
            let mut inter_deps = Vec::new();

            // Check path dependencies
            for path_dep in &member.path_dependencies {
                if member_names.contains(&path_dep.name) {
                    inter_deps.push(path_dep.name.clone());
                }
            }

            // Check regular dependencies that might reference workspace members
            for dep_name in member.dependencies.keys() {
                let normalized = dep_name.replace('-', "_").to_lowercase();
                for member_name in &member_names {
                    let normalized_member = member_name.replace('-', "_").to_lowercase();
                    if normalized == normalized_member && !inter_deps.contains(member_name) {
                        inter_deps.push(member_name.clone());
                    }
                }
            }

            if !inter_deps.is_empty() {
                deps_map.insert(member.name.clone(), inter_deps);
            }
        }

        Ok(deps_map)
    }

    /// Check if this is a workspace (has members defined)
    pub fn is_workspace(&self) -> bool {
        !self.config.members.is_empty()
    }

    /// Find a member by name
    pub fn find_member(&mut self, name: &str) -> Result<Option<&WorkspaceMember>> {
        let members = self.members()?;
        Ok(members.iter().find(|m| m.name == name))
    }

    /// Find a member by path
    pub fn find_member_by_path(&mut self, path: &Path) -> Result<Option<&WorkspaceMember>> {
        let members = self.members()?;
        Ok(members.iter().find(|m| m.path == path))
    }

    /// Get topologically sorted members (dependencies first)
    pub fn sorted_members(&mut self) -> Result<Vec<&WorkspaceMember>> {
        let inter_deps = self.inter_workspace_dependencies()?;
        let members = self.members()?;

        // Build dependency graph
        // in_degree[X] = number of packages that X depends on (that haven't been processed yet)
        // graph[X] = list of packages that depend on X (X's dependents)
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize all members with in_degree 0
        for member in members {
            in_degree.insert(&member.name, 0);
            dependents.entry(&member.name).or_default();
        }

        // Build the graph: for each member, record its dependencies
        // inter_deps maps member_name -> list of dependencies
        for (member_name, deps) in &inter_deps {
            // member_name depends on each dep
            // So member_name's in_degree increases by the number of deps
            if let Some(degree) = in_degree.get_mut(member_name.as_str()) {
                *degree += deps.len();
            }
            // Each dep has member_name as a dependent
            for dep in deps {
                dependents.entry(dep.as_str()).or_default().push(member_name.as_str());
            }
        }

        // Kahn's algorithm for topological sort
        // Start with nodes that have no dependencies (in_degree == 0)
        let mut queue: std::collections::VecDeque<&str> =
            in_degree.iter().filter(|(_, &d)| d == 0).map(|(&n, _)| n).collect();

        // Sort the initial queue for deterministic ordering
        let mut queue_vec: Vec<&str> = queue.drain(..).collect();
        queue_vec.sort();
        queue = queue_vec.into_iter().collect();

        let mut sorted_names = Vec::new();

        while let Some(name) = queue.pop_front() {
            sorted_names.push(name);

            // For each package that depends on 'name', decrease its in_degree
            if let Some(deps_of_name) = dependents.get(name) {
                let mut new_ready: Vec<&str> = Vec::new();
                for &dependent in deps_of_name {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            new_ready.push(dependent);
                        }
                    }
                }
                // Sort for deterministic ordering
                new_ready.sort();
                for dep in new_ready {
                    queue.push_back(dep);
                }
            }
        }

        // Map names back to members
        let mut sorted_members = Vec::new();
        for name in sorted_names {
            if let Some(member) = members.iter().find(|m| m.name == name) {
                sorted_members.push(member);
            }
        }

        Ok(sorted_members)
    }

    /// Clear the members cache (useful after modifications)
    pub fn clear_cache(&mut self) {
        self.members_cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_config_default() {
        let config = WorkspaceConfig::default();
        assert!(config.members.is_empty());
        assert!(config.exclude.is_empty());
        assert!(config.shared_dependencies.is_empty());
    }

    #[test]
    fn test_workspace_member_parse_dep_string() {
        let (name, version) = WorkspaceMember::parse_dep_string("requests>=2.0");
        assert_eq!(name, "requests");
        assert_eq!(version, ">=2.0");

        let (name, version) = WorkspaceMember::parse_dep_string("flask");
        assert_eq!(name, "flask");
        assert_eq!(version, "*");
    }

    #[test]
    fn test_workspace_manager_empty() {
        let temp_dir = TempDir::new().unwrap();
        let manager = WorkspaceManager::load(temp_dir.path()).unwrap();
        assert!(!manager.is_workspace());
    }

    #[test]
    fn test_workspace_manager_with_pyproject() {
        let temp_dir = TempDir::new().unwrap();

        // Create a simple pyproject.toml
        let pyproject = r#"
[project]
name = "test-project"
version = "1.0.0"
dependencies = ["requests>=2.0", "flask"]
"#;
        std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

        let mut manager = WorkspaceManager::load(temp_dir.path()).unwrap();
        let members = manager.members().unwrap();

        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "test-project");
        assert_eq!(members[0].version, "1.0.0");
        assert!(members[0].dependencies.contains_key("requests"));
        assert!(members[0].dependencies.contains_key("flask"));
    }

    #[test]
    fn test_workspace_manager_with_workspace_config() {
        let temp_dir = TempDir::new().unwrap();

        // Create workspace pyproject.toml
        let pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
shared_dependencies = { requests = ">=2.28" }
"#;
        std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

        // Create a package
        let pkg_dir = temp_dir.path().join("packages").join("pkg-a");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let pkg_pyproject = r#"
[project]
name = "pkg-a"
version = "0.1.0"
dependencies = ["flask"]
"#;
        std::fs::write(pkg_dir.join("pyproject.toml"), pkg_pyproject).unwrap();

        let mut manager = WorkspaceManager::load(temp_dir.path()).unwrap();
        assert!(manager.is_workspace());

        let members = manager.members().unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "pkg-a");

        let all_deps = manager.all_dependencies().unwrap();
        assert!(all_deps.contains_key("requests"));
        assert!(all_deps.contains_key("flask"));
        // Shared dependency should take precedence
        assert_eq!(all_deps.get("requests"), Some(&">=2.28".to_string()));
    }

    #[test]
    fn test_workspace_enumerate_members() {
        let temp_dir = TempDir::new().unwrap();

        // Create workspace pyproject.toml
        let pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*", "libs/*"]
exclude = ["packages/deprecated"]
"#;
        std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

        // Create packages
        for name in &["pkg-a", "pkg-b", "deprecated"] {
            let pkg_dir = temp_dir.path().join("packages").join(name);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            let pkg_pyproject = format!(
                r#"
[project]
name = "{}"
version = "0.1.0"
"#,
                name
            );
            std::fs::write(pkg_dir.join("pyproject.toml"), pkg_pyproject).unwrap();
        }

        // Create libs
        let lib_dir = temp_dir.path().join("libs").join("lib-a");
        std::fs::create_dir_all(&lib_dir).unwrap();
        let lib_pyproject = r#"
[project]
name = "lib-a"
version = "0.1.0"
"#;
        std::fs::write(lib_dir.join("pyproject.toml"), lib_pyproject).unwrap();

        let manager = WorkspaceManager::load(temp_dir.path()).unwrap();
        let member_paths = manager.enumerate_members().unwrap();

        // Should have pkg-a, pkg-b, lib-a (deprecated is excluded)
        assert_eq!(member_paths.len(), 3);

        let path_names: Vec<String> = member_paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(path_names.contains(&"pkg-a".to_string()));
        assert!(path_names.contains(&"pkg-b".to_string()));
        assert!(path_names.contains(&"lib-a".to_string()));
        assert!(!path_names.contains(&"deprecated".to_string()));
    }

    #[test]
    fn test_workspace_path_dependencies() {
        let temp_dir = TempDir::new().unwrap();

        // Create workspace pyproject.toml
        let pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
"#;
        std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

        // Create pkg-a that depends on pkg-b via path
        let pkg_a_dir = temp_dir.path().join("packages").join("pkg-a");
        std::fs::create_dir_all(&pkg_a_dir).unwrap();
        let pkg_a_pyproject = r#"
[project]
name = "pkg-a"
version = "0.1.0"
dependencies = ["pkg-b @ file://../pkg-b"]
"#;
        std::fs::write(pkg_a_dir.join("pyproject.toml"), pkg_a_pyproject).unwrap();

        // Create pkg-b
        let pkg_b_dir = temp_dir.path().join("packages").join("pkg-b");
        std::fs::create_dir_all(&pkg_b_dir).unwrap();
        let pkg_b_pyproject = r#"
[project]
name = "pkg-b"
version = "0.1.0"
"#;
        std::fs::write(pkg_b_dir.join("pyproject.toml"), pkg_b_pyproject).unwrap();

        let mut manager = WorkspaceManager::load(temp_dir.path()).unwrap();
        let members = manager.members().unwrap();

        assert_eq!(members.len(), 2);

        // Find pkg-a and check its path dependencies
        let pkg_a = members.iter().find(|m| m.name == "pkg-a").unwrap();
        assert_eq!(pkg_a.path_dependencies.len(), 1);
        assert_eq!(pkg_a.path_dependencies[0].name, "pkg-b");
    }

    #[test]
    fn test_workspace_inter_dependencies() {
        let temp_dir = TempDir::new().unwrap();

        // Create workspace pyproject.toml
        let pyproject = r#"
[project]
name = "workspace-root"
version = "1.0.0"

[tool.dx-py.workspace]
members = ["packages/*"]
"#;
        std::fs::write(temp_dir.path().join("pyproject.toml"), pyproject).unwrap();

        // Create pkg-a that depends on pkg-b
        let pkg_a_dir = temp_dir.path().join("packages").join("pkg-a");
        std::fs::create_dir_all(&pkg_a_dir).unwrap();
        let pkg_a_pyproject = r#"
[project]
name = "pkg-a"
version = "0.1.0"
dependencies = ["pkg-b @ file://../pkg-b"]
"#;
        std::fs::write(pkg_a_dir.join("pyproject.toml"), pkg_a_pyproject).unwrap();

        // Create pkg-b
        let pkg_b_dir = temp_dir.path().join("packages").join("pkg-b");
        std::fs::create_dir_all(&pkg_b_dir).unwrap();
        let pkg_b_pyproject = r#"
[project]
name = "pkg-b"
version = "0.1.0"
"#;
        std::fs::write(pkg_b_dir.join("pyproject.toml"), pkg_b_pyproject).unwrap();

        let mut manager = WorkspaceManager::load(temp_dir.path()).unwrap();
        let inter_deps = manager.inter_workspace_dependencies().unwrap();

        // pkg-a should depend on pkg-b
        assert!(inter_deps.contains_key("pkg-a"));
    }

    #[test]
    fn test_path_dependency_struct() {
        let dep = PathDependency {
            name: "my-package".to_string(),
            path: PathBuf::from("../my-package"),
            editable: true,
            version: Some(">=1.0".to_string()),
        };

        assert_eq!(dep.name, "my-package");
        assert!(dep.editable);
        assert_eq!(dep.version, Some(">=1.0".to_string()));
    }
}
