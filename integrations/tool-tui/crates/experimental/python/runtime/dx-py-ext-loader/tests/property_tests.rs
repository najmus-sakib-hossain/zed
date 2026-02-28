//! Property-based tests for dx-py-ext-loader
//!
//! These tests validate the correctness properties defined in the design document.

use proptest::prelude::*;

use dx_py_ext_loader::abi::{AbiCompatibility, AbiVersion};
use dx_py_ext_loader::discovery::ExtensionDiscovery;
use dx_py_ext_loader::loader::ExtensionLoader;

// =============================================================================
// Generators
// =============================================================================

/// Generate valid ABI versions (Python 3.x where x is 0-15)
fn arb_abi_version() -> impl Strategy<Value = AbiVersion> {
    (3u32..=3u32, 0u32..=15u32, 0u32..=7u32)
        .prop_map(|(major, minor, flags)| AbiVersion::new(major, minor, flags))
}

/// Generate valid Python 3 minor versions
fn arb_python3_minor() -> impl Strategy<Value = u32> {
    0u32..=15u32
}

/// Generate valid extension filenames for Windows
fn arb_windows_extension_filename() -> impl Strategy<Value = String> {
    (any::<[char; 8]>(), 0u32..=15u32).prop_map(|(name_chars, minor)| {
        let name: String = name_chars
            .iter()
            .filter(|c| c.is_ascii_alphanumeric() || **c == '_')
            .take(8)
            .collect();
        let name = if name.is_empty() { "module" } else { &name };
        format!("{}.cp3{}-win_amd64.pyd", name, minor)
    })
}

/// Generate valid extension filenames for Linux
fn arb_linux_extension_filename() -> impl Strategy<Value = String> {
    (any::<[char; 8]>(), 0u32..=15u32).prop_map(|(name_chars, minor)| {
        let name: String = name_chars
            .iter()
            .filter(|c| c.is_ascii_alphanumeric() || **c == '_')
            .take(8)
            .collect();
        let name = if name.is_empty() { "module" } else { &name };
        format!("{}.cpython-3{}-x86_64-linux-gnu.so", name, minor)
    })
}

/// Generate valid extension filenames (platform-agnostic)
fn arb_extension_filename() -> impl Strategy<Value = String> {
    prop_oneof![
        arb_windows_extension_filename(),
        arb_linux_extension_filename(),
    ]
}

