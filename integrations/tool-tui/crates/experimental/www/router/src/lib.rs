//! # dx-router: File-Based Router for DX-WWW
//!
//! Next.js-style file-based routing with binary-first approach.
//!
//! ## File Mapping
//!
//! | File Path | Route |
//! |-----------|-------|
//! | `pages/index.pg` | `/` |
//! | `pages/about.pg` | `/about` |
//! | `pages/blog/index.pg` | `/blog` |
//! | `pages/blog/[slug].pg` | `/blog/:slug` |
//! | `pages/docs/[...path].pg` | `/docs/*` |
//! | `pages/(marketing)/pricing.pg` | `/pricing` |
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dx_www_router::{Router, scan_pages_directory};
//!
//! // Scan pages directory and build router
//! let routes = scan_pages_directory("www/pages")?;
//! let router = Router::new(routes);
//!
//! // Match a URL
//! if let Some(matched) = router.match_path("/blog/hello-world") {
//!     println!("Route: {}", matched.route.pattern);
//!     println!("Params: {:?}", matched.params);
//! }
//! ```

#![forbid(unsafe_code)]

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

/// Router errors
#[derive(Debug, Error)]
pub enum RouterError {
    #[error("Invalid route pattern: {0}")]
    InvalidPattern(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("WalkDir error: {0}")]
    WalkDirError(#[from] walkdir::Error),

    #[error("Duplicate route: {0}")]
    DuplicateRoute(String),
}

pub type RouterResult<T> = Result<T, RouterError>;

/// Route segment types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Segment {
    /// Static segment: `about` matches only "about"
    Static(String),
    /// Dynamic segment: `[slug]` matches any single segment
    Dynamic(String),
    /// Catch-all segment: `[...path]` matches any remaining segments
    CatchAll(String),
    /// Optional catch-all: `[[...path]]` matches zero or more segments
    OptionalCatchAll(String),
}

/// A single route definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Unique route ID (hash of pattern)
    pub id: u64,
    /// The URL pattern (e.g., "/blog/[slug]")
    pub pattern: String,
    /// Parsed segments
    pub segments: Vec<Segment>,
    /// Path to the source file
    pub file_path: PathBuf,
    /// Layout file path (if any)
    pub layout: Option<PathBuf>,
    /// Is this a catch-all route?
    pub is_catch_all: bool,
    /// Route specificity score (higher = more specific)
    pub specificity: u32,
}

impl Route {
    /// Create a new route from pattern and file path
    pub fn new(pattern: &str, file_path: PathBuf) -> RouterResult<Self> {
        let segments = parse_pattern(pattern)?;
        let is_catch_all = segments.iter().any(|s| {
            matches!(s, Segment::CatchAll(_) | Segment::OptionalCatchAll(_))
        });
        let specificity = calculate_specificity(&segments);
        let id = xxhash_rust::xxh3::xxh3_64(pattern.as_bytes());

        Ok(Self {
            id,
            pattern: pattern.to_string(),
            segments,
            file_path,
            layout: None,
            is_catch_all,
            specificity,
        })
    }

    /// Set the layout for this route
    pub fn with_layout(mut self, layout: PathBuf) -> Self {
        self.layout = Some(layout);
        self
    }
}

/// Match result containing route and extracted parameters
#[derive(Debug, Clone)]
pub struct RouteMatch<'a> {
    /// The matched route
    pub route: &'a Route,
    /// Extracted dynamic parameters
    pub params: HashMap<String, String>,
    /// Catch-all path segments (if any)
    pub catch_all: Option<Vec<String>>,
}

/// The file-based router
#[derive(Debug, Clone, Default)]
pub struct Router {
    /// All registered routes, sorted by specificity
    routes: Vec<Route>,
}

impl Router {
    /// Create a new empty router
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Create a router from a list of routes
    pub fn from_routes(mut routes: Vec<Route>) -> Self {
        // Sort by specificity (descending) so more specific routes match first
        routes.sort_by(|a, b| b.specificity.cmp(&a.specificity));
        Self { routes }
    }

    /// Add a route to the router
    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
        // Re-sort after adding
        self.routes.sort_by(|a, b| b.specificity.cmp(&a.specificity));
    }

    /// Match a URL path to a route
    pub fn match_path(&self, path: &str) -> Option<RouteMatch<'_>> {
        let path = normalize_path(path);
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        for route in &self.routes {
            if let Some(matched) = try_match_route(route, &path_segments) {
                return Some(matched);
            }
        }

        None
    }

    /// Get all routes
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// Find a route by ID
    pub fn find_by_id(&self, id: u64) -> Option<&Route> {
        self.routes.iter().find(|r| r.id == id)
    }

    /// Find a route by file path
    pub fn find_by_path(&self, file_path: &Path) -> Option<&Route> {
        self.routes.iter().find(|r| r.file_path == file_path)
    }
}

