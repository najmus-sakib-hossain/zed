//! Property-based tests for stdio transport.
//!
//! Feature: dcp-production

use dcp::compat::stdio::{
    frame_message, unframe_message, MessageFramer, StdioConfig, StdioTransport,
};
use proptest::prelude::*;
use std::io::Cursor;

/// Generate arbitrary JSON-RPC-like messages
fn arb_json_message() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::string::string_regex(r#"[a-zA-Z0-9_\-:,\{\}\[\]"' ]"#).unwrap(),
        1..100,
    )
    .prop_map(|chars| {
        let content: String = chars.join("");
        format!(r#"{{"jsonrpc":"2.0","method":"test","data":"{}","id":1}}"#, content)
    })
}

/// Generate arbitrary message content (no newlines)
fn arb_message_content() -> impl Strategy<Value = String> {
    prop::string::string_regex(r#"[^\n\r]{1,1000}"#)
        .unwrap()
        .prop_filter("non-empty", |s| !s.trim().is_empty())
}

/// Generate chunk boundaries for splitting data
fn arb_chunk_boundaries(len: usize) -> impl Strategy<Value = Vec<usize>> {
    if len == 0 {
        return Just(vec![]).boxed();
    }
    prop::collection::vec(0..len, 0..10)
        .prop_map(move |mut boundaries| {
            boundaries.push(0);
            boundaries.push(len);
            boundaries.sort();
            boundaries.dedup();
            boundaries
        })
        .boxed()
}

// =============================================================================
// Property 13: Stdio Newline Framing
// Feature: dcp-production, Property 13: Messages framed with newlines round-trip correctly
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 13: Stdio Newline Framing
    /// Messages framed with newlines should round-trip correctly through the framer.
    #[test]
    fn prop_stdio_newline_framing(message in arb_message_content()) {
        // Frame the message
        let framed = frame_message(&message);

        // Verify framing adds newline
        prop_assert!(framed.ends_with('\n'), "Framed message should end with newline");

        // Unframe should recover original
        let unframed = unframe_message(&framed);
        prop_assert_eq!(unframed, message.as_str(), "Unframed should match original");
    }

    /// Property 13b: Multiple messages through framer
    /// Multiple newline-delimited messages should be correctly separated.
    #[test]
    fn prop_stdio_multiple_messages(messages in prop::collection::vec(arb_message_content(), 1..10)) {
        let mut framer = MessageFramer::new();

        // Concatenate all framed messages
        let mut data = Vec::new();
        for msg in &messages {
            data.extend_from_slice(msg.as_bytes());
            data.push(b'\n');
        }

        // Feed to framer
        let parsed = framer.feed(&data).unwrap();

        // Should get same number of messages
        prop_assert_eq!(parsed.len(), messages.len(), "Should parse same number of messages");

        // Each message should match (trimmed)
        for (parsed_msg, original) in parsed.iter().zip(messages.iter()) {
            prop_assert_eq!(parsed_msg, original.trim(), "Parsed message should match original");
        }
    }

    /// Property 13c: Partial message buffering
    /// Messages split across chunks should be correctly reassembled.
    #[test]
    fn prop_stdio_partial_buffering(
        message in arb_message_content(),
        split_point in 0usize..1000usize
    ) {
        let framed = frame_message(&message);
        let bytes = framed.as_bytes();

        // Clamp split point to valid range
        let split = split_point.min(bytes.len());

        let mut framer = MessageFramer::new();

        // Feed first chunk
        let first_chunk = &bytes[..split];
        let messages1 = framer.feed(first_chunk).unwrap();

        // Feed second chunk
        let second_chunk = &bytes[split..];
        let messages2 = framer.feed(second_chunk).unwrap();

        // Total messages should be 1
        let total_messages = messages1.len() + messages2.len();
        prop_assert_eq!(total_messages, 1, "Should get exactly one message");

        // The message should match original
        let parsed = if !messages1.is_empty() {
            &messages1[0]
        } else {
            &messages2[0]
        };
        prop_assert_eq!(parsed, message.trim(), "Parsed should match original");
    }

    /// Property 13d: Arbitrary chunk boundaries
    /// Messages should be correctly parsed regardless of chunk boundaries.
    #[test]
    fn prop_stdio_arbitrary_chunks(
        messages in prop::collection::vec(arb_message_content(), 1..5),
        num_chunks in 1usize..10usize
    ) {
        // Build complete data
        let mut data = Vec::new();
        for msg in &messages {
            data.extend_from_slice(msg.as_bytes());
            data.push(b'\n');
        }

        // Split into chunks
        let chunk_size = (data.len() / num_chunks).max(1);
        let chunks: Vec<&[u8]> = data.chunks(chunk_size).collect();

        let mut framer = MessageFramer::new();
        let mut all_parsed = Vec::new();

        for chunk in chunks {
            let parsed = framer.feed(chunk).unwrap();
            all_parsed.extend(parsed);
        }

        // Should get all messages
        prop_assert_eq!(all_parsed.len(), messages.len(), "Should parse all messages");

        for (parsed, original) in all_parsed.iter().zip(messages.iter()) {
            prop_assert_eq!(parsed, original.trim(), "Each message should match");
        }
    }
}

