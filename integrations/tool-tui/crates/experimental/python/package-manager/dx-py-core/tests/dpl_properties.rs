//! Property-based tests for DPL format structure validity
//!
//! **Feature: dx-py-package-manager, Property 3: DPL Format Structure Validity**
//! **Validates: Requirements 2.1, 2.3, 2.4**

use dx_py_core::{
    fnv1a_hash,
    headers::{DplEntry, DplHeader},
    DPL_MAGIC, PROTOCOL_VERSION,
};
use proptest::prelude::*;

/// Generate arbitrary valid DPL headers
fn arb_dpl_header() -> impl Strategy<Value = DplHeader> {
    (
        1u32..10000u32,  // package_count
        64u32..10000u32, // hash_table_offset
        64u32..10000u32, // hash_table_size
        64u32..10000u32, // entries_offset
        // Python version string (up to 15 chars)
        "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}",
        // Platform string (up to 31 chars)
        "[a-z_0-9]{10,30}",
        // resolved_at timestamp
        0u64..u64::MAX,
        // content_hash
        proptest::collection::vec(any::<u8>(), 32),
    )
        .prop_map(
            |(
                package_count,
                hash_table_offset,
                hash_table_size,
                entries_offset,
                python_version_str,
                platform_str,
                resolved_at,
                hash_vec,
            )| {
                let mut python_version = [0u8; 16];
                let pv_bytes = python_version_str.as_bytes();
                python_version[..pv_bytes.len().min(15)]
                    .copy_from_slice(&pv_bytes[..pv_bytes.len().min(15)]);

                let mut platform = [0u8; 32];
                let pl_bytes = platform_str.as_bytes();
                platform[..pl_bytes.len().min(31)]
                    .copy_from_slice(&pl_bytes[..pl_bytes.len().min(31)]);

                let mut content_hash = [0u8; 32];
                content_hash.copy_from_slice(&hash_vec);

                DplHeader {
                    magic: *DPL_MAGIC,
                    version: PROTOCOL_VERSION,
                    package_count,
                    _padding: 0,
                    hash_table_offset,
                    hash_table_size,
                    entries_offset,
                    python_version,
                    platform,
                    resolved_at,
                    content_hash,
                }
            },
        )
}

/// Generate arbitrary valid DPL entries
fn arb_dpl_entry() -> impl Strategy<Value = DplEntry> {
    (
        // Package name (1-47 chars, alphanumeric with hyphens/underscores)
        "[a-z][a-z0-9_-]{0,46}",
        // Version string (1-23 chars)
        "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
        // Source type (0-3)
        0u8..4u8,
        // Source hash
        proptest::collection::vec(any::<u8>(), 32),
    )
        .prop_map(|(name, version, source_type, hash_vec)| {
            let mut entry = DplEntry::new(&name, &version, [0u8; 32]);
            entry.source_type = source_type;
            entry.source_hash.copy_from_slice(&hash_vec);
            entry
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 3: DPL Entry is exactly 128 bytes
    ///
    /// *For any* valid DPL entry, the struct size SHALL be exactly 128 bytes.
    /// **Validates: Requirements 2.3**
    #[test]
    fn prop_dpl_entry_size_is_128_bytes(_entry in arb_dpl_entry()) {
        prop_assert_eq!(std::mem::size_of::<DplEntry>(), 128);
    }

    /// Property 3: DPL Header has valid magic number
    ///
    /// *For any* valid DPL header, the magic number SHALL be "DPL\x01".
    /// **Validates: Requirements 2.1**
    #[test]
    fn prop_dpl_header_has_valid_magic(header in arb_dpl_header()) {
        prop_assert_eq!(&header.magic, DPL_MAGIC);
    }

    /// Property 3: DPL Header has valid hash table configuration
    ///
    /// *For any* valid DPL header, hash_table_size SHALL be > 0.
    /// **Validates: Requirements 2.1**
    #[test]
    fn prop_dpl_header_hash_table_valid(header in arb_dpl_header()) {
        let hash_table_size = header.hash_table_size;
        prop_assert!(hash_table_size > 0, "hash_table_size must be > 0");
    }

    /// Property 3: DPL Header has valid Python version metadata
    ///
    /// *For any* valid DPL header, python_version SHALL be present and valid.
    /// **Validates: Requirements 2.4**
    #[test]
    fn prop_dpl_header_python_version_valid(header in arb_dpl_header()) {
        let pv = header.python_version_str();
        prop_assert!(!pv.is_empty(), "python_version must not be empty");
        // Should contain at least one dot (e.g., "3.12")
        prop_assert!(pv.contains('.'), "python_version must contain a dot: {}", pv);
    }

    /// Property 3: DPL Header has valid platform metadata
    ///
    /// *For any* valid DPL header, platform SHALL be present and valid.
    /// **Validates: Requirements 2.4**
    #[test]
    fn prop_dpl_header_platform_valid(header in arb_dpl_header()) {
        let platform = header.platform_str();
        prop_assert!(!platform.is_empty(), "platform must not be empty");
    }

    /// Property 3: DPL Entry name hash is consistent
    ///
    /// *For any* valid DPL entry, the name_hash SHALL equal fnv1a_hash of the name.
    /// **Validates: Requirements 2.1**
    #[test]
    fn prop_dpl_entry_name_hash_consistent(entry in arb_dpl_entry()) {
        let name = entry.name_str();
        let expected_hash = fnv1a_hash(name.as_bytes());
        let actual_hash = entry.name_hash;
        prop_assert_eq!(actual_hash, expected_hash,
            "name_hash mismatch for '{}': expected {}, got {}", name, expected_hash, actual_hash);
    }

    /// Property 3: DPL Entry can be safely cast via bytemuck
    ///
    /// *For any* valid DPL entry bytes, casting to DplEntry and back
    /// SHALL produce identical bytes.
    /// **Validates: Requirements 2.3**
    #[test]
    fn prop_dpl_entry_bytemuck_roundtrip(entry in arb_dpl_entry()) {
        let bytes: &[u8] = bytemuck::bytes_of(&entry);
        let recovered: &DplEntry = bytemuck::from_bytes(bytes);

        prop_assert_eq!(entry.name_hash, recovered.name_hash);
        prop_assert_eq!(entry.name, recovered.name);
        prop_assert_eq!(entry.version, recovered.version);
        prop_assert_eq!(entry.source_type, recovered.source_type);
        prop_assert_eq!(entry.source_hash, recovered.source_hash);
    }

    /// Property 3: DPL Entry name and version strings are retrievable
    ///
    /// *For any* valid DPL entry, name_str() and version_str() SHALL return
    /// the original strings (up to the max length).
    /// **Validates: Requirements 2.3**
    #[test]
    fn prop_dpl_entry_strings_retrievable(
        name in "[a-z][a-z0-9_-]{0,46}",
        version in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}"
    ) {
        let entry = DplEntry::new(&name, &version, [0u8; 32]);

        let retrieved_name = entry.name_str();
        let retrieved_version = entry.version_str();

        // Names should match (up to max length)
        let expected_name = &name[..name.len().min(47)];
        prop_assert_eq!(retrieved_name, expected_name);

        // Versions should match (up to max length)
        let expected_version = &version[..version.len().min(23)];
        prop_assert_eq!(retrieved_version, expected_version);
    }
}