/// Parse a pattern string into segments
fn parse_pattern(pattern: &str) -> RouterResult<Vec<Segment>> {
    let pattern = normalize_path(pattern);
    let parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let mut segments = Vec::new();

    for part in parts {
        let segment = if part.starts_with("[[...") && part.ends_with("]]") {
            // Optional catch-all: [[...path]]
            let name = part
                .trim_start_matches("[[...")
                .trim_end_matches("]]");
            Segment::OptionalCatchAll(name.to_string())
        } else if part.starts_with("[...") && part.ends_with(']') {
            // Catch-all: [...path]
            let name = part
                .trim_start_matches("[...")
                .trim_end_matches(']');
            Segment::CatchAll(name.to_string())
        } else if part.starts_with('[') && part.ends_with(']') {
            // Dynamic: [slug]
            let name = part.trim_start_matches('[').trim_end_matches(']');
            Segment::Dynamic(name.to_string())
        } else if part.starts_with('(') && part.ends_with(')') {
            // Route group: (marketing) - skip in pattern
            continue;
        } else {
            // Static segment
            Segment::Static(part.to_string())
        };
        segments.push(segment);
    }

    Ok(segments)
}

/// Calculate route specificity (higher = more specific)
fn calculate_specificity(segments: &[Segment]) -> u32 {
    let mut score = 0u32;

    for (i, segment) in segments.iter().enumerate() {
        let position_weight = 1000u32.saturating_sub(i as u32 * 10);
        match segment {
            Segment::Static(_) => score += position_weight * 3,
            Segment::Dynamic(_) => score += position_weight * 2,
            Segment::CatchAll(_) => score += position_weight,
            Segment::OptionalCatchAll(_) => score += position_weight / 2,
        }
    }

    // Add bonus for more segments (longer paths are more specific)
    score += segments.len() as u32 * 100;

    score
}

/// Normalize a path (remove trailing slashes, ensure leading slash)
fn normalize_path(path: &str) -> String {
    let path = path.trim_matches('/');
    if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path)
    }
}

/// Try to match a route against path segments
fn try_match_route<'a>(route: &'a Route, path_segments: &[&str]) -> Option<RouteMatch<'a>> {
    let mut params = HashMap::new();
    let mut catch_all = None;
    let mut path_idx = 0;

    for segment in &route.segments {
        match segment {
            Segment::Static(expected) => {
                if path_idx >= path_segments.len() || path_segments[path_idx] != expected {
                    return None;
                }
                path_idx += 1;
            }
            Segment::Dynamic(name) => {
                if path_idx >= path_segments.len() {
                    return None;
                }
                params.insert(name.clone(), path_segments[path_idx].to_string());
                path_idx += 1;
            }
            Segment::CatchAll(name) => {
                if path_idx >= path_segments.len() {
                    return None; // Catch-all requires at least one segment
                }
                let remaining: Vec<String> = path_segments[path_idx..]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                params.insert(name.clone(), remaining.join("/"));
                catch_all = Some(remaining);
                path_idx = path_segments.len();
            }
            Segment::OptionalCatchAll(name) => {
                let remaining: Vec<String> = path_segments[path_idx..]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                if !remaining.is_empty() {
                    params.insert(name.clone(), remaining.join("/"));
                    catch_all = Some(remaining);
                }
                path_idx = path_segments.len();
            }
        }
    }

    // All path segments must be consumed (unless catch-all)
    if path_idx != path_segments.len() {
        return None;
    }

    Some(RouteMatch {
        route,
        params,
        catch_all,
    })
}

/// Scan a pages directory and generate routes
pub fn scan_pages_directory(pages_dir: &Path) -> RouterResult<Vec<Route>> {
    let mut routes = Vec::new();
    let mut layouts: HashMap<PathBuf, PathBuf> = HashMap::new();

    // First pass: find all layouts
    for entry in WalkDir::new(pages_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name == "_layout.lyt" {
                if let Some(parent) = path.parent() {
                    layouts.insert(parent.to_path_buf(), path.to_path_buf());
                }
            }
        }
    }

    // Second pass: process page files
    for entry in WalkDir::new(pages_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip directories and non-page files
        if !path.is_file() {
            continue;
        }

        let extension = path.extension().and_then(|e| e.to_str());
        if extension != Some("pg") {
            continue;
        }

        // Skip special files
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('_') {
                continue; // _layout.pg, _error.pg, _loading.pg
            }
        }

        // Convert file path to route pattern
        let pattern = file_path_to_pattern(pages_dir, path)?;
        let mut route = Route::new(&pattern, path.to_path_buf())?;

        // Find applicable layout
        let mut current = path.parent();
        while let Some(dir) = current {
            if let Some(layout) = layouts.get(dir) {
                route = route.with_layout(layout.clone());
                break;
            }
            if dir == pages_dir {
                break;
            }
            current = dir.parent();
        }

        routes.push(route);
    }

    Ok(routes)
}

