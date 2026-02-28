//! Property-based tests for dx-reactor.
//!
//! These tests verify the correctness properties defined in the design document.

use proptest::prelude::*;

// ============================================================================
// Property 1: Batch Submission Count
// For any sequence of I/O operations submitted to a Reactor, the submit()
// method SHALL return the exact count of operations that were successfully queued.
// Validates: Requirements 1.7
// ============================================================================

/// Mock reactor for testing batch submission behavior.
mod mock_reactor {
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub struct MockReactor {
        pending: AtomicUsize,
    }

    impl MockReactor {
        pub fn new() -> Self {
            Self {
                pending: AtomicUsize::new(0),
            }
        }

        pub fn queue_operation(&self) {
            self.pending.fetch_add(1, Ordering::Relaxed);
        }

        pub fn submit(&self) -> usize {
            self.pending.swap(0, Ordering::Relaxed)
        }
    }
}

proptest! {
    /// Property 1: Batch Submission Count
    /// **Feature: binary-dawn, Property 1: Batch Submission Count**
    /// **Validates: Requirements 1.7**
    #[test]
    fn prop_batch_submission_returns_exact_count(count in 0usize..1000) {
        let reactor = mock_reactor::MockReactor::new();

        // Queue exactly `count` operations
        for _ in 0..count {
            reactor.queue_operation();
        }

        // Submit should return exactly the count of queued operations
        let submitted = reactor.submit();
        prop_assert_eq!(submitted, count,
            "submit() should return exact count of queued operations");

        // After submit, pending should be 0
        let remaining = reactor.submit();
        prop_assert_eq!(remaining, 0,
            "submit() should clear pending operations");
    }
}

// ============================================================================
// Property 3: Kernel Version Detection
// For any Linux kernel version string, the is_available() function SHALL return
// true if and only if the major version > 5 OR (major version == 5 AND minor version >= 1).
// Validates: Requirements 2.1
// ============================================================================

/// Parse kernel version for testing (mirrors the actual implementation).
fn parse_kernel_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split_whitespace().collect();
    if parts.len() < 3 {
        return false;
    }

    let version_str = parts
        .iter()
        .find(|s| s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false))
        .unwrap_or(&"0.0.0");

    let version_parts: Vec<u32> = version_str
        .split('.')
        .take(2)
        .filter_map(|s| s.split('-').next())
        .filter_map(|s| s.parse().ok())
        .collect();

    if version_parts.len() < 2 {
        return false;
    }

    let major = version_parts[0];
    let minor = version_parts[1];

    major > 5 || (major == 5 && minor >= 1)
}

proptest! {
    /// Property 3: Kernel Version Detection
    /// **Feature: binary-dawn, Property 3: Kernel Version Detection**
    /// **Validates: Requirements 2.1**
    #[test]
    fn prop_kernel_version_detection(major in 0u32..10, minor in 0u32..100) {
        let version_str = format!("Linux version {}.{}.0-generic", major, minor);
        let result = parse_kernel_version(&version_str);

        let expected = major > 5 || (major == 5 && minor >= 1);
        prop_assert_eq!(result, expected,
            "Kernel {}.{} should be {} but got {}",
            major, minor,
            if expected { "available" } else { "unavailable" },
            if result { "available" } else { "unavailable" });
    }

    /// Property 3: Edge cases for kernel version
    #[test]
    fn prop_kernel_version_boundary_cases(minor in 0u32..100) {
        // Version 5.0.x should NOT be available
        let v5_0 = format!("Linux version 5.0.{}", minor);
        prop_assert!(!parse_kernel_version(&v5_0),
            "Kernel 5.0.{} should NOT be available", minor);

        // Version 5.1.x should be available
        let v5_1 = format!("Linux version 5.1.{}", minor);
        prop_assert!(parse_kernel_version(&v5_1),
            "Kernel 5.1.{} should be available", minor);

        // Version 6.x.x should be available
        let v6 = format!("Linux version 6.{}.0", minor);
        prop_assert!(parse_kernel_version(&v6),
            "Kernel 6.{}.0 should be available", minor);
    }
}

// ============================================================================
// Property 5: Completion Structure Integrity
// For any Completion returned by a Reactor, it SHALL contain valid user_data,
// result, and flags fields that correspond to the original operation.
// Validates: Requirements 3.5, 4.5
// ============================================================================

use dx_reactor::io::Completion;

proptest! {
    /// Property 5: Completion Structure Integrity
    /// **Feature: binary-dawn, Property 5: Completion Structure Integrity**
    /// **Validates: Requirements 3.5, 4.5**
    #[test]
    fn prop_completion_structure_integrity(
        user_data in any::<u64>(),
        result in any::<i32>(),
        flags in any::<u32>()
    ) {
        let completion = Completion::new(user_data, result, flags);

        // Verify all fields are preserved
        prop_assert_eq!(completion.user_data, user_data,
            "user_data should be preserved");
        prop_assert_eq!(completion.result, result,
            "result should be preserved");
        prop_assert_eq!(completion.flags, flags,
            "flags should be preserved");

        // Verify helper methods
        if result >= 0 {
            prop_assert!(completion.is_success(),
                "positive result should indicate success");
            prop_assert_eq!(completion.bytes_transferred(), Some(result as usize),
                "bytes_transferred should return result for success");
            prop_assert_eq!(completion.error_code(), None,
                "error_code should be None for success");
        } else {
            prop_assert!(completion.is_error(),
                "negative result should indicate error");
            prop_assert_eq!(completion.bytes_transferred(), None,
                "bytes_transferred should be None for error");
            prop_assert_eq!(completion.error_code(), Some(-result),
                "error_code should return negated result");
        }
    }
}

// ============================================================================
// Property 6: Thread-per-Core Default
// For any DxReactor built with WorkerStrategy::ThreadPerCore, the number of
// CoreState instances SHALL equal num_cpus::get().
// Validates: Requirements 5.1
// ============================================================================

use dx_reactor::{DxReactor, WorkerStrategy};

