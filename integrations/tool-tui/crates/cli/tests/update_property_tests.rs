//! Property-based tests for Update Checker
//!
//! These tests verify universal properties for the update checking mechanism,
//! including caching behavior and version comparison.
//!
//! Feature: cli-production-ready, Property 4: Update Check Caching
//! **Validates: Requirements 9.5**
//!
//! Run with: cargo test --test update_property_tests

use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

// ============================================================================
// Test Structures (mirrors update.rs)
// ============================================================================

const CACHE_DURATION: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateCache {
    checked_at: SystemTime,
    latest_version: String,
    release_url: Option<String>,
}

/// Simulated update checker for testing
struct TestUpdateChecker {
    cache_path: PathBuf,
    network_call_count: std::cell::RefCell<u32>,
}

impl TestUpdateChecker {
    fn new(cache_path: PathBuf) -> Self {
        Self {
            cache_path,
            network_call_count: std::cell::RefCell::new(0),
        }
    }

    fn read_cache(&self) -> Option<UpdateCache> {
        if !self.cache_path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&self.cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn write_cache(&self, version: &str, release_url: Option<String>) {
        if let Some(parent) = self.cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let cache = UpdateCache {
            checked_at: SystemTime::now(),
            latest_version: version.to_string(),
            release_url,
        };
        let content = serde_json::to_string_pretty(&cache).unwrap();
        let _ = std::fs::write(&self.cache_path, content);
    }

    /// Simulate a check that uses cache if valid
    fn check(&self, simulated_latest: &str) -> (String, bool) {
        // Check cache first
        if let Some(cached) = self.read_cache()
            && let Ok(elapsed) = cached.checked_at.elapsed()
            && elapsed < CACHE_DURATION
        {
            // Cache hit - no network call
            return (cached.latest_version, false);
        }

        // Cache miss - simulate network call
        *self.network_call_count.borrow_mut() += 1;
        self.write_cache(simulated_latest, None);
        (simulated_latest.to_string(), true)
    }

    fn network_calls(&self) -> u32 {
        *self.network_call_count.borrow()
    }
}

/// Version comparison logic
fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = v.trim_start_matches('v').split('.').collect();
        if parts.len() >= 3 {
            Some((
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                parts[2].split('-').next()?.parse().ok()?,
            ))
        } else {
            None
        }
    };

    match (parse_version(latest), parse_version(current)) {
        (Some((l_major, l_minor, l_patch)), Some((c_major, c_minor, c_patch))) => {
            (l_major, l_minor, l_patch) > (c_major, c_minor, c_patch)
        }
        _ => latest != current,
    }
}

// ============================================================================
// Arbitrary Generators
// ============================================================================

