//! Workspace Manager
//!
//! Loads and maintains the Binary Workspace Manifest.

use crate::bwm::{BwmHeader, BwmSerializer, PackageData, WorkspaceData};
use crate::error::WorkspaceError;
use memmap2::Mmap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Hash of a package.json file for change detection
pub type PackageJsonHash = [u8; 32];

/// Workspace Manager for loading and querying workspace manifests
pub struct WorkspaceManager {
    /// Memory-mapped manifest data
    mmap: Option<Mmap>,
    /// Parsed workspace data (for non-mmap access)
    pub data: Option<WorkspaceData>,
    /// Path to manifest file
    manifest_path: Option<PathBuf>,
    /// Package name to index lookup
    name_index: rustc_hash::FxHashMap<String, u32>,
    /// Workspace root directory
    workspace_root: Option<PathBuf>,
    /// Cached hashes of package.json files for change detection
    pub package_json_hashes: rustc_hash::FxHashMap<PathBuf, PackageJsonHash>,
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new() -> Self {
        Self {
            mmap: None,
            data: None,
            manifest_path: None,
            name_index: rustc_hash::FxHashMap::default(),
            workspace_root: None,
            package_json_hashes: rustc_hash::FxHashMap::default(),
        }
    }

    /// Create a new workspace manager with a workspace root
    pub fn with_root(root: PathBuf) -> Self {
        Self {
            mmap: None,
            data: None,
            manifest_path: None,
            name_index: rustc_hash::FxHashMap::default(),
            workspace_root: Some(root),
            package_json_hashes: rustc_hash::FxHashMap::default(),
        }
    }

    /// Set the workspace root directory
    pub fn set_workspace_root(&mut self, root: PathBuf) {
        self.workspace_root = Some(root);
    }

    /// Load workspace manifest from memory-mapped file
    pub fn load(&mut self, path: &Path) -> Result<(), WorkspaceError> {
        let file = std::fs::File::open(path).map_err(|_| WorkspaceError::ManifestNotFound {
            path: path.to_path_buf(),
        })?;

        // Memory-map the file
        let mmap = unsafe { Mmap::map(&file) }?;

        // Validate header
        if mmap.len() < BwmHeader::SIZE {
            return Err(WorkspaceError::ManifestCorrupted {
                reason: "file too small".to_string(),
                hash_mismatch: false,
            });
        }

        let header: &BwmHeader = bytemuck::from_bytes(&mmap[..BwmHeader::SIZE]);
        header.validate_magic()?;
        header.validate_version()?;

        // Verify content hash
        let computed_hash = blake3::hash(&mmap[BwmHeader::SIZE..]);
        if computed_hash.as_bytes() != &header.content_hash {
            return Err(WorkspaceError::ManifestCorrupted {
                reason: "content hash mismatch".to_string(),
                hash_mismatch: true,
            });
        }

        // Parse and build index
        let data = BwmSerializer::deserialize(&mmap)?;
        self.build_name_index(&data);

        self.mmap = Some(mmap);
        self.data = Some(data);
        self.manifest_path = Some(path.to_path_buf());

        Ok(())
    }