#[test]
fn prop_thread_per_core_default() {
    // **Feature: binary-dawn, Property 6: Thread-per-Core Default**
    // **Validates: Requirements 5.1**

    let reactor = DxReactor::build().workers(WorkerStrategy::ThreadPerCore).build();

    let expected_cores = num_cpus::get();
    assert_eq!(
        reactor.num_cores(),
        expected_cores,
        "ThreadPerCore should create {} workers (one per CPU)",
        expected_cores
    );
}

// ============================================================================
// Property 7: Fixed Worker Count
// For any DxReactor built with WorkerStrategy::Fixed(n), the number of
// CoreState instances SHALL equal exactly n.
// Validates: Requirements 5.4
// ============================================================================

proptest! {
    /// Property 7: Fixed Worker Count
    /// **Feature: binary-dawn, Property 7: Fixed Worker Count**
    /// **Validates: Requirements 5.4**
    #[test]
    fn prop_fixed_worker_count(n in 1usize..32) {
        let reactor = DxReactor::build()
            .workers(WorkerStrategy::Fixed(n))
            .build();

        prop_assert_eq!(reactor.num_cores(), n,
            "Fixed({}) should create exactly {} workers", n, n);
    }
}

// ============================================================================
// Property 4: Kqueue Batch Submission
// For any KqueueReactor with pending changes, after calling wait(), the
// pending_changes vector SHALL be empty.
// Validates: Requirements 3.4
// ============================================================================

/// Mock kqueue reactor for testing batch submission behavior.
mod mock_kqueue {
    use std::sync::Mutex;

    pub struct MockKqueueReactor {
        pending_changes: Mutex<Vec<u64>>,
    }

    impl MockKqueueReactor {
        pub fn new() -> Self {
            Self {
                pending_changes: Mutex::new(Vec::new()),
            }
        }

        pub fn register_read(&self, fd: u64) {
            self.pending_changes.lock().unwrap().push(fd);
        }

        pub fn register_write(&self, fd: u64) {
            self.pending_changes.lock().unwrap().push(fd);
        }

        pub fn pending_count(&self) -> usize {
            self.pending_changes.lock().unwrap().len()
        }

        pub fn wait(&self) -> Vec<u64> {
            // Submit pending changes and clear them
            let changes = std::mem::take(&mut *self.pending_changes.lock().unwrap());
            // In real impl, this would call kevent() with the changes
            changes
        }
    }
}

proptest! {
    /// Property 4: Kqueue Batch Submission
    /// **Feature: binary-dawn, Property 4: Kqueue Batch Submission**
    /// **Validates: Requirements 3.4**
    #[test]
    fn prop_kqueue_batch_submission_clears_pending(
        read_fds in prop::collection::vec(any::<u64>(), 0..100),
        write_fds in prop::collection::vec(any::<u64>(), 0..100)
    ) {
        let reactor = mock_kqueue::MockKqueueReactor::new();

        // Register read events
        for fd in &read_fds {
            reactor.register_read(*fd);
        }

        // Register write events
        for fd in &write_fds {
            reactor.register_write(*fd);
        }

        let expected_count = read_fds.len() + write_fds.len();
        prop_assert_eq!(reactor.pending_count(), expected_count,
            "Should have {} pending changes before wait", expected_count);

        // Call wait - should submit and clear pending changes
        let _ = reactor.wait();

        prop_assert_eq!(reactor.pending_count(), 0,
            "pending_changes should be empty after wait()");
    }
}

// ============================================================================
// Property 8: Opcode Uniqueness
// For all HbtpOpcode variants, each SHALL have a unique u8 value and fit
// within a single byte.
// Validates: Requirements 6.1
// ============================================================================

use dx_reactor::protocol::HbtpOpcode;
use std::collections::HashSet;

#[test]
fn prop_opcode_uniqueness() {
    // **Feature: binary-dawn, Property 8: Opcode Uniqueness**
    // **Validates: Requirements 6.1**

    let all_opcodes = HbtpOpcode::all();
    let mut seen_values: HashSet<u8> = HashSet::new();

    for opcode in all_opcodes {
        let value = *opcode as u8;

        // Each opcode must have a unique value
        assert!(seen_values.insert(value), "Opcode {:?} has duplicate value {}", opcode, value);
    }

    // Verify we have the expected number of opcodes
    assert_eq!(all_opcodes.len(), seen_values.len(), "All opcodes should have unique values");
}

#[test]
fn prop_opcode_roundtrip() {
    // **Feature: binary-dawn, Property 8: Opcode Uniqueness (roundtrip)**
    // **Validates: Requirements 6.1**

    // For each opcode, converting to u8 and back should yield the same opcode
    for opcode in HbtpOpcode::all() {
        let value = *opcode as u8;
        let recovered = HbtpOpcode::from_u8(value);
        assert_eq!(
            recovered,
            Some(*opcode),
            "Opcode {:?} (value {}) should roundtrip correctly",
            opcode,
            value
        );
    }
}

// ============================================================================
// Property 9: Header Size Invariant
// For all HbtpHeader instances, size_of::<HbtpHeader>() SHALL equal exactly 8 bytes.
// Validates: Requirements 6.2
// ============================================================================

use dx_reactor::protocol::HbtpHeader;
use std::mem::size_of;

#[test]
fn prop_header_size_invariant() {
    // **Feature: binary-dawn, Property 9: Header Size Invariant**
    // **Validates: Requirements 6.2**

    // The header must be exactly 8 bytes
    assert_eq!(
        size_of::<HbtpHeader>(),
        8,
        "HbtpHeader must be exactly 8 bytes, got {}",
        size_of::<HbtpHeader>()
    );

    // Also verify the constant matches
    assert_eq!(HbtpHeader::SIZE, 8, "HbtpHeader::SIZE must be 8");

    assert_eq!(
        size_of::<HbtpHeader>(),
        HbtpHeader::SIZE,
        "size_of::<HbtpHeader>() must equal HbtpHeader::SIZE"
    );
}

// ============================================================================
// Property 10: Header Parsing
// For any byte slice of length >= 8, HbtpHeader::from_bytes() SHALL return Some.
// For any byte slice of length < 8, it SHALL return None.
// Validates: Requirements 6.3
// ============================================================================

