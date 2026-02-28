//! Binary Dependency Graph (BDG)
//!
//! Flat binary array representation of dependency trees.

use crate::error::{Result, SecurityError};
use crate::index::VulnerabilityIndex;
use std::collections::HashMap;
use std::path::Path;

/// Dependency node flags
pub mod flags {
    /// Node is vulnerable
    pub const VULNERABLE: u8 = 0b0000_0001;
    /// Node is a dev dependency
    pub const DEV_DEP: u8 = 0b0000_0010;
    /// Node is optional
    pub const OPTIONAL: u8 = 0b0000_0100;
}

/// Node in the binary dependency graph
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepNode {
    /// Unique node identifier
    pub id: u32,
    /// Version encoded as u32
    pub version: u32,
    /// Parent node ID (0 for root)
    pub parent: u32,
    /// Flags (bit 0: vulnerable, bit 1: dev-dep, etc.)
    pub flags: u8,
}

impl DepNode {
    /// Check if node is marked vulnerable
    pub fn is_vulnerable(&self) -> bool {
        self.flags & flags::VULNERABLE != 0
    }

    /// Mark node as vulnerable
    pub fn set_vulnerable(&mut self) {
        self.flags |= flags::VULNERABLE;
    }
}

/// Binary Dependency Graph
pub struct BinaryDependencyGraph {
    nodes: Vec<DepNode>,
    /// Package names indexed by node ID
    names: Vec<String>,
}

