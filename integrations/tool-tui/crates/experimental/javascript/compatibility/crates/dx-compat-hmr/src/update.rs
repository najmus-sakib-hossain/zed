//! HMR update types and propagation logic.

use std::collections::{HashSet, VecDeque};

/// Type alias for importer lookup callback
type ImportersCallback<'a> = Box<dyn Fn(&str) -> Vec<String> + 'a>;

/// Type alias for dependency acceptance callback
type DependencyCallback<'a> = Box<dyn Fn(&str, &str) -> bool + 'a>;

/// HMR update message.
#[derive(Debug, Clone)]
pub struct HmrUpdate {
    /// File path
    pub path: String,
    /// Content hash
    pub hash: String,
    /// Update type
    pub update_type: UpdateType,
}

/// Type of HMR update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType {
    /// JavaScript update
    Js,
    /// CSS update
    Css,
    /// Full page reload required
    FullReload,
}

/// Result of update propagation.
#[derive(Debug, Clone)]
pub struct PropagationResult {
    /// Modules that will be updated (accepted the change)
    pub updated: Vec<String>,
    /// Modules that need disposal before update
    pub disposed: Vec<String>,
    /// Whether a full reload is required
    pub needs_reload: bool,
    /// The boundary module that accepted the update (if any)
    pub boundary: Option<String>,
}

impl PropagationResult {
    /// Create a result indicating full reload is needed.
    pub fn full_reload() -> Self {
        Self {
            updated: Vec::new(),
            disposed: Vec::new(),
            needs_reload: true,
            boundary: None,
        }
    }

    /// Create a result indicating successful hot update.
    pub fn hot_update(boundary: String, updated: Vec<String>, disposed: Vec<String>) -> Self {
        Self {
            updated,
            disposed,
            needs_reload: false,
            boundary: Some(boundary),
        }
    }
}

/// Update propagator that walks the module graph to find accepting boundaries.
pub struct UpdatePropagator<'a> {
    /// Function to get importers of a module
    get_importers: ImportersCallback<'a>,
    /// Function to check if a module self-accepts
    is_self_accepting: Box<dyn Fn(&str) -> bool + 'a>,
    /// Function to check if a module accepts a dependency
    accepts_dependency: DependencyCallback<'a>,
    /// Function to check if a module declines updates
    is_declined: Box<dyn Fn(&str) -> bool + 'a>,
}

impl<'a> UpdatePropagator<'a> {
    /// Create a new update propagator.
    pub fn new(
        get_importers: impl Fn(&str) -> Vec<String> + 'a,
        is_self_accepting: impl Fn(&str) -> bool + 'a,
        accepts_dependency: impl Fn(&str, &str) -> bool + 'a,
        is_declined: impl Fn(&str) -> bool + 'a,
    ) -> Self {
        Self {
            get_importers: Box::new(get_importers),
            is_self_accepting: Box::new(is_self_accepting),
            accepts_dependency: Box::new(accepts_dependency),
            is_declined: Box::new(is_declined),
        }
    }

    /// Propagate an update through the module graph.
    ///
    /// Starting from the changed module, walks up the import chain to find
    /// a module that accepts the update. Returns the propagation result.
    pub fn propagate(&self, changed_path: &str) -> PropagationResult {
        // Track visited modules to avoid cycles
        let mut visited: HashSet<String> = HashSet::new();
        // Track modules that need to be updated
        let mut to_update: Vec<String> = Vec::new();
        // Track modules that need disposal
        let mut to_dispose: Vec<String> = Vec::new();
        // Queue for BFS traversal
        let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();

        // Start with the changed module
        queue.push_back((changed_path.to_string(), vec![changed_path.to_string()]));

        while let Some((current, path)) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            // Check if this module declines updates
            if (self.is_declined)(&current) {
                return PropagationResult::full_reload();
            }

            // Check if this module self-accepts
            if (self.is_self_accepting)(&current) {
                // Found a boundary - this module accepts the update
                to_dispose.extend(path.iter().cloned());
                to_update.push(current.clone());
                return PropagationResult::hot_update(current, to_update, to_dispose);
            }

            // Get importers of this module
            let importers = (self.get_importers)(&current);

            if importers.is_empty() {
                // Reached the root without finding an accepting module
                return PropagationResult::full_reload();
            }

            for importer in importers {
                // Check if the importer accepts this dependency
                if (self.accepts_dependency)(&importer, &current) {
                    // Found a boundary
                    to_dispose.extend(path.iter().cloned());
                    to_update.push(importer.clone());
                    return PropagationResult::hot_update(importer, to_update, to_dispose);
                }

                // Continue propagating up
                let mut new_path = path.clone();
                new_path.push(importer.clone());
                queue.push_back((importer, new_path));
            }
        }

