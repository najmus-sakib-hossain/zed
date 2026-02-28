//! # File-System Router
//!
//! This module implements the file-system based routing system for the DX WWW Framework.
//! It maps page files to URL routes and handles dynamic routing with parameter extraction.
//!
//! ## Routing Rules
//!
//! - `pages/index.pg` → `/`
//! - `pages/about.pg` → `/about`
//! - `pages/blog/post.pg` → `/blog/post`
//! - `pages/user/[id].pg` → `/user/:id` (dynamic)
//! - `pages/docs/[...slug].pg` → `/docs/*` (catch-all)
//! - `pages/_layout.pg` → Layout applied to siblings and children

mod layout;
mod matcher;
mod pattern;

pub use layout::LayoutResolver;
pub use matcher::RouteMatcher;
pub use pattern::{RoutePattern, RouteSegment};

use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{DxError, DxResult};
use crate::project::{PageFile, Project};

// =============================================================================
// Route Types
// =============================================================================

/// Represents a resolved route in the routing table.
#[derive(Debug, Clone)]
pub struct Route {
    /// The URL path pattern (e.g., "/user/:id")
    pub path: String,

    /// The source file path
    pub file_path: PathBuf,

    /// The compiled binary object path
    pub binary_path: PathBuf,

    /// The route pattern for matching
    pub pattern: RoutePattern,

    /// Layout chain (from root to leaf)
    pub layout_chain: Vec<PathBuf>,

    /// Whether this route has a data loader
    pub has_data_loader: bool,

    /// Route metadata
    pub metadata: RouteMetadata,
}

/// Metadata for a route.
#[derive(Debug, Clone, Default)]
pub struct RouteMetadata {
    /// Page title
    pub title: Option<String>,

    /// Page description
    pub description: Option<String>,

    /// Additional meta tags
    pub meta_tags: HashMap<String, String>,

    /// Whether this is a special page
    pub is_special: bool,
}

/// Result of matching a URL path against the router.
#[derive(Debug, Clone)]
pub struct RouteMatch<'a> {
    /// The matched route
    pub route: &'a Route,

    /// Extracted parameters from the URL
    pub params: HashMap<String, String>,

    /// The matched URL path
    pub path: String,
}

// =============================================================================
// File System Router
// =============================================================================

/// File-system based router for the DX WWW Framework.
///
/// Maps page files in the `pages/` directory to URL routes.
#[derive(Debug, Clone)]
pub struct FileSystemRouter {
    /// Static routes (exact match)
    static_routes: HashMap<String, Route>,

    /// Dynamic routes (with parameters)
    dynamic_routes: Vec<Route>,

    /// Catch-all routes
    catch_all_routes: Vec<Route>,

    /// Layout resolver
    layout_resolver: LayoutResolver,

    /// Error page route
    error_route: Option<Route>,

    /// 404 page route
    not_found_route: Option<Route>,

    /// Whether to use trailing slashes
    trailing_slash: bool,

    /// Whether matching is case-sensitive
    case_sensitive: bool,
}

impl Default for FileSystemRouter {
    fn default() -> Self {
        Self {
            static_routes: HashMap::new(),
            dynamic_routes: Vec::new(),
            catch_all_routes: Vec::new(),
            layout_resolver: LayoutResolver::new(),
            error_route: None,
            not_found_route: None,
            trailing_slash: false,
            case_sensitive: false,
        }
    }
}

impl FileSystemRouter {
    /// Create a new empty file system router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new file system router from a project.
    ///
    /// # Arguments
    ///
    /// * `project` - The scanned project structure
    ///
    /// # Returns
    ///
    /// The configured router
    pub fn from_project(project: &Project) -> DxResult<Self> {
        let mut router = Self {
            static_routes: HashMap::new(),
            dynamic_routes: Vec::new(),
            catch_all_routes: Vec::new(),
            layout_resolver: LayoutResolver::new(),
            error_route: None,
            not_found_route: None,
            trailing_slash: project.config.routing.trailing_slash,
            case_sensitive: project.config.routing.case_sensitive,
        };

        // Build layout resolver
        router.layout_resolver.add_layouts(&project.layouts);

        // Add all pages
        for page in &project.pages {
            router.add_page(page, project)?;
        }

        Ok(router)
    }