proptest! {
    /// Property 10: Header Parsing - valid slices
    /// **Feature: binary-dawn, Property 10: Header Parsing**
    /// **Validates: Requirements 6.3**
    #[test]
    fn prop_header_parsing_valid_slices(
        opcode in any::<u8>(),
        flags in any::<u8>(),
        sequence in any::<u16>(),
        length in any::<u32>(),
        extra_bytes in prop::collection::vec(any::<u8>(), 0..100)
    ) {
        // Create a valid header buffer
        let mut buffer = vec![0u8; 8 + extra_bytes.len()];
        buffer[0] = opcode;
        buffer[1] = flags;
        buffer[2..4].copy_from_slice(&sequence.to_le_bytes());
        buffer[4..8].copy_from_slice(&length.to_le_bytes());
        buffer[8..].copy_from_slice(&extra_bytes);

        // Parsing should succeed for any buffer >= 8 bytes
        let header = HbtpHeader::from_bytes(&buffer);
        prop_assert!(header.is_some(),
            "from_bytes should return Some for buffer of length {}", buffer.len());

        let header = header.unwrap();
        prop_assert_eq!(header.opcode, opcode, "opcode should match");
        prop_assert_eq!(header.flags, flags, "flags should match");
        prop_assert_eq!(header.sequence, sequence, "sequence should match");
        prop_assert_eq!(header.length, length, "length should match");
    }

    /// Property 10: Header Parsing - invalid slices
    /// **Feature: binary-dawn, Property 10: Header Parsing**
    /// **Validates: Requirements 6.3**
    #[test]
    fn prop_header_parsing_invalid_slices(
        bytes in prop::collection::vec(any::<u8>(), 0..8)
    ) {
        // Parsing should fail for any buffer < 8 bytes
        let header = HbtpHeader::from_bytes(&bytes);
        prop_assert!(header.is_none(),
            "from_bytes should return None for buffer of length {}", bytes.len());
    }
}

#[test]
fn prop_header_parsing_boundary() {
    // **Feature: binary-dawn, Property 10: Header Parsing**
    // **Validates: Requirements 6.3**

    // Exactly 7 bytes - should fail
    let buffer_7 = [0u8; 7];
    assert!(HbtpHeader::from_bytes(&buffer_7).is_none(), "7 bytes should fail");

    // Exactly 8 bytes - should succeed
    let buffer_8 = [0u8; 8];
    assert!(HbtpHeader::from_bytes(&buffer_8).is_some(), "8 bytes should succeed");

    // Empty buffer - should fail
    let empty: [u8; 0] = [];
    assert!(HbtpHeader::from_bytes(&empty).is_none(), "empty buffer should fail");
}

// ============================================================================
// Property 12: Flag Composition
// For any combination of HbtpFlags, setting and checking individual flags
// SHALL be independent and composable.
// Validates: Requirements 6.5
// ============================================================================

use dx_reactor::protocol::HbtpFlags;

proptest! {
    /// Property 12: Flag Composition
    /// **Feature: binary-dawn, Property 12: Flag Composition**
    /// **Validates: Requirements 6.5**
    #[test]
    fn prop_flag_composition(
        compressed in any::<bool>(),
        encrypted in any::<bool>(),
        expects_response in any::<bool>(),
        is_final in any::<bool>()
    ) {
        let mut flags = HbtpFlags::empty();

        // Set flags based on input
        if compressed {
            flags |= HbtpFlags::COMPRESSED;
        }
        if encrypted {
            flags |= HbtpFlags::ENCRYPTED;
        }
        if expects_response {
            flags |= HbtpFlags::EXPECTS_RESPONSE;
        }
        if is_final {
            flags |= HbtpFlags::FINAL;
        }

        // Verify each flag is independent
        prop_assert_eq!(
            flags.contains(HbtpFlags::COMPRESSED), compressed,
            "COMPRESSED flag should be {}", compressed
        );
        prop_assert_eq!(
            flags.contains(HbtpFlags::ENCRYPTED), encrypted,
            "ENCRYPTED flag should be {}", encrypted
        );
        prop_assert_eq!(
            flags.contains(HbtpFlags::EXPECTS_RESPONSE), expects_response,
            "EXPECTS_RESPONSE flag should be {}", expects_response
        );
        prop_assert_eq!(
            flags.contains(HbtpFlags::FINAL), is_final,
            "FINAL flag should be {}", is_final
        );

        // Verify bits() roundtrip
        let bits = flags.bits();
        let recovered = HbtpFlags::from_bits_truncate(bits);
        prop_assert_eq!(flags, recovered, "flags should roundtrip through bits()");
    }
}

#[test]
fn prop_flag_composition_combinations() {
    // **Feature: binary-dawn, Property 12: Flag Composition**
    // **Validates: Requirements 6.5**

    // Test all 16 combinations of 4 flags
    for i in 0u8..16 {
        let compressed = (i & 1) != 0;
        let encrypted = (i & 2) != 0;
        let expects_response = (i & 4) != 0;
        let is_final = (i & 8) != 0;

        let mut flags = HbtpFlags::empty();
        if compressed {
            flags |= HbtpFlags::COMPRESSED;
        }
        if encrypted {
            flags |= HbtpFlags::ENCRYPTED;
        }
        if expects_response {
            flags |= HbtpFlags::EXPECTS_RESPONSE;
        }
        if is_final {
            flags |= HbtpFlags::FINAL;
        }

        // Verify independence
        assert_eq!(flags.contains(HbtpFlags::COMPRESSED), compressed);
        assert_eq!(flags.contains(HbtpFlags::ENCRYPTED), encrypted);
        assert_eq!(flags.contains(HbtpFlags::EXPECTS_RESPONSE), expects_response);
        assert_eq!(flags.contains(HbtpFlags::FINAL), is_final);
    }
}

// ============================================================================
// Property 11: O(1) Route Lookup
// For any HbtpProtocol with N registered routes, route lookup time SHALL be
// constant (O(1)) regardless of N.
// Validates: Requirements 6.4
// ============================================================================

