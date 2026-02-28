//! Property-based tests for connection multiplexing.
//!
//! Feature: dcp-production, Property 17: Stream Multiplexing Isolation

use bytes::Bytes;
use dcp::multiplex::{MultiplexError, MultiplexedConnection, StreamHeader, StreamStatus};
use proptest::prelude::*;
use tokio::runtime::Runtime;

/// Create a runtime for async tests
fn rt() -> Runtime {
    Runtime::new().unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 17: Stream Multiplexing Isolation
    /// For any multiplexed connection with multiple active streams, an error on one
    /// stream SHALL not affect the processing of other streams, and responses SHALL
    /// be correctly routed by stream ID.
    /// **Validates: Requirements 13.1, 13.2, 13.4**
    #[test]
    fn prop_stream_isolation_on_error(
        num_streams in 2usize..10,
        error_stream_idx in 0usize..10,
        data_sizes in prop::collection::vec(1usize..100, 2..10),
    ) {
        let rt = rt();
        rt.block_on(async {
            let conn = MultiplexedConnection::new();

            // Open multiple streams
            let mut stream_ids = Vec::new();
            for _ in 0..num_streams {
                let id = conn.open_stream().await.unwrap();
                stream_ids.push(id);
            }

            // Send data to all streams
            for (i, &stream_id) in stream_ids.iter().enumerate() {
                let size = data_sizes.get(i % data_sizes.len()).copied().unwrap_or(10);
                let data = vec![i as u8; size];
                let header = StreamHeader::data(stream_id, data.len() as u32);
                conn.process_frame(header, Bytes::from(data)).await.unwrap();
            }

            // Reset one stream (the one at error_stream_idx % num_streams)
            let error_idx = error_stream_idx % num_streams;
            let error_stream_id = stream_ids[error_idx];
            conn.reset_stream(error_stream_id, Some("test error".to_string())).await.unwrap();

            // Verify error stream is in Reset state
            let status = conn.stream_status(error_stream_id).await.unwrap();
            assert_eq!(status, StreamStatus::Reset);

            // Verify all other streams are still functional
            for (i, &stream_id) in stream_ids.iter().enumerate() {
                if i == error_idx {
                    // Error stream should return error on recv
                    let result = conn.recv(stream_id).await;
                    assert!(matches!(result, Err(MultiplexError::StreamError(_))));
                } else {
                    // Other streams should still have their data
                    let status = conn.stream_status(stream_id).await.unwrap();
                    assert_eq!(status, StreamStatus::Open);

                    let data = conn.recv(stream_id).await.unwrap();
                    assert!(data.is_some());
                    let received = data.unwrap();
                    let expected_byte = i as u8;
                    assert!(received.iter().all(|&b| b == expected_byte));
                }
            }
        });
    }

    /// Feature: dcp-production, Property 17: Stream Multiplexing Isolation
    /// Stream IDs SHALL correctly route data to the appropriate stream.
    /// **Validates: Requirements 13.1**
    #[test]
    fn prop_stream_id_routing(
        num_streams in 2usize..8,
        messages_per_stream in 1usize..5,
    ) {
        let rt = rt();
        rt.block_on(async {
            let conn = MultiplexedConnection::new();

            // Open streams
            let mut stream_ids = Vec::new();
            for _ in 0..num_streams {
                let id = conn.open_stream().await.unwrap();
                stream_ids.push(id);
            }

            // Send multiple messages to each stream with unique identifiers
            for msg_idx in 0..messages_per_stream {
                for (stream_idx, &stream_id) in stream_ids.iter().enumerate() {
                    // Create unique data: stream_idx in high nibble, msg_idx in low nibble
                    let marker = ((stream_idx as u8) << 4) | (msg_idx as u8);
                    let data = vec![marker; 10];
                    let header = StreamHeader::data(stream_id, data.len() as u32);
                    conn.process_frame(header, Bytes::from(data)).await.unwrap();
                }
            }

            // Verify each stream received its own messages in order
            for (stream_idx, &stream_id) in stream_ids.iter().enumerate() {
                for msg_idx in 0..messages_per_stream {
                    let data = conn.recv(stream_id).await.unwrap();
                    assert!(data.is_some(), "Stream {} should have message {}", stream_id, msg_idx);

                    let received = data.unwrap();
                    let expected_marker = ((stream_idx as u8) << 4) | (msg_idx as u8);
                    assert!(
                        received.iter().all(|&b| b == expected_marker),
                        "Stream {} message {} has wrong data", stream_id, msg_idx
                    );
                }

                // No more messages
                let data = conn.recv(stream_id).await.unwrap();
                assert!(data.is_none(), "Stream {} should have no more messages", stream_id);
            }
        });
    }

    /// Feature: dcp-production, Property 17: Stream Multiplexing Isolation
    /// Remote RST on one stream SHALL not affect other streams.
    /// **Validates: Requirements 13.4**
    #[test]
    fn prop_remote_rst_isolation(
        num_streams in 2usize..6,
        rst_stream_idx in 0usize..6,
    ) {
        let rt = rt();
        rt.block_on(async {
            let conn = MultiplexedConnection::new();

            // Open streams
            let mut stream_ids = Vec::new();
            for _ in 0..num_streams {
                let id = conn.open_stream().await.unwrap();
                stream_ids.push(id);
            }

            // Send data to all streams
            for &stream_id in &stream_ids {
                let header = StreamHeader::data(stream_id, 5);
                conn.process_frame(header, Bytes::from("hello")).await.unwrap();
            }

            // Simulate remote RST on one stream
            let rst_idx = rst_stream_idx % num_streams;
            let rst_stream_id = stream_ids[rst_idx];
            let rst_header = StreamHeader::rst(rst_stream_id);
            conn.process_frame(rst_header, Bytes::new()).await.unwrap();

            // Verify RST stream is reset
            let status = conn.stream_status(rst_stream_id).await.unwrap();
            assert_eq!(status, StreamStatus::Reset);

            // Verify other streams are unaffected
            for (i, &stream_id) in stream_ids.iter().enumerate() {
                if i != rst_idx {
                    let status = conn.stream_status(stream_id).await.unwrap();
                    assert_eq!(status, StreamStatus::Open);

                    // Data should still be available
                    let data = conn.recv(stream_id).await.unwrap();
                    assert_eq!(data, Some(Bytes::from("hello")));
                }
            }
        });
    }

    /// Feature: dcp-production, Property 17: Stream Multiplexing Isolation
    /// Concurrent streams SHALL be processed independently.
    /// **Validates: Requirements 13.2**
    #[test]
    fn prop_concurrent_stream_independence(
        stream_count in 2u16..20,
    ) {
        let rt = rt();
        rt.block_on(async {
            let conn = MultiplexedConnection::new();

            // Open many streams
            let mut stream_ids = Vec::new();
            for _ in 0..stream_count {
                let id = conn.open_stream().await.unwrap();
                stream_ids.push(id);
            }

            // Verify all streams are independent
            assert_eq!(conn.stream_count(), stream_count);

            // Each stream should have unique ID
            let unique_ids: std::collections::HashSet<_> = stream_ids.iter().collect();
            assert_eq!(unique_ids.len(), stream_ids.len());

            // Close half the streams
            for &stream_id in stream_ids.iter().take(stream_count as usize / 2) {
                conn.close_stream(stream_id).await.unwrap();
            }

            // Remaining streams should still be open
            for &stream_id in stream_ids.iter().skip(stream_count as usize / 2) {
                let status = conn.stream_status(stream_id).await.unwrap();
                assert_eq!(status, StreamStatus::Open);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_max_streams_constant() {
        // Verify the MAX_STREAMS constant matches requirements (65535)
        assert_eq!(dcp::multiplex::MAX_STREAMS, 65535);
    }

    #[tokio::test]
    async fn test_stream_header_encoding() {
        let header = StreamHeader::data(42, 100);
        let bytes = header.to_bytes();

        let decoded = StreamHeader::decode(&bytes).unwrap();
        assert_eq!(decoded.stream_id, 42);
        assert_eq!(decoded.length, 100);
    }
}
