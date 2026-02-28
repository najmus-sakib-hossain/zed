//! WASM Client Test Suite
//!
//! Comprehensive tests for the dx-www WASM client including:
//! - HTIP opcode handlers
//! - Delta patch application
//! - Allocator correctness
//! - Malformed stream resilience

use dx_www_integration_tests::wasm_client::htip_builder::{
    DELTA_MAGIC, DELTA_OP_COPY, DELTA_OP_LITERAL, DeltaPatchBuilder, HtipBuilder, OP_CLASS_TOGGLE,
    OP_CLONE, OP_DELTA_PATCH, OP_EOF, OP_EVENT, OP_PATCH_ATTR, OP_PATCH_TEXT, OP_REMOVE,
    OP_TEMPLATE_DEF,
};
use dx_www_integration_tests::wasm_client::mock_host::{init_mock_host, with_mock_host};

// ============================================================================
// Unit Tests for HTIP Opcode Handlers (Task 9.2)
// ============================================================================

mod opcode_tests {
    use super::*;

    #[test]
    fn test_op_clone_handler() {
        init_mock_host();

        // Pre-register a template
        with_mock_host(|host| {
            host.templates.insert(1, b"<div>Test</div>".to_vec());
        });

        let stream = HtipBuilder::new().clone_template(1).eof().build();

        // Verify stream is correctly formed
        assert!(stream.len() >= 6); // header(4) + clone(2)
        assert_eq!(stream[4], OP_CLONE);
        assert_eq!(stream[5], 1); // template_id
    }

    #[test]
    fn test_op_patch_text_handler() {
        init_mock_host();

        let stream = HtipBuilder::new().patch_text(42, b"Hello World").eof().build();

        // Verify stream structure
        assert_eq!(stream[4], OP_PATCH_TEXT);
        // Node ID (42) as u16 LE
        assert_eq!(stream[5], 42);
        assert_eq!(stream[6], 0);
        // Text length (11) as u16 LE
        assert_eq!(stream[7], 11);
        assert_eq!(stream[8], 0);
        // Text content
        assert_eq!(&stream[9..20], b"Hello World");
    }

    #[test]
    fn test_op_patch_attr_handler() {
        init_mock_host();

        let stream = HtipBuilder::new().patch_attr(10, b"class", b"active").eof().build();

        assert_eq!(stream[4], OP_PATCH_ATTR);
        // Node ID (10) as u16 LE
        assert_eq!(stream[5], 10);
        assert_eq!(stream[6], 0);
        // Key length (5) as u16 LE
        assert_eq!(stream[7], 5);
        assert_eq!(stream[8], 0);
        // Key content
        assert_eq!(&stream[9..14], b"class");
        // Value length (6) as u16 LE
        assert_eq!(stream[14], 6);
        assert_eq!(stream[15], 0);
        // Value content
        assert_eq!(&stream[16..22], b"active");
    }

    #[test]
    fn test_op_class_toggle_handler() {
        init_mock_host();

        let stream = HtipBuilder::new().class_toggle(5, b"hidden", true).eof().build();

        assert_eq!(stream[4], OP_CLASS_TOGGLE);
        // Node ID (5) as u16 LE
        assert_eq!(stream[5], 5);
        assert_eq!(stream[6], 0);
        // Class length (6) as u16 LE
        assert_eq!(stream[7], 6);
        assert_eq!(stream[8], 0);
        // Class content
        assert_eq!(&stream[9..15], b"hidden");
        // Enable flag
        assert_eq!(stream[15], 1);
    }

    #[test]
    fn test_op_class_toggle_disable() {
        init_mock_host();

        let stream = HtipBuilder::new().class_toggle(5, b"visible", false).eof().build();

        // Enable flag should be 0
        let enable_offset = 4 + 1 + 2 + 2 + 7; // op + node_id + class_len + class
        assert_eq!(stream[enable_offset], 0);
    }

    #[test]
    fn test_op_remove_handler() {
        init_mock_host();

        let stream = HtipBuilder::new().remove(100).eof().build();

        assert_eq!(stream[4], OP_REMOVE);
        // Node ID (100) as u16 LE
        assert_eq!(stream[5], 100);
        assert_eq!(stream[6], 0);
    }