/// Convert a file path to a route pattern
fn file_path_to_pattern(pages_dir: &Path, file_path: &Path) -> RouterResult<String> {
    let relative = file_path
        .strip_prefix(pages_dir)
        .map_err(|_| RouterError::InvalidPattern(file_path.display().to_string()))?;

    let mut parts = Vec::new();

    for component in relative.components() {
        if let std::path::Component::Normal(os_str) = component {
            let s = os_str.to_str().unwrap_or("");

            // Handle file extension
            let s = if s.ends_with(".pg") {
                &s[..s.len() - 3]
            } else {
                s
            };

            // Skip "index" - it maps to the parent directory
            if s == "index" {
                continue;
            }

            // Skip route groups (folders starting with parentheses)
            if s.starts_with('(') && s.ends_with(')') {
                continue;
            }

            parts.push(s.to_string());
        }
    }

    if parts.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", parts.join("/")))
    }
}

/// Link component data for client-side navigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkData {
    /// Target href
    pub href: String,
    /// Whether to prefetch on hover
    pub prefetch: bool,
    /// CSS class to add when active
    pub active_class: Option<String>,
}

impl LinkData {
    pub fn new(href: &str) -> Self {
        Self {
            href: href.to_string(),
            prefetch: true,
            active_class: None,
        }
    }

    pub fn with_prefetch(mut self, prefetch: bool) -> Self {
        self.prefetch = prefetch;
        self
    }

    pub fn with_active_class(mut self, class: &str) -> Self {
        self.active_class = Some(class.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_route() {
        let route = Route::new("/about", PathBuf::from("pages/about.pg")).unwrap();
        assert_eq!(route.pattern, "/about");
        assert_eq!(route.segments.len(), 1);
        assert!(matches!(&route.segments[0], Segment::Static(s) if s == "about"));
    }

    #[test]
    fn test_dynamic_route() {
        let route = Route::new("/blog/[slug]", PathBuf::from("pages/blog/[slug].pg")).unwrap();
        assert_eq!(route.segments.len(), 2);
        assert!(matches!(&route.segments[0], Segment::Static(s) if s == "blog"));
        assert!(matches!(&route.segments[1], Segment::Dynamic(s) if s == "slug"));
    }

    #[test]
    fn test_catch_all_route() {
        let route = Route::new("/docs/[...path]", PathBuf::from("pages/docs/[...path].pg")).unwrap();
        assert!(route.is_catch_all);
        assert!(matches!(&route.segments[1], Segment::CatchAll(s) if s == "path"));
    }

    #[test]
    fn test_router_match_static() {
        let router = Router::from_routes(vec![
            Route::new("/", PathBuf::from("pages/index.pg")).unwrap(),
            Route::new("/about", PathBuf::from("pages/about.pg")).unwrap(),
        ]);

        let matched = router.match_path("/about").unwrap();
        assert_eq!(matched.route.pattern, "/about");
        assert!(matched.params.is_empty());
    }

    #[test]
    fn test_router_match_dynamic() {
        let router = Router::from_routes(vec![
            Route::new("/blog/[slug]", PathBuf::from("pages/blog/[slug].pg")).unwrap(),
        ]);

        let matched = router.match_path("/blog/hello-world").unwrap();
        assert_eq!(matched.route.pattern, "/blog/[slug]");
        assert_eq!(matched.params.get("slug").unwrap(), "hello-world");
    }

    #[test]
    fn test_router_match_catch_all() {
        let router = Router::from_routes(vec![
            Route::new("/docs/[...path]", PathBuf::from("pages/docs/[...path].pg")).unwrap(),
        ]);

        let matched = router.match_path("/docs/guide/getting-started").unwrap();
        assert_eq!(matched.params.get("path").unwrap(), "guide/getting-started");
        assert_eq!(matched.catch_all.unwrap(), vec!["guide", "getting-started"]);
    }

    #[test]
    fn test_specificity_ordering() {
        let router = Router::from_routes(vec![
            Route::new("/blog/[slug]", PathBuf::from("pages/blog/[slug].pg")).unwrap(),
            Route::new("/blog/featured", PathBuf::from("pages/blog/featured.pg")).unwrap(),
        ]);

        // Static route should match before dynamic
        let matched = router.match_path("/blog/featured").unwrap();
        assert_eq!(matched.route.pattern, "/blog/featured");
    }

    #[test]
    fn test_file_path_to_pattern() {
        let pages_dir = Path::new("www/pages");

        assert_eq!(
            file_path_to_pattern(pages_dir, Path::new("www/pages/index.pg")).unwrap(),
            "/"
        );
        assert_eq!(
            file_path_to_pattern(pages_dir, Path::new("www/pages/about.pg")).unwrap(),
            "/about"
        );
        assert_eq!(
            file_path_to_pattern(pages_dir, Path::new("www/pages/blog/index.pg")).unwrap(),
            "/blog"
        );
        assert_eq!(
            file_path_to_pattern(pages_dir, Path::new("www/pages/blog/[slug].pg")).unwrap(),
            "/blog/[slug]"
        );
    }
}
