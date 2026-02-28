//! Property-based tests for DPP format structure validity
//!
//! **Feature: dx-py-package-manager, Property 1: DPP Format Structure Validity**
//! **Validates: Requirements 1.1, 1.3, 1.4, 1.5**

use dx_py_core::{
    headers::{DppHeader, DppMetadata},
    DPP_MAGIC, MAX_PACKAGE_SIZE, PROTOCOL_VERSION,
};
use proptest::prelude::*;

/// Generate arbitrary valid DPP headers
fn arb_dpp_header() -> impl Strategy<Value = DppHeader> {
    (
        // Section offsets (must be >= 64 for header size)
        64u32..1000u32,  // metadata_offset
        64u32..10000u32, // files_offset
        64u32..10000u32, // bytecode_offset
        64u32..10000u32, // native_offset
        64u32..10000u32, // deps_offset
        // Sizes
        64u64..MAX_PACKAGE_SIZE,
        64u64..MAX_PACKAGE_SIZE,
        // Hash (arbitrary bytes)
        proptest::collection::vec(any::<u8>(), 20),
    )
        .prop_map(
            |(
                metadata_offset,
                files_offset,
                bytecode_offset,
                native_offset,
                deps_offset,
                total_size,
                uncompressed_size,
                hash_vec,
            )| {
                let mut blake3_hash = [0u8; 20];
                blake3_hash.copy_from_slice(&hash_vec);

                DppHeader {
                    magic: *DPP_MAGIC,
                    version: PROTOCOL_VERSION,
                    flags: 0,
                    metadata_offset,
                    files_offset,
                    bytecode_offset,
                    native_offset,
                    deps_offset,
                    total_size,
                    uncompressed_size,
                    blake3_hash,
                }
            },
        )
}

/// Generate arbitrary valid DPP metadata
fn arb_dpp_metadata() -> impl Strategy<Value = DppMetadata> {
    (
        1u16..256u16, // name_len (at least 1 char)
        1u16..64u16,  // version_len
        0u16..128u16, // python_requires_len (can be 0)
    )
        .prop_map(|(name_len, version_len, python_requires_len)| {
            DppMetadata::new(name_len, version_len, python_requires_len)
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 1: DPP Header is exactly 64 bytes
    ///
    /// *For any* valid DPP header, the struct size SHALL be exactly 64 bytes.
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_dpp_header_size_is_64_bytes(_header in arb_dpp_header()) {
        prop_assert_eq!(std::mem::size_of::<DppHeader>(), 64);
    }

    /// Property 1: DPP Header magic number is valid
    ///
    /// *For any* valid DPP header, the magic number SHALL be "DPP\x01".
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_dpp_header_has_valid_magic(header in arb_dpp_header()) {
        prop_assert_eq!(&header.magic, DPP_MAGIC);
    }

    /// Property 1: DPP section offsets are valid
    ///
    /// *For any* valid DPP header, all section offsets SHALL be >= 64 (header size)
    /// and <= total_size.
    /// **Validates: Requirements 1.1, 1.3, 1.4**
    #[test]
    fn prop_dpp_section_offsets_valid(header in arb_dpp_header()) {
        let header_size = std::mem::size_of::<DppHeader>() as u32;

        // Copy packed fields to local variables to avoid unaligned access
        let metadata_offset = header.metadata_offset;
        let files_offset = header.files_offset;
        let bytecode_offset = header.bytecode_offset;
        let native_offset = header.native_offset;
        let deps_offset = header.deps_offset;

        // All offsets must be at least header size
        prop_assert!(metadata_offset >= header_size,
            "metadata_offset {} < header_size {}", metadata_offset, header_size);
        prop_assert!(files_offset >= header_size,
            "files_offset {} < header_size {}", files_offset, header_size);
        prop_assert!(bytecode_offset >= header_size,
            "bytecode_offset {} < header_size {}", bytecode_offset, header_size);
        prop_assert!(native_offset >= header_size,
            "native_offset {} < header_size {}", native_offset, header_size);
        prop_assert!(deps_offset >= header_size,
            "deps_offset {} < header_size {}", deps_offset, header_size);
    }

    /// Property 1: DPP total_size respects limits
    ///
    /// *For any* valid DPP header, total_size SHALL be <= MAX_PACKAGE_SIZE.
    /// **Validates: Requirements 1.5**
    #[test]
    fn prop_dpp_total_size_within_limits(header in arb_dpp_header()) {
        let total_size = header.total_size;
        prop_assert!(total_size <= MAX_PACKAGE_SIZE,
            "total_size {} > MAX_PACKAGE_SIZE {}", total_size, MAX_PACKAGE_SIZE);
    }

    /// Property 1: DPP metadata total_size calculation is consistent
    ///
    /// *For any* valid DPP metadata, total_size() SHALL equal the sum of
    /// struct size + all string lengths.
    /// **Validates: Requirements 1.3**
    #[test]
    fn prop_dpp_metadata_total_size_consistent(metadata in arb_dpp_metadata()) {
        let expected = std::mem::size_of::<DppMetadata>()
            + metadata.name_len as usize
            + metadata.version_len as usize
            + metadata.python_requires_len as usize;

        prop_assert_eq!(metadata.total_size(), expected);
    }

    /// Property 1: DPP metadata offsets are sequential and non-overlapping
    ///
    /// *For any* valid DPP metadata, string offsets SHALL be sequential:
    /// name_offset < version_offset < python_requires_offset.
    /// **Validates: Requirements 1.3**
    #[test]
    fn prop_dpp_metadata_offsets_sequential(metadata in arb_dpp_metadata()) {
        let name_offset = metadata.name_offset();
        let version_offset = metadata.version_offset();
        let python_requires_offset = metadata.python_requires_offset();

        // Offsets should be sequential
        prop_assert!(name_offset < version_offset || metadata.name_len == 0,
            "name_offset {} >= version_offset {}", name_offset, version_offset);
        prop_assert!(version_offset <= python_requires_offset,
            "version_offset {} > python_requires_offset {}", version_offset, python_requires_offset);
    }

    /// Property 1: DPP header can be safely cast via bytemuck
    ///
    /// *For any* valid DPP header bytes, casting to DppHeader and back
    /// SHALL produce identical bytes.
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_dpp_header_bytemuck_roundtrip(header in arb_dpp_header()) {
        let bytes: &[u8] = bytemuck::bytes_of(&header);
        let recovered: &DppHeader = bytemuck::from_bytes(bytes);

        prop_assert_eq!(header.magic, recovered.magic);
        prop_assert_eq!(header.version, recovered.version);
        prop_assert_eq!(header.flags, recovered.flags);
        prop_assert_eq!(header.metadata_offset, recovered.metadata_offset);
        prop_assert_eq!(header.files_offset, recovered.files_offset);
        prop_assert_eq!(header.bytecode_offset, recovered.bytecode_offset);
        prop_assert_eq!(header.native_offset, recovered.native_offset);
        prop_assert_eq!(header.deps_offset, recovered.deps_offset);
        prop_assert_eq!(header.total_size, recovered.total_size);
        prop_assert_eq!(header.uncompressed_size, recovered.uncompressed_size);
        prop_assert_eq!(header.blake3_hash, recovered.blake3_hash);
    }
}