    /// Add a page to the router.
    fn add_page(&mut self, page: &PageFile, project: &Project) -> DxResult<()> {
        let layout_chain = project
            .get_layouts_for_path(&page.relative_path)
            .iter()
            .map(|l| l.path.clone())
            .collect();

        let route = Route {
            path: page.route_path.clone(),
            file_path: page.path.clone(),
            binary_path: self.compute_binary_path(&page.path, project),
            pattern: RoutePattern::parse(&page.route_path),
            layout_chain,
            has_data_loader: false, // Will be updated after parsing
            metadata: RouteMetadata {
                is_special: page.is_special,
                ..Default::default()
            },
        };

        // Handle special pages
        if page.is_special {
            if page.route_path.contains("_error") {
                self.error_route = Some(route);
            } else if page.route_path.contains("_404") {
                self.not_found_route = Some(route);
            }
            return Ok(());
        }

        // Check for duplicate routes
        let normalized_path = self.normalize_path(&page.route_path);
        if self.static_routes.contains_key(&normalized_path) {
            return Err(DxError::DuplicateRoute {
                path: normalized_path.clone(),
                file1: self.static_routes[&normalized_path].file_path.clone(),
                file2: page.path.clone(),
            });
        }

        // Add to appropriate collection
        if page.is_catch_all {
            self.catch_all_routes.push(route);
        } else if page.is_dynamic {
            self.dynamic_routes.push(route);
        } else {
            self.static_routes.insert(normalized_path, route);
        }

        Ok(())
    }

    /// Match a URL path against the router.
    ///
    /// # Arguments
    ///
    /// * `path` - The URL path to match
    ///
    /// # Returns
    ///
    /// The matched route and extracted parameters, or None if no match
    pub fn match_route(&self, path: &str) -> Option<RouteMatch<'_>> {
        let normalized = self.normalize_path(path);

        // Try static routes first (fastest)
        if let Some(route) = self.static_routes.get(&normalized) {
            return Some(RouteMatch {
                route,
                params: HashMap::new(),
                path: normalized,
            });
        }

        // Try dynamic routes
        for route in &self.dynamic_routes {
            if let Some(params) = route.pattern.match_path(&normalized) {
                return Some(RouteMatch {
                    route,
                    params,
                    path: normalized,
                });
            }
        }

        // Try catch-all routes
        for route in &self.catch_all_routes {
            if let Some(params) = route.pattern.match_path(&normalized) {
                return Some(RouteMatch {
                    route,
                    params,
                    path: normalized,
                });
            }
        }