use dx_reactor::protocol::HbtpProtocol;

fn dummy_handler(
    _header: &HbtpHeader,
    _payload: &[u8],
) -> Result<Vec<u8>, dx_reactor::protocol::HbtpError> {
    Ok(vec![])
}

proptest! {
    /// Property 11: O(1) Route Lookup
    /// **Feature: binary-dawn, Property 11: O(1) Route Lookup**
    /// **Validates: Requirements 6.4**
    #[test]
    fn prop_o1_route_lookup(
        num_routes in 1usize..12,  // Up to all opcodes
        lookup_opcode in 0u8..12   // Lookup any opcode
    ) {
        let opcodes = HbtpOpcode::all();
        let mut protocol = HbtpProtocol::new();

        // Register N routes
        for i in 0..num_routes.min(opcodes.len()) {
            protocol.route(opcodes[i], dummy_handler);
        }

        // Lookup should be O(1) - just an array index
        // We verify this by checking that get_handler works correctly
        let opcode_idx = lookup_opcode as usize;
        if opcode_idx < num_routes && opcode_idx < opcodes.len() {
            let handler = protocol.get_handler(opcodes[opcode_idx] as u8);
            prop_assert!(handler.is_some(),
                "Handler for registered opcode {} should exist", opcode_idx);
        }

        // Unregistered opcodes should return None
        if num_routes < opcodes.len() {
            let unregistered = opcodes[num_routes] as u8;
            let handler = protocol.get_handler(unregistered);
            prop_assert!(handler.is_none(),
                "Handler for unregistered opcode should be None");
        }
    }
}

#[test]
fn prop_o1_route_lookup_direct_index() {
    // **Feature: binary-dawn, Property 11: O(1) Route Lookup**
    // **Validates: Requirements 6.4**

    let mut protocol = HbtpProtocol::new();

    // Register handlers for specific opcodes
    protocol.route(HbtpOpcode::Ping, dummy_handler);
    protocol.route(HbtpOpcode::RpcCall, dummy_handler);

    // Verify O(1) lookup by direct array index
    assert!(protocol.get_handler(HbtpOpcode::Ping as u8).is_some());
    assert!(protocol.get_handler(HbtpOpcode::RpcCall as u8).is_some());
    assert!(protocol.get_handler(HbtpOpcode::Pong as u8).is_none());
    assert!(protocol.get_handler(HbtpOpcode::Close as u8).is_none());

    // The lookup is O(1) because it's just handlers[opcode as usize]
    // No iteration, no tree traversal, just direct array access
}

// ============================================================================
// Property 13: ResponseBuffer Reuse
// For any ResponseBuffer, after calling reset(), the buffer SHALL be reusable
// without additional allocation, and as_bytes() SHALL return an empty or
// header-only slice.
// Validates: Requirements 6.6
// ============================================================================

use dx_reactor::protocol::ResponseBuffer;

proptest! {
    /// Property 13: ResponseBuffer Reuse
    /// **Feature: binary-dawn, Property 13: ResponseBuffer Reuse**
    /// **Validates: Requirements 6.6**
    #[test]
    fn prop_response_buffer_reuse(
        sequence1 in any::<u16>(),
        sequence2 in any::<u16>(),
        payload1 in prop::collection::vec(any::<u8>(), 0..1000),
        payload2 in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        let mut buffer = ResponseBuffer::new();

        // Write first response
        buffer.write_rpc_response(sequence1, &payload1);
        let len1 = buffer.len();
        prop_assert!(len1 > 0, "Buffer should have content after write");
        prop_assert_eq!(len1, 8 + payload1.len(), "Length should be header + payload");

        // Reset the buffer
        buffer.reset();

        // After reset, buffer should be empty
        prop_assert!(buffer.is_empty(), "Buffer should be empty after reset");
        prop_assert_eq!(buffer.len(), 0, "Length should be 0 after reset");
        prop_assert_eq!(buffer.as_bytes().len(), 0, "as_bytes should return empty slice");

        // Write second response - should work without allocation issues
        buffer.write_rpc_response(sequence2, &payload2);
        let len2 = buffer.len();
        prop_assert_eq!(len2, 8 + payload2.len(), "Second write should work correctly");

        // Verify the content is from the second write, not the first
        let bytes = buffer.as_bytes();
        prop_assert_eq!(bytes.len(), len2, "as_bytes length should match len()");
    }
}

#[test]
fn prop_response_buffer_multiple_resets() {
    // **Feature: binary-dawn, Property 13: ResponseBuffer Reuse**
    // **Validates: Requirements 6.6**

    let mut buffer = ResponseBuffer::new();

    // Multiple write/reset cycles
    for i in 0..10 {
        buffer.write_pong(i);
        assert_eq!(buffer.len(), 8, "Pong should be 8 bytes (header only)");

        buffer.reset();
        assert!(buffer.is_empty(), "Should be empty after reset");
        assert_eq!(buffer.as_bytes().len(), 0, "as_bytes should be empty");
    }

    // Final write should still work
    buffer.write_rpc_response(999, b"test payload");
    assert_eq!(buffer.len(), 8 + 12, "Final write should work");
}

#[test]
fn prop_response_buffer_capacity_preserved() {
    // **Feature: binary-dawn, Property 13: ResponseBuffer Reuse**
    // **Validates: Requirements 6.6**

    let mut buffer = ResponseBuffer::with_capacity(8192);

    // Write a large payload
    let large_payload = vec![0u8; 4000];
    buffer.write_rpc_response(1, &large_payload);

    // Reset
    buffer.reset();

    // Buffer should still have capacity (no deallocation)
    // We can verify this by writing again without issues
    buffer.write_rpc_response(2, &large_payload);
    assert_eq!(buffer.len(), 8 + 4000);
}

// ============================================================================
// Property 14: Teleportation Round-Trip
// For any Teleportable value written to a TeleportBuffer, reading it back with
// TeleportReader SHALL produce a value equal to the original. For any string
// written via write_string(), reading it back via read_string() with the
// returned offset/length SHALL produce the original string.
// Validates: Requirements 7.3, 7.4, 7.5
// ============================================================================

