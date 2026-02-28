//! Property-based tests for string methods
//!
//! Feature: dx-py-production-ready-v2
//! Property 1: String Method Round-Trip Consistency
//! Property 2: String Case Conversion Idempotence
//! Property 3: String Search Consistency
//! Validates: Requirements 1.1-1.10

use proptest::prelude::*;

use dx_py_core::PyStr;

// ===== Generators for property tests =====

/// Generate a non-empty string without leading/trailing whitespace
fn arb_trimmed_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]{1,50}".prop_filter("non-empty trimmed string", |s| !s.is_empty())
}

/// Generate a string that may contain whitespace
fn arb_string_with_whitespace() -> impl Strategy<Value = String> {
    "[ \t]*[a-zA-Z0-9]+[ \t]*".prop_filter("string with optional whitespace", |s| !s.trim().is_empty())
}

/// Generate a simple separator (single character)
fn arb_separator() -> impl Strategy<Value = String> {
    prop::sample::select(vec![",", ";", ":", "|", "-", "_", " "])
        .prop_map(|s| s.to_string())
}

/// Generate a string with separators
fn arb_string_with_separator() -> impl Strategy<Value = (String, String)> {
    (arb_separator(), prop::collection::vec(arb_trimmed_string(), 1..5))
        .prop_map(|(sep, parts)| {
            let joined = parts.join(&sep);
            (joined, sep)
        })
}

/// Generate a prefix/suffix pair
fn arb_prefix_suffix() -> impl Strategy<Value = (String, String, String)> {
    (arb_trimmed_string(), arb_trimmed_string())
        .prop_map(|(prefix, suffix)| {
            let full = format!("{}{}", prefix, suffix);
            (full, prefix, suffix)
        })
}

// ===== Property Tests =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready-v2, Property 1: String Method Round-Trip Consistency
    /// For any string s and separator sep, calling sep.join(s.split(sep)) should produce
    /// a string equivalent to s (when s does not start or end with sep and has no consecutive sep occurrences).
    /// Validates: Requirements 1.3, 1.4, 1.6
    #[test]
    fn prop_split_join_round_trip((s, sep) in arb_string_with_separator()) {
        let py_s = PyStr::new(&s);
        let py_sep = PyStr::new(&sep);
        
        // Split the string
        let parts = py_s.split(Some(&py_sep));
        
        // Join it back
        let rejoined = py_sep.join(&parts);
        
        // Should be equal to original
        prop_assert_eq!(rejoined.as_str(), s.as_str(),
            "split then join should produce original string");
    }

    /// Feature: dx-py-production-ready-v2, Property 2: String Case Conversion Idempotence
    /// For any string s, calling s.upper().upper() should equal s.upper(),
    /// and s.lower().lower() should equal s.lower().
    /// Validates: Requirements 1.1, 1.2
    #[test]
    fn prop_upper_idempotence(s in arb_trimmed_string()) {
        let py_s = PyStr::new(&s);
        
        let upper_once = py_s.upper();
        let upper_twice = upper_once.upper();
        
        prop_assert_eq!(upper_once.as_str(), upper_twice.as_str(),
            "upper() should be idempotent");
    }

    /// Feature: dx-py-production-ready-v2, Property 2: String Case Conversion Idempotence
    /// For any string s, calling s.lower().lower() should equal s.lower().
    /// Validates: Requirements 1.1, 1.2
    #[test]
    fn prop_lower_idempotence(s in arb_trimmed_string()) {
        let py_s = PyStr::new(&s);
        
        let lower_once = py_s.lower();
        let lower_twice = lower_once.lower();
        
        prop_assert_eq!(lower_once.as_str(), lower_twice.as_str(),
            "lower() should be idempotent");
    }

    /// Feature: dx-py-production-ready-v2, Property 3: String Search Consistency
    /// For any string s and substring sub, if s.find(sub) returns index i >= 0,
    /// then s[i:i+len(sub)] should equal sub.
    /// Validates: Requirements 1.8, 1.9, 1.10
    #[test]
    fn prop_find_returns_correct_index((full, prefix, _suffix) in arb_prefix_suffix()) {
        let py_full = PyStr::new(&full);
        let py_prefix = PyStr::new(&prefix);
        
        let index = py_full.find_index(&py_prefix);
        
        // Since prefix is at the start, index should be 0
        prop_assert_eq!(index, 0,
            "find() should return 0 for prefix");
        
        // Verify the substring at that index matches
        let slice = py_full.slice(Some(0), Some(prefix.len() as i64));
        prop_assert_eq!(slice.as_str(), prefix.as_str(),
            "substring at found index should match search string");
    }

    /// Feature: dx-py-production-ready-v2, Property 3: String Search Consistency
    /// If s.startswith(sub) is True, then s.find(sub) should return 0.
    /// Validates: Requirements 1.8, 1.10
    #[test]
    fn prop_startswith_implies_find_zero((full, prefix, _suffix) in arb_prefix_suffix()) {
        let py_full = PyStr::new(&full);
        let py_prefix = PyStr::new(&prefix);
        
        if py_full.startswith(&py_prefix) {
            let index = py_full.find_index(&py_prefix);
            prop_assert_eq!(index, 0,
                "if startswith is true, find should return 0");
        }
    }

    /// Feature: dx-py-production-ready-v2, Property 3: String Search Consistency
    /// If s.endswith(sub) is True, then s.find(sub) should return a non-negative index.
    /// Validates: Requirements 1.9, 1.10
    #[test]
    fn prop_endswith_implies_find_nonnegative((full, _prefix, suffix) in arb_prefix_suffix()) {
        let py_full = PyStr::new(&full);
        let py_suffix = PyStr::new(&suffix);
        
        if py_full.endswith(&py_suffix) {
            let index = py_full.find_index(&py_suffix);
            prop_assert!(index >= 0,
                "if endswith is true, find should return non-negative index");
        }
    }

    /// Feature: dx-py-production-ready-v2, Property 1: String Method Round-Trip Consistency
    /// strip() should remove leading and trailing whitespace
    /// Validates: Requirements 1.7
    #[test]
    fn prop_strip_removes_whitespace(s in arb_string_with_whitespace()) {
        let py_s = PyStr::new(&s);
        let stripped = py_s.strip();
        
        // Stripped string should not start or end with whitespace
        let stripped_str = stripped.as_str();
        prop_assert!(!stripped_str.starts_with(' ') && !stripped_str.starts_with('\t'),
            "stripped string should not start with whitespace");
        prop_assert!(!stripped_str.ends_with(' ') && !stripped_str.ends_with('\t'),
            "stripped string should not end with whitespace");
    }

    /// Feature: dx-py-production-ready-v2, Property 2: String Case Conversion Idempotence
    /// strip() should be idempotent
    /// Validates: Requirements 1.7
    #[test]
    fn prop_strip_idempotence(s in arb_string_with_whitespace()) {
        let py_s = PyStr::new(&s);
        
        let strip_once = py_s.strip();
        let strip_twice = strip_once.strip();
        
        prop_assert_eq!(strip_once.as_str(), strip_twice.as_str(),
            "strip() should be idempotent");
    }

    /// Feature: dx-py-production-ready-v2, Property 1: String Method Round-Trip Consistency
    /// replace(old, new).replace(new, old) should restore original when old and new don't overlap
    /// Validates: Requirements 1.5
    #[test]
    fn prop_replace_round_trip(
        base in "[a-z]{5,20}",
        old in "[A-Z]{1,3}",
        new in "[0-9]{1,3}"
    ) {
        // Create a string with the old pattern
        let s = format!("{}{}{}",  &base[..2], old, &base[2..]);
        let py_s = PyStr::new(&s);
        let py_old = PyStr::new(&old);
        let py_new = PyStr::new(&new);
        
        // Replace old with new
        let replaced = py_s.replace(&py_old, &py_new);
        
        // Replace new back with old
        let restored = replaced.replace(&py_new, &py_old);
        
        prop_assert_eq!(restored.as_str(), s.as_str(),
            "replace round-trip should restore original");
    }

    /// Feature: dx-py-production-ready-v2, Property 3: String Search Consistency
    /// find() should return -1 for non-existent substrings
    /// Validates: Requirements 1.10
    #[test]
    fn prop_find_nonexistent_returns_minus_one(
        s in "[a-z]{5,20}",
        needle in "[A-Z]{3,5}"
    ) {
        let py_s = PyStr::new(&s);
        let py_needle = PyStr::new(&needle);
        
        // Since s is lowercase and needle is uppercase, needle shouldn't be found
        let index = py_s.find_index(&py_needle);
        
        prop_assert_eq!(index, -1,
            "find() should return -1 for non-existent substring");
    }

    /// Feature: dx-py-production-ready-v2, Property 2: String Case Conversion Idempotence
    /// upper() and lower() are inverses for ASCII letters
    /// Validates: Requirements 1.1, 1.2
    #[test]
    fn prop_case_conversion_consistency(s in "[a-zA-Z]{1,50}") {
        let py_s = PyStr::new(&s);
        
        // upper().lower() should equal lower()
        let upper_lower = py_s.upper().lower();
        let just_lower = py_s.lower();
        
        prop_assert_eq!(upper_lower.as_str(), just_lower.as_str(),
            "upper().lower() should equal lower()");
        
        // lower().upper() should equal upper()
        let lower_upper = py_s.lower().upper();
        let just_upper = py_s.upper();
        
        prop_assert_eq!(lower_upper.as_str(), just_upper.as_str(),
            "lower().upper() should equal upper()");
    }
}

