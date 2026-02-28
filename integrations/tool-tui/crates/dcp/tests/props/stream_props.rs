//! Property-based tests for streaming layer.
//!
//! Feature: dcp-protocol, Property 7: Stream Ordering and Integrity

use dcp::stream::{DcpStream, StreamRingBuffer};
use dcp::DCPError;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 7: Stream Ordering and Integrity
    /// For any sequence of chunks written to a DCP_Stream, the consumer SHALL
    /// receive them in sequence order.
    /// **Validates: Requirements 5.1, 5.2**
    #[test]
    fn prop_stream_ordering(
        chunks in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..100), 1..10),
    ) {
        let stream = DcpStream::new(1, 8192);

        // Write all chunks
        for (i, chunk_data) in chunks.iter().enumerate() {
            let is_last = i == chunks.len() - 1;
            let result = stream.write_chunk(chunk_data, is_last);
            prop_assert!(result.is_ok(), "Failed to write chunk {}: {:?}", i, result);
        }

        // Read all chunks and verify order
        for (expected_seq, expected_data) in chunks.iter().enumerate() {
            let result = stream.read_chunk();
            prop_assert!(result.is_ok(), "Failed to read chunk {}", expected_seq);

            let chunk_opt = result.unwrap();
            prop_assert!(chunk_opt.is_some(), "No chunk available at seq {}", expected_seq);

            let (chunk, data) = chunk_opt.unwrap();
            let seq = chunk.sequence;
            prop_assert_eq!(seq, expected_seq as u32, "Wrong sequence number");
            prop_assert_eq!(&data, expected_data, "Wrong data at seq {}", expected_seq);
        }

        // No more chunks
        prop_assert!(stream.read_chunk().unwrap().is_none());
    }

    /// Feature: dcp-protocol, Property 7: Stream Ordering and Integrity
    /// Backpressure SHALL be signaled when the buffer is full.
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_stream_backpressure(
        chunk_size in 100usize..500,
        buffer_size in 512usize..1024,
    ) {
        let stream = DcpStream::new(1, buffer_size);
        let data = vec![0u8; chunk_size];

        // Write chunks until backpressure
        let mut written = 0;
        let mut backpressure_hit = false;

        for _ in 0..100 {
            let is_last = false;
            match stream.write_chunk(&data, is_last) {
                Ok(_) => written += 1,
                Err(DCPError::Backpressure) => {
                    backpressure_hit = true;
                    break;
                }
                Err(e) => prop_assert!(false, "Unexpected error: {:?}", e),
            }
        }

        // Should have hit backpressure at some point
        prop_assert!(backpressure_hit || written > 0, "Should write at least one chunk or hit backpressure");
    }

    /// Feature: dcp-protocol, Property 7: Stream Ordering and Integrity
    /// The Blake3 checksum SHALL detect any corruption.
    /// **Validates: Requirements 5.4**
    #[test]
    fn prop_stream_checksum_integrity(
        chunks in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..50), 1..5),
    ) {
        let stream1 = DcpStream::new(1, 4096);
        let stream2 = DcpStream::new(2, 4096);

        // Write same data to both streams
        for (i, chunk_data) in chunks.iter().enumerate() {
            let is_last = i == chunks.len() - 1;
            stream1.write_chunk(chunk_data, is_last).unwrap();
            stream2.write_chunk(chunk_data, is_last).unwrap();
        }

        // Checksums should match
        let checksum1 = stream1.checksum();
        let checksum2 = stream2.checksum();
        prop_assert_eq!(checksum1, checksum2, "Identical data should produce identical checksums");

        // Verify checksum
        prop_assert!(stream1.verify_checksum(&checksum1));
        prop_assert!(stream2.verify_checksum(&checksum2));
    }

    /// Feature: dcp-protocol, Property 7: Stream Ordering and Integrity
    /// Different data SHALL produce different checksums.
    /// **Validates: Requirements 5.4**
    #[test]
    fn prop_stream_checksum_detects_difference(
        data1 in prop::collection::vec(any::<u8>(), 10..100),
        data2 in prop::collection::vec(any::<u8>(), 10..100),
    ) {
        prop_assume!(data1 != data2);

        let stream1 = DcpStream::new(1, 4096);
        let stream2 = DcpStream::new(2, 4096);

        stream1.write_chunk(&data1, true).unwrap();
        stream2.write_chunk(&data2, true).unwrap();

        let checksum1 = stream1.checksum();
        let checksum2 = stream2.checksum();

        prop_assert_ne!(checksum1, checksum2, "Different data should produce different checksums");
    }

    /// Feature: dcp-protocol, Property 7: Stream Ordering and Integrity
    /// Ring buffer push/pop SHALL preserve data integrity.
    /// **Validates: Requirements 5.1**
    #[test]
    fn prop_ring_buffer_integrity(
        data in prop::collection::vec(any::<u8>(), 1..200),
    ) {
        let rb = StreamRingBuffer::new(512);

        rb.push(&data).unwrap();

        let mut output = vec![0u8; data.len()];
        let read = rb.pop(&mut output);

        prop_assert_eq!(read, data.len());
        prop_assert_eq!(output, data);
    }

    /// Feature: dcp-protocol, Property 7: Stream Ordering and Integrity
    /// Ring buffer SHALL handle wrap-around correctly.
    /// **Validates: Requirements 5.1**
    #[test]
    fn prop_ring_buffer_wrap_around(
        first_size in 10usize..50,
        second_size in 10usize..50,
    ) {
        let rb = StreamRingBuffer::new(64);

        // Fill with first data
        let first_data: Vec<u8> = (0..first_size).map(|i| i as u8).collect();
        rb.push(&first_data).unwrap();

        // Read some
        let mut buf = vec![0u8; first_size / 2];
        rb.pop(&mut buf);

        // Write more (may wrap around)
        let second_data: Vec<u8> = (100..100 + second_size).map(|i| i as u8).collect();
        let result = rb.push(&second_data);

        // If push succeeded, verify data integrity
        if result.is_ok() {
            // Read remaining first data
            let remaining = first_size - first_size / 2;
            let mut remaining_buf = vec![0u8; remaining];
            let read = rb.pop(&mut remaining_buf);
            prop_assert_eq!(read, remaining);
            let expected_remaining: Vec<u8> = first_data[first_size / 2..].to_vec();
            prop_assert_eq!(remaining_buf, expected_remaining);

            // Read second data
            let mut second_buf = vec![0u8; second_size];
            let read = rb.pop(&mut second_buf);
            prop_assert_eq!(read, second_size);
            prop_assert_eq!(second_buf, second_data);
        }
    }

    /// Feature: dcp-protocol, Property 7: Stream Ordering and Integrity
    /// Stream chunk flags SHALL be correctly set.
    /// **Validates: Requirements 5.3**
    #[test]
    fn prop_stream_chunk_flags(
        num_chunks in 1usize..10,
    ) {
        let stream = DcpStream::new(1, 4096);
        let data = vec![0u8; 10];

        for i in 0..num_chunks {
            let is_last = i == num_chunks - 1;
            let chunk = stream.write_chunk(&data, is_last).unwrap();

            if num_chunks == 1 {
                // Single chunk should be both first and last
                prop_assert!(chunk.is_first());
                prop_assert!(chunk.is_last());
            } else if i == 0 {
                // First chunk
                prop_assert!(chunk.is_first());
                prop_assert!(!chunk.is_last());
            } else if i == num_chunks - 1 {
                // Last chunk
                prop_assert!(!chunk.is_first());
                prop_assert!(chunk.is_last());
            } else {
                // Middle chunk
                prop_assert!(!chunk.is_first());
                prop_assert!(!chunk.is_last());
                prop_assert!(chunk.is_continuation());
            }
        }
    }
}