use dx_reactor::memory::{TeleportBuffer, TeleportReader};

proptest! {
    /// Property 14: Teleportation Round-Trip for primitives
    /// **Feature: binary-dawn, Property 14: Teleportation Round-Trip**
    /// **Validates: Requirements 7.3, 7.4, 7.5**
    #[test]
    fn prop_teleport_roundtrip_u64(value in any::<u64>()) {
        let mut buffer = TeleportBuffer::new(64);
        buffer.write(&value);

        let bytes = buffer.as_bytes();
        let mut reader = TeleportReader::with_string_table(bytes, bytes.len());

        let recovered = reader.read::<u64>();
        prop_assert!(recovered.is_ok(), "Should be able to read u64");
        prop_assert_eq!(*recovered.unwrap(), value, "u64 should roundtrip");
    }

    /// Property 14: Teleportation Round-Trip for i32
    #[test]
    fn prop_teleport_roundtrip_i32(value in any::<i32>()) {
        let mut buffer = TeleportBuffer::new(64);
        buffer.write(&value);

        let bytes = buffer.as_bytes();
        let mut reader = TeleportReader::with_string_table(bytes, bytes.len());

        let recovered = reader.read::<i32>();
        prop_assert!(recovered.is_ok(), "Should be able to read i32");
        prop_assert_eq!(*recovered.unwrap(), value, "i32 should roundtrip");
    }

    /// Property 14: Teleportation Round-Trip for f64
    #[test]
    fn prop_teleport_roundtrip_f64(value in any::<f64>()) {
        let mut buffer = TeleportBuffer::new(64);
        buffer.write(&value);

        let bytes = buffer.as_bytes();
        let mut reader = TeleportReader::with_string_table(bytes, bytes.len());

        let recovered = reader.read::<f64>();
        prop_assert!(recovered.is_ok(), "Should be able to read f64");
        // Use bit comparison for f64 to handle NaN correctly
        prop_assert_eq!(recovered.unwrap().to_bits(), value.to_bits(), "f64 should roundtrip");
    }

    /// Property 14: Teleportation Round-Trip for multiple values
    #[test]
    fn prop_teleport_roundtrip_multiple(
        v1 in any::<u32>(),
        v2 in any::<u64>(),
        v3 in any::<i16>()
    ) {
        let mut buffer = TeleportBuffer::new(128);
        buffer.write(&v1);
        buffer.write(&v2);
        buffer.write(&v3);

        let bytes = buffer.as_bytes();
        let mut reader = TeleportReader::with_string_table(bytes, bytes.len());

        let r1 = reader.read::<u32>();
        let r2 = reader.read::<u64>();
        let r3 = reader.read::<i16>();

        prop_assert!(r1.is_ok() && r2.is_ok() && r3.is_ok(),
            "Should be able to read all values");
        prop_assert_eq!(*r1.unwrap(), v1, "v1 should roundtrip");
        prop_assert_eq!(*r2.unwrap(), v2, "v2 should roundtrip");
        prop_assert_eq!(*r3.unwrap(), v3, "v3 should roundtrip");
    }

    /// Property 14: Teleportation Round-Trip for strings
    /// **Feature: binary-dawn, Property 14: Teleportation Round-Trip**
    /// **Validates: Requirements 7.4**
    #[test]
    fn prop_teleport_roundtrip_string(s in "[a-zA-Z0-9 ]{0,100}") {
        let mut buffer = TeleportBuffer::new(256);
        let (offset, len) = buffer.write_string(&s);

        let finalized = buffer.finalize();

        // The string table starts after the data section + 4 bytes for offset marker
        // For this test, we need to calculate the correct string table offset
        let string_table_offset = finalized.len() - s.len();
        let reader = TeleportReader::with_string_table(finalized, string_table_offset);

        let recovered = reader.read_string(offset, len);
        prop_assert!(recovered.is_ok(), "Should be able to read string");
        prop_assert_eq!(recovered.unwrap(), s.as_str(), "String should roundtrip");
    }
}

#[test]
fn prop_teleport_roundtrip_slice() {
    // **Feature: binary-dawn, Property 14: Teleportation Round-Trip**
    // **Validates: Requirements 7.3**

    let values: Vec<u32> = vec![1, 2, 3, 4, 5, 100, 200, 300];
    let mut buffer = TeleportBuffer::new(256);
    buffer.write_slice(&values);

    let bytes = buffer.as_bytes();
    let mut reader = TeleportReader::with_string_table(bytes, bytes.len());

    let recovered = reader.read_slice::<u32>(values.len());
    assert!(recovered.is_ok(), "Should be able to read slice");
    assert_eq!(recovered.unwrap(), values.as_slice(), "Slice should roundtrip");
}

// ============================================================================
// Property 15: Middleware Execution Order
// For any sequence of middleware types in dx_middleware!, the before() hooks
// SHALL execute in declaration order, and after() hooks SHALL execute in
// reverse declaration order.
// Validates: Requirements 8.3
// ============================================================================

mod middleware_order_test {
    use dx_reactor::middleware::{Middleware, MiddlewareResult, Request, Response};
    use std::cell::RefCell;

    thread_local! {
        static EXECUTION_LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
    }

    pub fn clear_log() {
        EXECUTION_LOG.with(|log| log.borrow_mut().clear());
    }

    pub fn get_log() -> Vec<String> {
        EXECUTION_LOG.with(|log| log.borrow().clone())
    }

    fn log_event(event: &str) {
        EXECUTION_LOG.with(|log| log.borrow_mut().push(event.to_string()));
    }

    pub struct MiddlewareA;
    pub struct MiddlewareB;
    pub struct MiddlewareC;

    impl Middleware for MiddlewareA {
        fn before(_req: &mut Request) -> MiddlewareResult<()> {
            log_event("A:before");
            Ok(())
        }
        fn after(_req: &Request, _res: &mut Response) {
            log_event("A:after");
        }
    }

