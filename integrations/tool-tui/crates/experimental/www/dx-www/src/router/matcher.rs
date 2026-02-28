//! # Route Matcher
//!
//! Optimized route matching using a trie-based structure for fast lookups.

use std::collections::HashMap;

use super::Route;
use super::pattern::{RoutePattern, RouteSegment};

// =============================================================================
// Route Matcher
// =============================================================================

/// Optimized route matcher using a trie structure.
#[derive(Debug, Default)]
pub struct RouteMatcher {
    /// Root node of the trie
    root: TrieNode,
}

/// A node in the route trie.
#[derive(Debug, Default)]
struct TrieNode {
    /// Child nodes for static segments
    static_children: HashMap<String, Box<TrieNode>>,

    /// Child node for dynamic parameter
    param_child: Option<(String, Box<TrieNode>)>,

    /// Child node for catch-all
    catch_all_child: Option<(String, Box<TrieNode>)>,

    /// Route index at this node (if any)
    route_index: Option<usize>,
}

impl RouteMatcher {
    /// Create a new route matcher.
    pub fn new() -> Self {
        Self {
            root: TrieNode::default(),
        }
    }

    /// Build a matcher from a list of routes.
    pub fn from_routes(routes: &[Route]) -> Self {
        let mut matcher = Self::new();
        for (index, route) in routes.iter().enumerate() {
            matcher.insert(&route.pattern, index);
        }
        matcher
    }

    /// Insert a route pattern into the trie.
    pub fn insert(&mut self, pattern: &RoutePattern, route_index: usize) {
        let mut node = &mut self.root;

        for segment in &pattern.segments {
            node = match segment {
                RouteSegment::Static(s) => {
                    let s_lower = s.to_lowercase();
                    node.static_children
                        .entry(s_lower)
                        .or_insert_with(|| Box::new(TrieNode::default()))
                }
                RouteSegment::Param(name) => {
                    if node.param_child.is_none() {
                        node.param_child = Some((name.clone(), Box::new(TrieNode::default())));
                    }
                    &mut node.param_child.as_mut().unwrap().1
                }
                RouteSegment::CatchAll(name) => {
                    if node.catch_all_child.is_none() {
                        node.catch_all_child = Some((name.clone(), Box::new(TrieNode::default())));
                    }
                    &mut node.catch_all_child.as_mut().unwrap().1
                }
            };
        }

        node.route_index = Some(route_index);
    }

    /// Match a path and return the route index and extracted parameters.
    pub fn match_path(&self, path: &str) -> Option<(usize, HashMap<String, String>)> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut params = HashMap::new();

        self.match_segments(&self.root, &segments, 0, &mut params)
    }

    /// Recursively match path segments against the trie.
    fn match_segments(
        &self,
        node: &TrieNode,
        segments: &[&str],
        index: usize,
        params: &mut HashMap<String, String>,
    ) -> Option<(usize, HashMap<String, String>)> {
        // If we've consumed all segments, check if this node has a route
        if index >= segments.len() {
            return node.route_index.map(|idx| (idx, params.clone()));
        }

        let segment = segments[index];

        // Try static match first (most specific)
        if let Some(child) = node.static_children.get(&segment.to_lowercase()) {
            if let Some(result) = self.match_segments(child, segments, index + 1, params) {
                return Some(result);
            }
        }

        // Try parameter match
        if let Some((name, child)) = &node.param_child {
            params.insert(name.clone(), segment.to_string());
            if let Some(result) = self.match_segments(child, segments, index + 1, params) {
                return Some(result);
            }
            params.remove(name);
        }

        // Try catch-all match
        if let Some((name, child)) = &node.catch_all_child {
            let remaining: Vec<&str> = segments[index..].to_vec();
            params.insert(name.clone(), remaining.join("/"));
            if let result @ Some(_) = child.route_index.map(|idx| (idx, params.clone())) {
                return result;
            }
            params.remove(name);
        }

        None
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_matching() {
        let mut matcher = RouteMatcher::new();
        matcher.insert(&RoutePattern::parse("/about"), 0);
        matcher.insert(&RoutePattern::parse("/contact"), 1);

        let result = matcher.match_path("/about");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, 0);

        let result = matcher.match_path("/contact");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, 1);

        let result = matcher.match_path("/other");
        assert!(result.is_none());
    }

    #[test]
    fn test_dynamic_matching() {
        let mut matcher = RouteMatcher::new();
        matcher.insert(&RoutePattern::parse("/user/:id"), 0);

        let result = matcher.match_path("/user/123");
        assert!(result.is_some());
        let (index, params) = result.unwrap();
        assert_eq!(index, 0);
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_catch_all_matching() {
        let mut matcher = RouteMatcher::new();
        matcher.insert(&RoutePattern::parse("/docs/*slug"), 0);

        let result = matcher.match_path("/docs/getting-started/intro");
        assert!(result.is_some());
        let (index, params) = result.unwrap();
        assert_eq!(index, 0);
        assert_eq!(params.get("slug"), Some(&"getting-started/intro".to_string()));
    }

    #[test]
    fn test_priority_static_over_dynamic() {
        let mut matcher = RouteMatcher::new();
        matcher.insert(&RoutePattern::parse("/user/:id"), 0);
        matcher.insert(&RoutePattern::parse("/user/me"), 1);

        // Static should match over dynamic
        let result = matcher.match_path("/user/me");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, 1);

        // Dynamic should still work for other values
        let result = matcher.match_path("/user/123");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, 0);
    }

    #[test]
    fn test_nested_routes() {
        let mut matcher = RouteMatcher::new();
        matcher.insert(&RoutePattern::parse("/api/users"), 0);
        matcher.insert(&RoutePattern::parse("/api/users/:id"), 1);
        matcher.insert(&RoutePattern::parse("/api/users/:id/posts"), 2);

        let result = matcher.match_path("/api/users");
        assert_eq!(result.unwrap().0, 0);

        let result = matcher.match_path("/api/users/123");
        assert_eq!(result.unwrap().0, 1);

        let result = matcher.match_path("/api/users/123/posts");
        assert_eq!(result.unwrap().0, 2);
    }
}