// =============================================================================
// Property 13e: EOF Handling
// Feature: dcp-production, Property 13e: EOF triggers graceful shutdown
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 13e: EOF handling triggers shutdown
    /// Reading from empty input (EOF) should trigger shutdown flag.
    #[test]
    fn prop_stdio_eof_shutdown(
        messages in prop::collection::vec(arb_json_message(), 0..5)
    ) {
        let mut transport = StdioTransport::new();

        // Build input with messages followed by EOF
        let mut input = String::new();
        for msg in &messages {
            input.push_str(msg);
            input.push('\n');
        }

        let mut reader = Cursor::new(input);

        // Read all messages
        let read_messages = transport.read_all_messages(&mut reader).unwrap();

        // Should have read all messages
        prop_assert_eq!(read_messages.len(), messages.len(), "Should read all messages");

        // After EOF, shutdown should be signaled
        prop_assert!(transport.is_shutdown(), "EOF should trigger shutdown");
    }

    /// Property 13f: Shutdown handle propagation
    /// Shutdown signal should be visible through the handle.
    #[test]
    fn prop_stdio_shutdown_handle(_seed in any::<u64>()) {
        let transport = StdioTransport::new();
        let handle = transport.shutdown_handle();

        // Initially not shutdown
        prop_assert!(!handle.load(std::sync::atomic::Ordering::SeqCst));

        // Signal shutdown
        transport.shutdown();

        // Handle should reflect shutdown
        prop_assert!(handle.load(std::sync::atomic::Ordering::SeqCst));
        prop_assert!(transport.is_shutdown());
    }
}

// =============================================================================
// Property 13g: Message Size Limits
// Feature: dcp-production, Property 13g: Oversized messages are rejected
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 13g: Message size limit enforcement
    /// Messages exceeding max size should be rejected.
    #[test]
    fn prop_stdio_size_limit(
        size_limit in 10usize..100usize,
        message_size in 1usize..200usize
    ) {
        let mut framer = MessageFramer::new().with_max_size(size_limit);

        // Generate message of specified size (without newline)
        let message: String = (0..message_size).map(|_| 'x').collect();

        let result = framer.feed(message.as_bytes());

        if message_size > size_limit {
            // Should fail for oversized messages
            prop_assert!(result.is_err(), "Oversized message should be rejected");
        } else {
            // Should succeed for valid size (but no complete message without newline)
            prop_assert!(result.is_ok(), "Valid size should be accepted");
            prop_assert!(framer.has_partial(), "Should have partial message");
        }
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_unframe_basic() {
        let msg = r#"{"jsonrpc":"2.0","method":"test"}"#;
        let framed = frame_message(msg);
        assert!(framed.ends_with('\n'));
        assert_eq!(unframe_message(&framed), msg);
    }

    #[test]
    fn test_framer_empty_lines() {
        let mut framer = MessageFramer::new();
        let messages = framer.feed(b"\n\n\n").unwrap();
        assert!(messages.is_empty(), "Empty lines should produce no messages");
    }

    #[test]
    fn test_transport_config() {
        let config = StdioConfig {
            max_message_size: 1024,
            auto_flush: false,
            stderr_logging: false,
            buffer_size: 512,
        };
        let transport = StdioTransport::with_config(config);
        assert!(!transport.is_shutdown());
    }

    #[test]
    fn test_crlf_handling() {
        let msg = "test message\r\n";
        let unframed = unframe_message(msg);
        assert_eq!(unframed, "test message");
    }
}
