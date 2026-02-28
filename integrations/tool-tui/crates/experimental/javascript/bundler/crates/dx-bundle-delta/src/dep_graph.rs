//! Dependency graph for change propagation

use dx_bundle_core::ModuleId;
use std::collections::{HashMap, HashSet};

/// Module dependency graph
pub struct DependencyGraph {
    /// Module → modules that depend on it (reverse deps)
    dependents: HashMap<ModuleId, Vec<ModuleId>>,
    /// Module → modules it depends on (forward deps)
    dependencies: HashMap<ModuleId, Vec<ModuleId>>,
}

impl DependencyGraph {
    /// Create new dependency graph
    pub fn new() -> Self {
        Self {
            dependents: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Add dependency edge
    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId) {
        self.dependencies.entry(from).or_default().push(to);

        self.dependents.entry(to).or_default().push(from);
    }

    /// Get all modules that depend on the given module (transitively)
    pub fn get_dependents(&self, module: ModuleId) -> Vec<ModuleId> {
        let mut result = Vec::new();
        let mut queue = vec![module];
        let mut visited = HashSet::new();

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(deps) = self.dependents.get(&current) {
                for &dep in deps {
                    result.push(dep);
                    queue.push(dep);
                }
            }
        }

        result
    }

    /// Get direct dependencies of module
    pub fn get_dependencies(&self, module: ModuleId) -> Option<&Vec<ModuleId>> {
        self.dependencies.get(&module)
    }

    /// Check for circular dependencies
    pub fn has_cycle(&self, module: ModuleId) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        self.has_cycle_dfs(module, &mut visited, &mut stack)
    }

    fn has_cycle_dfs(
        &self,
        module: ModuleId,
        visited: &mut HashSet<ModuleId>,
        stack: &mut HashSet<ModuleId>,
    ) -> bool {
        if stack.contains(&module) {
            return true;
        }

        if visited.contains(&module) {
            return false;
        }

        visited.insert(module);
        stack.insert(module);

        if let Some(deps) = self.dependencies.get(&module) {
            for &dep in deps {
                if self.has_cycle_dfs(dep, visited, stack) {
                    return true;
                }
            }
        }

        stack.remove(&module);
        false
    }

    /// Topological sort (for bundle ordering)
    pub fn topological_sort(&self) -> Option<Vec<ModuleId>> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp = HashSet::new();

        // Collect all modules (both sources and targets)
        let mut all_modules: HashSet<ModuleId> = HashSet::new();
        for &module in self.dependencies.keys() {
            all_modules.insert(module);
        }
        for deps in self.dependencies.values() {
            for &dep in deps {
                all_modules.insert(dep);
            }
        }

        for module in all_modules {
            if !visited.contains(&module)
                && !self.topo_visit(module, &mut visited, &mut temp, &mut result)
            {
                return None; // Cycle detected
            }
        }

        // Result is already in dependency order (dependencies before dependents)
        Some(result)
    }

    fn topo_visit(
        &self,
        module: ModuleId,
        visited: &mut HashSet<ModuleId>,
        temp: &mut HashSet<ModuleId>,
        result: &mut Vec<ModuleId>,
    ) -> bool {
        if temp.contains(&module) {
            return false; // Cycle
        }

        if visited.contains(&module) {
            return true;
        }

        temp.insert(module);

        if let Some(deps) = self.dependencies.get(&module) {
            for &dep in deps {
                if !self.topo_visit(dep, visited, temp, result) {
                    return false;
                }
            }
        }

        temp.remove(&module);
        visited.insert(module);
        result.push(module);

        true
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();

        // Add dependencies: 1 -> 2 -> 3
        graph.add_dependency(1, 2);
        graph.add_dependency(2, 3);

        // Get dependents of 3
        let deps = graph.get_dependents(3);
        assert!(deps.contains(&2));
        assert!(deps.contains(&1));
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        // Create cycle: 1 -> 2 -> 3 -> 1
        graph.add_dependency(1, 2);
        graph.add_dependency(2, 3);
        graph.add_dependency(3, 1);

        assert!(graph.has_cycle(1));
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = DependencyGraph::new();

        graph.add_dependency(1, 2);
        graph.add_dependency(2, 3);

        let sorted = graph.topological_sort().unwrap();

        // 3 should come before 2, and 2 before 1
        let pos_1 = sorted.iter().position(|&x| x == 1).unwrap();
        let pos_2 = sorted.iter().position(|&x| x == 2).unwrap();
        let pos_3 = sorted.iter().position(|&x| x == 3).unwrap();

        assert!(pos_3 < pos_2);
        assert!(pos_2 < pos_1);
    }
}