impl BinaryDependencyGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            names: Vec::new(),
        }
    }

    /// Parse lockfile into binary graph
    pub fn from_lockfile(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;

        if path.ends_with("Cargo.lock") {
            self.parse_cargo_lock(&content)
        } else if path.ends_with("package-lock.json") {
            self.parse_package_lock(&content)
        } else {
            Err(SecurityError::LockfileParseError("Unknown lockfile format".to_string()))
        }
    }

    /// Parse Cargo.lock format (TOML)
    fn parse_cargo_lock(&mut self, content: &str) -> Result<()> {
        self.nodes.clear();
        self.names.clear();

        // Simple TOML parser for Cargo.lock format
        // Format:
        // [[package]]
        // name = "package-name"
        // version = "1.2.3"
        // dependencies = ["dep1", "dep2 1.0.0"]

        let mut current_name: Option<String> = None;
        let mut current_version: Option<String> = None;
        let mut current_deps: Vec<String> = Vec::new();
        let mut packages: Vec<(String, String, Vec<String>)> = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            if line == "[[package]]" {
                // Save previous package if exists
                if let (Some(name), Some(version)) = (current_name.take(), current_version.take()) {
                    packages.push((name, version, std::mem::take(&mut current_deps)));
                }
                current_deps.clear();
                continue;
            }

            if let Some(rest) = line.strip_prefix("name = ") {
                current_name = Some(parse_toml_string(rest));
            } else if let Some(rest) = line.strip_prefix("version = ") {
                current_version = Some(parse_toml_string(rest));
            } else if let Some(rest) = line.strip_prefix("dependencies = ") {
                // Parse array of dependencies
                current_deps = parse_toml_array(rest);
            } else if line.starts_with('"') && !current_deps.is_empty() {
                // Continuation of dependencies array
                let dep = parse_toml_string(line.trim_end_matches(','));
                if !dep.is_empty() {
                    current_deps.push(dep);
                }
            }
        }

        // Save last package
        if let (Some(name), Some(version)) = (current_name, current_version) {
            packages.push((name, version, current_deps));
        }

        // Build name -> id mapping
        let mut name_to_id: HashMap<String, u32> = HashMap::new();

        // First pass: create all nodes
        for (name, version, _) in &packages {
            let version_num = parse_version(version);
            let id = self.add_node(name.clone(), version_num, 0, 0);
            name_to_id.insert(name.clone(), id);
        }

        // Second pass: set parent relationships
        for (name, _, deps) in &packages {
            if let Some(&parent_id) = name_to_id.get(name) {
                for dep in deps {
                    // Extract package name from dependency (may include version)
                    let dep_name = dep.split_whitespace().next().unwrap_or(dep);
                    if let Some(&child_id) = name_to_id.get(dep_name) {
                        // Set parent for the dependency
                        if let Some(node) = self.nodes.get_mut(child_id as usize) {
                            // Only set parent if not already set (first parent wins)
                            if node.parent == 0 && child_id != parent_id {
                                node.parent = parent_id;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse package-lock.json format (JSON)
    fn parse_package_lock(&mut self, content: &str) -> Result<()> {
        self.nodes.clear();
        self.names.clear();

        // Parse JSON using serde_json
        let json: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| SecurityError::LockfileParseError(e.to_string()))?;

        // Handle both v2/v3 (packages) and v1 (dependencies) formats
        if let Some(packages) = json.get("packages").and_then(|p| p.as_object()) {
            self.parse_npm_v2_packages(packages)?;
        } else if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
            self.parse_npm_v1_dependencies(deps, 0)?;
        }

        Ok(())
    }

    /// Parse npm v2/v3 packages format
    fn parse_npm_v2_packages(
        &mut self,
        packages: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()> {
        let mut name_to_id: HashMap<String, u32> = HashMap::new();

        // First pass: create all nodes
        for (path, pkg) in packages {
            // Skip root package (empty path)
            if path.is_empty() {
                continue;
            }

            // Extract package name from path (e.g., "node_modules/lodash" -> "lodash")
            let name = path.rsplit("node_modules/").next().unwrap_or(path).to_string();

            let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0");

            let version_num = parse_version(version);
            let dev = pkg.get("dev").and_then(|d| d.as_bool()).unwrap_or(false);
            let optional = pkg.get("optional").and_then(|o| o.as_bool()).unwrap_or(false);

            let mut flags = 0u8;
            if dev {
                flags |= flags::DEV_DEP;
            }
            if optional {
                flags |= flags::OPTIONAL;
            }

            let id = self.add_node(name.clone(), version_num, 0, flags);
            name_to_id.insert(name, id);
        }

        // Second pass: set parent relationships from dependencies
        for (path, pkg) in packages {
            if path.is_empty() {
                continue;
            }

            let name = path.rsplit("node_modules/").next().unwrap_or(path).to_string();

            if let Some(&parent_id) = name_to_id.get(&name) {
                if let Some(deps) = pkg.get("dependencies").and_then(|d| d.as_object()) {
                    for dep_name in deps.keys() {
                        if let Some(&child_id) = name_to_id.get(dep_name) {
                            if let Some(node) = self.nodes.get_mut(child_id as usize) {
                                if node.parent == 0 && child_id != parent_id {
                                    node.parent = parent_id;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse npm v1 dependencies format (recursive)
    fn parse_npm_v1_dependencies(
        &mut self,
        deps: &serde_json::Map<String, serde_json::Value>,
        parent_id: u32,
    ) -> Result<()> {
        for (name, pkg) in deps {
            let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0");

            let version_num = parse_version(version);
            let dev = pkg.get("dev").and_then(|d| d.as_bool()).unwrap_or(false);
            let optional = pkg.get("optional").and_then(|o| o.as_bool()).unwrap_or(false);

            let mut flags = 0u8;
            if dev {
                flags |= flags::DEV_DEP;
            }
            if optional {
                flags |= flags::OPTIONAL;
            }

            let id = self.add_node(name.clone(), version_num, parent_id, flags);

            // Recursively parse nested dependencies
            if let Some(nested_deps) = pkg.get("dependencies").and_then(|d| d.as_object()) {
                self.parse_npm_v1_dependencies(nested_deps, id)?;
            }
        }

        Ok(())
    }

    /// Propagate vulnerability flags using bitwise OR
    pub fn propagate_vulnerabilities(&mut self) {
        // Build parent-child relationships
        let node_count = self.nodes.len();
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); node_count];

        for (idx, node) in self.nodes.iter().enumerate() {
            // Add to parent's children list (parent 0 is valid for root's children)
            if (node.parent as usize) < node_count && idx != node.parent as usize {
                children[node.parent as usize].push(idx);
            }
        }

        // Propagate from vulnerable nodes to children using BFS
        let mut queue: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_vulnerable())
            .map(|(i, _)| i)
            .collect();

        while let Some(idx) = queue.pop() {
            for &child_idx in &children[idx] {
                if !self.nodes[child_idx].is_vulnerable() {
                    self.nodes[child_idx].set_vulnerable();
                    queue.push(child_idx);
                }
            }
        }
    }

    /// Get all vulnerable nodes
    pub fn vulnerable_nodes(&self) -> Vec<&DepNode> {
        self.nodes.iter().filter(|n| n.is_vulnerable()).collect()
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, name: String, version: u32, parent: u32, flags: u8) -> u32 {
        let id = self.nodes.len() as u32;
        self.nodes.push(DepNode {
            id,
            version,
            parent,
            flags,
        });
        self.names.push(name);
        id
    }

    /// Get node by ID
    pub fn get_node(&self, id: u32) -> Option<&DepNode> {
        self.nodes.get(id as usize)
    }

    /// Get all nodes
    pub fn nodes(&self) -> &[DepNode] {
        &self.nodes
    }

    /// Get mutable reference to nodes
    pub fn nodes_mut(&mut self) -> &mut [DepNode] {
        &mut self.nodes
    }

    /// Get package name by node ID
    pub fn get_name(&self, id: u32) -> Option<&str> {
        self.names.get(id as usize).map(|s| s.as_str())
    }

    /// Get all package names
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Mark vulnerabilities from a vulnerability index
    pub fn mark_vulnerabilities_from_index<I: VulnerabilityIndex>(&mut self, index: &I) {
        let seed = index.hash_seed();
        for (idx, name) in self.names.iter().enumerate() {
            // Compute package hash for lookup using the index's hash function
            let package_hash = crate::index::hash_package(name, seed);
            if let Some(vuln) = index.lookup(package_hash) {
                // Check if version is in affected range
                let node_version = self.nodes[idx].version;
                if node_version >= vuln.version_range.0 && node_version <= vuln.version_range.1 {
                    self.nodes[idx].set_vulnerable();
                }
            }
        }
    }
}

impl Default for BinaryDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a TOML string value (removes quotes)
fn parse_toml_string(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

/// Parse a TOML array of strings
fn parse_toml_array(s: &str) -> Vec<String> {
    let s = s.trim();
    if !s.starts_with('[') {
        return Vec::new();
    }

    // Handle single-line array
    if s.ends_with(']') {
        return s[1..s.len() - 1]
            .split(',')
            .map(|item| parse_toml_string(item.trim()))
            .filter(|s| !s.is_empty())
            .collect();
    }

    // Multi-line array - just return empty, will be parsed line by line
    Vec::new()
}

/// Parse a semver version string to u32
/// Format: major * 1_000_000 + minor * 1_000 + patch
fn parse_version(version: &str) -> u32 {
    let parts: Vec<&str> = version.split('.').collect();
    let major = parts.first().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let minor = parts
        .get(1)
        .and_then(|s| {
            // Handle versions like "1.2.3-beta" by taking only numeric prefix
            s.split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|n| n.parse::<u32>().ok())
        })
        .unwrap_or(0);
    let patch = parts
        .get(2)
        .and_then(|s| {
            s.split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|n| n.parse::<u32>().ok())
        })
        .unwrap_or(0);

    major * 1_000_000 + minor * 1_000 + patch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.2.3"), 1_002_003);
        assert_eq!(parse_version("0.1.0"), 1_000);
        assert_eq!(parse_version("10.20.30"), 10_020_030);
        assert_eq!(parse_version("1.0.0-beta"), 1_000_000);
    }

    #[test]
    fn test_dep_node_flags() {
        let mut node = DepNode {
            id: 0,
            version: 1_000_000,
            parent: 0,
            flags: 0,
        };

        assert!(!node.is_vulnerable());
        node.set_vulnerable();
        assert!(node.is_vulnerable());
    }

    #[test]
    fn test_parse_cargo_lock() {
        let cargo_lock = r#"
[[package]]
name = "root"
version = "1.0.0"
dependencies = ["dep1", "dep2 0.1.0"]

[[package]]
name = "dep1"
version = "0.5.0"

[[package]]
name = "dep2"
version = "0.1.0"
dependencies = ["dep1"]
"#;

        let mut graph = BinaryDependencyGraph::new();
        graph.parse_cargo_lock(cargo_lock).unwrap();

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.names[0], "root");
        assert_eq!(graph.names[1], "dep1");
        assert_eq!(graph.names[2], "dep2");
    }

    #[test]
    fn test_parse_package_lock_v2() {
        let package_lock = r#"{
            "name": "test-project",
            "lockfileVersion": 2,
            "packages": {
                "": {
                    "name": "test-project",
                    "version": "1.0.0"
                },
                "node_modules/lodash": {
                    "version": "4.17.21"
                },
                "node_modules/express": {
                    "version": "4.18.2",
                    "dependencies": {
                        "lodash": "^4.17.0"
                    }
                }
            }
        }"#;

        let mut graph = BinaryDependencyGraph::new();
        graph.parse_package_lock(package_lock).unwrap();

        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_vulnerability_propagation() {
        let mut graph = BinaryDependencyGraph::new();

        // Create a simple tree: root -> child1 -> grandchild
        let root = graph.add_node("root".to_string(), 1_000_000, 0, 0);
        let child1 = graph.add_node("child1".to_string(), 1_000_000, root, 0);
        let _grandchild = graph.add_node("grandchild".to_string(), 1_000_000, child1, 0);

        // Mark root as vulnerable
        graph.nodes[root as usize].set_vulnerable();

        // Propagate
        graph.propagate_vulnerabilities();

        // All descendants should be vulnerable
        assert!(graph.nodes[0].is_vulnerable());
        assert!(graph.nodes[1].is_vulnerable());
        assert!(graph.nodes[2].is_vulnerable());
    }

    #[test]
    fn test_get_node() {
        let mut graph = BinaryDependencyGraph::new();
        let id = graph.add_node("test".to_string(), 1_000_000, 0, 0);

        let node = graph.get_node(id).unwrap();
        assert_eq!(node.id, id);
        assert_eq!(node.version, 1_000_000);
    }

    #[test]
    fn test_vulnerable_nodes() {
        let mut graph = BinaryDependencyGraph::new();

        graph.add_node("safe1".to_string(), 1_000_000, 0, 0);
        let vuln_id = graph.add_node("vulnerable".to_string(), 1_000_000, 0, flags::VULNERABLE);
        graph.add_node("safe2".to_string(), 1_000_000, 0, 0);

        let vulnerable = graph.vulnerable_nodes();
        assert_eq!(vulnerable.len(), 1);
        assert_eq!(vulnerable[0].id, vuln_id);
    }

    #[test]
    fn test_get_name() {
        let mut graph = BinaryDependencyGraph::new();
        let id = graph.add_node("my-package".to_string(), 1_000_000, 0, 0);

        assert_eq!(graph.get_name(id), Some("my-package"));
        assert_eq!(graph.get_name(999), None);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate a valid package name
    fn arb_package_name() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z0-9_-]{0,20}")
            .unwrap()
            .prop_filter("non-empty", |s| !s.is_empty())
    }

    /// Generate a valid semver version string
    fn arb_version() -> impl Strategy<Value = String> {
        (0u32..100, 0u32..100, 0u32..100)
            .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
    }

    /// Generate a Cargo.lock package entry
    fn arb_cargo_package(name: String, version: String, deps: Vec<String>) -> String {
        let mut entry = format!(
            r#"[[package]]
name = "{}"
version = "{}""#,
            name, version
        );

        if !deps.is_empty() {
            entry.push_str("\ndependencies = [");
            for (i, dep) in deps.iter().enumerate() {
                if i > 0 {
                    entry.push_str(", ");
                }
                entry.push_str(&format!("\"{}\"", dep));
            }
            entry.push(']');
        }

        entry
    }

    /// Generate a simple dependency tree for Cargo.lock
    fn arb_cargo_lock_content() -> impl Strategy<Value = (String, Vec<(String, String)>)> {
        // Generate 1-5 packages
        prop::collection::vec((arb_package_name(), arb_version()), 1..6).prop_map(|packages| {
            // Ensure unique names
            let mut seen = std::collections::HashSet::new();
            let unique_packages: Vec<_> =
                packages.into_iter().filter(|(name, _)| seen.insert(name.clone())).collect();

            let mut content = String::new();

            for (i, (name, version)) in unique_packages.iter().enumerate() {
                // First package has no deps, others may depend on earlier packages
                let deps: Vec<String> = if i == 0 {
                    vec![]
                } else {
                    // Randomly pick some earlier packages as dependencies
                    unique_packages[..i].iter().take(2).map(|(n, _)| n.clone()).collect()
                };

                content.push_str(&arb_cargo_package(name.clone(), version.clone(), deps));
                content.push_str("\n\n");
            }

            (content, unique_packages)
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-security, Property 6: Dependency Graph Round-Trip**
        /// **Validates: Requirements 7.1**
        ///
        /// For any valid lockfile (Cargo.lock), parsing into BinaryDependencyGraph
        /// and extracting all dependencies SHALL preserve all package names and versions.
        #[test]
        fn prop_cargo_lock_round_trip(
            (content, expected_packages) in arb_cargo_lock_content()
        ) {
            let mut graph = BinaryDependencyGraph::new();
            let result = graph.parse_cargo_lock(&content);

            prop_assert!(result.is_ok(), "Parsing should succeed");

            // Verify all packages are present
            prop_assert_eq!(
                graph.node_count(),
                expected_packages.len(),
                "Should have same number of packages"
            );

            // Verify package names are preserved
            for (name, version) in &expected_packages {
                let found = graph.names().iter().any(|n| n == name);
                prop_assert!(
                    found,
                    "Package '{}' should be in graph",
                    name
                );

                // Find the node and verify version
                if let Some(idx) = graph.names().iter().position(|n| n == name) {
                    let node = &graph.nodes()[idx];
                    let expected_version = parse_version(version);
                    prop_assert_eq!(
                        node.version,
                        expected_version,
                        "Version for '{}' should match",
                        name
                    );
                }
            }
        }

        /// **Feature: dx-security, Property 7: Vulnerability Propagation Correctness**
        /// **Validates: Requirements 7.3**
        ///
        /// For any BinaryDependencyGraph where a parent node is marked vulnerable,
        /// after propagate_vulnerabilities(), all transitive children SHALL have
        /// their vulnerable flag set.
        #[test]
        fn prop_vulnerability_propagation_correctness(
            num_nodes in 2usize..20,
            vulnerable_root_idx in 0usize..10,
        ) {
            let mut graph = BinaryDependencyGraph::new();

            // Create a chain of dependencies: 0 -> 1 -> 2 -> ... -> n-1
            for i in 0..num_nodes {
                let parent = if i == 0 { 0 } else { (i - 1) as u32 };
                graph.add_node(format!("pkg{}", i), 1_000_000, parent, 0);
            }

            // Mark a node as vulnerable (ensure it's within bounds)
            let vuln_idx = vulnerable_root_idx % num_nodes;
            graph.nodes_mut()[vuln_idx].set_vulnerable();

            // Propagate
            graph.propagate_vulnerabilities();

            // All descendants of the vulnerable node should be vulnerable
            for i in vuln_idx..num_nodes {
                prop_assert!(
                    graph.nodes()[i].is_vulnerable(),
                    "Node {} should be vulnerable (descendant of {})",
                    i,
                    vuln_idx
                );
            }

            // Nodes before the vulnerable node should NOT be vulnerable
            // (unless they were already vulnerable)
            for i in 0..vuln_idx {
                if i != vuln_idx {
                    prop_assert!(
                        !graph.nodes()[i].is_vulnerable(),
                        "Node {} should NOT be vulnerable (ancestor of {})",
                        i,
                        vuln_idx
                    );
                }
            }
        }

        /// Version parsing should be deterministic
        #[test]
        fn prop_version_parsing_deterministic(version in arb_version()) {
            let parsed1 = parse_version(&version);
            let parsed2 = parse_version(&version);

            prop_assert_eq!(
                parsed1, parsed2,
                "Same version string should always produce same parsed value"
            );
        }

        /// Version parsing should preserve ordering
        #[test]
        fn prop_version_ordering(
            major1 in 0u32..100,
            minor1 in 0u32..100,
            patch1 in 0u32..100,
            major2 in 0u32..100,
            minor2 in 0u32..100,
            patch2 in 0u32..100,
        ) {
            let v1 = format!("{}.{}.{}", major1, minor1, patch1);
            let v2 = format!("{}.{}.{}", major2, minor2, patch2);

            let parsed1 = parse_version(&v1);
            let parsed2 = parse_version(&v2);

            // Compare tuples for expected ordering
            let tuple1 = (major1, minor1, patch1);
            let tuple2 = (major2, minor2, patch2);

            if tuple1 < tuple2 {
                prop_assert!(parsed1 < parsed2, "v1 < v2 should hold");
            } else if tuple1 > tuple2 {
                prop_assert!(parsed1 > parsed2, "v1 > v2 should hold");
            } else {
                prop_assert_eq!(parsed1, parsed2, "v1 == v2 should hold");
            }
        }
    }
}