// ===== Unit tests for edge cases =====

#[test]
fn test_split_empty_string() {
    let s = PyStr::new("");
    let sep = PyStr::new(",");
    let parts = s.split(Some(&sep));
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].as_str(), "");
}

#[test]
fn test_split_no_separator() {
    let s = PyStr::new("hello");
    let sep = PyStr::new(",");
    let parts = s.split(Some(&sep));
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].as_str(), "hello");
}

#[test]
fn test_split_whitespace() {
    let s = PyStr::new("  hello   world  ");
    let parts = s.split(None);
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].as_str(), "hello");
    assert_eq!(parts[1].as_str(), "world");
}

#[test]
fn test_join_empty_list() {
    let sep = PyStr::new(",");
    let empty: Vec<PyStr> = vec![];
    let result = sep.join(&empty);
    assert_eq!(result.as_str(), "");
}

#[test]
fn test_replace_count() {
    let s = PyStr::new("aaa");
    let old = PyStr::new("a");
    let new = PyStr::new("b");
    
    let result = s.replace_count(&old, &new, Some(2));
    assert_eq!(result.as_str(), "bba");
    
    let result_all = s.replace_count(&old, &new, None);
    assert_eq!(result_all.as_str(), "bbb");
}

#[test]
fn test_find_index_not_found() {
    let s = PyStr::new("hello");
    let needle = PyStr::new("xyz");
    assert_eq!(s.find_index(&needle), -1);
}

#[test]
fn test_startswith_empty_prefix() {
    let s = PyStr::new("hello");
    let empty = PyStr::new("");
    assert!(s.startswith(&empty));
}

#[test]
fn test_endswith_empty_suffix() {
    let s = PyStr::new("hello");
    let empty = PyStr::new("");
    assert!(s.endswith(&empty));
}