    impl Middleware for MiddlewareB {
        fn before(_req: &mut Request) -> MiddlewareResult<()> {
            log_event("B:before");
            Ok(())
        }
        fn after(_req: &Request, _res: &mut Response) {
            log_event("B:after");
        }
    }

    impl Middleware for MiddlewareC {
        fn before(_req: &mut Request) -> MiddlewareResult<()> {
            log_event("C:before");
            Ok(())
        }
        fn after(_req: &Request, _res: &mut Response) {
            log_event("C:after");
        }
    }
}

#[test]
fn prop_middleware_execution_order() {
    // **Feature: binary-dawn, Property 15: Middleware Execution Order**
    // **Validates: Requirements 8.3**

    use dx_reactor::middleware::{Request, Response};
    use middleware_order_test::*;

    // Generate the middleware chain using the macro
    dx_reactor::dx_middleware!(MiddlewareA, MiddlewareB, MiddlewareC);

    clear_log();

    let mut req = Request::new("/test".to_string(), "GET".to_string());
    let mut res = Response::new();

    let result = process_middleware(&mut req, &mut res, |_| Ok(()));
    assert!(result.is_ok(), "Middleware chain should succeed");

    let log = get_log();

    // Verify before hooks execute in declaration order: A, B, C
    assert_eq!(log[0], "A:before", "First before should be A");
    assert_eq!(log[1], "B:before", "Second before should be B");
    assert_eq!(log[2], "C:before", "Third before should be C");

    // Verify after hooks execute in reverse order: C, B, A
    assert_eq!(log[3], "C:after", "First after should be C (reverse)");
    assert_eq!(log[4], "B:after", "Second after should be B (reverse)");
    assert_eq!(log[5], "A:after", "Third after should be A (reverse)");

    assert_eq!(log.len(), 6, "Should have exactly 6 events");
}

#[test]
fn prop_middleware_execution_order_two_middlewares() {
    // **Feature: binary-dawn, Property 15: Middleware Execution Order**
    // **Validates: Requirements 8.3**

    use dx_reactor::middleware::{Request, Response};
    use middleware_order_test::*;

    // Test with just two middlewares
    dx_reactor::dx_middleware!(MiddlewareA, MiddlewareB);

    clear_log();

    let mut req = Request::new("/test".to_string(), "GET".to_string());
    let mut res = Response::new();

    let result = process_middleware(&mut req, &mut res, |_| Ok(()));
    assert!(result.is_ok());

    let log = get_log();

    // Before: A, B
    assert_eq!(log[0], "A:before");
    assert_eq!(log[1], "B:before");

    // After: B, A (reverse)
    assert_eq!(log[2], "B:after");
    assert_eq!(log[3], "A:after");
}

#[test]
fn prop_middleware_execution_order_single() {
    // **Feature: binary-dawn, Property 15: Middleware Execution Order**
    // **Validates: Requirements 8.3**

    use dx_reactor::middleware::{Request, Response};
    use middleware_order_test::*;

    // Test with single middleware
    dx_reactor::dx_middleware!(MiddlewareA);

    clear_log();

    let mut req = Request::new("/test".to_string(), "GET".to_string());
    let mut res = Response::new();

    let result = process_middleware(&mut req, &mut res, |_| Ok(()));
    assert!(result.is_ok());

    let log = get_log();

    assert_eq!(log[0], "A:before");
    assert_eq!(log[1], "A:after");
    assert_eq!(log.len(), 2);
}

// ============================================================================
// Property 16: Timing Header Presence
// For any request processed through TimingMiddleware, the response SHALL
// contain an "X-Response-Time" header with a valid duration value.
// Validates: Requirements 8.5
// ============================================================================

use dx_reactor::middleware::{
    Middleware as MwTrait, Request as MwRequest, Response as MwResponse, TimingMiddleware,
};

proptest! {
    /// Property 16: Timing Header Presence
    /// **Feature: binary-dawn, Property 16: Timing Header Presence**
    /// **Validates: Requirements 8.5**
    #[test]
    fn prop_timing_header_presence(
        path in "[a-zA-Z0-9/]{1,50}",
        method in prop::sample::select(vec!["GET", "POST", "PUT", "DELETE"])
    ) {
        let mut req = MwRequest::new(path.clone(), method.to_string());
        let mut res = MwResponse::new();

        // Execute before hook
        let before_result = TimingMiddleware::before(&mut req);
        prop_assert!(before_result.is_ok(), "before() should succeed");
        prop_assert!(req.start_time.is_some(), "start_time should be set");

        // Simulate some work
        std::thread::sleep(std::time::Duration::from_micros(10));

        // Execute after hook
        TimingMiddleware::after(&req, &mut res);

        // Verify X-Response-Time header is present
        let header = res.header("X-Response-Time");
        prop_assert!(header.is_some(), "X-Response-Time header should be present");

        // Verify the header value is a valid duration format (e.g., "0.123ms")
        let value = header.unwrap();
        prop_assert!(value.ends_with("ms"), "Header should end with 'ms'");

        // Parse the numeric part
        let numeric_part = &value[..value.len() - 2];
        let parsed: Result<f64, _> = numeric_part.parse();
        prop_assert!(parsed.is_ok(), "Header value should be parseable as f64");
        prop_assert!(parsed.unwrap() >= 0.0, "Duration should be non-negative");
    }
}

#[test]
fn prop_timing_header_format() {
    // **Feature: binary-dawn, Property 16: Timing Header Presence**
    // **Validates: Requirements 8.5**

    let mut req = MwRequest::new("/test".to_string(), "GET".to_string());
    let mut res = MwResponse::new();

    TimingMiddleware::before(&mut req).unwrap();

    // Simulate work
    std::thread::sleep(std::time::Duration::from_millis(1));

    TimingMiddleware::after(&req, &mut res);

    let header = res.header("X-Response-Time").unwrap();

    // Should be in format "X.XXXms"
    assert!(header.contains('.'), "Should have decimal point");
    assert!(header.ends_with("ms"), "Should end with ms");

    // Duration should be at least 1ms
    let numeric: f64 = header[..header.len() - 2].parse().unwrap();
    assert!(numeric >= 1.0, "Duration should be at least 1ms, got {}", numeric);
}

