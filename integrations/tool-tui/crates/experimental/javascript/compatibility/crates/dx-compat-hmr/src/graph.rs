//! Dependency graph for HMR.
//!
//! Tracks module dependencies to determine which modules need to be
//! invalidated when a file changes.

use parking_lot::RwLock;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Dfs;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Module dependency graph.
pub struct DependencyGraph {
    graph: RwLock<DiGraph<PathBuf, ()>>,
    node_map: RwLock<HashMap<PathBuf, NodeIndex>>,
}

impl DependencyGraph {
    /// Create a new dependency graph.
    pub fn new() -> Self {
        Self {
            graph: RwLock::new(DiGraph::new()),
            node_map: RwLock::new(HashMap::new()),
        }
    }

    /// Add a module to the graph.
    pub fn add_module(&self, path: impl AsRef<Path>) -> NodeIndex {
        let path = path.as_ref().to_path_buf();

        let mut node_map = self.node_map.write();
        if let Some(&idx) = node_map.get(&path) {
            return idx;
        }

        let mut graph = self.graph.write();
        let idx = graph.add_node(path.clone());
        node_map.insert(path, idx);
        idx
    }

    /// Add a dependency edge (from depends on to).
    pub fn add_dependency(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) {
        let from_idx = self.add_module(from);
        let to_idx = self.add_module(to);

        let mut graph = self.graph.write();
        // Check if edge already exists
        if !graph.contains_edge(from_idx, to_idx) {
            graph.add_edge(from_idx, to_idx, ());
        }
    }

    /// Remove a module and its edges.
    pub fn remove_module(&self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();

        let mut node_map = self.node_map.write();
        if let Some(idx) = node_map.remove(&path) {
            let mut graph = self.graph.write();
            graph.remove_node(idx);
        }
    }

    /// Get all modules that depend on the given module (directly or indirectly).
    pub fn get_dependents(&self, path: impl AsRef<Path>) -> Vec<PathBuf> {
        let path = path.as_ref().to_path_buf();

        let node_map = self.node_map.read();
        let Some(&start_idx) = node_map.get(&path) else {
            return Vec::new();
        };

        let graph = self.graph.read();

        // Build reverse graph for finding dependents
        let mut reverse_graph = DiGraph::new();
        let mut reverse_node_map: HashMap<NodeIndex, NodeIndex> = HashMap::new();

        for idx in graph.node_indices() {
            let new_idx = reverse_graph.add_node(graph[idx].clone());
            reverse_node_map.insert(idx, new_idx);
        }

        for edge in graph.edge_indices() {
            if let Some((source, target)) = graph.edge_endpoints(edge) {
                let rev_source = reverse_node_map[&target];
                let rev_target = reverse_node_map[&source];
                reverse_graph.add_edge(rev_source, rev_target, ());
            }
        }

        // DFS from the changed module in reverse graph
        let start_in_reverse = reverse_node_map[&start_idx];
        let mut dfs = Dfs::new(&reverse_graph, start_in_reverse);
        let mut dependents = Vec::new();

        while let Some(idx) = dfs.next(&reverse_graph) {
            if idx != start_in_reverse {
                dependents.push(reverse_graph[idx].clone());
            }
        }

        dependents
    }

    /// Get direct dependencies of a module.
    pub fn get_dependencies(&self, path: impl AsRef<Path>) -> Vec<PathBuf> {
        let path = path.as_ref().to_path_buf();

        let node_map = self.node_map.read();
        let Some(&idx) = node_map.get(&path) else {
            return Vec::new();
        };

        let graph = self.graph.read();
        graph.neighbors(idx).map(|neighbor_idx| graph[neighbor_idx].clone()).collect()
    }

    /// Check if a module exists in the graph.
    pub fn has_module(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref().to_path_buf();
        self.node_map.read().contains_key(&path)
    }

    /// Get all modules in the graph.
    pub fn all_modules(&self) -> Vec<PathBuf> {
        self.node_map.read().keys().cloned().collect()
    }

    /// Clear all modules and dependencies.
    pub fn clear(&self) {
        self.graph.write().clear();
        self.node_map.write().clear();
    }

    /// Get the number of modules.
    pub fn module_count(&self) -> usize {
        self.node_map.read().len()
    }

    /// Get the number of dependency edges.
    pub fn edge_count(&self) -> usize {
        self.graph.read().edge_count()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared dependency graph handle.
pub type SharedDependencyGraph = Arc<DependencyGraph>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_module() {
        let graph = DependencyGraph::new();
        graph.add_module("a.js");
        graph.add_module("b.js");

        assert!(graph.has_module("a.js"));
        assert!(graph.has_module("b.js"));
        assert!(!graph.has_module("c.js"));
    }

    #[test]
    fn test_add_dependency() {
        let graph = DependencyGraph::new();
        graph.add_dependency("a.js", "b.js");
        graph.add_dependency("a.js", "c.js");

        let deps = graph.get_dependencies("a.js");
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_get_dependents() {
        let graph = DependencyGraph::new();
        // a depends on b, b depends on c
        graph.add_dependency("a.js", "b.js");
        graph.add_dependency("b.js", "c.js");

        // When c changes, both a and b should be invalidated
        let dependents = graph.get_dependents("c.js");
        assert!(dependents.iter().any(|p| p.to_str() == Some("b.js")));
        assert!(dependents.iter().any(|p| p.to_str() == Some("a.js")));
    }

    #[test]
    fn test_remove_module() {
        let graph = DependencyGraph::new();
        graph.add_module("a.js");
        graph.add_module("b.js");

        assert!(graph.has_module("a.js"));
        graph.remove_module("a.js");
        assert!(!graph.has_module("a.js"));
    }
}
