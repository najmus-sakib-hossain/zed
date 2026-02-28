//! Plugin builder for registering hooks.
//!
//! Provides the API for plugins to register onLoad and onResolve handlers.

use crate::error::PluginResult;
use crate::loader::Loader;
use parking_lot::RwLock;
use regex::Regex;
use std::sync::Arc;

/// Arguments passed to onLoad handlers.
#[derive(Debug, Clone)]
pub struct OnLoadArgs {
    /// The resolved path
    pub path: String,
    /// The namespace
    pub namespace: String,
    /// The suffix (query string, etc.)
    pub suffix: String,
}

/// Result from onLoad handlers.
#[derive(Debug, Clone)]
pub struct OnLoadResult {
    /// The file contents
    pub contents: String,
    /// The loader to use
    pub loader: Loader,
    /// Resolve directory for imports
    pub resolve_dir: Option<String>,
}

/// Arguments passed to onResolve handlers.
#[derive(Debug, Clone)]
pub struct OnResolveArgs {
    /// The import path
    pub path: String,
    /// The importer path
    pub importer: String,
    /// The namespace
    pub namespace: String,
    /// The resolve directory
    pub resolve_dir: String,
    /// The import kind
    pub kind: ImportKind,
}

/// Result from onResolve handlers.
#[derive(Debug, Clone)]
pub struct OnResolveResult {
    /// The resolved path
    pub path: String,
    /// The namespace
    pub namespace: Option<String>,
    /// Whether to mark as external
    pub external: bool,
    /// Side effects
    pub side_effects: Option<bool>,
}

/// Import kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    /// Static import
    Import,
    /// Dynamic import
    DynamicImport,
    /// Require
    Require,
    /// Entry point
    EntryPoint,
}

/// Filter for matching paths.
#[derive(Clone)]
pub struct Filter {
    pattern: Regex,
    namespace: Option<String>,
}

impl Filter {
    /// Create a new filter from a regex pattern.
    pub fn new(pattern: &str, namespace: Option<&str>) -> PluginResult<Self> {
        let regex = Regex::new(pattern)
            .map_err(|e| crate::error::PluginError::Handler(format!("Invalid regex: {}", e)))?;

        Ok(Self {
            pattern: regex,
            namespace: namespace.map(|s| s.to_string()),
        })
    }

    /// Check if the filter matches the given path and namespace.
    pub fn matches(&self, path: &str, namespace: &str) -> bool {
        // Check namespace first
        if let Some(ref ns) = self.namespace {
            if ns != namespace {
                return false;
            }
        }

        // Check path pattern
        self.pattern.is_match(path)
    }

    /// Get the pattern string.
    pub fn pattern(&self) -> &str {
        self.pattern.as_str()
    }

    /// Get the namespace filter.
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }
}

/// Handler function type for onLoad.
pub type OnLoadHandler = Arc<dyn Fn(&OnLoadArgs) -> Option<OnLoadResult> + Send + Sync>;

/// Handler function type for onResolve.
pub type OnResolveHandler = Arc<dyn Fn(&OnResolveArgs) -> Option<OnResolveResult> + Send + Sync>;

/// Registered onLoad handler.
struct LoadHandler {
    filter: Filter,
    handler: OnLoadHandler,
}

/// Registered onResolve handler.
struct ResolveHandler {
    filter: Filter,
    handler: OnResolveHandler,
}

/// Plugin builder for registering hooks.
pub struct PluginBuilder {
    name: String,
    on_load_handlers: RwLock<Vec<LoadHandler>>,
    on_resolve_handlers: RwLock<Vec<ResolveHandler>>,
}