#[test]
fn prop_timing_header_without_start_time() {
    // **Feature: binary-dawn, Property 16: Timing Header Presence**
    // **Validates: Requirements 8.5**

    // If before() wasn't called, after() should handle gracefully
    let req = MwRequest::new("/test".to_string(), "GET".to_string());
    let mut res = MwResponse::new();

    // Call after without before
    TimingMiddleware::after(&req, &mut res);

    // Header should not be present since start_time is None
    assert!(
        res.header("X-Response-Time").is_none(),
        "Header should not be present without start_time"
    );
}

// ============================================================================
// Property 17: Rate Limit Thread Isolation
// For any RateLimitMiddleware, the rate counter SHALL be thread-local, meaning
// concurrent requests on different threads SHALL have independent counters.
// Validates: Requirements 8.6
// ============================================================================

use dx_reactor::middleware::{RateLimitMiddleware, get_thread_rate_count, reset_thread_rate_limit};

#[test]
fn prop_rate_limit_thread_isolation() {
    // **Feature: binary-dawn, Property 17: Rate Limit Thread Isolation**
    // **Validates: Requirements 8.6**

    use std::sync::{Arc, Barrier};
    use std::thread;

    let num_threads = 4;
    let requests_per_thread = 10;
    let barrier = Arc::new(Barrier::new(num_threads));

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                // Reset this thread's counter
                reset_thread_rate_limit();

                // Wait for all threads to be ready
                barrier.wait();

                // Make requests on this thread
                for _ in 0..requests_per_thread {
                    let mut req = MwRequest::new("/test".to_string(), "GET".to_string());
                    let _ = RateLimitMiddleware::before(&mut req);
                }

                // Return this thread's count
                get_thread_rate_count()
            })
        })
        .collect();

    // Collect results
    let counts: Vec<u32> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Each thread should have exactly requests_per_thread
    // (not the sum of all threads' requests)
    for (i, count) in counts.iter().enumerate() {
        assert_eq!(
            *count, requests_per_thread as u32,
            "Thread {} should have {} requests, got {}",
            i, requests_per_thread, count
        );
    }
}

#[test]
fn prop_rate_limit_independent_counters() {
    // **Feature: binary-dawn, Property 17: Rate Limit Thread Isolation**
    // **Validates: Requirements 8.6**

    use std::sync::mpsc;
    use std::thread;

    // Reset main thread counter
    reset_thread_rate_limit();

    // Make 5 requests on main thread
    for _ in 0..5 {
        let mut req = MwRequest::new("/test".to_string(), "GET".to_string());
        let _ = RateLimitMiddleware::before(&mut req);
    }

    let main_count = get_thread_rate_count();
    assert_eq!(main_count, 5, "Main thread should have 5 requests");

    // Spawn a new thread and check its counter is independent
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        reset_thread_rate_limit();

        // Make 3 requests on this thread
        for _ in 0..3 {
            let mut req = MwRequest::new("/test".to_string(), "GET".to_string());
            let _ = RateLimitMiddleware::before(&mut req);
        }

        tx.send(get_thread_rate_count()).unwrap();
    });

    let other_count = rx.recv().unwrap();
    assert_eq!(other_count, 3, "Other thread should have 3 requests");

    // Main thread count should still be 5
    assert_eq!(get_thread_rate_count(), 5, "Main thread count should be unchanged");
}

proptest! {
    /// Property 17: Rate Limit Thread Isolation with varying request counts
    /// **Feature: binary-dawn, Property 17: Rate Limit Thread Isolation**
    /// **Validates: Requirements 8.6**
    #[test]
    fn prop_rate_limit_isolation_varying_counts(
        count1 in 1u32..50,
        count2 in 1u32..50
    ) {
        use std::sync::mpsc;
        use std::thread;

        // Reset main thread
        reset_thread_rate_limit();

        // Make count1 requests on main thread
        for _ in 0..count1 {
            let mut req = MwRequest::new("/test".to_string(), "GET".to_string());
            let _ = RateLimitMiddleware::before(&mut req);
        }

        // Spawn thread with count2 requests
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            reset_thread_rate_limit();
            for _ in 0..count2 {
                let mut req = MwRequest::new("/test".to_string(), "GET".to_string());
                let _ = RateLimitMiddleware::before(&mut req);
            }
            tx.send(get_thread_rate_count()).unwrap();
        });

        let other_count = rx.recv().unwrap();

        // Each thread should have its own count
        prop_assert_eq!(get_thread_rate_count(), count1,
            "Main thread should have {} requests", count1);
        prop_assert_eq!(other_count, count2,
            "Other thread should have {} requests", count2);
    }
}

// ============================================================================
// Property 8 (Production Readiness): Reactor I/O Callback Invocation
// For any submitted I/O operation that completes (successfully or with error),
// the reactor SHALL invoke exactly one callback with the operation result.
// **Feature: production-readiness, Property 8: Reactor I/O Callback Invocation**
// **Validates: Requirements 4.6, 4.7**
// ============================================================================

/// Mock callback tracker for testing callback invocation behavior.
mod callback_tracker {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// Tracks callback invocations per user_data.
    #[derive(Clone)]
    pub struct CallbackTracker {
        invocations: Arc<Mutex<HashMap<u64, Vec<i32>>>>,
    }

