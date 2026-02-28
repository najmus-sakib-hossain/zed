//! Failure categorization for test results

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// Categories of test failures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureCategory {
    /// Failed to load a C extension module
    CExtensionLoad,
    /// Called an unimplemented CPython API function
    MissingApi,
    /// Async/await related issues
    AsyncBehavior,
    /// Module import failed
    ImportError,
    /// General runtime error
    RuntimeError,
    /// Type conversion or mismatch issue
    TypeMismatch,
    /// Memory management issue
    MemoryError,
    /// Assertion failure in test
    AssertionError,
    /// Timeout during test execution
    Timeout,
    /// Unknown/uncategorized failure
    Unknown,
}

impl FailureCategory {
    /// Get a human-readable description of the category
    pub fn description(&self) -> &'static str {
        match self {
            Self::CExtensionLoad => "C Extension Loading Failure",
            Self::MissingApi => "Missing CPython API Function",
            Self::AsyncBehavior => "Async/Await Behavior Issue",
            Self::ImportError => "Module Import Error",
            Self::RuntimeError => "Runtime Error",
            Self::TypeMismatch => "Type Mismatch",
            Self::MemoryError => "Memory Management Error",
            Self::AssertionError => "Test Assertion Failure",
            Self::Timeout => "Test Timeout",
            Self::Unknown => "Unknown Error",
        }
    }

    /// Check if this category indicates a DX-Py compatibility issue
    pub fn is_compatibility_issue(&self) -> bool {
        matches!(
            self,
            Self::CExtensionLoad | Self::MissingApi | Self::AsyncBehavior | Self::TypeMismatch
        )
    }
}

/// A single test failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    /// Name of the failed test
    pub test_name: String,
    /// Error message
    pub error_message: String,
    /// Full traceback if available
    pub traceback: Option<String>,
}

impl TestFailure {
    /// Create a new test failure
    pub fn new(test_name: impl Into<String>, error_message: impl Into<String>) -> Self {
        Self {
            test_name: test_name.into(),
            error_message: error_message.into(),
            traceback: None,
        }
    }

    /// Add traceback to the failure
    pub fn with_traceback(mut self, traceback: impl Into<String>) -> Self {
        self.traceback = Some(traceback.into());
        self
    }
}

// Regex patterns for categorization
static C_EXTENSION_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)cannot load.*\.pyd").unwrap(),
        Regex::new(r"(?i)cannot load.*\.so").unwrap(),
        Regex::new(r"(?i)ImportError:.*\.pyd").unwrap(),
        Regex::new(r"(?i)ImportError:.*\.so").unwrap(),
        Regex::new(r"(?i)failed to load.*extension").unwrap(),
        Regex::new(r"(?i)DLL load failed").unwrap(),
        Regex::new(r"(?i)undefined symbol").unwrap(),
        Regex::new(r"(?i)ABI.*mismatch").unwrap(),
    ]
});

static MISSING_API_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)unimplemented.*API").unwrap(),
        Regex::new(r"(?i)missing.*function.*Py").unwrap(),
        Regex::new(r"(?i)NotImplementedError.*CPython").unwrap(),
        Regex::new(r"(?i)Py[A-Z][a-zA-Z_]+.*not implemented").unwrap(),
    ]
});

static ASYNC_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)RuntimeError:.*event loop").unwrap(),
        Regex::new(r"(?i)asyncio.*error").unwrap(),
        Regex::new(r"(?i)coroutine.*never awaited").unwrap(),
        Regex::new(r"(?i)await.*outside.*async").unwrap(),
        Regex::new(r"(?i)Task.*cancelled").unwrap(),
    ]
});

static IMPORT_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)ImportError:").unwrap(),
        Regex::new(r"(?i)ModuleNotFoundError:").unwrap(),
        Regex::new(r"(?i)No module named").unwrap(),
    ]
});

static TYPE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)TypeError:").unwrap(),
        Regex::new(r"(?i)type.*mismatch").unwrap(),
        Regex::new(r"(?i)expected.*got").unwrap(),
        Regex::new(r"(?i)cannot convert").unwrap(),
    ]
});

static MEMORY_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)MemoryError").unwrap(),
        Regex::new(r"(?i)out of memory").unwrap(),
        Regex::new(r"(?i)segmentation fault").unwrap(),
        Regex::new(r"(?i)SIGSEGV").unwrap(),
        Regex::new(r"(?i)double free").unwrap(),
        Regex::new(r"(?i)use after free").unwrap(),
    ]
});

static ASSERTION_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)AssertionError").unwrap(),
        Regex::new(r"(?i)assert.*failed").unwrap(),
    ]
});

static TIMEOUT_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)timeout").unwrap(),
        Regex::new(r"(?i)timed out").unwrap(),
        Regex::new(r"(?i)deadline exceeded").unwrap(),
    ]
});

/// Categorizes test failures by analyzing error messages and tracebacks
pub struct FailureCategorizer {
    // Could add custom patterns here in the future
}

impl FailureCategorizer {
    /// Create a new failure categorizer
    pub fn new() -> Self {
        Self {}
    }