        None
    }

    /// Get the error page route.
    pub fn error_route(&self) -> Option<&Route> {
        self.error_route.as_ref()
    }

    /// Get the 404 page route.
    pub fn not_found_route(&self) -> Option<&Route> {
        self.not_found_route.as_ref()
    }

    /// Get all routes.
    pub fn all_routes(&self) -> Vec<&Route> {
        let mut routes: Vec<&Route> = self.static_routes.values().collect();
        routes.extend(self.dynamic_routes.iter());
        routes.extend(self.catch_all_routes.iter());
        routes
    }

    /// Get the number of routes.
    pub fn route_count(&self) -> usize {
        self.static_routes.len() + self.dynamic_routes.len() + self.catch_all_routes.len()
    }

    /// Normalize a URL path.
    fn normalize_path(&self, path: &str) -> String {
        let mut normalized = path.to_string();

        // Handle case sensitivity
        if !self.case_sensitive {
            normalized = normalized.to_lowercase();
        }

        // Handle trailing slashes
        if self.trailing_slash {
            if !normalized.ends_with('/') && normalized != "/" {
                normalized.push('/');
            }
        } else {
            while normalized.len() > 1 && normalized.ends_with('/') {
                normalized.pop();
            }
        }

        // Ensure leading slash
        if !normalized.starts_with('/') {
            normalized = format!("/{normalized}");
        }

        normalized
    }

    /// Compute the binary output path for a source file.
    fn compute_binary_path(&self, source_path: &PathBuf, project: &Project) -> PathBuf {
        let relative = source_path.strip_prefix(&project.root).unwrap_or(source_path);

        let mut binary_path = project.output_dir().join(relative);
        binary_path.set_extension(crate::BINARY_EXTENSION);
        binary_path
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_route(path: &str) -> Route {
        Route {
            path: path.to_string(),
            file_path: PathBuf::from(format!("pages{path}.pg")),
            binary_path: PathBuf::from(format!(".dx/build/pages{path}.dxob")),
            pattern: RoutePattern::parse(path),
            layout_chain: Vec::new(),
            has_data_loader: false,
            metadata: RouteMetadata::default(),
        }
    }

    #[test]
    fn test_static_route_matching() {
        let mut router = FileSystemRouter {
            static_routes: HashMap::new(),
            dynamic_routes: Vec::new(),
            catch_all_routes: Vec::new(),
            layout_resolver: LayoutResolver::new(),
            error_route: None,
            not_found_route: None,
            trailing_slash: false,
            case_sensitive: false,
        };

        router.static_routes.insert("/".to_string(), create_test_route("/"));
        router.static_routes.insert("/about".to_string(), create_test_route("/about"));

        assert!(router.match_route("/").is_some());
        assert!(router.match_route("/about").is_some());
        assert!(router.match_route("/nonexistent").is_none());
    }

    #[test]
    fn test_dynamic_route_matching() {
        let mut router = FileSystemRouter {
            static_routes: HashMap::new(),
            dynamic_routes: Vec::new(),
            catch_all_routes: Vec::new(),
            layout_resolver: LayoutResolver::new(),
            error_route: None,
            not_found_route: None,
            trailing_slash: false,
            case_sensitive: false,
        };

        router.dynamic_routes.push(create_test_route("/user/:id"));

        let result = router.match_route("/user/123");
        assert!(result.is_some());
        let route_match = result.unwrap();
        assert_eq!(route_match.params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_catch_all_route_matching() {
        let mut router = FileSystemRouter {
            static_routes: HashMap::new(),
            dynamic_routes: Vec::new(),
            catch_all_routes: Vec::new(),
            layout_resolver: LayoutResolver::new(),
            error_route: None,
            not_found_route: None,
            trailing_slash: false,
            case_sensitive: false,
        };

        router.catch_all_routes.push(create_test_route("/docs/*slug"));

        let result = router.match_route("/docs/getting-started/intro");
        assert!(result.is_some());
        let route_match = result.unwrap();
        assert_eq!(route_match.params.get("slug"), Some(&"getting-started/intro".to_string()));
    }

    #[test]
    fn test_case_insensitive_matching() {
        let mut router = FileSystemRouter {
            static_routes: HashMap::new(),
            dynamic_routes: Vec::new(),
            catch_all_routes: Vec::new(),
            layout_resolver: LayoutResolver::new(),
            error_route: None,
            not_found_route: None,
            trailing_slash: false,
            case_sensitive: false,
        };

        router.static_routes.insert("/about".to_string(), create_test_route("/about"));

        assert!(router.match_route("/ABOUT").is_some());
        assert!(router.match_route("/About").is_some());
    }

    #[test]
    fn test_trailing_slash_handling() {
        let mut router = FileSystemRouter {
            static_routes: HashMap::new(),
            dynamic_routes: Vec::new(),
            catch_all_routes: Vec::new(),
            layout_resolver: LayoutResolver::new(),
            error_route: None,
            not_found_route: None,
            trailing_slash: false,
            case_sensitive: false,
        };

        router.static_routes.insert("/about".to_string(), create_test_route("/about"));

        assert!(router.match_route("/about/").is_some());
    }
}