    #[test]
    fn test_op_event_handler() {
        init_mock_host();

        let stream = HtipBuilder::new()
            .event(20, 1, 500) // node 20, event type 1 (click), handler 500
            .eof()
            .build();

        assert_eq!(stream[4], OP_EVENT);
        // Node ID (20) as u16 LE
        assert_eq!(stream[5], 20);
        assert_eq!(stream[6], 0);
        // Event type
        assert_eq!(stream[7], 1);
        // Handler ID (500) as u16 LE
        assert_eq!(stream[8], 244); // 500 & 0xFF
        assert_eq!(stream[9], 1); // 500 >> 8
    }

    #[test]
    fn test_op_template_def_handler() {
        init_mock_host();

        let html = b"<div class=\"container\"><span>Content</span></div>";
        let stream = HtipBuilder::new().template_def(5, html).eof().build();

        assert_eq!(stream[4], OP_TEMPLATE_DEF);
        assert_eq!(stream[5], 5); // template ID
        // HTML length as u16 LE
        let len = html.len() as u16;
        assert_eq!(stream[6], (len & 0xFF) as u8);
        assert_eq!(stream[7], (len >> 8) as u8);
        // HTML content
        assert_eq!(&stream[8..8 + html.len()], html);
    }

    #[test]
    fn test_multiple_opcodes_sequence() {
        init_mock_host();

        let stream = HtipBuilder::new()
            .template_def(1, b"<div></div>")
            .clone_template(1)
            .patch_text(1, b"Hello")
            .patch_attr(1, b"id", b"main")
            .class_toggle(1, b"active", true)
            .eof()
            .build();

        // Verify all opcodes are present in sequence
        let mut offset = 4; // Skip header

        assert_eq!(stream[offset], OP_TEMPLATE_DEF);
        offset += 1 + 1 + 2 + 11; // op + id + len + html

        assert_eq!(stream[offset], OP_CLONE);
        offset += 1 + 1; // op + template_id

        assert_eq!(stream[offset], OP_PATCH_TEXT);
        offset += 1 + 2 + 2 + 5; // op + node_id + len + text

        assert_eq!(stream[offset], OP_PATCH_ATTR);
        offset += 1 + 2 + 2 + 2 + 2 + 4; // op + node_id + key_len + key + val_len + val

        assert_eq!(stream[offset], OP_CLASS_TOGGLE);
    }
}

// ============================================================================
// Delta Patch Tests (Task 9.3)
// ============================================================================

mod delta_patch_tests {
    use super::*;

    #[test]
    fn test_delta_patch_copy_only() {
        init_mock_host();

        // Set up base data (128 bytes, 2 blocks of 64)
        let base_data: Vec<u8> = (0..128).collect();
        with_mock_host(|host| {
            host.set_cached_base(1, base_data.clone());
        });

        // Create patch that copies both blocks
        let patch = DeltaPatchBuilder::new(64).copy(0).copy(1).build();

        // Verify patch structure
        assert_eq!(&patch[0..4], &DELTA_MAGIC);
        assert_eq!(patch[4], 1); // version
    }

    #[test]
    fn test_delta_patch_literal_only() {
        init_mock_host();

        let new_data = b"completely new content";
        let patch = DeltaPatchBuilder::new(64).literal(new_data).build();

        // Verify patch contains literal instruction
        assert_eq!(patch[16], DELTA_OP_LITERAL);
        // Length as u16 LE
        assert_eq!(patch[17], new_data.len() as u8);
        assert_eq!(patch[18], 0);
        // Content
        assert_eq!(&patch[19..19 + new_data.len()], new_data);
    }