    /// Load from raw bytes (for testing)
    pub fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), WorkspaceError> {
        let data = BwmSerializer::deserialize(bytes)?;
        self.build_name_index(&data);
        self.data = Some(data);
        Ok(())
    }

    /// Build name-to-index lookup table
    fn build_name_index(&mut self, data: &WorkspaceData) {
        self.name_index.clear();
        for (idx, pkg) in data.packages.iter().enumerate() {
            self.name_index.insert(pkg.name.clone(), idx as u32);
        }
    }

    /// Get package by name with O(1) lookup
    pub fn get_package(&self, name: &str) -> Option<&crate::bwm::PackageData> {
        let idx = *self.name_index.get(name)?;
        self.data.as_ref()?.packages.get(idx as usize)
    }

    /// Get package by index
    pub fn get_package_by_index(&self, idx: u32) -> Option<&crate::bwm::PackageData> {
        self.data.as_ref()?.packages.get(idx as usize)
    }

    /// Get all packages in topological order
    pub fn topological_order(&self) -> &[u32] {
        self.data.as_ref().map(|d| d.topological_order.as_slice()).unwrap_or(&[])
    }

    /// Get direct dependencies of a package
    pub fn dependencies(&self, pkg_idx: u32) -> Vec<u32> {
        let data = match &self.data {
            Some(d) => d,
            None => return Vec::new(),
        };

        data.dependency_edges
            .iter()
            .filter(|(from, _)| *from == pkg_idx)
            .map(|(_, to)| *to)
            .collect()
    }

    /// Get number of packages
    pub fn package_count(&self) -> usize {
        self.data.as_ref().map(|d| d.packages.len()).unwrap_or(0)
    }

    /// Incrementally update manifest when package.json changes
    pub fn update_package(&mut self, path: &Path) -> Result<(), WorkspaceError> {
        // Check if this is a package.json file
        if !path.ends_with("package.json") {
            return Ok(());
        }

        // Compute hash of the changed file
        let new_hash = self.compute_package_json_hash(path)?;

        // Check if the file actually changed
        if let Some(old_hash) = self.package_json_hashes.get(path) {
            if *old_hash == new_hash {
                // No actual change, skip regeneration
                return Ok(());
            }
        }

        // Update the hash cache
        self.package_json_hashes.insert(path.to_path_buf(), new_hash);

        // Trigger incremental regeneration
        self.regenerate_incremental(path)
    }

    /// Regenerate entire manifest from source files
    pub fn regenerate(&mut self) -> Result<(), WorkspaceError> {
        let root = self.workspace_root.clone().ok_or_else(|| WorkspaceError::ManifestNotFound {
            path: PathBuf::from("workspace root not set"),
        })?;

        // Scan for all package.json files
        let package_jsons = self.scan_package_jsons(&root)?;

        // Parse each package.json and build workspace data
        let mut packages = Vec::new();
        let mut name_to_idx: rustc_hash::FxHashMap<String, u32> = rustc_hash::FxHashMap::default();

        for (idx, pkg_path) in package_jsons.iter().enumerate() {
            let pkg_data = self.parse_package_json(pkg_path)?;
            name_to_idx.insert(pkg_data.name.clone(), idx as u32);

            // Update hash cache
            let hash = self.compute_package_json_hash(pkg_path)?;
            self.package_json_hashes.insert(pkg_path.clone(), hash);

            packages.push(pkg_data);
        }

        // Build dependency edges
        let dependency_edges = self.build_dependency_edges(&packages, &name_to_idx);

        // Create workspace data
        let mut workspace_data = WorkspaceData {
            packages,
            dependency_edges,
            topological_order: Vec::new(),
        };

        // Compute topological order
        workspace_data.compute_topological_order()?;

        // Update internal state
        self.build_name_index(&workspace_data);
        self.data = Some(workspace_data);

        // Serialize and save if manifest path is set
        if let Some(manifest_path) = &self.manifest_path {
            self.save_manifest(manifest_path)?;
        }

        Ok(())
    }

    /// Regenerate only the affected parts of the manifest
    fn regenerate_incremental(&mut self, changed_path: &Path) -> Result<(), WorkspaceError> {
        // Parse the changed package.json
        let changed_pkg = self.parse_package_json(changed_path)?;

        // Check if this is a new package or an update
        let existing_idx = self.name_index.get(&changed_pkg.name).copied();

        if let Some(idx) = existing_idx {
            if let Some(data) = &mut self.data {
                // Update existing package
                data.packages[idx as usize] = changed_pkg;

                // Rebuild dependency edges (dependencies might have changed)
                let name_to_idx: rustc_hash::FxHashMap<String, u32> = data
                    .packages
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (p.name.clone(), i as u32))
                    .collect();

                let packages_ref = &data.packages;
                data.dependency_edges = build_dependency_edges_static(packages_ref, &name_to_idx);

                // Recompute topological order
                data.compute_topological_order()?;

                return Ok(());
            }
        }

        // New package or no existing data - do full regeneration
        self.regenerate()
    }

    /// Detect added/removed packages by scanning the workspace
    pub fn detect_package_changes(&self) -> Result<PackageChanges, WorkspaceError> {
        let root =
            self.workspace_root.as_ref().ok_or_else(|| WorkspaceError::ManifestNotFound {
                path: PathBuf::from("workspace root not set"),
            })?;

        // Scan current package.json files
        let current_packages = self.scan_package_jsons(root)?;
        let current_set: HashSet<PathBuf> = current_packages.into_iter().collect();

        // Get previously known packages
        let known_set: HashSet<PathBuf> = self.package_json_hashes.keys().cloned().collect();

        // Find added packages
        let added: Vec<PathBuf> = current_set.difference(&known_set).cloned().collect();

        // Find removed packages
        let removed: Vec<PathBuf> = known_set.difference(&current_set).cloned().collect();

        // Find modified packages (hash changed)
        let mut modified = Vec::new();
        for path in current_set.intersection(&known_set) {
            let new_hash = self.compute_package_json_hash(path)?;
            if let Some(old_hash) = self.package_json_hashes.get(path) {
                if *old_hash != new_hash {
                    modified.push(path.clone());
                }
            }
        }

        Ok(PackageChanges {
            added,
            removed,
            modified,
        })
    }

    /// Apply detected package changes to the workspace
    pub fn apply_package_changes(
        &mut self,
        changes: &PackageChanges,
    ) -> Result<(), WorkspaceError> {
        if changes.is_empty() {
            return Ok(());
        }

        // Handle removed packages
        for removed_path in &changes.removed {
            self.package_json_hashes.remove(removed_path);
        }

        // For added or modified packages, trigger regeneration
        if !changes.added.is_empty() || !changes.modified.is_empty() {
            self.regenerate()?;
        } else if !changes.removed.is_empty() {
            // Only removals - rebuild from remaining packages
            self.regenerate()?;
        }

        Ok(())
    }

    /// Scan for package.json files in the workspace
    fn scan_package_jsons(&self, root: &Path) -> Result<Vec<PathBuf>, WorkspaceError> {
        let mut package_jsons = Vec::new();
        Self::scan_directory_for_packages(root, &mut package_jsons, 0)?;
        Ok(package_jsons)
    }

    /// Recursively scan a directory for package.json files
    fn scan_directory_for_packages(
        dir: &Path,
        results: &mut Vec<PathBuf>,
        depth: usize,
    ) -> Result<(), WorkspaceError> {
        // Limit recursion depth to avoid infinite loops
        const MAX_DEPTH: usize = 10;
        if depth > MAX_DEPTH {
            return Ok(());
        }

        // Skip node_modules and hidden directories (but not the root)
        if depth > 0 {
            if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
                if name == "node_modules" || name.starts_with('.') {
                    return Ok(());
                }
            }
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return Ok(()), // Skip unreadable directories
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() && path.file_name().map(|n| n == "package.json").unwrap_or(false) {
                results.push(path);
            } else if path.is_dir() {
                Self::scan_directory_for_packages(&path, results, depth + 1)?;
            }
        }

        Ok(())
    }

    /// Compute Blake3 hash of a package.json file
    fn compute_package_json_hash(&self, path: &Path) -> Result<PackageJsonHash, WorkspaceError> {
        let content = std::fs::read(path).map_err(WorkspaceError::Io)?;
        let hash = blake3::hash(&content);
        Ok(*hash.as_bytes())
    }

    /// Parse a package.json file into PackageData
    #[cfg(feature = "config")]
    fn parse_package_json(&self, path: &Path) -> Result<PackageData, WorkspaceError> {
        let content = std::fs::read_to_string(path).map_err(WorkspaceError::Io)?;

        let json: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| WorkspaceError::ManifestCorrupted {
                reason: format!("Invalid JSON in {}: {}", path.display(), e),
                hash_mismatch: false,
            })?;

        let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed").to_string();

        let version_str = json.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0");
        let version = parse_version(version_str);

        let is_private = json.get("private").and_then(|v| v.as_bool()).unwrap_or(false);

        // Extract dependencies
        let mut dependencies = Vec::new();
        for dep_field in &["dependencies", "devDependencies", "peerDependencies"] {
            if let Some(deps) = json.get(*dep_field).and_then(|v| v.as_object()) {
                for dep_name in deps.keys() {
                    dependencies.push(dep_name.clone());
                }
            }
        }

        // Get relative path from workspace root
        let pkg_path = if let Some(root) = &self.workspace_root {
            path.parent()
                .and_then(|p| p.strip_prefix(root).ok())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| path.parent().unwrap_or(path).to_string_lossy().to_string())
        } else {
            path.parent().unwrap_or(path).to_string_lossy().to_string()
        };

        Ok(PackageData {
            name,
            path: pkg_path,
            version,
            dependencies,
            is_private,
        })
    }

    /// Parse a package.json file into PackageData (fallback without serde_json)
    #[cfg(not(feature = "config"))]
    fn parse_package_json(&self, path: &Path) -> Result<PackageData, WorkspaceError> {
        let content = std::fs::read_to_string(path).map_err(|e| WorkspaceError::Io(e))?;

        // Simple JSON parsing without serde_json
        let name = extract_json_string(&content, "name").unwrap_or_else(|| "unnamed".to_string());
        let version_str =
            extract_json_string(&content, "version").unwrap_or_else(|| "0.0.0".to_string());
        let version = parse_version(&version_str);
        let is_private =
            content.contains("\"private\": true") || content.contains("\"private\":true");

        // Get relative path from workspace root
        let pkg_path = if let Some(root) = &self.workspace_root {
            path.parent()
                .and_then(|p| p.strip_prefix(root).ok())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| path.parent().unwrap_or(path).to_string_lossy().to_string())
        } else {
            path.parent().unwrap_or(path).to_string_lossy().to_string()
        };

        Ok(PackageData {
            name,
            path: pkg_path,
            version,
            dependencies: Vec::new(), // Cannot parse dependencies without serde_json
            is_private,
        })
    }

    /// Build dependency edges from package data
    fn build_dependency_edges(
        &self,
        packages: &[PackageData],
        name_to_idx: &rustc_hash::FxHashMap<String, u32>,
    ) -> Vec<(u32, u32)> {
        let mut edges = Vec::new();

        for (from_idx, pkg) in packages.iter().enumerate() {
            for dep_name in &pkg.dependencies {
                if let Some(&to_idx) = name_to_idx.get(dep_name) {
                    edges.push((from_idx as u32, to_idx));
                }
            }
        }

        edges
    }

    /// Save the manifest to disk
    fn save_manifest(&self, path: &Path) -> Result<(), WorkspaceError> {
        if let Some(data) = &self.data {
            let bytes = BwmSerializer::serialize(data)?;
            std::fs::write(path, bytes).map_err(WorkspaceError::Io)?;
        }
        Ok(())
    }

    /// Check if manifest is loaded
    pub fn is_loaded(&self) -> bool {
        self.data.is_some()
    }
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents changes detected in workspace packages
#[derive(Debug, Clone, Default)]
pub struct PackageChanges {
    /// Newly added package.json files
    pub added: Vec<PathBuf>,
    /// Removed package.json files
    pub removed: Vec<PathBuf>,
    /// Modified package.json files (content changed)
    pub modified: Vec<PathBuf>,
}