fn arbitrary_version() -> impl Strategy<Value = String> {
    (0u32..100, 0u32..100, 0u32..100)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

fn arbitrary_version_with_prefix() -> impl Strategy<Value = String> {
    prop_oneof![
        arbitrary_version(),
        arbitrary_version().prop_map(|v| format!("v{}", v)),
    ]
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 4: Update Check Caching
    /// *For any* sequence of update check calls within a 24-hour window,
    /// only the first call SHALL make a network request; subsequent calls
    /// SHALL return the cached result.
    ///
    /// **Validates: Requirements 9.5**
    #[test]
    fn prop_cache_prevents_repeated_network_calls(
        latest_version in arbitrary_version(),
        num_checks in 2u32..10,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("update-cache.json");
        let checker = TestUpdateChecker::new(cache_path);

        // Perform multiple checks
        for _ in 0..num_checks {
            let _ = checker.check(&latest_version);
        }

        // Only one network call should have been made
        prop_assert_eq!(
            checker.network_calls(), 1,
            "Expected 1 network call, got {}",
            checker.network_calls()
        );
    }

    /// Property 4b: Cache returns consistent version
    /// *For any* cached version, subsequent checks within the cache window
    /// SHALL return the same version.
    ///
    /// **Validates: Requirements 9.5**
    #[test]
    fn prop_cache_returns_consistent_version(
        cached_version in arbitrary_version(),
        different_version in arbitrary_version(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("update-cache.json");
        let checker = TestUpdateChecker::new(cache_path);

        // First check caches the version
        let (first_result, first_was_network) = checker.check(&cached_version);
        prop_assert!(first_was_network, "First check should make network call");
        prop_assert_eq!(first_result, cached_version.clone());

        // Second check should return cached version even if "network" would return different
        let (second_result, second_was_network) = checker.check(&different_version);
        prop_assert!(!second_was_network, "Second check should use cache");
        prop_assert_eq!(second_result, cached_version,
            "Cached version should be returned, not new version");
    }

    /// Property: Version comparison is transitive
    /// *For any* three versions a, b, c where a > b and b > c, then a > c
    ///
    /// **Validates: Requirements 9.2**
    #[test]
    fn prop_version_comparison_transitive(
        major in 1u32..50,
        minor in 0u32..50,
        patch in 0u32..50,
    ) {
        let v1 = format!("{}.{}.{}", major + 2, minor, patch);
        let v2 = format!("{}.{}.{}", major + 1, minor, patch);
        let v3 = format!("{}.{}.{}", major, minor, patch);

        // v1 > v2 > v3
        prop_assert!(is_newer_version(&v1, &v2), "{} should be newer than {}", v1, v2);
        prop_assert!(is_newer_version(&v2, &v3), "{} should be newer than {}", v2, v3);
        prop_assert!(is_newer_version(&v1, &v3), "{} should be newer than {}", v1, v3);
    }

    /// Property: Version comparison handles v prefix
    /// *For any* version, comparing with or without 'v' prefix should be equivalent
    ///
    /// **Validates: Requirements 9.2**
    #[test]
    fn prop_version_prefix_handling(
        major in 0u32..100,
        minor in 0u32..100,
        patch in 0u32..100,
    ) {
        let v_without = format!("{}.{}.{}", major, minor, patch);
        let v_with = format!("v{}.{}.{}", major, minor, patch);

        // Same version with/without prefix should not be "newer"
        prop_assert!(!is_newer_version(&v_without, &v_with));
        prop_assert!(!is_newer_version(&v_with, &v_without));
    }

    /// Property: Same version is not newer
    /// *For any* version, it should not be considered newer than itself
    ///
    /// **Validates: Requirements 9.3**
    #[test]
    fn prop_same_version_not_newer(
        version in arbitrary_version_with_prefix(),
    ) {
        prop_assert!(
            !is_newer_version(&version, &version),
            "Version {} should not be newer than itself",
            version
        );
    }
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

#[test]
fn test_cache_file_creation() {
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("subdir").join("update-cache.json");
    let checker = TestUpdateChecker::new(cache_path.clone());

    checker.check("1.0.0");

    assert!(cache_path.exists(), "Cache file should be created");
}

#[test]
fn test_cache_content_valid_json() {
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("update-cache.json");
    let checker = TestUpdateChecker::new(cache_path.clone());

    checker.check("2.0.0");

    let content = std::fs::read_to_string(&cache_path).unwrap();
    let cache: UpdateCache = serde_json::from_str(&content).unwrap();
    assert_eq!(cache.latest_version, "2.0.0");
}

#[test]
fn test_missing_cache_triggers_network() {
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("nonexistent").join("cache.json");
    let checker = TestUpdateChecker::new(cache_path);

    let (_, was_network) = checker.check("1.0.0");
    assert!(was_network, "Missing cache should trigger network call");
}

#[test]
fn test_version_comparison_major() {
    assert!(is_newer_version("2.0.0", "1.0.0"));
    assert!(is_newer_version("10.0.0", "9.0.0"));
    assert!(!is_newer_version("1.0.0", "2.0.0"));
}

#[test]
fn test_version_comparison_minor() {
    assert!(is_newer_version("1.1.0", "1.0.0"));
    assert!(is_newer_version("1.10.0", "1.9.0"));
    assert!(!is_newer_version("1.0.0", "1.1.0"));
}

#[test]
fn test_version_comparison_patch() {
    assert!(is_newer_version("1.0.1", "1.0.0"));
    assert!(is_newer_version("1.0.10", "1.0.9"));
    assert!(!is_newer_version("1.0.0", "1.0.1"));
}

#[test]
fn test_version_with_prerelease() {
    // Pre-release versions should still compare by major.minor.patch
    assert!(is_newer_version("1.1.0-beta", "1.0.0"));
    assert!(is_newer_version("2.0.0-alpha", "1.9.9"));
}