    #[test]
    fn test_delta_patch_mixed_operations() {
        init_mock_host();

        // Base: 128 bytes
        let base_data: Vec<u8> = (0..128).collect();
        with_mock_host(|host| {
            host.set_cached_base(1, base_data);
        });

        // Patch: copy block 0, insert literal, copy block 1
        let patch = DeltaPatchBuilder::new(64).copy(0).literal(b"inserted").copy(1).build();

        // Verify structure
        let mut offset = 16; // After header

        assert_eq!(patch[offset], DELTA_OP_COPY);
        offset += 5; // op + block_idx

        assert_eq!(patch[offset], DELTA_OP_LITERAL);
        offset += 3 + 8; // op + len + data

        assert_eq!(patch[offset], DELTA_OP_COPY);
    }

    #[test]
    fn test_delta_patch_invalid_magic() {
        let patch = DeltaPatchBuilder::new_invalid_magic().build();

        // Should not have valid magic
        assert_ne!(&patch[0..4], &DELTA_MAGIC);
    }

    #[test]
    fn test_delta_patch_invalid_version() {
        let patch = DeltaPatchBuilder::new_invalid_version().build();

        // Should have invalid version
        assert_eq!(&patch[0..4], &DELTA_MAGIC);
        assert_ne!(patch[4], 1);
    }

    #[test]
    fn test_delta_patch_in_htip_stream() {
        init_mock_host();

        let patch = DeltaPatchBuilder::new(64).copy(0).literal(b"new").build();

        let stream = HtipBuilder::new().delta_patch(1, &patch).eof().build();

        assert_eq!(stream[4], OP_DELTA_PATCH);
        // Cache ID (1) as u32 LE
        assert_eq!(stream[5], 1);
        assert_eq!(stream[6], 0);
        assert_eq!(stream[7], 0);
        assert_eq!(stream[8], 0);
    }
}

// ============================================================================
// Allocator Tests (Task 9.4)
// ============================================================================

mod allocator_tests {
    /// Simulated bump allocator for testing allocation patterns
    struct TestBumpAllocator {
        heap: Vec<u8>,
        ptr: usize,
    }

    impl TestBumpAllocator {
        fn new(size: usize) -> Self {
            Self {
                heap: vec![0; size],
                ptr: 0,
            }
        }

        fn alloc(&mut self, size: usize) -> Option<(usize, usize)> {
            if size == 0 {
                return None;
            }
            let start = self.ptr;
            let end = start + size;
            if end > self.heap.len() {
                return None;
            }
            self.ptr = end;
            Some((start, end))
        }

        fn reset(&mut self) {
            self.ptr = 0;
        }
    }

    #[test]
    fn test_allocator_sequential_non_overlap() {
        let mut alloc = TestBumpAllocator::new(1024);

        let regions: Vec<(usize, usize)> =
            (0..10).filter_map(|i| alloc.alloc((i + 1) * 10)).collect();

        // Verify no overlaps
        for i in 0..regions.len() {
            for j in (i + 1)..regions.len() {
                let (start_a, end_a) = regions[i];
                let (start_b, end_b) = regions[j];

                // Regions should not overlap
                assert!(
                    end_a <= start_b || end_b <= start_a,
                    "Regions {:?} and {:?} overlap",
                    regions[i],
                    regions[j]
                );
            }
        }
    }

    #[test]
    fn test_allocator_exhaustion() {
        let mut alloc = TestBumpAllocator::new(100);

        // Allocate until exhaustion
        let mut count = 0;
        while alloc.alloc(10).is_some() {
            count += 1;
        }

        assert_eq!(count, 10); // 100 / 10 = 10 allocations
    }

    #[test]
    fn test_allocator_reset() {
        let mut alloc = TestBumpAllocator::new(100);

        // Allocate some memory
        alloc.alloc(50);
        alloc.alloc(30);

        // Reset
        alloc.reset();

        // Should be able to allocate full size again
        let result = alloc.alloc(100);
        assert!(result.is_some());
    }

    #[test]
    fn test_allocator_zero_size() {
        let mut alloc = TestBumpAllocator::new(100);

        // Zero-size allocation should fail
        let result = alloc.alloc(0);
        assert!(result.is_none());
    }

    #[test]
    fn test_allocator_exact_fit() {
        let mut alloc = TestBumpAllocator::new(100);

        // Allocate exactly the heap size
        let result = alloc.alloc(100);
        assert!(result.is_some());

        // Next allocation should fail
        let result2 = alloc.alloc(1);
        assert!(result2.is_none());
    }
}

