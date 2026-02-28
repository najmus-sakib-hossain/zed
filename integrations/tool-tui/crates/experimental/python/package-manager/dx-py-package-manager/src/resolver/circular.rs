//! Circular Dependency Detection and Handling
//!
//! Implements detection and graceful handling of circular dependencies in the
//! dependency graph. Circular dependencies can occur in Python packages and
//! need to be handled carefully to avoid infinite loops during resolution.
//!
//! # Strategies
//!
//! 1. **Detection**: Identify cycles during dependency graph traversal
//! 2. **Breaking**: Allow cycles to be broken at specific points
//! 3. **Reporting**: Provide clear error messages about circular dependencies
//!
//! # Examples
//! ```ignore
//! use dx_py_package_manager::resolver::circular::{CircularDependencyDetector, CycleHandling};
//!
//! let mut detector = CircularDependencyDetector::new(CycleHandling::Error);
//! detector.enter("package_a")?;
//! detector.enter("package_b")?;
//! // If package_b depends on package_a, this would detect the cycle
//! ```

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{Error, Result};

/// Strategy for handling circular dependencies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CycleHandling {
    /// Treat circular dependencies as errors (strictest)
    #[default]
    Error,
    /// Warn about circular dependencies but continue
    Warn,
    /// Silently break cycles (most permissive)
    Break,
}

impl CycleHandling {
    /// Parse from string (for CLI flags)
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "error" | "strict" | "fail" => Some(CycleHandling::Error),
            "warn" | "warning" => Some(CycleHandling::Warn),
            "break" | "ignore" | "allow" => Some(CycleHandling::Break),
            _ => None,
        }
    }
}

/// A detected circular dependency cycle
#[derive(Debug, Clone)]
pub struct DependencyCycle {
    /// Packages involved in the cycle, in order
    pub packages: Vec<String>,
    /// The edge that completes the cycle (from -> to)
    pub closing_edge: (String, String),
}

impl DependencyCycle {
    /// Create a new cycle from a path
    pub fn new(path: Vec<String>, closing_edge: (String, String)) -> Self {
        Self {
            packages: path,
            closing_edge,
        }
    }

    /// Get a human-readable description of the cycle
    pub fn description(&self) -> String {
        if self.packages.is_empty() {
            return format!("{} -> {}", self.closing_edge.0, self.closing_edge.1);
        }

        let mut desc = self.packages.join(" -> ");
        desc.push_str(&format!(" -> {}", self.closing_edge.1));
        desc
    }

    /// Get the length of the cycle
    pub fn len(&self) -> usize {
        self.packages.len()
    }

    /// Check if the cycle is empty
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

impl std::fmt::Display for DependencyCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Circular dependency: {}", self.description())
    }
}

/// Circular dependency detector using DFS with path tracking
#[derive(Debug)]
pub struct CircularDependencyDetector {
    /// How to handle detected cycles
    handling: CycleHandling,
    /// Current traversal path (stack)
    path: Vec<String>,
    /// Set of packages in current path (for O(1) lookup)
    path_set: HashSet<String>,
    /// Fully visited packages (no need to revisit)
    visited: HashSet<String>,
    /// Detected cycles
    cycles: Vec<DependencyCycle>,
    /// Edges that were broken to resolve cycles
    broken_edges: Vec<(String, String)>,
}

impl Default for CircularDependencyDetector {
    fn default() -> Self {
        Self::new(CycleHandling::default())
    }
}

impl CircularDependencyDetector {
    /// Create a new detector with the specified handling strategy
    pub fn new(handling: CycleHandling) -> Self {
        Self {
            handling,
            path: Vec::new(),
            path_set: HashSet::new(),
            visited: HashSet::new(),
            cycles: Vec::new(),
            broken_edges: Vec::new(),
        }
    }

    /// Enter a package during traversal
    ///
    /// Returns an error if a cycle is detected and handling is set to Error.
    pub fn enter(&mut self, package: &str) -> Result<bool> {
        let package = package.to_lowercase();

        // Check if we're creating a cycle
        if self.path_set.contains(&package) {
            return self.handle_cycle(&package);
        }

        // Check if already fully visited
        if self.visited.contains(&package) {
            return Ok(false); // Skip, already processed
        }

        // Add to current path
        self.path.push(package.clone());
        self.path_set.insert(package);

        Ok(true) // Continue traversal
    }

    /// Leave a package after processing its dependencies
    pub fn leave(&mut self, package: &str) {
        let package = package.to_lowercase();

        // Remove from path
        if let Some(pos) = self.path.iter().position(|p| p == &package) {
            self.path.truncate(pos);
            self.path_set.remove(&package);
        }

        // Mark as fully visited
        self.visited.insert(package);
    }