        // No accepting boundary found
        PropagationResult::full_reload()
    }

    /// Propagate multiple updates at once.
    ///
    /// Returns a combined result for all changed modules.
    pub fn propagate_batch(&self, changed_paths: &[String]) -> PropagationResult {
        let mut all_updated: Vec<String> = Vec::new();
        let mut all_disposed: Vec<String> = Vec::new();
        let mut boundaries: Vec<String> = Vec::new();

        for path in changed_paths {
            let result = self.propagate(path);

            if result.needs_reload {
                return PropagationResult::full_reload();
            }

            all_updated.extend(result.updated);
            all_disposed.extend(result.disposed);
            if let Some(boundary) = result.boundary {
                boundaries.push(boundary);
            }
        }

        // Deduplicate
        let updated: Vec<String> =
            all_updated.into_iter().collect::<HashSet<_>>().into_iter().collect();
        let disposed: Vec<String> =
            all_disposed.into_iter().collect::<HashSet<_>>().into_iter().collect();

        PropagationResult {
            updated,
            disposed,
            needs_reload: false,
            boundary: boundaries.first().cloned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_graph() -> HashMap<String, Vec<String>> {
        // Create a simple module graph:
        // main.js -> app.js -> component.js
        //                   -> utils.js
        let mut graph = HashMap::new();
        graph.insert("component.js".to_string(), vec!["app.js".to_string()]);
        graph.insert("utils.js".to_string(), vec!["app.js".to_string()]);
        graph.insert("app.js".to_string(), vec!["main.js".to_string()]);
        graph.insert("main.js".to_string(), vec![]);
        graph
    }

    #[test]
    fn test_self_accepting_module() {
        let graph = create_test_graph();
        let self_accepting = HashSet::from(["component.js".to_string()]);

        let propagator = UpdatePropagator::new(
            |path| graph.get(path).cloned().unwrap_or_default(),
            |path| self_accepting.contains(path),
            |_, _| false,
            |_| false,
        );

        let result = propagator.propagate("component.js");

        assert!(!result.needs_reload);
        assert_eq!(result.boundary, Some("component.js".to_string()));
    }

    #[test]
    fn test_parent_accepts_dependency() {
        let graph = create_test_graph();
        let dep_accepting: HashMap<String, HashSet<String>> =
            HashMap::from([("app.js".to_string(), HashSet::from(["utils.js".to_string()]))]);

        let propagator = UpdatePropagator::new(
            |path| graph.get(path).cloned().unwrap_or_default(),
            |_| false,
            |parent, dep| dep_accepting.get(parent).map_or(false, |deps| deps.contains(dep)),
            |_| false,
        );

        let result = propagator.propagate("utils.js");

        assert!(!result.needs_reload);
        assert_eq!(result.boundary, Some("app.js".to_string()));
    }

    #[test]
    fn test_no_accepting_boundary() {
        let graph = create_test_graph();

        let propagator = UpdatePropagator::new(
            |path| graph.get(path).cloned().unwrap_or_default(),
            |_| false,
            |_, _| false,
            |_| false,
        );

        let result = propagator.propagate("component.js");

        assert!(result.needs_reload);
    }

    #[test]
    fn test_declined_module() {
        let graph = create_test_graph();
        let declined = HashSet::from(["app.js".to_string()]);

        let propagator = UpdatePropagator::new(
            |path| graph.get(path).cloned().unwrap_or_default(),
            |_| false,
            |_, _| false,
            |path| declined.contains(path),
        );

        let result = propagator.propagate("component.js");

        assert!(result.needs_reload);
    }

    #[test]
    fn test_batch_propagation() {
        let graph = create_test_graph();
        let self_accepting = HashSet::from(["component.js".to_string(), "utils.js".to_string()]);

        let propagator = UpdatePropagator::new(
            |path| graph.get(path).cloned().unwrap_or_default(),
            |path| self_accepting.contains(path),
            |_, _| false,
            |_| false,
        );

        let result =
            propagator.propagate_batch(&["component.js".to_string(), "utils.js".to_string()]);

        assert!(!result.needs_reload);
        assert_eq!(result.updated.len(), 2);
    }
}
