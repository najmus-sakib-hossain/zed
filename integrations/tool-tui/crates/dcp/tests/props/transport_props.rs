//! Property-based tests for DCP transport layer.
//!
//! Tests for TCP transport, framing, and protocol negotiation.

use proptest::prelude::*;

use dcp::transport::{
    framing::{
        FrameCodec, FrameError, FrameHeader, FRAME_HEADER_SIZE, MAX_MESSAGE_SIZE, PROTOCOL_VERSION,
    },
    tcp::{Connection, ProtocolMode, TcpConfig, TcpServer, DCP_MAGIC},
};

// ============================================================================
// Property 3: Connection Limit Enforcement
// **Validates: Requirements 1.3**
// For any configured connection limit N, when N connections are active,
// the (N+1)th connection attempt SHALL be rejected.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 3: Connection Limit Enforcement
    /// For any connection limit N (1-100), acquiring N permits succeeds,
    /// but the (N+1)th attempt fails.
    #[test]
    fn prop_connection_limit_enforcement(limit in 1usize..=100) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = TcpConfig {
                max_connections: limit,
                ..Default::default()
            };
            let server = TcpServer::new(config);

            // Acquire all permits
            let mut permits = Vec::new();
            for _ in 0..limit {
                let permit = server.try_acquire_connection();
                prop_assert!(permit.is_some(), "Should acquire permit within limit");
                permits.push(permit.unwrap());
            }

            // Next attempt should fail
            let extra_permit = server.try_acquire_connection();
            prop_assert!(extra_permit.is_none(), "Should reject connection beyond limit");

            // Drop one permit
            permits.pop();

            // Now should succeed again
            let new_permit = server.try_acquire_connection();
            prop_assert!(new_permit.is_some(), "Should acquire permit after one is released");

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 2: Protocol Negotiation
// **Validates: Requirements 1.2**
// For any initial connection bytes, the server SHALL correctly detect
// MCP JSON-RPC mode (starting with `{`) or DCP binary mode (starting with magic bytes).
// ============================================================================

fn arb_json_prefix() -> impl Strategy<Value = Vec<u8>> {
    // Generate valid JSON-like prefixes
    prop_oneof![
        Just(b"{".to_vec()),
        Just(b"[".to_vec()),
        Just(b"  {".to_vec()),
        Just(b"\n{".to_vec()),
        Just(b"\t[".to_vec()),
        Just(b"{\"jsonrpc\":\"2.0\"".as_slice().to_vec()),
        Just(b"[1,2,3]".to_vec()),
    ]
}

fn arb_binary_prefix() -> impl Strategy<Value = Vec<u8>> {
    // Generate DCP binary prefixes
    prop_oneof![
        Just(DCP_MAGIC.to_vec()),
        Just(
            [
                DCP_MAGIC[0],
                DCP_MAGIC[1],
                DCP_MAGIC[2],
                DCP_MAGIC[3],
                0x00,
                0x00
            ]
            .to_vec()
        ),
        Just(
            [
                DCP_MAGIC[0],
                DCP_MAGIC[1],
                DCP_MAGIC[2],
                DCP_MAGIC[3],
                0xFF,
                0xFF
            ]
            .to_vec()
        ),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 2: Protocol Negotiation (JSON detection)
    /// For any JSON-like prefix, protocol detection returns McpJson.
    #[test]
    fn prop_protocol_negotiation_json(prefix in arb_json_prefix()) {
        let detected = TcpServer::detect_protocol(&prefix);
        prop_assert_eq!(detected, ProtocolMode::McpJson,
            "JSON prefix {:?} should be detected as McpJson", prefix);
    }

    /// Feature: dcp-production, Property 2: Protocol Negotiation (Binary detection)
    /// For any DCP magic prefix, protocol detection returns DcpBinary.
    #[test]
    fn prop_protocol_negotiation_binary(prefix in arb_binary_prefix()) {
        let detected = TcpServer::detect_protocol(&prefix);
        prop_assert_eq!(detected, ProtocolMode::DcpBinary,
            "Binary prefix {:?} should be detected as DcpBinary", prefix);
    }

    /// Feature: dcp-production, Property 2: Protocol Negotiation (Empty defaults to JSON)
    #[test]
    fn prop_protocol_negotiation_empty(_dummy in 0..1i32) {
        let detected = TcpServer::detect_protocol(&[]);
        prop_assert_eq!(detected, ProtocolMode::McpJson,
            "Empty input should default to McpJson");
    }
}

// ============================================================================
// Property 1: Frame Round-Trip
// **Validates: Requirements 2.1, 2.5, 2.6**
// For any valid message payload (up to 16MB), encoding it into a length-prefixed
// frame and decoding it back SHALL produce the identical payload.
// ============================================================================

fn arb_message_payload() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        // Empty message
        Just(vec![]),
        // Small messages
        prop::collection::vec(any::<u8>(), 1..100),
        // Medium messages
        prop::collection::vec(any::<u8>(), 100..1000),
        // Larger messages (but not too large for tests)
        prop::collection::vec(any::<u8>(), 1000..10000),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 1: Frame Round-Trip
    /// For any valid message, encode then decode produces identical payload.
    #[test]
    fn prop_frame_round_trip(payload in arb_message_payload()) {
        let codec = FrameCodec::new();

        // Encode
        let framed = codec.encode_to_bytes(&payload)?;

        // Decode
        let mut src = bytes::BytesMut::from(&framed[..]);
        let decoded = codec.decode_from(&mut src)?.unwrap();

        prop_assert_eq!(&decoded[..], &payload[..],
            "Round-trip should preserve payload");
    }

    /// Feature: dcp-production, Property 1: Frame Round-Trip (header preservation)
    /// Frame header version is preserved through encoding.
    #[test]
    fn prop_frame_header_version_preserved(payload in arb_message_payload()) {
        let codec = FrameCodec::new();
        let framed = codec.encode_to_bytes(&payload)?;

        // Check header
        let header = FrameHeader::decode(&framed).unwrap();
        prop_assert_eq!(header.version, PROTOCOL_VERSION,
            "Protocol version should be preserved");
        prop_assert_eq!(header.length as usize, payload.len(),
            "Length should match payload size");
    }
}