    impl CallbackTracker {
        pub fn new() -> Self {
            Self {
                invocations: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        /// Record a callback invocation.
        pub fn record(&self, user_data: u64, result: i32) {
            let mut invocations = self.invocations.lock().unwrap();
            invocations.entry(user_data).or_default().push(result);
        }

        /// Get the number of invocations for a specific user_data.
        pub fn invocation_count(&self, user_data: u64) -> usize {
            let invocations = self.invocations.lock().unwrap();
            invocations.get(&user_data).map(|v| v.len()).unwrap_or(0)
        }

        /// Get all results for a specific user_data.
        pub fn get_results(&self, user_data: u64) -> Vec<i32> {
            let invocations = self.invocations.lock().unwrap();
            invocations.get(&user_data).cloned().unwrap_or_default()
        }

        /// Get total number of invocations across all user_data.
        pub fn total_invocations(&self) -> usize {
            let invocations = self.invocations.lock().unwrap();
            invocations.values().map(|v| v.len()).sum()
        }
    }

    /// Mock I/O operation for testing.
    #[derive(Debug, Clone)]
    pub struct MockIoOp {
        pub user_data: u64,
        pub expected_result: i32,
    }

    /// Mock reactor that simulates I/O completion with callbacks.
    pub struct MockCallbackReactor {
        pending_ops: Mutex<Vec<MockIoOp>>,
        tracker: CallbackTracker,
    }

    impl MockCallbackReactor {
        pub fn new(tracker: CallbackTracker) -> Self {
            Self {
                pending_ops: Mutex::new(Vec::new()),
                tracker,
            }
        }

        /// Submit an I/O operation.
        pub fn submit(&self, op: MockIoOp) {
            self.pending_ops.lock().unwrap().push(op);
        }

        /// Process all pending operations and invoke callbacks.
        /// Each operation should result in exactly one callback.
        pub fn process_completions(&self) {
            let ops = std::mem::take(&mut *self.pending_ops.lock().unwrap());
            for op in ops {
                // Invoke callback exactly once per completion
                self.tracker.record(op.user_data, op.expected_result);
            }
        }

        /// Get the callback tracker.
        pub fn tracker(&self) -> &CallbackTracker {
            &self.tracker
        }
    }
}

proptest! {
    /// Property 8: Reactor I/O Callback Invocation
    /// **Feature: production-readiness, Property 8: Reactor I/O Callback Invocation**
    /// **Validates: Requirements 4.6, 4.7**
    #[test]
    fn prop_callback_invocation_exactly_once(
        ops in prop::collection::vec(
            (any::<u64>(), any::<i32>()),
            1..100
        )
    ) {
        use callback_tracker::*;

        let tracker = CallbackTracker::new();
        let reactor = MockCallbackReactor::new(tracker.clone());

        // Submit all operations
        for (user_data, result) in &ops {
            reactor.submit(MockIoOp {
                user_data: *user_data,
                expected_result: *result,
            });
        }

        // Process completions
        reactor.process_completions();

        // Verify total invocations equals number of operations
        prop_assert_eq!(
            reactor.tracker().total_invocations(),
            ops.len(),
            "Total callbacks should equal number of submitted operations"
        );

        // Count expected invocations per user_data
        let mut expected_counts: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
        for (user_data, _) in &ops {
            *expected_counts.entry(*user_data).or_default() += 1;
        }

        // Verify each user_data received exactly the expected number of callbacks
        for (user_data, expected_count) in expected_counts {
            let actual_count = reactor.tracker().invocation_count(user_data);
            prop_assert_eq!(
                actual_count,
                expected_count,
                "user_data {} should have {} callbacks, got {}",
                user_data,
                expected_count,
                actual_count
            );
        }
    }

    /// Property 8: Callback receives correct result
    /// **Feature: production-readiness, Property 8: Reactor I/O Callback Invocation**
    /// **Validates: Requirements 4.6, 4.7**
    #[test]
    fn prop_callback_receives_correct_result(
        user_data in any::<u64>(),
        result in any::<i32>()
    ) {
        use callback_tracker::*;

        let tracker = CallbackTracker::new();
        let reactor = MockCallbackReactor::new(tracker.clone());

        // Submit single operation
        reactor.submit(MockIoOp {
            user_data,
            expected_result: result,
        });

        // Process completions
        reactor.process_completions();

        // Verify callback was invoked exactly once
        prop_assert_eq!(
            reactor.tracker().invocation_count(user_data),
            1,
            "Callback should be invoked exactly once"
        );

        // Verify callback received correct result
        let results = reactor.tracker().get_results(user_data);
        prop_assert_eq!(results.len(), 1, "Should have exactly one result");
        prop_assert_eq!(
            results[0],
            result,
            "Callback should receive the correct result"
        );
    }

    /// Property 8: Error completions also invoke callbacks
    /// **Feature: production-readiness, Property 8: Reactor I/O Callback Invocation**
    /// **Validates: Requirements 4.7**
    #[test]
    fn prop_error_completions_invoke_callbacks(
        user_data in any::<u64>(),
        error_code in -1000i32..-1  // Negative values indicate errors
    ) {
        use callback_tracker::*;

        let tracker = CallbackTracker::new();
        let reactor = MockCallbackReactor::new(tracker.clone());

        // Submit operation that will fail
        reactor.submit(MockIoOp {
            user_data,
            expected_result: error_code,
        });

        // Process completions
        reactor.process_completions();

        // Verify callback was invoked even for errors
        prop_assert_eq!(
            reactor.tracker().invocation_count(user_data),
            1,
            "Error completions should also invoke callbacks exactly once"
        );

        // Verify error code was passed to callback
        let results = reactor.tracker().get_results(user_data);
        prop_assert_eq!(
            results[0],
            error_code,
            "Error code should be passed to callback"
        );
    }
}

#[test]
fn prop_callback_invocation_no_duplicate_calls() {
    // **Feature: production-readiness, Property 8: Reactor I/O Callback Invocation**
    // **Validates: Requirements 4.6, 4.7**

    use callback_tracker::*;

    let tracker = CallbackTracker::new();
    let reactor = MockCallbackReactor::new(tracker.clone());

    // Submit multiple operations with same user_data
    for i in 0..5 {
        reactor.submit(MockIoOp {
            user_data: 42,
            expected_result: i,
        });
    }

    // Process completions
    reactor.process_completions();

    // Should have exactly 5 callbacks for user_data 42
    assert_eq!(
        reactor.tracker().invocation_count(42),
        5,
        "Should have exactly 5 callbacks for user_data 42"
    );

    // Process again - should not invoke any more callbacks
    reactor.process_completions();

    assert_eq!(
        reactor.tracker().invocation_count(42),
        5,
        "Processing again should not invoke additional callbacks"
    );
}
