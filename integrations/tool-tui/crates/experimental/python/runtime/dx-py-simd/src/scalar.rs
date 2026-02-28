//! Scalar (non-SIMD) string engine implementation
//!
//! This provides a fallback implementation that produces identical results
//! to the SIMD implementations, ensuring correctness across all platforms.

use crate::engine::SimdStringEngine;

/// Scalar string engine - fallback for CPUs without SIMD support
pub struct ScalarStringEngine;

impl ScalarStringEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ScalarStringEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SimdStringEngine for ScalarStringEngine {
    fn find(&self, haystack: &str, needle: &str) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        if needle.len() > haystack.len() {
            return None;
        }

        haystack.find(needle)
    }

    fn count(&self, haystack: &str, needle: &str) -> usize {
        if needle.is_empty() {
            return haystack.len() + 1;
        }

        haystack.matches(needle).count()
    }

    fn eq(&self, a: &str, b: &str) -> bool {
        a == b
    }

    fn to_lowercase(&self, s: &str) -> String {
        // Fast path for ASCII-only strings
        if s.is_ascii() {
            s.to_ascii_lowercase()
        } else {
            s.to_lowercase()
        }
    }

    fn to_uppercase(&self, s: &str) -> String {
        // Fast path for ASCII-only strings
        if s.is_ascii() {
            s.to_ascii_uppercase()
        } else {
            s.to_uppercase()
        }
    }

    fn split<'a>(&self, s: &'a str, delimiter: &str) -> Vec<&'a str> {
        if delimiter.is_empty() {
            // Split into individual characters
            s.char_indices().map(|(i, c)| &s[i..i + c.len_utf8()]).collect()
        } else {
            s.split(delimiter).collect()
        }
    }

    fn join(&self, parts: &[&str], separator: &str) -> String {
        parts.join(separator)
    }

    fn replace(&self, s: &str, from: &str, to: &str) -> String {
        if from.is_empty() {
            return s.to_string();
        }
        s.replace(from, to)
    }

    fn name(&self) -> &'static str {
        "Scalar"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find() {
        let engine = ScalarStringEngine::new();

        assert_eq!(engine.find("hello world", "world"), Some(6));
        assert_eq!(engine.find("hello world", "foo"), None);
        assert_eq!(engine.find("hello", ""), Some(0));
        assert_eq!(engine.find("", "hello"), None);
    }

    #[test]
    fn test_count() {
        let engine = ScalarStringEngine::new();

        assert_eq!(engine.count("hello hello hello", "hello"), 3);
        assert_eq!(engine.count("aaa", "a"), 3);
        assert_eq!(engine.count("aaa", "aa"), 1); // Non-overlapping
        assert_eq!(engine.count("hello", "x"), 0);
    }

    #[test]
    fn test_eq() {
        let engine = ScalarStringEngine::new();

        assert!(engine.eq("hello", "hello"));
        assert!(!engine.eq("hello", "world"));
        assert!(!engine.eq("hello", "hell"));
        assert!(engine.eq("", ""));
    }

    #[test]
    fn test_to_lowercase() {
        let engine = ScalarStringEngine::new();

        assert_eq!(engine.to_lowercase("HELLO"), "hello");
        assert_eq!(engine.to_lowercase("Hello World"), "hello world");
        assert_eq!(engine.to_lowercase("hello"), "hello");
        assert_eq!(engine.to_lowercase("123"), "123");
    }

    #[test]
    fn test_to_uppercase() {
        let engine = ScalarStringEngine::new();

        assert_eq!(engine.to_uppercase("hello"), "HELLO");
        assert_eq!(engine.to_uppercase("Hello World"), "HELLO WORLD");
        assert_eq!(engine.to_uppercase("HELLO"), "HELLO");
        assert_eq!(engine.to_uppercase("123"), "123");
    }

    #[test]
    fn test_split() {
        let engine = ScalarStringEngine::new();

        assert_eq!(engine.split("a,b,c", ","), vec!["a", "b", "c"]);
        assert_eq!(engine.split("hello", ","), vec!["hello"]);
        assert_eq!(engine.split("a::b::c", "::"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_join() {
        let engine = ScalarStringEngine::new();

        assert_eq!(engine.join(&["a", "b", "c"], ","), "a,b,c");
        assert_eq!(engine.join(&["hello"], ","), "hello");
        assert_eq!(engine.join(&[], ","), "");
    }

    #[test]
    fn test_replace() {
        let engine = ScalarStringEngine::new();

        assert_eq!(engine.replace("hello world", "world", "rust"), "hello rust");
        assert_eq!(engine.replace("aaa", "a", "b"), "bbb");
        assert_eq!(engine.replace("hello", "x", "y"), "hello");
    }
}
