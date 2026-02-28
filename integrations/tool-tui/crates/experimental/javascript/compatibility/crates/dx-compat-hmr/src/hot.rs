//! import.meta.hot API implementation.
//!
//! Provides the Hot Module Replacement API that modules use to accept updates,
//! register dispose handlers, and manage module state during hot updates.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Callback type for accept handlers.
pub type AcceptCallback = Box<dyn Fn(&[String]) + Send + Sync>;

/// Callback type for dispose handlers.
pub type DisposeCallback = Box<dyn Fn(&mut HashMap<String, String>) + Send + Sync>;

/// Hot module state for tracking handlers and data.
#[derive(Default)]
struct HotModuleState {
    /// Accept handlers for self-accepting modules
    self_accept_handlers: Vec<AcceptCallback>,
    /// Accept handlers for specific dependencies
    dep_accept_handlers: HashMap<String, Vec<AcceptCallback>>,
    /// Dispose handlers called before module replacement
    dispose_handlers: Vec<DisposeCallback>,
    /// Whether this module declines updates (forces full reload)
    declined: bool,
    /// Whether this module has been invalidated
    invalidated: bool,
}

/// Hot module API.
///
/// Provides the `import.meta.hot` API for Hot Module Replacement.
/// Each module gets its own HotModule instance to manage its update behavior.
pub struct HotModule {
    /// Module path
    path: String,
    /// Preserved data between updates
    pub data: RwLock<HashMap<String, String>>,
    /// Internal state
    state: RwLock<HotModuleState>,
}

impl HotModule {
    /// Create a new hot module for the given path.
    pub fn new() -> Self {
        Self {
            path: String::new(),
            data: RwLock::new(HashMap::new()),
            state: RwLock::new(HotModuleState::default()),
        }
    }

    /// Create a new hot module with a specific path.
    pub fn with_path(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            data: RwLock::new(HashMap::new()),
            state: RwLock::new(HotModuleState::default()),
        }
    }

    /// Get the module path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Accept updates for this module (self-accepting).
    ///
    /// When this module is updated, the callback will be called with the
    /// list of updated module paths. The module is responsible for applying
    /// the update.
    ///
    /// # Example
    /// ```ignore
    /// import.meta.hot.accept((updatedModules) => {
    ///     // Re-render with new module
    /// });
    /// ```
    pub fn accept(&self, callback: impl Fn(&[String]) + Send + Sync + 'static) {
        let mut state = self.state.write();
        state.self_accept_handlers.push(Box::new(callback));
    }

    /// Accept updates for specific dependencies.
    ///
    /// When any of the specified dependencies are updated, the callback
    /// will be called. This allows a module to handle updates to its
    /// dependencies without being replaced itself.
    ///
    /// # Example
    /// ```ignore
    /// import.meta.hot.accept(['./dep.js'], (updatedModules) => {
    ///     // Handle dependency update
    /// });
    /// ```
    pub fn accept_deps(&self, deps: &[&str], callback: impl Fn(&[String]) + Send + Sync + 'static) {
        let mut state = self.state.write();
        let callback = Arc::new(callback);

        for dep in deps {
            state.dep_accept_handlers.entry(dep.to_string()).or_default().push(Box::new({
                let callback = Arc::clone(&callback);
                move |modules| callback(modules)
            }));
        }
    }

    /// Register a dispose callback.
    ///
    /// The dispose callback is called before this module is replaced.
    /// Use it to clean up side effects (timers, event listeners, etc.)
    /// and optionally pass data to the new module via the data parameter.
    ///
    /// # Example
    /// ```ignore
    /// import.meta.hot.dispose((data) => {
    ///     clearInterval(intervalId);
    ///     data.savedState = currentState;
    /// });
    /// ```
    pub fn dispose(&self, callback: impl Fn(&mut HashMap<String, String>) + Send + Sync + 'static) {
        let mut state = self.state.write();
        state.dispose_handlers.push(Box::new(callback));
    }

    /// Decline updates for this module.
    ///
    /// When this module is updated, a full page reload will be triggered
    /// instead of hot replacement. Use this for modules that cannot be
    /// safely hot-replaced.
    ///
    /// # Example
    /// ```ignore
    /// import.meta.hot.decline();
    /// ```
    pub fn decline(&self) {
        let mut state = self.state.write();
        state.declined = true;
    }

    /// Invalidate this module.
    ///
    /// Forces the module to be re-executed. This is useful when a module
    /// detects that it cannot apply an update and needs to be fully replaced.
    ///
    /// # Example
    /// ```ignore
    /// import.meta.hot.invalidate();
    /// ```
    pub fn invalidate(&self) {
        let mut state = self.state.write();
        state.invalidated = true;
    }

    /// Check if this module accepts self-updates.
    pub fn is_self_accepting(&self) -> bool {
        let state = self.state.read();
        !state.self_accept_handlers.is_empty()
    }

    /// Check if this module accepts updates for a specific dependency.
    pub fn accepts_dependency(&self, dep: &str) -> bool {
        let state = self.state.read();
        state.dep_accept_handlers.contains_key(dep)
    }

    /// Check if this module has declined updates.
    pub fn is_declined(&self) -> bool {
        let state = self.state.read();
        state.declined
    }

    /// Check if this module has been invalidated.
    pub fn is_invalidated(&self) -> bool {
        let state = self.state.read();
        state.invalidated
    }

    /// Clear the invalidated flag.
    pub fn clear_invalidated(&self) {
        let mut state = self.state.write();
        state.invalidated = false;
    }

    /// Apply an update to this module.
    ///
    /// This is called by the HMR runtime when an update is available.
    /// Returns `true` if the update was handled, `false` if a full reload is needed.
    pub fn apply_update(&self, updated_modules: &[String]) -> bool {
        // Check if declined
        if self.is_declined() {
            return false;
        }

        // Run dispose handlers first
        self.run_dispose_handlers();

        // Run accept handlers
        let state = self.state.read();

        // Try self-accept handlers
        if !state.self_accept_handlers.is_empty() {
            for handler in &state.self_accept_handlers {
                handler(updated_modules);
            }
            return true;
        }

        // Try dependency accept handlers
        for module in updated_modules {
            if let Some(handlers) = state.dep_accept_handlers.get(module) {
                for handler in handlers {
                    handler(updated_modules);
                }
                return true;
            }
        }

        // No handlers found - bubble up
        false
    }

    /// Run all dispose handlers.
    ///
    /// Called before the module is replaced. Handlers receive mutable
    /// access to the data map to preserve state for the new module.
    pub fn run_dispose_handlers(&self) {
        let state = self.state.read();
        let mut data = self.data.write();

        for handler in &state.dispose_handlers {
            handler(&mut data);
        }
    }

    /// Prune stale accept handlers.
    ///
    /// Called after an update to remove handlers that are no longer valid.
    pub fn prune_stale_handlers(&self) {
        let mut state = self.state.write();
        state.self_accept_handlers.clear();
        state.dep_accept_handlers.clear();
        state.dispose_handlers.clear();
        state.invalidated = false;
        // Note: declined flag is preserved
    }
}