// =============================================================================
// Property 2: Extension Loading Determinism
// **Validates: Requirements 1.1, 1.6**
//
// For any valid C extension file path and ABI version, the Extension_Loader
// should either successfully load the extension (if ABI-compatible) or reject
// it with a specific incompatibility reason (if ABI-incompatible). The decision
// should be deterministic based solely on the extension's metadata.
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: dx-py-game-changer, Property 2: Extension Loading Determinism**
    /// **Validates: Requirements 1.1, 1.6**
    ///
    /// For any valid ABI version pair, the compatibility check should be deterministic.
    /// Running the same check multiple times should always produce the same result.
    #[test]
    fn prop_abi_compatibility_is_deterministic(
        runtime_abi in arb_abi_version(),
        extension_abi in arb_abi_version()
    ) {
        // Run compatibility check multiple times
        let result1 = runtime_abi.is_compatible_with(&extension_abi);
        let result2 = runtime_abi.is_compatible_with(&extension_abi);
        let result3 = runtime_abi.is_compatible_with(&extension_abi);

        // All results should be identical
        prop_assert_eq!(&result1, &result2, "Compatibility check not deterministic");
        prop_assert_eq!(&result2, &result3, "Compatibility check not deterministic");
    }

    /// **Feature: dx-py-game-changer, Property 2: Extension Loading Determinism**
    /// **Validates: Requirements 1.1, 1.6**
    ///
    /// For any extension filename with embedded ABI version, parsing should be deterministic.
    #[test]
    fn prop_abi_parsing_is_deterministic(filename in arb_extension_filename()) {
        let result1 = AbiVersion::from_filename(&filename);
        let result2 = AbiVersion::from_filename(&filename);
        let result3 = AbiVersion::from_filename(&filename);

        prop_assert_eq!(result1, result2, "ABI parsing not deterministic");
        prop_assert_eq!(result2, result3, "ABI parsing not deterministic");
    }

    /// **Feature: dx-py-game-changer, Property 2: Extension Loading Determinism**
    /// **Validates: Requirements 1.1, 1.6**
    ///
    /// Compatibility decisions should be consistent: if A is compatible with B,
    /// then checking A.is_compatible_with(B) should always return a loadable result.
    #[test]
    fn prop_compatibility_decision_is_binary(
        runtime_abi in arb_abi_version(),
        extension_abi in arb_abi_version()
    ) {
        let result = runtime_abi.is_compatible_with(&extension_abi);

        // Result should be one of the three valid states
        match &result {
            AbiCompatibility::FullyCompatible => {
                prop_assert!(result.can_load());
                prop_assert!(result.incompatibility_reason().is_none());
            }
            AbiCompatibility::Compatible { warnings } => {
                prop_assert!(result.can_load());
                prop_assert!(result.incompatibility_reason().is_none());
                // Warnings should be non-empty for this variant
                prop_assert!(!warnings.is_empty());
            }
            AbiCompatibility::Incompatible { reason } => {
                prop_assert!(!result.can_load());
                prop_assert!(result.incompatibility_reason().is_some());
                // Reason should be non-empty
                prop_assert!(!reason.is_empty());
            }
        }
    }

    /// **Feature: dx-py-game-changer, Property 2: Extension Loading Determinism**
    /// **Validates: Requirements 1.1, 1.6**
    ///
    /// Same version should always be fully compatible with itself.
    #[test]
    fn prop_same_version_is_fully_compatible(abi in arb_abi_version()) {
        let result = abi.is_compatible_with(&abi);
        prop_assert_eq!(result, AbiCompatibility::FullyCompatible);
    }

    /// **Feature: dx-py-game-changer, Property 2: Extension Loading Determinism**
    /// **Validates: Requirements 1.1, 1.6**
    ///
    /// Major version mismatch should always be incompatible.
    #[test]
    fn prop_major_version_mismatch_is_incompatible(
        minor1 in arb_python3_minor(),
        minor2 in arb_python3_minor()
    ) {
        let python3 = AbiVersion::new(3, minor1, 0);
        let python2 = AbiVersion::new(2, minor2, 0);

        let result = python3.is_compatible_with(&python2);
        prop_assert!(!result.can_load(), "Major version mismatch should be incompatible");
        prop_assert!(
            result.incompatibility_reason().is_some(),
            "Should have incompatibility reason"
        );
    }

    /// **Feature: dx-py-game-changer, Property 2: Extension Loading Determinism**
    /// **Validates: Requirements 1.1, 1.6**
    ///
    /// Newer runtime should be able to load extensions built for older versions.
    #[test]
    fn prop_newer_runtime_can_load_older_extensions(
        runtime_minor in 1u32..=15u32,
        extension_minor_offset in 1u32..=5u32
    ) {
        // Ensure extension is older than runtime
        let extension_minor = runtime_minor.saturating_sub(extension_minor_offset);
        if extension_minor >= runtime_minor {
            return Ok(()); // Skip if we can't make extension older
        }

        let runtime = AbiVersion::new(3, runtime_minor, 0);
        let extension = AbiVersion::new(3, extension_minor, 0);

        let result = runtime.is_compatible_with(&extension);
        prop_assert!(
            result.can_load(),
            "Newer runtime (3.{}) should load older extension (3.{})",
            runtime_minor,
            extension_minor
        );
    }

    /// **Feature: dx-py-game-changer, Property 2: Extension Loading Determinism**
    /// **Validates: Requirements 1.1, 1.6**
    ///
    /// Older runtime should NOT be able to load extensions built for newer versions.
    #[test]
    fn prop_older_runtime_cannot_load_newer_extensions(
        runtime_minor in 0u32..=14u32,
        extension_minor_offset in 1u32..=5u32
    ) {
        let extension_minor = runtime_minor + extension_minor_offset;
        if extension_minor > 15 {
            return Ok(()); // Skip if extension version is out of range
        }

        let runtime = AbiVersion::new(3, runtime_minor, 0);
        let extension = AbiVersion::new(3, extension_minor, 0);

        let result = runtime.is_compatible_with(&extension);
        prop_assert!(
            !result.can_load(),
            "Older runtime (3.{}) should NOT load newer extension (3.{})",
            runtime_minor,
            extension_minor
        );
    }

    /// **Feature: dx-py-game-changer, Property 2: Extension Loading Determinism**
    /// **Validates: Requirements 1.1, 1.6**
    ///
    /// Extension filename parsing should extract the correct minor version.
    #[test]
    fn prop_filename_parsing_extracts_correct_version(minor in 0u32..=15u32) {
        // Windows format
        let win_filename = format!("module.cp3{}-win_amd64.pyd", minor);
        let win_abi = AbiVersion::from_filename(&win_filename);
        prop_assert!(win_abi.is_some(), "Should parse Windows filename");
        prop_assert_eq!(win_abi.unwrap().minor, minor, "Should extract correct minor version");

        // Linux format
        let linux_filename = format!("module.cpython-3{}-x86_64-linux-gnu.so", minor);
        let linux_abi = AbiVersion::from_filename(&linux_filename);
        prop_assert!(linux_abi.is_some(), "Should parse Linux filename");
        prop_assert_eq!(linux_abi.unwrap().minor, minor, "Should extract correct minor version");
    }
}

// =============================================================================
// Additional unit tests for edge cases
// =============================================================================

#[test]
fn test_extension_loader_deterministic_not_found() {
    let loader = ExtensionLoader::default();

    // Loading a non-existent module should always fail the same way
    let result1 = loader.load("nonexistent_module_xyz");
    let result2 = loader.load("nonexistent_module_xyz");

    assert!(result1.is_err());
    assert!(result2.is_err());

    // Both should be NotFound errors
    match (result1, result2) {
        (Err(e1), Err(e2)) => {
            assert_eq!(e1.extension_name(), e2.extension_name());
        }
        _ => panic!("Both should be errors"),
    }
}

#[test]
fn test_discovery_deterministic_search() {
    let discovery = ExtensionDiscovery::default();

    // Searching for the same module should always return the same result
    let result1 = discovery.find_extension("nonexistent");
    let result2 = discovery.find_extension("nonexistent");

    assert!(result1.is_err());
    assert!(result2.is_err());
}

#[test]
fn test_abi_version_display_deterministic() {
    let abi = AbiVersion::new(3, 11, 0);

    let display1 = format!("{}", abi);
    let display2 = format!("{}", abi);

    assert_eq!(display1, display2);
}