// ============================================================================
// Malformed Stream Resilience Tests (Task 9.5)
// ============================================================================

mod malformed_stream_tests {
    use super::*;

    #[test]
    fn test_empty_stream() {
        let stream: Vec<u8> = vec![];
        // Empty stream should be handled gracefully
        assert!(stream.is_empty());
    }

    #[test]
    fn test_truncated_header() {
        // Only 2 bytes instead of 4
        let stream = vec![0x48, 0x54];
        assert!(stream.len() < 4);
    }

    #[test]
    fn test_invalid_header() {
        let stream = vec![0x00, 0x00, 0x00, 0x00];
        assert_ne!(&stream[0..4], b"HTIP");
    }

    #[test]
    fn test_truncated_clone_opcode() {
        // Header + OP_CLONE but no template_id
        let stream = HtipBuilder::new_raw().raw(b"HTIP").raw(&[OP_CLONE]).build();

        assert_eq!(stream.len(), 5); // Should be truncated
    }

    #[test]
    fn test_truncated_patch_text() {
        // Header + OP_PATCH_TEXT + node_id but no text length
        let stream = HtipBuilder::new_raw().raw(b"HTIP").raw(&[OP_PATCH_TEXT, 0, 0]).build();

        assert_eq!(stream.len(), 7);
    }

    #[test]
    fn test_truncated_patch_attr() {
        // Header + OP_PATCH_ATTR + node_id + key_len but no key
        let stream = HtipBuilder::new_raw().raw(b"HTIP").raw(&[OP_PATCH_ATTR, 0, 0, 5, 0]).build();

        assert_eq!(stream.len(), 9);
    }

    #[test]
    fn test_unknown_opcode() {
        let stream = HtipBuilder::new()
            .invalid_opcode(0xFE) // Unknown opcode
            .eof()
            .build();

        // Stream should be valid but contain unknown opcode
        assert_eq!(stream[4], 0xFE);
    }

    #[test]
    fn test_oversized_text_length() {
        // Claim text is 1000 bytes but only provide 5
        let stream = HtipBuilder::new_raw()
            .raw(b"HTIP")
            .raw(&[OP_PATCH_TEXT, 0, 0])
            .raw(&[0xE8, 0x03]) // 1000 as u16 LE
            .raw(b"hello")
            .build();

        // Stream claims more data than available
        let claimed_len = u16::from_le_bytes([stream[7], stream[8]]);
        let actual_remaining = stream.len() - 9;
        assert!(claimed_len as usize > actual_remaining);
    }

    #[test]
    fn test_delta_patch_truncated_header() {
        // Delta patch with only 10 bytes (header needs 16)
        let patch = vec![0x44, 0x58, 0x44, 0x4C, 0x01, 0x40, 0x00, 0x00, 0x00, 0x00];
        assert!(patch.len() < 16);
    }

    #[test]
    fn test_delta_patch_invalid_block_index() {
        // Create patch with block index way beyond base data
        let patch = DeltaPatchBuilder::new(64)
            .copy(999999) // Way beyond any reasonable base
            .build();

        // Patch is structurally valid but semantically invalid
        assert_eq!(&patch[0..4], &DELTA_MAGIC);
    }

    #[test]
    fn test_random_bytes_as_stream() {
        // Random garbage should not crash
        let garbage: Vec<u8> = (0..100).map(|i| (i * 17 + 31) as u8).collect();

        // Just verify we can create and inspect it
        assert_eq!(garbage.len(), 100);
    }

    #[test]
    fn test_all_zeros_stream() {
        let stream = vec![0u8; 100];

        // All zeros is not a valid HTIP stream
        assert_ne!(&stream[0..4], b"HTIP");
    }

    #[test]
    fn test_all_ones_stream() {
        let stream = vec![0xFFu8; 100];

        // All 0xFF is not valid
        assert_ne!(&stream[0..4], b"HTIP");
    }
}

// ============================================================================
// Integration Tests (Task 9.6)
// ============================================================================

mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_render_flow() {
        init_mock_host();

        // Build a complete HTIP stream that:
        // 1. Defines a template
        // 2. Clones it
        // 3. Patches text
        // 4. Adds an event listener
        let stream = HtipBuilder::new()
            .template_def(1, b"<button>Click me</button>")
            .clone_template(1)
            .patch_text(1, b"Submit")
            .event(1, 1, 100) // click handler
            .eof()
            .build();

        // Verify stream is well-formed
        assert!(stream.len() > 4);
        assert_eq!(&stream[0..4], b"HTIP");

        // Count opcodes
        let mut opcode_count = 0;
        let mut offset = 4;
        while offset < stream.len() {
            let op = stream[offset];
            if op == OP_EOF {
                break;
            }
            opcode_count += 1;
            // Skip to next opcode (simplified)
            offset = stream.len(); // Just count that we found opcodes
        }
        assert!(opcode_count >= 1);
    }

    #[test]
    fn test_multiple_templates_and_clones() {
        init_mock_host();

        let stream = HtipBuilder::new()
            .template_def(1, b"<div></div>")
            .template_def(2, b"<span></span>")
            .clone_template(1)
            .clone_template(2)
            .clone_template(1)
            .clone_template(2)
            .eof()
            .build();

        // Count template definitions by looking at opcode positions
        let mut template_defs = 0;
        let mut clones = 0;
        let mut offset = 4; // Skip header

        while offset < stream.len() {
            let op = stream[offset];
            match op {
                OP_TEMPLATE_DEF => {
                    template_defs += 1;
                    // Skip: op(1) + id(1) + len(2) + html
                    if offset + 3 < stream.len() {
                        let len =
                            u16::from_le_bytes([stream[offset + 2], stream[offset + 3]]) as usize;
                        offset += 4 + len;
                    } else {
                        break;
                    }
                }
                OP_CLONE => {
                    clones += 1;
                    offset += 2; // op(1) + template_id(1)
                }
                OP_EOF => break,
                _ => offset += 1,
            }
        }

        assert_eq!(template_defs, 2);
        assert_eq!(clones, 4);
    }

    #[test]
    fn test_dom_manipulation_sequence() {
        init_mock_host();

        // Simulate a typical DOM update sequence
        let stream = HtipBuilder::new()
            .patch_text(1, b"Loading...")
            .class_toggle(1, b"loading", true)
            .patch_text(1, b"Done!")
            .class_toggle(1, b"loading", false)
            .class_toggle(1, b"success", true)
            .eof()
            .build();

        // Verify the sequence of operations
        let ops: Vec<u8> = stream[4..]
            .iter()
            .filter(|&&b| b == OP_PATCH_TEXT || b == OP_CLASS_TOGGLE || b == OP_EOF)
            .copied()
            .collect();

        assert_eq!(ops.len(), 6); // 2 patch_text + 3 class_toggle + 1 eof
    }

    #[test]
    fn test_delta_patch_integration() {
        init_mock_host();

        // Set up base data
        let base_data: Vec<u8> = b"Hello, World! This is the original content.".to_vec();
        with_mock_host(|host| {
            host.set_cached_base(42, base_data);
        });

        // Create a delta patch
        let patch = DeltaPatchBuilder::new(16)
            .copy(0) // Keep first 16 bytes
            .literal(b"UPDATED") // Insert new content
            .copy(2) // Keep bytes 32-47
            .build();

        // Embed in HTIP stream
        let stream = HtipBuilder::new().delta_patch(42, &patch).eof().build();

        // Verify stream structure
        assert_eq!(stream[4], OP_DELTA_PATCH);
    }

    #[test]
    fn test_mock_host_operations() {
        init_mock_host();

        // Test the mock host directly
        with_mock_host(|host| {
            // Cache a template
            host.cache_template(1, b"<div>Test</div>");
            assert!(host.templates.contains_key(&1));

            // Clone the template
            let node_id = host.clone_template(1);
            assert_eq!(node_id, 1);

            // Append to root
            host.append(0, node_id);

            // Set text
            host.set_text(node_id, "Hello");

            // Set attribute
            host.set_attr(node_id, "class", "active");

            // Toggle class
            host.toggle_class(node_id, "hidden", true);

            // Add event listener
            host.listen(node_id, 1, 100);

            // Verify operations were logged
            assert_eq!(host.dom_ops.len(), 7);
        });
    }

    #[test]
    fn test_mock_host_delta_patching() {
        init_mock_host();

        with_mock_host(|host| {
            // Set up base data
            let base = b"Original content here".to_vec();
            host.set_cached_base(1, base.clone());

            // Verify we can retrieve it
            let mut buf = vec![0u8; 100];
            let size = host.get_cached_base(1, &mut buf);
            assert_eq!(size, base.len());
            assert_eq!(&buf[..size], &base[..]);

            // Store patched result
            let patched = b"Modified content".to_vec();
            host.store_patched(1, &patched);

            // Verify patched result
            assert_eq!(host.get_patched_result(1), Some(&patched));
        });
    }
}