    /// Handle a detected cycle
    fn handle_cycle(&mut self, package: &str) -> Result<bool> {
        // Find where the cycle starts
        let cycle_start = self.path.iter().position(|p| p == package).unwrap_or(0);
        let cycle_path: Vec<String> = self.path[cycle_start..].to_vec();

        let closing_edge = (self.path.last().cloned().unwrap_or_default(), package.to_string());

        let cycle = DependencyCycle::new(cycle_path, closing_edge.clone());

        match self.handling {
            CycleHandling::Error => Err(Error::CircularDependency(cycle.description())),
            CycleHandling::Warn => {
                // Log warning (in real implementation, use proper logging)
                eprintln!("Warning: {}", cycle);
                self.cycles.push(cycle);
                Ok(false) // Skip this edge
            }
            CycleHandling::Break => {
                self.cycles.push(cycle);
                self.broken_edges.push(closing_edge);
                Ok(false) // Skip this edge
            }
        }
    }

    /// Check if a package is currently in the traversal path
    pub fn is_in_path(&self, package: &str) -> bool {
        self.path_set.contains(&package.to_lowercase())
    }

    /// Check if a package has been fully visited
    pub fn is_visited(&self, package: &str) -> bool {
        self.visited.contains(&package.to_lowercase())
    }

    /// Get all detected cycles
    pub fn cycles(&self) -> &[DependencyCycle] {
        &self.cycles
    }

    /// Get all broken edges
    pub fn broken_edges(&self) -> &[(String, String)] {
        &self.broken_edges
    }

    /// Check if any cycles were detected
    pub fn has_cycles(&self) -> bool {
        !self.cycles.is_empty()
    }

    /// Reset the detector for a new traversal
    pub fn reset(&mut self) {
        self.path.clear();
        self.path_set.clear();
        self.visited.clear();
        self.cycles.clear();
        self.broken_edges.clear();
    }

    /// Get the current traversal path
    pub fn current_path(&self) -> &[String] {
        &self.path
    }
}

/// Dependency graph for cycle detection
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Adjacency list: package -> dependencies
    edges: HashMap<String, HashSet<String>>,
    /// Reverse adjacency list: package -> dependents
    reverse_edges: HashMap<String, HashSet<String>>,
}