impl Default for HotModule {
    fn default() -> Self {
        Self::new()
    }
}

/// HMR runtime that manages all hot modules.
pub struct HmrRuntime {
    /// All registered hot modules by path
    modules: RwLock<HashMap<String, Arc<HotModule>>>,
}

impl HmrRuntime {
    /// Create a new HMR runtime.
    pub fn new() -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
        }
    }

    /// Get or create a hot module for the given path.
    pub fn get_or_create(&self, path: &str) -> Arc<HotModule> {
        let mut modules = self.modules.write();

        if let Some(module) = modules.get(path) {
            Arc::clone(module)
        } else {
            let module = Arc::new(HotModule::with_path(path));
            modules.insert(path.to_string(), Arc::clone(&module));
            module
        }
    }

    /// Get a hot module by path.
    pub fn get(&self, path: &str) -> Option<Arc<HotModule>> {
        let modules = self.modules.read();
        modules.get(path).cloned()
    }

    /// Remove a hot module.
    pub fn remove(&self, path: &str) -> Option<Arc<HotModule>> {
        let mut modules = self.modules.write();
        modules.remove(path)
    }

    /// Apply updates to affected modules.
    ///
    /// Returns the list of modules that need full reload (couldn't be hot-updated).
    pub fn apply_updates(&self, updated_paths: &[String]) -> Vec<String> {
        let mut needs_reload = Vec::new();
        let modules = self.modules.read();

        for path in updated_paths {
            if let Some(module) = modules.get(path) {
                if !module.apply_update(updated_paths) {
                    needs_reload.push(path.clone());
                }
            } else {
                // Module not registered - needs reload
                needs_reload.push(path.clone());
            }
        }

        needs_reload
    }

    /// Check if any module in the update chain can accept the update.
    ///
    /// Walks up the dependency chain to find an accepting module.
    pub fn can_accept_update(
        &self,
        path: &str,
        get_importers: impl Fn(&str) -> Vec<String>,
    ) -> bool {
        let modules = self.modules.read();

        // Check if the module itself accepts
        if let Some(module) = modules.get(path) {
            if module.is_self_accepting() {
                return true;
            }
            if module.is_declined() {
                return false;
            }
        }

        // Check importers recursively
        let importers = get_importers(path);
        for importer in importers {
            if let Some(module) = modules.get(&importer) {
                if module.accepts_dependency(path) {
                    return true;
                }
            }
            // Recursively check importer's importers
            if self.can_accept_update(&importer, &get_importers) {
                return true;
            }
        }

        false
    }

    /// Get all registered module paths.
    pub fn all_paths(&self) -> Vec<String> {
        let modules = self.modules.read();
        modules.keys().cloned().collect()
    }

    /// Clear all modules.
    pub fn clear(&self) {
        let mut modules = self.modules.write();
        modules.clear();
    }
}

