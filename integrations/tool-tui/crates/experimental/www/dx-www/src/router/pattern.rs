//! # Route Pattern Matching
//!
//! This module implements route pattern parsing and matching for dynamic routes.

use std::collections::HashMap;

// =============================================================================
// Route Pattern
// =============================================================================

/// A parsed route pattern that can match URL paths.
#[derive(Debug, Clone)]
pub struct RoutePattern {
    /// The original pattern string
    pub pattern: String,

    /// Parsed segments
    pub segments: Vec<RouteSegment>,

    /// Whether this is a catch-all pattern
    pub is_catch_all: bool,
}

/// A segment of a route pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum RouteSegment {
    /// Static segment (exact match)
    Static(String),

    /// Dynamic parameter segment (:param)
    Param(String),

    /// Catch-all segment (*param)
    CatchAll(String),
}

impl RoutePattern {
    /// Parse a route pattern string.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pattern = RoutePattern::parse("/user/:id");
    /// let pattern = RoutePattern::parse("/docs/*slug");
    /// ```
    pub fn parse(pattern: &str) -> Self {
        let mut segments = Vec::new();
        let mut is_catch_all = false;

        for part in pattern.split('/').filter(|s| !s.is_empty()) {
            let segment = if part.starts_with(':') {
                RouteSegment::Param(part[1..].to_string())
            } else if part.starts_with('*') {
                is_catch_all = true;
                RouteSegment::CatchAll(part[1..].to_string())
            } else {
                RouteSegment::Static(part.to_string())
            };
            segments.push(segment);
        }

        Self {
            pattern: pattern.to_string(),
            segments,
            is_catch_all,
        }
    }

    /// Match a URL path against this pattern.
    ///
    /// Returns the extracted parameters if the path matches, None otherwise.
    pub fn match_path(&self, path: &str) -> Option<HashMap<String, String>> {
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut params = HashMap::new();
        let mut path_idx = 0;

        for segment in &self.segments {
            match segment {
                RouteSegment::Static(expected) => {
                    if path_idx >= path_parts.len() {
                        return None;
                    }
                    if path_parts[path_idx].to_lowercase() != expected.to_lowercase() {
                        return None;
                    }
                    path_idx += 1;
                }
                RouteSegment::Param(name) => {
                    if path_idx >= path_parts.len() {
                        return None;
                    }
                    params.insert(name.clone(), path_parts[path_idx].to_string());
                    path_idx += 1;
                }
                RouteSegment::CatchAll(name) => {
                    // Catch-all consumes the rest of the path
                    let remaining: Vec<&str> = path_parts[path_idx..].to_vec();
                    params.insert(name.clone(), remaining.join("/"));
                    return Some(params);
                }
            }
        }

        // Check if we consumed all path parts (unless catch-all)
        if !self.is_catch_all && path_idx != path_parts.len() {
            return None;
        }

        Some(params)
    }

    /// Check if this pattern is static (no dynamic segments).
    pub fn is_static(&self) -> bool {
        self.segments.iter().all(|s| matches!(s, RouteSegment::Static(_)))
    }

    /// Get the parameter names in this pattern.
    pub fn param_names(&self) -> Vec<&str> {
        self.segments
            .iter()
            .filter_map(|s| match s {
                RouteSegment::Param(name) | RouteSegment::CatchAll(name) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Calculate the specificity of this pattern (more specific = higher score).
    ///
    /// Used for ordering routes when multiple could match.
    /// Static segments are more specific than dynamic ones, and we also
    /// penalize longer paths to prefer shorter exact matches.
    pub fn specificity(&self) -> u32 {
        let mut static_count = 0u32;
        let mut param_count = 0u32;
        let mut has_catch_all = false;

        for segment in &self.segments {
            match segment {
                RouteSegment::Static(_) => static_count += 1,
                RouteSegment::Param(_) => param_count += 1,
                RouteSegment::CatchAll(_) => has_catch_all = true,
            }
        }

        // Higher score = more specific
        // - Catch-all routes are least specific (score starts at 0)
        // - Dynamic segments reduce specificity
        // - Static segments increase specificity
        // Formula: prioritize static-only routes, then penalize dynamic segments
        if has_catch_all {
            // Catch-all is least specific
            1
        } else if param_count > 0 {
            // Routes with params are less specific than pure static
            // But more static segments = more specific within dynamic routes
            100 + static_count * 10 - param_count
        } else {
            // Pure static routes are most specific
            // Shorter static paths are actually more specific for exact matching
            1000 + static_count * 10
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_static_pattern() {
        let pattern = RoutePattern::parse("/about");
        assert_eq!(pattern.segments.len(), 1);
        assert!(matches!(&pattern.segments[0], RouteSegment::Static(s) if s == "about"));
        assert!(pattern.is_static());
    }

    #[test]
    fn test_parse_dynamic_pattern() {
        let pattern = RoutePattern::parse("/user/:id");
        assert_eq!(pattern.segments.len(), 2);
        assert!(matches!(&pattern.segments[0], RouteSegment::Static(s) if s == "user"));
        assert!(matches!(&pattern.segments[1], RouteSegment::Param(s) if s == "id"));
        assert!(!pattern.is_static());
    }

    #[test]
    fn test_parse_catch_all_pattern() {
        let pattern = RoutePattern::parse("/docs/*slug");
        assert_eq!(pattern.segments.len(), 2);
        assert!(matches!(&pattern.segments[0], RouteSegment::Static(s) if s == "docs"));
        assert!(matches!(&pattern.segments[1], RouteSegment::CatchAll(s) if s == "slug"));
        assert!(pattern.is_catch_all);
    }

    #[test]
    fn test_match_static_path() {
        let pattern = RoutePattern::parse("/about");
        assert!(pattern.match_path("/about").is_some());
        assert!(pattern.match_path("/other").is_none());
    }

    #[test]
    fn test_match_dynamic_path() {
        let pattern = RoutePattern::parse("/user/:id");

        let result = pattern.match_path("/user/123");
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));

        assert!(pattern.match_path("/user").is_none());
        assert!(pattern.match_path("/user/123/extra").is_none());
    }

    #[test]
    fn test_match_catch_all_path() {
        let pattern = RoutePattern::parse("/docs/*slug");

        let result = pattern.match_path("/docs/getting-started/intro");
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("slug"), Some(&"getting-started/intro".to_string()));

        let result = pattern.match_path("/docs");
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("slug"), Some(&"".to_string()));
    }

    #[test]
    fn test_multiple_params() {
        let pattern = RoutePattern::parse("/user/:userId/post/:postId");

        let result = pattern.match_path("/user/42/post/123");
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("userId"), Some(&"42".to_string()));
        assert_eq!(params.get("postId"), Some(&"123".to_string()));
    }

    #[test]
    fn test_param_names() {
        let pattern = RoutePattern::parse("/user/:id/post/:postId");
        let names = pattern.param_names();
        assert_eq!(names, vec!["id", "postId"]);
    }

    #[test]
    fn test_specificity() {
        let static_pattern = RoutePattern::parse("/about");
        let dynamic_pattern = RoutePattern::parse("/user/:id");
        let catch_all_pattern = RoutePattern::parse("/docs/*slug");

        assert!(static_pattern.specificity() > dynamic_pattern.specificity());
        assert!(dynamic_pattern.specificity() > catch_all_pattern.specificity());
    }
}