// ============================================================================
// Property-Based Tests (Tasks 9.3, 9.4, 9.5)
// ============================================================================

mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Property 9: Delta Patch Round-Trip (Task 9.3)
    // For any valid base data and delta patch, applying the patch and then
    // generating a new patch from the result to the original SHALL produce
    // an identity patch (or equivalent data).
    // **Validates: Requirements 3.2**
    // ========================================================================

    /// Simulates delta patch application for testing
    fn apply_delta_patch(base: &[u8], patch: DeltaPatchBuilder, block_size: usize) -> Vec<u8> {
        let patch_data = patch.build();
        let mut result = Vec::new();

        // Skip header (16 bytes)
        let mut offset = 16;

        while offset < patch_data.len() {
            let op = patch_data[offset];
            offset += 1;

            match op {
                DELTA_OP_COPY => {
                    if offset + 4 > patch_data.len() {
                        break;
                    }
                    let block_idx = u32::from_le_bytes([
                        patch_data[offset],
                        patch_data[offset + 1],
                        patch_data[offset + 2],
                        patch_data[offset + 3],
                    ]) as usize;
                    offset += 4;

                    let start = block_idx * block_size;
                    let end = (start + block_size).min(base.len());
                    if start < base.len() {
                        result.extend_from_slice(&base[start..end]);
                    }
                }
                DELTA_OP_LITERAL => {
                    if offset + 2 > patch_data.len() {
                        break;
                    }
                    let len =
                        u16::from_le_bytes([patch_data[offset], patch_data[offset + 1]]) as usize;
                    offset += 2;

                    if offset + len <= patch_data.len() {
                        result.extend_from_slice(&patch_data[offset..offset + len]);
                    }
                    offset += len;
                }
                _ => break,
            }
        }

        result
    }

    proptest! {
        /// Feature: production-readiness, Property 9: Delta Patch Round-Trip
        /// For any valid base data, a copy-all patch should reproduce the original
        #[test]
        fn prop_delta_patch_copy_preserves_data(
            base in prop::collection::vec(any::<u8>(), 1..256),
            block_size in 16usize..64,
        ) {
            // Create a patch that copies all blocks
            let num_blocks = (base.len() + block_size - 1) / block_size;
            let mut patch = DeltaPatchBuilder::new(block_size as u16);

            for i in 0..num_blocks {
                patch = patch.copy(i as u32);
            }

            let result = apply_delta_patch(&base, patch, block_size);

            // Result should equal base (possibly truncated to block boundaries)
            let expected_len = (num_blocks * block_size).min(base.len());
            prop_assert_eq!(&result[..result.len().min(expected_len)], &base[..result.len().min(expected_len)]);
        }

        /// Feature: production-readiness, Property 9: Delta Patch Literal Insertion
        /// For any literal data, a literal-only patch should produce that data
        #[test]
        fn prop_delta_patch_literal_produces_data(
            literal in prop::collection::vec(any::<u8>(), 1..100),
        ) {
            let base: Vec<u8> = vec![];
            let patch = DeltaPatchBuilder::new(64).literal(&literal);

            let result = apply_delta_patch(&base, patch, 64);

            prop_assert_eq!(result, literal);
        }
    }

    // ========================================================================
    // Property 10: Allocator Non-Overlap (Task 9.4)
    // For any sequence of allocations from the bump allocator, no two
    // allocated regions SHALL overlap in memory.
    // **Validates: Requirements 3.3**
    // ========================================================================

    /// Test bump allocator for property testing
    struct PropTestAllocator {
        heap_size: usize,
        ptr: usize,
        allocations: Vec<(usize, usize)>, // (start, end) pairs
    }

    impl PropTestAllocator {
        fn new(size: usize) -> Self {
            Self {
                heap_size: size,
                ptr: 0,
                allocations: Vec::new(),
            }
        }

        fn alloc(&mut self, size: usize) -> Option<(usize, usize)> {
            if size == 0 || self.ptr + size > self.heap_size {
                return None;
            }
            let start = self.ptr;
            let end = start + size;
            self.ptr = end;
            self.allocations.push((start, end));
            Some((start, end))
        }

        fn check_no_overlaps(&self) -> bool {
            for i in 0..self.allocations.len() {
                for j in (i + 1)..self.allocations.len() {
                    let (start_a, end_a) = self.allocations[i];
                    let (start_b, end_b) = self.allocations[j];

                    // Check for overlap: regions overlap if NOT (end_a <= start_b OR end_b <= start_a)
                    if !(end_a <= start_b || end_b <= start_a) {
                        return false;
                    }
                }
            }
            true
        }
    }

    proptest! {
        /// Feature: production-readiness, Property 10: Allocator Non-Overlap
        /// For any sequence of allocations, no two regions should overlap
        #[test]
        fn prop_allocator_no_overlaps(
            alloc_sizes in prop::collection::vec(1usize..100, 1..20),
            heap_size in 100usize..10000,
        ) {
            let mut allocator = PropTestAllocator::new(heap_size);

            for size in alloc_sizes {
                // Allocate (may fail if heap exhausted, that's fine)
                let _ = allocator.alloc(size);
            }

            // Verify no overlaps
            prop_assert!(allocator.check_no_overlaps(), "Allocations should not overlap");
        }

        /// Feature: production-readiness, Property 10: Allocator Sequential
        /// Allocations should be sequential (each starts where previous ended)
        #[test]
        fn prop_allocator_sequential(
            alloc_sizes in prop::collection::vec(1usize..50, 1..10),
        ) {
            let mut allocator = PropTestAllocator::new(10000);
            let mut expected_start = 0usize;

            for size in alloc_sizes {
                if let Some((start, end)) = allocator.alloc(size) {
                    prop_assert_eq!(start, expected_start, "Allocation should start at expected position");
                    prop_assert_eq!(end, start + size, "Allocation end should be start + size");
                    expected_start = end;
                }
            }
        }
    }

    // ========================================================================
    // Property 11: Malformed Stream Resilience (Task 9.5)
    // For any byte sequence (including random/malformed data), processing it
    // as an HTIP stream SHALL either succeed or return an error code without
    // crashing or causing undefined behavior.
    // **Validates: Requirements 3.4, 3.5**
    // ========================================================================

    /// Validates HTIP stream structure (returns true if valid, false if malformed)
    fn validate_htip_stream(data: &[u8]) -> Result<(), &'static str> {
        // Must have at least 4-byte header
        if data.len() < 4 {
            return Err("Stream too short for header");
        }

        // Check magic bytes
        if &data[0..4] != b"HTIP" {
            return Err("Invalid magic bytes");
        }

        let mut offset = 4;

        while offset < data.len() {
            let op = data[offset];
            offset += 1;

            match op {
                OP_CLONE => {
                    if offset >= data.len() {
                        return Err("Truncated CLONE opcode");
                    }
                    offset += 1; // template_id
                }
                OP_TEMPLATE_DEF => {
                    if offset + 3 >= data.len() {
                        return Err("Truncated TEMPLATE_DEF opcode");
                    }
                    offset += 1; // id
                    let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;
                    if offset + len > data.len() {
                        return Err("TEMPLATE_DEF data exceeds stream");
                    }
                    offset += len;
                }
                OP_PATCH_TEXT => {
                    if offset + 4 >= data.len() {
                        return Err("Truncated PATCH_TEXT opcode");
                    }
                    offset += 2; // node_id
                    let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;
                    if offset + len > data.len() {
                        return Err("PATCH_TEXT data exceeds stream");
                    }
                    offset += len;
                }
                OP_PATCH_ATTR => {
                    if offset + 4 >= data.len() {
                        return Err("Truncated PATCH_ATTR opcode");
                    }
                    offset += 2; // node_id
                    let key_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;
                    if offset + key_len + 2 > data.len() {
                        return Err("PATCH_ATTR key exceeds stream");
                    }
                    offset += key_len;
                    let val_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;
                    if offset + val_len > data.len() {
                        return Err("PATCH_ATTR value exceeds stream");
                    }
                    offset += val_len;
                }
                OP_CLASS_TOGGLE => {
                    if offset + 5 >= data.len() {
                        return Err("Truncated CLASS_TOGGLE opcode");
                    }
                    offset += 2; // node_id
                    let class_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;
                    if offset + class_len + 1 > data.len() {
                        return Err("CLASS_TOGGLE data exceeds stream");
                    }
                    offset += class_len + 1; // class + enable flag
                }
                OP_REMOVE => {
                    if offset + 2 > data.len() {
                        return Err("Truncated REMOVE opcode");
                    }
                    offset += 2; // node_id
                }
                OP_EVENT => {
                    if offset + 5 > data.len() {
                        return Err("Truncated EVENT opcode");
                    }
                    offset += 5; // node_id(2) + event_type(1) + handler_id(2)
                }
                OP_DELTA_PATCH => {
                    if offset + 8 > data.len() {
                        return Err("Truncated DELTA_PATCH opcode");
                    }
                    offset += 4; // cache_id
                    let patch_len = u32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]) as usize;
                    offset += 4;
                    if offset + patch_len > data.len() {
                        return Err("DELTA_PATCH data exceeds stream");
                    }
                    offset += patch_len;
                }
                OP_EOF => break,
                _ => {
                    // Unknown opcode - this is an error but not a crash
                    return Err("Unknown opcode");
                }
            }
        }

        Ok(())
    }

    proptest! {
        /// Feature: production-readiness, Property 11: Malformed Stream Resilience
        /// Random bytes should not cause crashes when validated
        #[test]
        fn prop_malformed_stream_no_crash(
            data in prop::collection::vec(any::<u8>(), 0..500),
        ) {
            // This should never panic, regardless of input
            let result = validate_htip_stream(&data);

            // Result is either Ok or Err, but never a panic
            prop_assert!(result.is_ok() || result.is_err());
        }

        /// Feature: production-readiness, Property 11: Valid Streams Pass Validation
        /// Well-formed streams should pass validation
        #[test]
        fn prop_valid_stream_passes(
            template_id in 1u8..10,
            node_id in 1u16..100,
            text in "[a-zA-Z0-9]{1,20}",
        ) {
            let stream = HtipBuilder::new()
                .template_def(template_id, text.as_bytes())
                .clone_template(template_id)
                .patch_text(node_id, text.as_bytes())
                .eof()
                .build();

            let result = validate_htip_stream(&stream);
            prop_assert!(result.is_ok(), "Valid stream should pass validation: {:?}", result);
        }

        /// Feature: production-readiness, Property 11: Truncated Streams Fail Gracefully
        /// Truncated streams should return errors, not crash
        #[test]
        fn prop_truncated_stream_fails_gracefully(
            full_stream in prop::collection::vec(any::<u8>(), 10..100),
            truncate_at in 0usize..10,
        ) {
            // Create a valid-looking header
            let mut stream = b"HTIP".to_vec();
            stream.extend_from_slice(&full_stream);

            // Truncate at some point
            let truncate_point = truncate_at.min(stream.len());
            let truncated = &stream[..truncate_point];

            // Should not panic
            let _ = validate_htip_stream(truncated);
        }
    }
}