impl Default for HmrRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_hot_module_accept() {
        let hot = HotModule::new();
        let called = Arc::new(AtomicUsize::new(0));

        let called_clone = Arc::clone(&called);
        hot.accept(move |_| {
            called_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(hot.is_self_accepting());

        let handled = hot.apply_update(&["test.js".to_string()]);
        assert!(handled);
        assert_eq!(called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_hot_module_dispose() {
        let hot = HotModule::new();
        let disposed = Arc::new(AtomicUsize::new(0));

        let disposed_clone = Arc::clone(&disposed);
        hot.dispose(move |data| {
            disposed_clone.fetch_add(1, Ordering::SeqCst);
            data.insert("key".to_string(), "value".to_string());
        });

        hot.run_dispose_handlers();

        assert_eq!(disposed.load(Ordering::SeqCst), 1);
        assert_eq!(hot.data.read().get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_hot_module_decline() {
        let hot = HotModule::new();

        assert!(!hot.is_declined());
        hot.decline();
        assert!(hot.is_declined());

        // Declined modules should not accept updates
        let handled = hot.apply_update(&["test.js".to_string()]);
        assert!(!handled);
    }

    #[test]
    fn test_hot_module_invalidate() {
        let hot = HotModule::new();

        assert!(!hot.is_invalidated());
        hot.invalidate();
        assert!(hot.is_invalidated());

        hot.clear_invalidated();
        assert!(!hot.is_invalidated());
    }

    #[test]
    fn test_hot_module_accept_deps() {
        let hot = HotModule::new();
        let called = Arc::new(AtomicUsize::new(0));

        let called_clone = Arc::clone(&called);
        hot.accept_deps(&["./dep.js"], move |_| {
            called_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(hot.accepts_dependency("./dep.js"));
        assert!(!hot.accepts_dependency("./other.js"));
    }

    #[test]
    fn test_hmr_runtime() {
        let runtime = HmrRuntime::new();

        let module = runtime.get_or_create("test.js");
        module.accept(|_| {});

        // Same path should return same module
        let module2 = runtime.get_or_create("test.js");
        assert!(Arc::ptr_eq(&module, &module2));

        // Different path should return different module
        let module3 = runtime.get_or_create("other.js");
        assert!(!Arc::ptr_eq(&module, &module3));
    }

    #[test]
    fn test_hmr_runtime_apply_updates() {
        let runtime = HmrRuntime::new();

        // Module that accepts updates
        let accepting = runtime.get_or_create("accepting.js");
        accepting.accept(|_| {});

        // Module that doesn't accept
        let _non_accepting = runtime.get_or_create("non-accepting.js");

        let needs_reload = runtime.apply_updates(&[
            "accepting.js".to_string(),
            "non-accepting.js".to_string(),
            "unknown.js".to_string(),
        ]);

        // accepting.js should be handled, others need reload
        assert!(!needs_reload.contains(&"accepting.js".to_string()));
        assert!(needs_reload.contains(&"non-accepting.js".to_string()));
        assert!(needs_reload.contains(&"unknown.js".to_string()));
    }

    #[test]
    fn test_dispose_before_accept() {
        let hot = HotModule::new();
        let order = Arc::new(RwLock::new(Vec::new()));

        let order_clone = Arc::clone(&order);
        hot.dispose(move |_| {
            order_clone.write().push("dispose");
        });

        let order_clone = Arc::clone(&order);
        hot.accept(move |_| {
            order_clone.write().push("accept");
        });

        hot.apply_update(&["test.js".to_string()]);

        let order = order.read();
        assert_eq!(order.as_slice(), &["dispose", "accept"]);
    }
}