    /// Categorize a test failure
    pub fn categorize(&self, failure: &TestFailure) -> FailureCategory {
        let text = self.get_searchable_text(failure);

        // Check patterns in order of specificity
        if self.matches_any(&text, &C_EXTENSION_PATTERNS) {
            return FailureCategory::CExtensionLoad;
        }

        if self.matches_any(&text, &MISSING_API_PATTERNS) {
            return FailureCategory::MissingApi;
        }

        if self.matches_any(&text, &ASYNC_PATTERNS) {
            return FailureCategory::AsyncBehavior;
        }

        if self.matches_any(&text, &MEMORY_PATTERNS) {
            return FailureCategory::MemoryError;
        }

        if self.matches_any(&text, &TIMEOUT_PATTERNS) {
            return FailureCategory::Timeout;
        }

        // Import errors that aren't C extension related
        if self.matches_any(&text, &IMPORT_PATTERNS) {
            return FailureCategory::ImportError;
        }

        // Check assertion errors BEFORE type errors (AssertionError contains "Error")
        if self.matches_any(&text, &ASSERTION_PATTERNS) {
            return FailureCategory::AssertionError;
        }

        if self.matches_any(&text, &TYPE_PATTERNS) {
            return FailureCategory::TypeMismatch;
        }

        // Check for generic runtime errors
        if text.contains("RuntimeError") || text.contains("Exception") {
            return FailureCategory::RuntimeError;
        }

        FailureCategory::Unknown
    }

    /// Categorize multiple failures and return counts by category
    pub fn categorize_all(&self, failures: &[TestFailure]) -> Vec<(FailureCategory, usize)> {
        use std::collections::HashMap;

        let mut counts: HashMap<FailureCategory, usize> = HashMap::new();

        for failure in failures {
            let category = self.categorize(failure);
            *counts.entry(category).or_insert(0) += 1;
        }

        let mut result: Vec<_> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending
        result
    }

    /// Get combined text to search for patterns
    fn get_searchable_text(&self, failure: &TestFailure) -> String {
        let mut text = failure.error_message.clone();
        if let Some(ref tb) = failure.traceback {
            text.push('\n');
            text.push_str(tb);
        }
        text
    }

    /// Check if text matches any of the patterns
    fn matches_any(&self, text: &str, patterns: &[Regex]) -> bool {
        patterns.iter().any(|p| p.is_match(text))
    }
}

impl Default for FailureCategorizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_c_extension() {
        let categorizer = FailureCategorizer::new();

        let failure = TestFailure::new(
            "test_numpy",
            "ImportError: cannot load numpy/_core/_multiarray_umath.pyd",
        );

        assert_eq!(categorizer.categorize(&failure), FailureCategory::CExtensionLoad);
    }

    #[test]
    fn test_categorize_missing_api() {
        let categorizer = FailureCategorizer::new();

        let failure = TestFailure::new(
            "test_extension",
            "NotImplementedError: CPython API PyArray_NewFromDescr not implemented",
        );

        assert_eq!(categorizer.categorize(&failure), FailureCategory::MissingApi);
    }

    #[test]
    fn test_categorize_async() {
        let categorizer = FailureCategorizer::new();

        let failure =
            TestFailure::new("test_async", "RuntimeError: This event loop is already running");

        assert_eq!(categorizer.categorize(&failure), FailureCategory::AsyncBehavior);
    }

    #[test]
    fn test_categorize_import() {
        let categorizer = FailureCategorizer::new();

        let failure =
            TestFailure::new("test_import", "ModuleNotFoundError: No module named 'nonexistent'");

        assert_eq!(categorizer.categorize(&failure), FailureCategory::ImportError);
    }

    #[test]
    fn test_categorize_assertion() {
        let categorizer = FailureCategorizer::new();

        let failure = TestFailure::new("test_values", "AssertionError: Expected 5, got 3");

        assert_eq!(categorizer.categorize(&failure), FailureCategory::AssertionError);
    }

    #[test]
    fn test_categorize_unknown() {
        let categorizer = FailureCategorizer::new();

        let failure = TestFailure::new("test_something", "Some random error message");

        assert_eq!(categorizer.categorize(&failure), FailureCategory::Unknown);
    }

    #[test]
    fn test_is_compatibility_issue() {
        assert!(FailureCategory::CExtensionLoad.is_compatibility_issue());
        assert!(FailureCategory::MissingApi.is_compatibility_issue());
        assert!(!FailureCategory::AssertionError.is_compatibility_issue());
        assert!(!FailureCategory::Unknown.is_compatibility_issue());
    }

    #[test]
    fn test_categorize_all() {
        let categorizer = FailureCategorizer::new();

        let failures = vec![
            TestFailure::new("test1", "AssertionError: failed"),
            TestFailure::new("test2", "AssertionError: also failed"),
            TestFailure::new("test3", "ImportError: no module"),
        ];

        let counts = categorizer.categorize_all(&failures);

        assert_eq!(counts.len(), 2);
        assert_eq!(counts[0], (FailureCategory::AssertionError, 2));
        assert_eq!(counts[1], (FailureCategory::ImportError, 1));
    }
}