impl PluginBuilder {
    /// Create a new plugin builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            on_load_handlers: RwLock::new(Vec::new()),
            on_resolve_handlers: RwLock::new(Vec::new()),
        }
    }

    /// Get the plugin name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Register an onLoad handler.
    ///
    /// The filter is a regex pattern that matches against file paths.
    /// The namespace is optional and filters by namespace.
    pub fn on_load<F>(&self, filter: &str, namespace: Option<&str>, handler: F) -> PluginResult<()>
    where
        F: Fn(&OnLoadArgs) -> Option<OnLoadResult> + Send + Sync + 'static,
    {
        let filter = Filter::new(filter, namespace)?;
        self.on_load_handlers.write().push(LoadHandler {
            filter,
            handler: Arc::new(handler),
        });
        Ok(())
    }

    /// Register an onResolve handler.
    ///
    /// The filter is a regex pattern that matches against import paths.
    /// The namespace is optional and filters by namespace.
    pub fn on_resolve<F>(
        &self,
        filter: &str,
        namespace: Option<&str>,
        handler: F,
    ) -> PluginResult<()>
    where
        F: Fn(&OnResolveArgs) -> Option<OnResolveResult> + Send + Sync + 'static,
    {
        let filter = Filter::new(filter, namespace)?;
        self.on_resolve_handlers.write().push(ResolveHandler {
            filter,
            handler: Arc::new(handler),
        });
        Ok(())
    }

    /// Run onLoad handlers for a path.
    pub fn run_on_load(&self, args: &OnLoadArgs) -> Option<OnLoadResult> {
        let handlers = self.on_load_handlers.read();
        for handler in handlers.iter() {
            if handler.filter.matches(&args.path, &args.namespace) {
                if let Some(result) = (handler.handler)(args) {
                    return Some(result);
                }
            }
        }
        None
    }

    /// Run onResolve handlers for a path.
    pub fn run_on_resolve(&self, args: &OnResolveArgs) -> Option<OnResolveResult> {
        let handlers = self.on_resolve_handlers.read();
        for handler in handlers.iter() {
            if handler.filter.matches(&args.path, &args.namespace) {
                if let Some(result) = (handler.handler)(args) {
                    return Some(result);
                }
            }
        }
        None
    }

    /// Get the number of registered onLoad handlers.
    pub fn on_load_count(&self) -> usize {
        self.on_load_handlers.read().len()
    }

    /// Get the number of registered onResolve handlers.
    pub fn on_resolve_count(&self) -> usize {
        self.on_resolve_handlers.read().len()
    }
}

impl Default for PluginBuilder {
    fn default() -> Self {
        Self::new("unnamed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_matches() {
        let filter = Filter::new(r"\.css$", None).unwrap();
        assert!(filter.matches("style.css", "file"));
        assert!(!filter.matches("script.js", "file"));
    }

    #[test]
    fn test_filter_with_namespace() {
        let filter = Filter::new(r".*", Some("virtual")).unwrap();
        assert!(filter.matches("anything", "virtual"));
        assert!(!filter.matches("anything", "file"));
    }

    #[test]
    fn test_on_load_handler() {
        let builder = PluginBuilder::new("test");

        builder
            .on_load(r"\.txt$", None, |args| {
                Some(OnLoadResult {
                    contents: format!("Loaded: {}", args.path),
                    loader: Loader::Text,
                    resolve_dir: None,
                })
            })
            .unwrap();

        let args = OnLoadArgs {
            path: "test.txt".to_string(),
            namespace: "file".to_string(),
            suffix: String::new(),
        };

        let result = builder.run_on_load(&args);
        assert!(result.is_some());
        assert!(result.unwrap().contents.contains("test.txt"));
    }

    #[test]
    fn test_on_resolve_handler() {
        let builder = PluginBuilder::new("test");

        builder
            .on_resolve(r"^virtual:", None, |args| {
                Some(OnResolveResult {
                    path: args.path.replace("virtual:", "/virtual/"),
                    namespace: Some("virtual".to_string()),
                    external: false,
                    side_effects: None,
                })
            })
            .unwrap();

        let args = OnResolveArgs {
            path: "virtual:module".to_string(),
            importer: "index.js".to_string(),
            namespace: "file".to_string(),
            resolve_dir: "/project".to_string(),
            kind: ImportKind::Import,
        };

        let result = builder.run_on_resolve(&args);
        assert!(result.is_some());
        assert_eq!(result.unwrap().namespace, Some("virtual".to_string()));
    }
}
