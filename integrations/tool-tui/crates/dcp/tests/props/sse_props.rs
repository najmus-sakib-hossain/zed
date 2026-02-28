//! Property-based tests for SSE transport.
//!
//! Feature: dcp-production

use dcp::compat::sse::{
    EventBuffer, SseEndpointConfig, SseEvent, SseEventType, SseMessageHandler, SseTransport,
};
use proptest::prelude::*;

/// Generate arbitrary JSON-like data
fn arb_json_data() -> impl Strategy<Value = String> {
    prop::string::string_regex(r#"[a-zA-Z0-9_\-:,\{\}\[\]"' ]{1,200}"#)
        .unwrap()
        .prop_map(|s| format!(r#"{{"data":"{}"}}"#, s.replace('"', "'")))
}

/// Generate arbitrary event type
fn arb_event_type() -> impl Strategy<Value = SseEventType> {
    prop_oneof![
        Just(SseEventType::Message),
        Just(SseEventType::Endpoint),
        Just(SseEventType::Error),
        Just(SseEventType::Ping),
    ]
}

// =============================================================================
// Property 14: SSE Event Format
// Feature: dcp-production, Property 14: SSE events are correctly formatted
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 14: SSE Event Format
    /// SSE events should be correctly formatted with proper structure.
    #[test]
    fn prop_sse_event_format(
        event_type in arb_event_type(),
        data in arb_json_data(),
        id in prop::option::of(1u64..10000u64),
        retry in prop::option::of(1000u32..60000u32)
    ) {
        let mut event = match event_type {
            SseEventType::Message => SseEvent::message(data.clone()),
            SseEventType::Endpoint => SseEvent::endpoint(data.clone()),
            SseEventType::Error => SseEvent::error(data.clone()),
            SseEventType::Ping => SseEvent::ping(),
        };

        if let Some(id_val) = id {
            event = event.with_id(id_val.to_string());
        }
        if let Some(retry_val) = retry {
            event = event.with_retry(retry_val);
        }

        let formatted = event.format();

        // Must have event type line
        prop_assert!(
            formatted.contains(&format!("event: {}", event_type.as_str())),
            "Event must contain event type"
        );

        // Must have data line(s)
        prop_assert!(formatted.contains("data:"), "Event must contain data line");

        // Must end with double newline
        prop_assert!(formatted.ends_with("\n\n"), "Event must end with double newline");

        // If ID was set, must contain it
        if let Some(id_val) = id {
            prop_assert!(
                formatted.contains(&format!("id: {}", id_val)),
                "Event must contain ID"
            );
        }

        // If retry was set, must contain it
        if let Some(retry_val) = retry {
            prop_assert!(
                formatted.contains(&format!("retry: {}", retry_val)),
                "Event must contain retry"
            );
        }
    }

    /// Property 14b: Multiline data handling
    /// SSE events with multiline data should have each line prefixed with "data: ".
    #[test]
    fn prop_sse_multiline_data(lines in prop::collection::vec("[a-zA-Z0-9 ]{1,50}", 1..5)) {
        let data = lines.join("\n");
        let event = SseEvent::message(data.clone());
        let formatted = event.format();

        // Each line should be prefixed with "data: "
        for line in lines.iter() {
            prop_assert!(
                formatted.contains(&format!("data: {}", line)),
                "Each line should be prefixed with 'data: '"
            );
        }
    }

    /// Property 14c: Event type string consistency
    /// Event type strings should be consistent.
    #[test]
    fn prop_sse_event_type_consistency(event_type in arb_event_type()) {
        let type_str = event_type.as_str();

        // Type string should be non-empty
        prop_assert!(!type_str.is_empty());

        // Type string should be lowercase
        prop_assert_eq!(type_str, type_str.to_lowercase());

        // Type string should match expected values
        let valid_types = ["message", "endpoint", "error", "ping"];
        prop_assert!(valid_types.contains(&type_str));
    }
}

// =============================================================================
// Property 15: SSE Reconnection Replay
// Feature: dcp-production, Property 15: Events are correctly replayed on reconnection
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 15: SSE Reconnection Replay
    /// Events after Last-Event-ID should be correctly replayed.
    #[test]
    fn prop_sse_reconnection_replay(
        num_events in 1usize..20usize,
        last_event_id in 0u64..15u64
    ) {
        let transport = SseTransport::with_buffer_size(50);

        // Send events
        for i in 0..num_events {
            transport.format_response(&format!(r#"{{"id":{}}}"#, i));
        }

        // Get replay events
        let replay = transport.get_replay_events(last_event_id);

        // Calculate expected replay count
        let expected_count = if last_event_id as usize >= num_events {
            0
        } else {
            num_events - last_event_id as usize
        };

        prop_assert_eq!(
            replay.len(),
            expected_count,
            "Replay should contain events after last_event_id"
        );
    }

    /// Property 15b: Event buffer respects max size
    /// Buffer should not exceed max size.
    #[test]
    fn prop_sse_buffer_max_size(
        buffer_size in 5usize..20usize,
        num_events in 1usize..50usize
    ) {
        let mut buffer = EventBuffer::new(buffer_size);

        for i in 0..num_events {
            buffer.push(i as u64, format!("event{}", i));
        }

        prop_assert!(
            buffer.len() <= buffer_size,
            "Buffer should not exceed max size"
        );
    }

    /// Property 15c: Buffer maintains order
    /// Events should be returned in order.
    #[test]
    fn prop_sse_buffer_order(num_events in 1usize..20usize) {
        let mut buffer = EventBuffer::new(100);

        for i in 0..num_events {
            buffer.push(i as u64, format!("event{}", i));
        }

        let events = buffer.events_after(0);

        // Events should be in order (excluding first which is ID 0)
        for (idx, event) in events.iter().enumerate() {
            let expected = format!("event{}", idx + 1);
            prop_assert_eq!(event, &expected, "Events should be in order");
        }
    }

    /// Property 15d: Latest ID tracking
    /// Latest ID should be the last ID pushed.
    #[test]
    fn prop_sse_latest_id(events in prop::collection::vec(1u64..1000u64, 1..20)) {
        let mut buffer = EventBuffer::new(100);

        let mut last_id = 0u64;
        for id in &events {
            buffer.push(*id, format!("event{}", id));
            last_id = *id;
        }

        prop_assert_eq!(
            buffer.latest_id(),
            Some(last_id),
            "Latest ID should be the last ID pushed"
        );
    }
}

// =============================================================================
// Property 15e: Transport Event ID Sequencing
// Feature: dcp-production, Property 15e: Event IDs are sequential
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 15e: Event IDs are sequential
    /// Event IDs should increment sequentially.
    #[test]
    fn prop_sse_sequential_ids(num_events in 1usize..20usize) {
        let transport = SseTransport::new();

        let mut prev_id = 0u64;
        for _ in 0..num_events {
            let formatted = transport.format_response(r#"{"test":true}"#);

            // Extract ID from formatted event
            let id_line = formatted.lines()
                .find(|l| l.starts_with("id: "))
                .expect("Event should have ID");
            let id: u64 = id_line.strip_prefix("id: ")
                .unwrap()
                .parse()
                .expect("ID should be numeric");

            prop_assert!(id > prev_id, "IDs should be sequential");
            prev_id = id;
        }
    }

    /// Property 15f: Parse Last-Event-ID
    /// Last-Event-ID header should be correctly parsed.
    #[test]
    fn prop_sse_parse_last_event_id(id in 0u64..u64::MAX) {
        let header = id.to_string();
        let parsed = SseTransport::parse_last_event_id(&header);

        prop_assert_eq!(parsed, Some(id), "Should parse valid ID");
    }

    /// Property 15g: Parse Last-Event-ID with whitespace
    /// Last-Event-ID with whitespace should be correctly parsed.
    #[test]
    fn prop_sse_parse_last_event_id_whitespace(
        id in 0u64..1000000u64,
        leading_spaces in 0usize..5usize,
        trailing_spaces in 0usize..5usize
    ) {
        let header = format!(
            "{}{}{}",
            " ".repeat(leading_spaces),
            id,
            " ".repeat(trailing_spaces)
        );
        let parsed = SseTransport::parse_last_event_id(&header);

        prop_assert_eq!(parsed, Some(id), "Should parse ID with whitespace");
    }
}

// =============================================================================
// Property: Message Handler
// Feature: dcp-production, Message handler validates JSON
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Message handler accepts valid JSON
    #[test]
    fn prop_sse_message_handler_valid_json(data in arb_json_data()) {
        let handler = SseMessageHandler::new();
        let result = handler.handle_message(&data);

        prop_assert!(result.is_ok(), "Valid JSON should be accepted");
        prop_assert!(handler.has_pending(), "Should have pending response");
    }

    /// Message handler rejects invalid JSON
    #[test]
    fn prop_sse_message_handler_invalid_json(
        data in "[a-zA-Z]{5,20}"
    ) {
        let handler = SseMessageHandler::new();
        let result = handler.handle_message(&data);

        prop_assert!(result.is_err(), "Invalid JSON should be rejected");
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_headers() {
        let headers = SseTransport::headers();
        assert!(headers.iter().any(|(k, v)| *k == "Content-Type" && *v == "text/event-stream"));
        assert!(headers.iter().any(|(k, v)| *k == "Cache-Control" && *v == "no-cache"));
    }

    #[test]
    fn test_sse_endpoint_config_default() {
        let config = SseEndpointConfig::default();
        assert_eq!(config.events_path, "/events");
        assert_eq!(config.messages_path, "/message");
        assert_eq!(config.ping_interval_secs, 30);
    }

    #[test]
    fn test_sse_transport_reset() {
        let transport = SseTransport::new();
        transport.format_response(r#"{"test":1}"#);
        transport.format_response(r#"{"test":2}"#);

        let (len, _) = transport.buffer_stats();
        assert_eq!(len, 2);

        transport.reset();

        let (len, _) = transport.buffer_stats();
        assert_eq!(len, 0);
        assert_eq!(transport.current_event_id(), 0);
    }

    #[test]
    fn test_event_numeric_id() {
        let event = SseEvent::message("test".to_string()).with_id("42".to_string());
        assert_eq!(event.numeric_id(), Some(42));

        let event2 = SseEvent::message("test".to_string()).with_id("invalid".to_string());
        assert_eq!(event2.numeric_id(), None);

        let event3 = SseEvent::message("test".to_string());
        assert_eq!(event3.numeric_id(), None);
    }
}