impl DependencyGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a dependency edge
    pub fn add_edge(&mut self, from: &str, to: &str) {
        let from = from.to_lowercase();
        let to = to.to_lowercase();

        self.edges.entry(from.clone()).or_default().insert(to.clone());
        self.reverse_edges.entry(to).or_default().insert(from);
    }

    /// Get dependencies of a package
    pub fn dependencies(&self, package: &str) -> Option<&HashSet<String>> {
        self.edges.get(&package.to_lowercase())
    }

    /// Get dependents of a package (reverse dependencies)
    pub fn dependents(&self, package: &str) -> Option<&HashSet<String>> {
        self.reverse_edges.get(&package.to_lowercase())
    }

    /// Check if an edge exists
    pub fn has_edge(&self, from: &str, to: &str) -> bool {
        self.edges
            .get(&from.to_lowercase())
            .map(|deps| deps.contains(&to.to_lowercase()))
            .unwrap_or(false)
    }

    /// Find all cycles in the graph using Tarjan's algorithm
    pub fn find_all_cycles(&self) -> Vec<DependencyCycle> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for package in self.edges.keys() {
            if !visited.contains(package) {
                self.find_cycles_dfs(package, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    /// DFS helper for cycle detection
    fn find_cycles_dfs(
        &self,
        package: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<DependencyCycle>,
    ) {
        visited.insert(package.to_string());
        rec_stack.insert(package.to_string());
        path.push(package.to_string());

        if let Some(deps) = self.edges.get(package) {
            for dep in deps {
                if !visited.contains(dep) {
                    self.find_cycles_dfs(dep, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(dep) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|p| p == dep).unwrap_or(0);
                    let cycle_path: Vec<String> = path[cycle_start..].to_vec();
                    let closing_edge = (package.to_string(), dep.clone());
                    cycles.push(DependencyCycle::new(cycle_path, closing_edge));
                }
            }
        }

        path.pop();
        rec_stack.remove(package);
    }

    /// Get strongly connected components (SCCs) using Kosaraju's algorithm
    ///
    /// Each SCC with more than one node represents a cycle.
    pub fn strongly_connected_components(&self) -> Vec<Vec<String>> {
        let mut visited = HashSet::new();
        let mut finish_order = Vec::new();

        // First DFS to get finish order
        for package in self.edges.keys() {
            if !visited.contains(package) {
                self.dfs_finish_order(package, &mut visited, &mut finish_order);
            }
        }

        // Second DFS on reverse graph in reverse finish order
        let mut visited = HashSet::new();
        let mut sccs = Vec::new();

        for package in finish_order.into_iter().rev() {
            if !visited.contains(&package) {
                let mut scc = Vec::new();
                self.dfs_reverse(&package, &mut visited, &mut scc);
                sccs.push(scc);
            }
        }

        sccs
    }

    /// DFS to compute finish order
    fn dfs_finish_order(
        &self,
        package: &str,
        visited: &mut HashSet<String>,
        finish_order: &mut Vec<String>,
    ) {
        visited.insert(package.to_string());

        if let Some(deps) = self.edges.get(package) {
            for dep in deps {
                if !visited.contains(dep) {
                    self.dfs_finish_order(dep, visited, finish_order);
                }
            }
        }

        finish_order.push(package.to_string());
    }

    /// DFS on reverse graph
    fn dfs_reverse(&self, package: &str, visited: &mut HashSet<String>, scc: &mut Vec<String>) {
        visited.insert(package.to_string());
        scc.push(package.to_string());

        if let Some(dependents) = self.reverse_edges.get(package) {
            for dep in dependents {
                if !visited.contains(dep) {
                    self.dfs_reverse(dep, visited, scc);
                }
            }
        }
    }

    /// Topological sort (returns None if graph has cycles)
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Initialize in-degrees
        for package in self.edges.keys() {
            in_degree.entry(package.clone()).or_insert(0);
        }
        for deps in self.edges.values() {
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        // Find all nodes with in-degree 0
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(pkg, _)| pkg.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(package) = queue.pop_front() {
            result.push(package.clone());

            if let Some(deps) = self.edges.get(&package) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }

        // If we processed all nodes, no cycle exists
        if result.len() == in_degree.len() {
            Some(result)
        } else {
            None // Cycle detected
        }
    }

    /// Get all packages in the graph
    pub fn packages(&self) -> impl Iterator<Item = &String> {
        self.edges.keys()
    }

    /// Get the number of packages
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_cycle() {
        let mut detector = CircularDependencyDetector::new(CycleHandling::Error);

        assert!(detector.enter("a").unwrap());
        assert!(detector.enter("b").unwrap());
        assert!(detector.enter("c").unwrap());
        detector.leave("c");
        detector.leave("b");
        detector.leave("a");

        assert!(!detector.has_cycles());
    }

    #[test]
    fn test_cycle_detection_error() {
        let mut detector = CircularDependencyDetector::new(CycleHandling::Error);

        assert!(detector.enter("a").unwrap());
        assert!(detector.enter("b").unwrap());

        // Try to enter 'a' again - should error
        let result = detector.enter("a");
        assert!(result.is_err());
    }

    #[test]
    fn test_cycle_detection_warn() {
        let mut detector = CircularDependencyDetector::new(CycleHandling::Warn);

        assert!(detector.enter("a").unwrap());
        assert!(detector.enter("b").unwrap());

        // Try to enter 'a' again - should warn but continue
        let result = detector.enter("a");
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false (skip)
        assert!(detector.has_cycles());
    }

    #[test]
    fn test_cycle_detection_break() {
        let mut detector = CircularDependencyDetector::new(CycleHandling::Break);

        assert!(detector.enter("a").unwrap());
        assert!(detector.enter("b").unwrap());

        // Try to enter 'a' again - should break silently
        let result = detector.enter("a");
        assert!(result.is_ok());
        assert!(!result.unwrap());
        assert!(detector.has_cycles());
        assert_eq!(detector.broken_edges().len(), 1);
    }

    #[test]
    fn test_dependency_graph_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a", "b");
        graph.add_edge("b", "c");
        graph.add_edge("c", "a"); // Creates cycle

        let cycles = graph.find_all_cycles();
        assert!(!cycles.is_empty());

        // Topological sort should fail
        assert!(graph.topological_sort().is_none());
    }

    #[test]
    fn test_dependency_graph_no_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a", "b");
        graph.add_edge("b", "c");
        graph.add_edge("a", "c");

        let cycles = graph.find_all_cycles();
        assert!(cycles.is_empty());

        // Topological sort should succeed
        let sorted = graph.topological_sort();
        assert!(sorted.is_some());
    }

    #[test]
    fn test_scc_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a", "b");
        graph.add_edge("b", "c");
        graph.add_edge("c", "a"); // SCC: {a, b, c}
        graph.add_edge("c", "d"); // d is separate

        let sccs = graph.strongly_connected_components();

        // Should have at least one SCC with 3 nodes
        let large_scc = sccs.iter().find(|scc| scc.len() == 3);
        assert!(large_scc.is_some());
    }

    #[test]
    fn test_cycle_description() {
        let cycle = DependencyCycle::new(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ("c".to_string(), "a".to_string()),
        );

        let desc = cycle.description();
        assert!(desc.contains("a"));
        assert!(desc.contains("b"));
        assert!(desc.contains("c"));
    }

    #[test]
    fn test_cycle_handling_from_str() {
        assert_eq!(CycleHandling::from_str("error"), Some(CycleHandling::Error));
        assert_eq!(CycleHandling::from_str("warn"), Some(CycleHandling::Warn));
        assert_eq!(CycleHandling::from_str("break"), Some(CycleHandling::Break));
        assert_eq!(CycleHandling::from_str("invalid"), None);
    }

    #[test]
    fn test_graph_reverse_edges() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a", "b");
        graph.add_edge("a", "c");

        let dependents = graph.dependents("b").unwrap();
        assert!(dependents.contains("a"));

        let dependents = graph.dependents("c").unwrap();
        assert!(dependents.contains("a"));
    }
}
