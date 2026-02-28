//! Property tests for version format validity
//!
//! Property 8: Version Format Validity
//! Validates: Requirements 3.1 (Semantic Versioning 2.0.0)

use proptest::prelude::*;

/// The current crate version from Cargo.toml
const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Parse a semantic version string into (major, minor, patch) components
fn parse_semver(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let major = parts[0].parse::<u32>().ok()?;
    let minor = parts[1].parse::<u32>().ok()?;
    // Handle pre-release suffixes like "0.2.0-alpha"
    let patch_str = parts[2].split('-').next()?;
    let patch = patch_str.parse::<u32>().ok()?;

    Some((major, minor, patch))
}

/// Check if a version string follows semantic versioning format
fn is_valid_semver(version: &str) -> bool {
    // Basic format: MAJOR.MINOR.PATCH
    // Optional: -prerelease+buildmetadata
    let re = regex_lite::Regex::new(
        r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$"
    ).unwrap();
    re.is_match(version)
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    /// Property: The crate version should always be a valid semantic version
    #[test]
    fn prop_crate_version_is_valid_semver(_seed in 0u32..1000) {
        // This test doesn't use the seed, but proptest requires at least one strategy
        prop_assert!(
            is_valid_semver(CRATE_VERSION),
            "Crate version '{}' is not a valid semantic version",
            CRATE_VERSION
        );
    }

    /// Property: The crate version should be parseable into numeric components
    #[test]
    fn prop_crate_version_has_numeric_components(_seed in 0u32..1000) {
        let parsed = parse_semver(CRATE_VERSION);
        prop_assert!(
            parsed.is_some(),
            "Crate version '{}' could not be parsed into (major, minor, patch)",
            CRATE_VERSION
        );
    }

    /// Property: Valid semver strings should be parseable
    #[test]
    fn prop_valid_semver_is_parseable(
        major in 0u32..100,
        minor in 0u32..100,
        patch in 0u32..100
    ) {
        let version = format!("{}.{}.{}", major, minor, patch);
        prop_assert!(
            is_valid_semver(&version),
            "Generated version '{}' should be valid semver",
            version
        );

        let parsed = parse_semver(&version);
        prop_assert!(parsed.is_some(), "Version '{}' should be parseable", version);

        let (m, n, p) = parsed.unwrap();
        prop_assert_eq!(m, major, "Major version mismatch");
        prop_assert_eq!(n, minor, "Minor version mismatch");
        prop_assert_eq!(p, patch, "Patch version mismatch");
    }

    /// Property: Invalid version formats should be rejected
    #[test]
    fn prop_invalid_versions_rejected(
        s in "[a-z]{1,5}"
    ) {
        // Random lowercase strings should not be valid semver
        prop_assert!(
            !is_valid_semver(&s),
            "Random string '{}' should not be valid semver",
            s
        );
    }

    /// Property: Versions with leading zeros in major should be invalid (except 0)
    #[test]
    fn prop_leading_zeros_invalid(
        major in 1u32..100,
        minor in 0u32..100,
        patch in 0u32..100
    ) {
        let version = format!("0{}.{}.{}", major, minor, patch);
        prop_assert!(
            !is_valid_semver(&version),
            "Version with leading zero '{}' should be invalid",
            version
        );
    }

    /// Property: Version comparison should be consistent
    #[test]
    fn prop_version_comparison_consistent(
        major1 in 0u32..10,
        minor1 in 0u32..10,
        patch1 in 0u32..10,
        major2 in 0u32..10,
        minor2 in 0u32..10,
        patch2 in 0u32..10
    ) {
        let v1 = (major1, minor1, patch1);
        let v2 = (major2, minor2, patch2);

        // Comparison should be transitive and consistent
        if v1 < v2 {
            prop_assert!(!(v2 < v1), "Version comparison should be antisymmetric");
        }
        if v1 == v2 {
            prop_assert!(v2 == v1, "Version equality should be symmetric");
        }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[test]
fn test_current_version_format() {
    // The current crate version should be valid semver
    assert!(
        is_valid_semver(CRATE_VERSION),
        "Current crate version '{}' is not valid semver",
        CRATE_VERSION
    );
}

#[test]
fn test_current_version_is_0_0_1() {
    // After dx-production-fixes, version should be 0.0.1
    let parsed = parse_semver(CRATE_VERSION);
    assert!(parsed.is_some(), "Version should be parseable");

    let (major, minor, patch) = parsed.unwrap();
    assert_eq!(major, 0, "Major version should be 0");
    assert_eq!(minor, 0, "Minor version should be 0");
    assert_eq!(patch, 1, "Patch version should be 1");
}

#[test]
fn test_valid_semver_examples() {
    assert!(is_valid_semver("0.0.0"));
    assert!(is_valid_semver("1.0.0"));
    assert!(is_valid_semver("0.0.1"));
    assert!(is_valid_semver("1.2.3"));
    assert!(is_valid_semver("10.20.30"));
    assert!(is_valid_semver("1.0.0-alpha"));
    assert!(is_valid_semver("1.0.0-alpha.1"));
    assert!(is_valid_semver("1.0.0+build"));
    assert!(is_valid_semver("1.0.0-alpha+build"));
}

#[test]
fn test_invalid_semver_examples() {
    assert!(!is_valid_semver(""));
    assert!(!is_valid_semver("1"));
    assert!(!is_valid_semver("1.0"));
    assert!(!is_valid_semver("1.0.0.0"));
    assert!(!is_valid_semver("v1.0.0"));
    assert!(!is_valid_semver("01.0.0"));
    assert!(!is_valid_semver("1.00.0"));
    assert!(!is_valid_semver("1.0.00"));
    assert!(!is_valid_semver("a.b.c"));
    assert!(!is_valid_semver("-1.0.0"));
}

#[test]
fn test_version_ordering() {
    // Test that version comparison follows semver rules
    let v1 = parse_semver("0.0.1").unwrap();
    let v2 = parse_semver("0.0.2").unwrap();
    let v3 = parse_semver("1.0.0").unwrap();

    assert!(v1 < v2, "0.1.0 should be less than 0.2.0");
    assert!(v2 < v3, "0.2.0 should be less than 1.0.0");
    assert!(v1 < v3, "0.1.0 should be less than 1.0.0");
}


// ============================================================================
// Property 4: Version Consistency
// Feature: dx-production-fixes, Property 4: Version Consistency
// Validates: Requirements 7.1, 7.2, 7.3, 7.4
// ============================================================================

/// Read version from a Cargo.toml file content
fn extract_version_from_cargo_toml(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version = ") || trimmed.starts_with("version=") {
            // Extract version string between quotes
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    return Some(trimmed[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }
    None
}

/// Extract workspace package version from Cargo.toml
fn extract_workspace_version(content: &str) -> Option<String> {
    let mut in_workspace_package = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[workspace.package]" {
            in_workspace_package = true;
            continue;
        }
        if in_workspace_package {
            if trimmed.starts_with('[') {
                // New section, stop looking
                break;
            }
            if trimmed.starts_with("version = ") || trimmed.starts_with("version=") {
                if let Some(start) = trimmed.find('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        return Some(trimmed[start + 1..start + 1 + end].to_string());
                    }
                }
            }
        }
    }
    None
}

#[test]
fn test_version_consistency_runtime() {
    // Property 4: Version Consistency
    // For all DX component Cargo.toml files, the version field SHALL be 0.0.1
    // Validates: Requirements 7.1
    assert_eq!(
        CRATE_VERSION, "0.0.1",
        "Runtime version should be 0.0.1 for consistency"
    );
}

proptest! {
    /// Property 4: Version Consistency - Runtime version is always 0.0.1
    /// Feature: dx-production-fixes, Property 4: Version Consistency
    /// Validates: Requirements 7.1, 7.2, 7.3, 7.4
    #[test]
    fn prop_version_consistency_runtime(_seed in 0u32..100) {
        // The runtime crate version should be 0.0.1
        prop_assert_eq!(
            CRATE_VERSION, "0.0.1",
            "Runtime version should be 0.0.1 for version consistency across all DX components"
        );
    }
}