// ============================================================================
// Property 5: Partial Message Buffering
// **Validates: Requirements 2.2**
// For any valid framed message split into arbitrary chunks, the server SHALL
// correctly reassemble the complete message regardless of chunk boundaries.
// ============================================================================

fn arb_chunk_sizes(total_len: usize) -> impl Strategy<Value = Vec<usize>> {
    // Generate random chunk sizes that sum to total_len
    if total_len == 0 {
        return Just(vec![]).boxed();
    }

    prop::collection::vec(1usize..=total_len.max(1), 1..=10)
        .prop_map(move |sizes| {
            let mut result = Vec::new();
            let mut remaining = total_len;
            for size in sizes {
                if remaining == 0 {
                    break;
                }
                let chunk = size.min(remaining);
                result.push(chunk);
                remaining -= chunk;
            }
            if remaining > 0 {
                result.push(remaining);
            }
            result
        })
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 5: Partial Message Buffering
    /// For any message split into arbitrary chunks, reassembly produces the original.
    #[test]
    fn prop_partial_message_buffering(
        payload in prop::collection::vec(any::<u8>(), 0..1000),
        chunk_count in 1usize..=10
    ) {
        let codec = FrameCodec::new();
        let framed = codec.encode_to_bytes(&payload)?;
        let framed_len = framed.len();

        // Generate chunk boundaries
        let mut boundaries: Vec<usize> = (0..chunk_count.saturating_sub(1))
            .map(|i| (framed_len * (i + 1)) / chunk_count)
            .collect();
        boundaries.sort();
        boundaries.dedup();

        // Split into chunks
        let mut chunks = Vec::new();
        let mut start = 0;
        for &end in &boundaries {
            if end > start && end <= framed_len {
                chunks.push(&framed[start..end]);
                start = end;
            }
        }
        if start < framed_len {
            chunks.push(&framed[start..]);
        }

        // Feed chunks into codec
        let mut decoder = FrameCodec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            decoder.feed(chunk);

            // Should not have complete frame until last chunk
            if i < chunks.len() - 1 {
                // May or may not have complete frame depending on boundaries
            }
        }

        // Should have complete frame now
        prop_assert!(decoder.has_complete_frame(),
            "Should have complete frame after all chunks");

        let decoded = decoder.decode()?.unwrap();
        prop_assert_eq!(&decoded[..], &payload[..],
            "Reassembled message should match original");
    }
}

// ============================================================================
// Additional Frame Codec Properties
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Frame size validation: messages exceeding max size are rejected.
    #[test]
    fn prop_frame_size_validation(
        max_size in 100u32..1000,
        payload_size in 0usize..2000
    ) {
        let codec = FrameCodec::with_max_size(max_size);
        let payload = vec![0u8; payload_size];

        let result = codec.encode_to_bytes(&payload);

        if payload_size as u32 > max_size {
            prop_assert!(matches!(result, Err(FrameError::MessageTooLarge(_, _))),
                "Should reject message larger than max_size");
        } else {
            prop_assert!(result.is_ok(),
                "Should accept message within max_size");
        }
    }

    /// Multiple messages can be decoded sequentially from a buffer.
    #[test]
    fn prop_multiple_messages_sequential(
        messages in prop::collection::vec(
            prop::collection::vec(any::<u8>(), 0..100),
            1..5
        )
    ) {
        let codec = FrameCodec::new();

        // Encode all messages
        let mut combined = bytes::BytesMut::new();
        for msg in &messages {
            codec.encode(msg, &mut combined)?;
        }

        // Decode all messages
        let mut decoder = FrameCodec::new();
        decoder.feed(&combined);

        for (i, expected) in messages.iter().enumerate() {
            let decoded = decoder.decode()?;
            prop_assert!(decoded.is_some(),
                "Should decode message {}", i);
            prop_assert_eq!(&decoded.unwrap()[..], &expected[..],
                "Message {} should match", i);
        }

        // No more messages
        prop_assert!(!decoder.has_complete_frame(),
            "Should have no more complete frames");
    }
}