impl PackageChanges {
    /// Check if there are no changes
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    /// Get total number of changes
    pub fn len(&self) -> usize {
        self.added.len() + self.removed.len() + self.modified.len()
    }
}

/// Parse a semver version string into (major, minor, patch)
fn parse_version(version: &str) -> (u16, u16, u16) {
    let parts: Vec<&str> = version.split('.').collect();
    let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts
        .get(2)
        .and_then(|s| {
            // Handle versions like "1.0.0-beta.1"
            s.split('-').next().and_then(|p| p.parse().ok())
        })
        .unwrap_or(0);
    (major, minor, patch)
}

/// Build dependency edges from package data (static version for borrow checker)
fn build_dependency_edges_static(
    packages: &[PackageData],
    name_to_idx: &rustc_hash::FxHashMap<String, u32>,
) -> Vec<(u32, u32)> {
    let mut edges = Vec::new();

    for (from_idx, pkg) in packages.iter().enumerate() {
        for dep_name in &pkg.dependencies {
            if let Some(&to_idx) = name_to_idx.get(dep_name) {
                edges.push((from_idx as u32, to_idx));
            }
        }
    }

    edges
}

/// Simple JSON string extraction without serde_json
#[cfg(not(feature = "config"))]
fn extract_json_string(content: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let start = content.find(&pattern)?;
    let after_key = &content[start + pattern.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();

    if !after_colon.starts_with('"') {
        return None;
    }

    let value_start = 1; // Skip opening quote
    let value_end = after_colon[value_start..].find('"')?;
    Some(after_colon[value_start..value_start + value_end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bwm::PackageData;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_workspace() -> WorkspaceData {
        let mut data = WorkspaceData {
            packages: vec![
                PackageData {
                    name: "pkg-a".to_string(),
                    path: "packages/a".to_string(),
                    version: (1, 0, 0),
                    dependencies: vec![],
                    is_private: false,
                },
                PackageData {
                    name: "pkg-b".to_string(),
                    path: "packages/b".to_string(),
                    version: (1, 0, 0),
                    dependencies: vec!["pkg-a".to_string()],
                    is_private: false,
                },
                PackageData {
                    name: "pkg-c".to_string(),
                    path: "packages/c".to_string(),
                    version: (1, 0, 0),
                    dependencies: vec!["pkg-b".to_string()],
                    is_private: false,
                },
            ],
            dependency_edges: vec![(0, 1), (1, 2)],
            topological_order: vec![],
        };
        data.compute_topological_order().unwrap();
        data
    }

    fn create_temp_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Create package directories
        let pkg_a = temp_dir.path().join("packages/a");
        let pkg_b = temp_dir.path().join("packages/b");
        fs::create_dir_all(&pkg_a).unwrap();
        fs::create_dir_all(&pkg_b).unwrap();

        // Create package.json files
        fs::write(pkg_a.join("package.json"), r#"{"name": "pkg-a", "version": "1.0.0"}"#).unwrap();

        fs::write(
            pkg_b.join("package.json"),
            r#"{"name": "pkg-b", "version": "1.0.0", "dependencies": {"pkg-a": "^1.0.0"}}"#,
        )
        .unwrap();

        temp_dir
    }

    #[test]
    fn test_workspace_manager_load_from_bytes() {
        let data = create_test_workspace();
        let bytes = BwmSerializer::serialize(&data).unwrap();

        let mut manager = WorkspaceManager::new();
        manager.load_from_bytes(&bytes).unwrap();

        assert_eq!(manager.package_count(), 3);
        assert!(manager.get_package("pkg-a").is_some());
        assert!(manager.get_package("pkg-b").is_some());
        assert!(manager.get_package("pkg-c").is_some());
        assert!(manager.get_package("pkg-d").is_none());
    }

    #[test]
    fn test_package_lookup_by_index() {
        let data = create_test_workspace();
        let bytes = BwmSerializer::serialize(&data).unwrap();

        let mut manager = WorkspaceManager::new();
        manager.load_from_bytes(&bytes).unwrap();

        let pkg = manager.get_package_by_index(0).unwrap();
        assert_eq!(pkg.name, "pkg-a");

        let pkg = manager.get_package_by_index(1).unwrap();
        assert_eq!(pkg.name, "pkg-b");

        assert!(manager.get_package_by_index(100).is_none());
    }

    #[test]
    fn test_topological_order() {
        let data = create_test_workspace();
        let bytes = BwmSerializer::serialize(&data).unwrap();

        let mut manager = WorkspaceManager::new();
        manager.load_from_bytes(&bytes).unwrap();

        let order = manager.topological_order();
        assert_eq!(order.len(), 3);

        // pkg-a should come before pkg-b, pkg-b before pkg-c
        let pos_a = order.iter().position(|&x| x == 0).unwrap();
        let pos_b = order.iter().position(|&x| x == 1).unwrap();
        let pos_c = order.iter().position(|&x| x == 2).unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_dependencies() {
        let data = create_test_workspace();
        let bytes = BwmSerializer::serialize(&data).unwrap();

        let mut manager = WorkspaceManager::new();
        manager.load_from_bytes(&bytes).unwrap();

        // pkg-a depends on pkg-b (edge 0 -> 1)
        let deps = manager.dependencies(0);
        assert_eq!(deps, vec![1]);

        // pkg-b depends on pkg-c (edge 1 -> 2)
        let deps = manager.dependencies(1);
        assert_eq!(deps, vec![2]);

        // pkg-c has no dependencies
        let deps = manager.dependencies(2);
        assert!(deps.is_empty());
    }

    #[test]
    fn test_regenerate_workspace() {
        let temp_dir = create_temp_workspace();

        let mut manager = WorkspaceManager::with_root(temp_dir.path().to_path_buf());
        manager.regenerate().unwrap();

        assert_eq!(manager.package_count(), 2);
        assert!(manager.get_package("pkg-a").is_some());
        assert!(manager.get_package("pkg-b").is_some());
    }

    #[test]
    fn test_detect_package_changes_added() {
        let temp_dir = create_temp_workspace();

        let mut manager = WorkspaceManager::with_root(temp_dir.path().to_path_buf());
        manager.regenerate().unwrap();

        // Add a new package
        let pkg_c = temp_dir.path().join("packages/c");
        fs::create_dir_all(&pkg_c).unwrap();
        fs::write(pkg_c.join("package.json"), r#"{"name": "pkg-c", "version": "1.0.0"}"#).unwrap();

        let changes = manager.detect_package_changes().unwrap();
        assert_eq!(changes.added.len(), 1);
        assert!(changes.removed.is_empty());
        assert!(changes.modified.is_empty());
    }

    #[test]
    fn test_detect_package_changes_removed() {
        let temp_dir = create_temp_workspace();

        let mut manager = WorkspaceManager::with_root(temp_dir.path().to_path_buf());
        manager.regenerate().unwrap();

        // Remove a package
        fs::remove_dir_all(temp_dir.path().join("packages/b")).unwrap();

        let changes = manager.detect_package_changes().unwrap();
        assert!(changes.added.is_empty());
        assert_eq!(changes.removed.len(), 1);
        assert!(changes.modified.is_empty());
    }

    #[test]
    fn test_detect_package_changes_modified() {
        let temp_dir = create_temp_workspace();

        let mut manager = WorkspaceManager::with_root(temp_dir.path().to_path_buf());
        manager.regenerate().unwrap();

        // Modify a package
        fs::write(
            temp_dir.path().join("packages/a/package.json"),
            r#"{"name": "pkg-a", "version": "2.0.0"}"#,
        )
        .unwrap();

        let changes = manager.detect_package_changes().unwrap();
        assert!(changes.added.is_empty());
        assert!(changes.removed.is_empty());
        assert_eq!(changes.modified.len(), 1);
    }

    #[test]
    fn test_apply_package_changes() {
        let temp_dir = create_temp_workspace();

        let mut manager = WorkspaceManager::with_root(temp_dir.path().to_path_buf());
        manager.regenerate().unwrap();

        // Add a new package
        let pkg_c = temp_dir.path().join("packages/c");
        fs::create_dir_all(&pkg_c).unwrap();
        fs::write(
            pkg_c.join("package.json"),
            r#"{"name": "pkg-c", "version": "1.0.0", "dependencies": {"pkg-b": "^1.0.0"}}"#,
        )
        .unwrap();

        let changes = manager.detect_package_changes().unwrap();
        manager.apply_package_changes(&changes).unwrap();

        assert_eq!(manager.package_count(), 3);
        assert!(manager.get_package("pkg-c").is_some());
    }

    #[test]
    fn test_update_package_triggers_regeneration() {
        let temp_dir = create_temp_workspace();

        let mut manager = WorkspaceManager::with_root(temp_dir.path().to_path_buf());
        manager.regenerate().unwrap();

        let pkg_a_path = temp_dir.path().join("packages/a/package.json");

        // Modify the package
        fs::write(&pkg_a_path, r#"{"name": "pkg-a", "version": "2.0.0"}"#).unwrap();

        manager.update_package(&pkg_a_path).unwrap();

        let pkg = manager.get_package("pkg-a").unwrap();
        assert_eq!(pkg.version, (2, 0, 0));
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.0.0"), (1, 0, 0));
        assert_eq!(parse_version("2.3.4"), (2, 3, 4));
        assert_eq!(parse_version("1.0.0-beta.1"), (1, 0, 0));
        assert_eq!(parse_version("0.0.1"), (0, 0, 1));
        assert_eq!(parse_version("invalid"), (0, 0, 0));
    }

    #[test]
    fn test_package_changes_is_empty() {
        let changes = PackageChanges::default();
        assert!(changes.is_empty());
        assert_eq!(changes.len(), 0);

        let changes = PackageChanges {
            added: vec![PathBuf::from("test")],
            removed: vec![],
            modified: vec![],
        };
        assert!(!changes.is_empty());
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn test_regeneration_idempotence() {
        let temp_dir = create_temp_workspace();

        let mut manager = WorkspaceManager::with_root(temp_dir.path().to_path_buf());

        // First regeneration
        manager.regenerate().unwrap();
        let first_data = manager.data.clone();

        // Second regeneration without changes
        manager.regenerate().unwrap();
        let second_data = manager.data.clone();

        // Results should be identical
        assert_eq!(first_data, second_data);
    }
}
