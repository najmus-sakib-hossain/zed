//! Test pattern filtering
//!
//! Supports glob patterns and regex for filtering tests.

use glob::Pattern;
use regex::Regex;

/// Test filter supporting glob and regex patterns
pub struct TestFilter {
    /// Glob pattern (if valid)
    glob: Option<Pattern>,
    /// Regex pattern (if valid)
    regex: Option<Regex>,
    /// Original pattern string
    pattern: String,
    /// Whether this is a regex pattern (starts with ^)
    #[allow(dead_code)]
    is_regex: bool,
}

impl TestFilter {
    /// Create a new test filter from a pattern string
    ///
    /// Tries to parse as regex if it starts with ^, otherwise as glob.
    pub fn new(pattern: &str) -> Self {
        // If pattern starts with ^, treat as regex
        let is_regex = pattern.starts_with('^');

        let (glob, regex) = if is_regex {
            (None, Regex::new(pattern).ok())
        } else {
            (Pattern::new(pattern).ok(), None)
        };

        Self {
            glob,
            regex,
            pattern: pattern.to_string(),
            is_regex,
        }
    }

    /// Check if a test name matches the filter
    pub fn matches(&self, test_name: &str) -> bool {
        if let Some(ref glob) = self.glob {
            return glob.matches(test_name);
        }

        if let Some(ref regex) = self.regex {
            return regex.is_match(test_name);
        }

        // Fallback: substring match
        test_name.contains(&self.pattern)
    }

    /// Check if this is a "match all" filter
    #[allow(dead_code)]
    pub fn matches_all(&self) -> bool {
        self.pattern == "*" || self.pattern.is_empty()
    }

    /// Get the original pattern string
    #[allow(dead_code)]
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Filter a list of test names
    #[allow(dead_code)]
    pub fn filter_tests<'a>(&self, tests: &'a [String]) -> Vec<&'a String> {
        if self.matches_all() {
            return tests.iter().collect();
        }

        tests.iter().filter(|t| self.matches(t)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_pattern() {
        let filter = TestFilter::new("test_*");
        assert!(filter.matches("test_foo"));
        assert!(filter.matches("test_bar"));
        assert!(!filter.matches("foo_test"));
    }

    #[test]
    fn test_glob_suffix() {
        let filter = TestFilter::new("*_test");
        assert!(filter.matches("foo_test"));
        assert!(filter.matches("bar_test"));
        assert!(!filter.matches("test_foo"));
    }

    #[test]
    fn test_glob_contains() {
        let filter = TestFilter::new("*auth*");
        assert!(filter.matches("test_auth_login"));
        assert!(filter.matches("auth_test"));
        assert!(filter.matches("test_authentication"));
        assert!(!filter.matches("test_login"));
    }

    #[test]
    fn test_regex_pattern() {
        let filter = TestFilter::new("^test_[a-z]+$");
        assert!(filter.matches("test_foo"));
        assert!(filter.matches("test_bar"));
        assert!(!filter.matches("test_123"));
        assert!(!filter.matches("Test_foo"));
    }

    #[test]
    fn test_match_all() {
        let filter = TestFilter::new("*");
        assert!(filter.matches_all());
        assert!(filter.matches("anything"));
        assert!(filter.matches("test_foo"));
    }

    #[test]
    fn test_empty_pattern() {
        let filter = TestFilter::new("");
        assert!(filter.matches_all());
    }

    #[test]
    fn test_filter_tests() {
        let filter = TestFilter::new("test_*");
        let tests = vec![
            "test_foo".to_string(),
            "test_bar".to_string(),
            "foo_test".to_string(),
            "other".to_string(),
        ];

        let filtered = filter.filter_tests(&tests);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&&"test_foo".to_string()));
        assert!(filtered.contains(&&"test_bar".to_string()));
    }

    #[test]
    fn test_class_method_pattern() {
        let filter = TestFilter::new("TestAuth::*");
        assert!(filter.matches("TestAuth::test_login"));
        assert!(filter.matches("TestAuth::test_logout"));
        assert!(!filter.matches("TestUser::test_login"));
    }
}

// Property tests
#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Feature: dx-py-test-runner, Property 18: Test Pattern Filtering
    // Validates: Requirements 8.3
    //
    // For any test pattern and set of test cases, the filtered results
    // SHALL contain exactly those tests whose names match the pattern.
    proptest! {
        #[test]
        fn prop_pattern_filtering(
            prefix in "[a-z]{3,5}",
            tests in prop::collection::vec("[a-z_]{5,15}", 1..20),
        ) {
            let pattern = format!("{}*", prefix);
            let filter = TestFilter::new(&pattern);

            for test in &tests {
                let matches = filter.matches(test);
                let should_match = test.starts_with(&prefix);
                prop_assert_eq!(matches, should_match,
                    "Pattern '{}' vs test '{}': got {}, expected {}",
                    pattern, test, matches, should_match);
            }
        }

        #[test]
        fn prop_match_all_matches_everything(test_name in "[a-zA-Z_]{1,30}") {
            let filter = TestFilter::new("*");
            prop_assert!(filter.matches(&test_name));
        }

        #[test]
        fn prop_filter_preserves_matching_tests(
            tests in prop::collection::vec("test_[a-z]{3,10}", 1..10),
        ) {
            let filter = TestFilter::new("test_*");
            let filtered = filter.filter_tests(&tests);

            // All original tests should be in filtered (since they all match)
            prop_assert_eq!(filtered.len(), tests.len());
        }
    }
}