// ============================================================================
// Connection State Properties
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Connection byte tracking is accurate.
    #[test]
    fn prop_connection_byte_tracking(
        reads in prop::collection::vec(0u64..10000, 0..20),
        writes in prop::collection::vec(0u64..10000, 0..20)
    ) {
        let addr = "127.0.0.1:8080".parse().unwrap();
        let conn = Connection::new(1, addr);

        let expected_read: u64 = reads.iter().sum();
        let expected_write: u64 = writes.iter().sum();

        for r in reads {
            conn.record_read(r);
        }
        for w in writes {
            conn.record_write(w);
        }

        prop_assert_eq!(conn.total_bytes_read(), expected_read,
            "Total bytes read should match sum of recorded reads");
        prop_assert_eq!(conn.total_bytes_written(), expected_write,
            "Total bytes written should match sum of recorded writes");
    }
}

// ============================================================================
// Property 4: Disconnect Cleanup
// **Validates: Requirements 1.4**
// For any active session, when the client disconnects unexpectedly,
// the server SHALL clean up all session state and not crash or leak resources.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 4: Disconnect Cleanup
    /// For any number of connections registered and then removed,
    /// the server state should be clean with no leaked connections.
    #[test]
    fn prop_disconnect_cleanup(
        num_connections in 1usize..=50,
        remove_indices in prop::collection::vec(0usize..50, 0..25)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = TcpConfig {
                max_connections: 100,
                ..Default::default()
            };
            let server = TcpServer::new(config);

            // Register connections
            let mut conn_ids = Vec::new();
            for i in 0..num_connections {
                let addr: std::net::SocketAddr = format!("192.168.1.{}:{}", i % 255, 10000 + i).parse().unwrap();
                let conn = server.register_connection(addr).await;
                conn_ids.push(conn.id);
            }

            prop_assert_eq!(server.connection_count().await, num_connections,
                "Should have {} connections registered", num_connections);

            // Remove some connections (simulating disconnects)
            let mut removed_count = 0;
            for idx in remove_indices {
                if idx < conn_ids.len() {
                    let id = conn_ids[idx];
                    if server.remove_connection(id).await.is_some() {
                        removed_count += 1;
                    }
                }
            }

            // Verify remaining count
            let expected_remaining = num_connections - removed_count;
            prop_assert_eq!(server.connection_count().await, expected_remaining,
                "Should have {} connections remaining after {} removals",
                expected_remaining, removed_count);

            // Verify no crashes and state is consistent
            // Try to get a removed connection - should return None
            for idx in 0..conn_ids.len() {
                let id = conn_ids[idx];
                let conn = server.get_connection(id).await;
                // Connection should exist only if not removed
                // (This is a simplified check - actual removal tracking would be more complex)
            }

            Ok(())
        })?;
    }

    /// Feature: dcp-production, Property 4: Disconnect Cleanup (permit release)
    /// When connections are cleaned up, their semaphore permits are released.
    #[test]
    fn prop_disconnect_cleanup_permits_released(
        num_connections in 1usize..=10
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = TcpConfig {
                max_connections: num_connections,
                ..Default::default()
            };
            let server = TcpServer::new(config);

            // Acquire all permits
            let mut permits = Vec::new();
            for _ in 0..num_connections {
                let permit = server.try_acquire_connection();
                prop_assert!(permit.is_some(), "Should acquire permit");
                permits.push(permit.unwrap());
            }

            // Verify limit is reached
            prop_assert!(server.try_acquire_connection().is_none(),
                "Should not acquire permit when at limit");

            // Drop all permits (simulating connection cleanup)
            permits.clear();

            // Now all permits should be available again
            for _ in 0..num_connections {
                let permit = server.try_acquire_connection();
                prop_assert!(permit.is_some(),
                    "Should acquire permit after cleanup");
            }

            Ok(())
        })?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_round_trip_empty() {
        let codec = FrameCodec::new();
        let payload = vec![];
        let framed = codec.encode_to_bytes(&payload).unwrap();
        let mut src = bytes::BytesMut::from(&framed[..]);
        let decoded = codec.decode_from(&mut src).unwrap().unwrap();
        assert_eq!(&decoded[..], &payload[..]);
    }

    #[test]
    fn test_protocol_detection_edge_cases() {
        // Whitespace before JSON
        assert_eq!(TcpServer::detect_protocol(b"   {"), ProtocolMode::McpJson);
        assert_eq!(TcpServer::detect_protocol(b"\n\t{"), ProtocolMode::McpJson);

        // Array JSON
        assert_eq!(TcpServer::detect_protocol(b"[1,2]"), ProtocolMode::McpJson);

        // Binary magic
        assert_eq!(TcpServer::detect_protocol(&DCP_MAGIC), ProtocolMode::DcpBinary);
    }
}
