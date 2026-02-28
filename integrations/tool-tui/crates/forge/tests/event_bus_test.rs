//! Unit tests for EventBus
//!
//! Comprehensive tests covering:
//! - Basic publish/subscribe functionality
//! - Multiple subscribers receiving events
//! - Different event types (ToolStarted, ToolCompleted, FileChanged, etc.)
//! - Subscriber management (adding/removing subscribers)
//! - Edge cases and error conditions
//!
//! **Validates: Requirements 5.1, 5.3**

use dx_forge::core::event_bus::{EventBus, ForgeEvent};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

// ============================================================================
// Basic Publish/Subscribe Tests
// ============================================================================

mod basic_publish_subscribe {
    use super::*;

    #[tokio::test]
    async fn test_publish_and_receive_single_event() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_tool_started("test-tool").unwrap();

        let event = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("Timeout waiting for event")
            .expect("Failed to receive event");

        match event {
            ForgeEvent::ToolStarted { tool_id, .. } => {
                assert_eq!(tool_id, "test-tool");
            }
            _ => panic!("Expected ToolStarted event, got {:?}", event),
        }
    }

    #[tokio::test]
    async fn test_publish_returns_ok() {
        let event_bus = EventBus::new();

        let result = event_bus.emit_tool_started("test-tool");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscribe_before_publish() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        // Subscribe first, then publish
        event_bus.emit_tool_completed("tool-1", 100).unwrap();

        let event = timeout(Duration::from_secs(1), receiver.recv())
            .await
            .expect("Timeout")
            .expect("Failed to receive");

        match event {
            ForgeEvent::ToolCompleted {
                tool_id,
                duration_ms,
                ..
            } => {
                assert_eq!(tool_id, "tool-1");
                assert_eq!(duration_ms, 100);
            }
            _ => panic!("Expected ToolCompleted event"),
        }
    }

    #[tokio::test]
    async fn test_publish_without_subscribers() {
        let event_bus = EventBus::new();

        // Publishing without any subscribers should not error
        let result = event_bus.emit_tool_started("orphan-event");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_events_in_sequence() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_tool_started("tool-1").unwrap();
        event_bus.emit_tool_completed("tool-1", 50).unwrap();
        event_bus.emit_tool_started("tool-2").unwrap();

        // Receive all three events in order
        let event1 = receiver.recv().await.unwrap();
        let event2 = receiver.recv().await.unwrap();
        let event3 = receiver.recv().await.unwrap();

        match event1 {
            ForgeEvent::ToolStarted { tool_id, .. } => assert_eq!(tool_id, "tool-1"),
            _ => panic!("Expected ToolStarted"),
        }

        match event2 {
            ForgeEvent::ToolCompleted {
                tool_id,
                duration_ms,
                ..
            } => {
                assert_eq!(tool_id, "tool-1");
                assert_eq!(duration_ms, 50);
            }
            _ => panic!("Expected ToolCompleted"),
        }

        match event3 {
            ForgeEvent::ToolStarted { tool_id, .. } => assert_eq!(tool_id, "tool-2"),
            _ => panic!("Expected ToolStarted"),
        }
    }
}

// ============================================================================
// Multiple Subscribers Tests
// ============================================================================

mod multiple_subscribers {
    use super::*;

    #[tokio::test]
    async fn test_multiple_global_subscribers_receive_same_event() {
        let event_bus = EventBus::new();
        let mut receiver1 = event_bus.subscribe();
        let mut receiver2 = event_bus.subscribe();
        let mut receiver3 = event_bus.subscribe();

        event_bus.emit_tool_started("shared-tool").unwrap();

        // All subscribers should receive the event
        let event1 = timeout(Duration::from_secs(1), receiver1.recv()).await.unwrap().unwrap();
        let event2 = timeout(Duration::from_secs(1), receiver2.recv()).await.unwrap().unwrap();
        let event3 = timeout(Duration::from_secs(1), receiver3.recv()).await.unwrap().unwrap();

        for event in [event1, event2, event3] {
            match event {
                ForgeEvent::ToolStarted { tool_id, .. } => {
                    assert_eq!(tool_id, "shared-tool");
                }
                _ => panic!("Expected ToolStarted event"),
            }
        }
    }

    #[tokio::test]
    async fn test_type_specific_subscriber_only_receives_matching_events() {
        let mut event_bus = EventBus::new();
        let mut tool_started_receiver = event_bus.subscribe_to_type("tool_started");

        // Publish different event types
        event_bus.emit_tool_started("my-tool").unwrap();
        event_bus.emit_pipeline_started("my-pipeline").unwrap();
        event_bus.emit_tool_completed("my-tool", 100).unwrap();

        // Type-specific subscriber should only receive tool_started
        let event = timeout(Duration::from_secs(1), tool_started_receiver.recv())
            .await
            .unwrap()
            .unwrap();

        match event {
            ForgeEvent::ToolStarted { tool_id, .. } => {
                assert_eq!(tool_id, "my-tool");
            }
            _ => panic!("Expected ToolStarted event"),
        }

        // Should not receive other event types
        let result = tool_started_receiver.try_recv();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_type_specific_subscribers_same_type() {
        let mut event_bus = EventBus::new();
        let mut receiver1 = event_bus.subscribe_to_type("tool_completed");
        let mut receiver2 = event_bus.subscribe_to_type("tool_completed");

        event_bus.emit_tool_completed("tool-x", 200).unwrap();

        // Both type-specific subscribers should receive the event
        let event1 = timeout(Duration::from_secs(1), receiver1.recv()).await.unwrap().unwrap();
        let event2 = timeout(Duration::from_secs(1), receiver2.recv()).await.unwrap().unwrap();

        for event in [event1, event2] {
            match event {
                ForgeEvent::ToolCompleted {
                    tool_id,
                    duration_ms,
                    ..
                } => {
                    assert_eq!(tool_id, "tool-x");
                    assert_eq!(duration_ms, 200);
                }
                _ => panic!("Expected ToolCompleted event"),
            }
        }
    }

    #[tokio::test]
    async fn test_global_and_type_specific_subscribers_together() {
        let mut event_bus = EventBus::new();
        let mut global_receiver = event_bus.subscribe();
        let mut type_receiver = event_bus.subscribe_to_type("pipeline_started");

        event_bus.emit_pipeline_started("pipeline-1").unwrap();

        // Both should receive the event
        let global_event =
            timeout(Duration::from_secs(1), global_receiver.recv()).await.unwrap().unwrap();
        let type_event =
            timeout(Duration::from_secs(1), type_receiver.recv()).await.unwrap().unwrap();

        match global_event {
            ForgeEvent::PipelineStarted { pipeline_id, .. } => {
                assert_eq!(pipeline_id, "pipeline-1");
            }
            _ => panic!("Expected PipelineStarted"),
        }

        match type_event {
            ForgeEvent::PipelineStarted { pipeline_id, .. } => {
                assert_eq!(pipeline_id, "pipeline-1");
            }
            _ => panic!("Expected PipelineStarted"),
        }
    }
}

// ============================================================================
// Different Event Types Tests
// ============================================================================

mod event_types {
    use super::*;

    #[tokio::test]
    async fn test_tool_started_event() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_tool_started("formatter").unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolStarted { tool_id, timestamp } => {
                assert_eq!(tool_id, "formatter");
                assert!(timestamp > 0);
            }
            _ => panic!("Expected ToolStarted"),
        }
    }

    #[tokio::test]
    async fn test_tool_completed_event() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_tool_completed("linter", 1500).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolCompleted {
                tool_id,
                duration_ms,
                timestamp,
            } => {
                assert_eq!(tool_id, "linter");
                assert_eq!(duration_ms, 1500);
                assert!(timestamp > 0);
            }
            _ => panic!("Expected ToolCompleted"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_started_event() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_pipeline_started("build-pipeline").unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::PipelineStarted {
                pipeline_id,
                timestamp,
            } => {
                assert_eq!(pipeline_id, "build-pipeline");
                assert!(timestamp > 0);
            }
            _ => panic!("Expected PipelineStarted"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_completed_event() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_pipeline_completed("deploy-pipeline", 30000).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::PipelineCompleted {
                pipeline_id,
                duration_ms,
                timestamp,
            } => {
                assert_eq!(pipeline_id, "deploy-pipeline");
                assert_eq!(duration_ms, 30000);
                assert!(timestamp > 0);
            }
            _ => panic!("Expected PipelineCompleted"),
        }
    }

    #[tokio::test]
    async fn test_package_installation_begin_event() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_package_installation_begin("lodash@4.17.21").unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::PackageInstallationBegin {
                package_id,
                timestamp,
            } => {
                assert_eq!(package_id, "lodash@4.17.21");
                assert!(timestamp > 0);
            }
            _ => panic!("Expected PackageInstallationBegin"),
        }
    }

    #[tokio::test]
    async fn test_package_installation_success_event() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_package_installation_success("react@18.2.0").unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::PackageInstallationSuccess {
                package_id,
                timestamp,
            } => {
                assert_eq!(package_id, "react@18.2.0");
                assert!(timestamp > 0);
            }
            _ => panic!("Expected PackageInstallationSuccess"),
        }
    }

    #[tokio::test]
    async fn test_security_violation_detected_event() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus
            .emit_security_violation_detected("SQL injection vulnerability detected", "critical")
            .unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::SecurityViolationDetected {
                description,
                severity,
                timestamp,
            } => {
                assert_eq!(description, "SQL injection vulnerability detected");
                assert_eq!(severity, "critical");
                assert!(timestamp > 0);
            }
            _ => panic!("Expected SecurityViolationDetected"),
        }
    }

    #[tokio::test]
    async fn test_magical_config_injection_event() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_magical_config_injection("database").unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::MagicalConfigInjection {
                config_section,
                timestamp,
            } => {
                assert_eq!(config_section, "database");
                assert!(timestamp > 0);
            }
            _ => panic!("Expected MagicalConfigInjection"),
        }
    }

    #[tokio::test]
    async fn test_custom_event() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        let custom_data = json!({
            "key": "value",
            "count": 42,
            "nested": {"inner": true}
        });

        event_bus.emit_custom("my_custom_event", custom_data.clone()).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::Custom {
                event_type,
                data,
                timestamp,
            } => {
                assert_eq!(event_type, "my_custom_event");
                assert_eq!(data, custom_data);
                assert!(timestamp > 0);
            }
            _ => panic!("Expected Custom event"),
        }
    }
}

// ============================================================================
// Edge Cases and Error Conditions Tests
// ============================================================================

mod edge_cases {
    use super::*;

    #[tokio::test]
    async fn test_empty_tool_id() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_tool_started("").unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolStarted { tool_id, .. } => {
                assert_eq!(tool_id, "");
            }
            _ => panic!("Expected ToolStarted"),
        }
    }

    #[tokio::test]
    async fn test_very_long_tool_id() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        let long_id = "a".repeat(10000);
        event_bus.emit_tool_started(&long_id).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolStarted { tool_id, .. } => {
                assert_eq!(tool_id.len(), 10000);
            }
            _ => panic!("Expected ToolStarted"),
        }
    }

    #[tokio::test]
    async fn test_special_characters_in_tool_id() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        let special_id = "tool-with-special-chars!@#$%^&*()_+-=[]{}|;':\",./<>?";
        event_bus.emit_tool_started(special_id).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolStarted { tool_id, .. } => {
                assert_eq!(tool_id, special_id);
            }
            _ => panic!("Expected ToolStarted"),
        }
    }

    #[tokio::test]
    async fn test_unicode_in_event_data() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        let unicode_id = "工具-🔧-инструмент";
        event_bus.emit_tool_started(unicode_id).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolStarted { tool_id, .. } => {
                assert_eq!(tool_id, unicode_id);
            }
            _ => panic!("Expected ToolStarted"),
        }
    }

    #[tokio::test]
    async fn test_zero_duration() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_tool_completed("instant-tool", 0).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolCompleted { duration_ms, .. } => {
                assert_eq!(duration_ms, 0);
            }
            _ => panic!("Expected ToolCompleted"),
        }
    }

    #[tokio::test]
    async fn test_max_duration() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_tool_completed("long-running-tool", u64::MAX).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolCompleted { duration_ms, .. } => {
                assert_eq!(duration_ms, u64::MAX);
            }
            _ => panic!("Expected ToolCompleted"),
        }
    }

    #[tokio::test]
    async fn test_custom_event_with_empty_data() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_custom("empty_event", json!({})).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::Custom { data, .. } => {
                assert_eq!(data, json!({}));
            }
            _ => panic!("Expected Custom event"),
        }
    }

    #[tokio::test]
    async fn test_custom_event_with_null_data() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_custom("null_event", json!(null)).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::Custom { data, .. } => {
                assert_eq!(data, json!(null));
            }
            _ => panic!("Expected Custom event"),
        }
    }

    #[tokio::test]
    async fn test_custom_event_with_array_data() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        let array_data = json!([1, 2, 3, "four", {"five": 5}]);
        event_bus.emit_custom("array_event", array_data.clone()).unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::Custom { data, .. } => {
                assert_eq!(data, array_data);
            }
            _ => panic!("Expected Custom event"),
        }
    }

    #[tokio::test]
    async fn test_subscribe_to_nonexistent_type() {
        let mut event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe_to_type("nonexistent_event_type");

        // Publish a different event type
        event_bus.emit_tool_started("tool").unwrap();

        // Should not receive anything
        let result = receiver.try_recv();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_custom_event_type_routing() {
        let mut event_bus = EventBus::new();
        let mut custom_receiver = event_bus.subscribe_to_type("my_custom_type");

        // Custom events should route to their event_type
        event_bus.emit_custom("my_custom_type", json!({"test": true})).unwrap();

        let event = timeout(Duration::from_secs(1), custom_receiver.recv()).await.unwrap().unwrap();

        match event {
            ForgeEvent::Custom { event_type, .. } => {
                assert_eq!(event_type, "my_custom_type");
            }
            _ => panic!("Expected Custom event"),
        }
    }
}

// ============================================================================
// Default Implementation Tests
// ============================================================================

mod default_impl {
    use super::*;

    #[test]
    fn test_event_bus_default() {
        let event_bus = EventBus::default();
        // Should be able to use the default-constructed event bus
        let result = event_bus.emit_tool_started("test");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_default_event_bus_works_same_as_new() {
        let event_bus = EventBus::default();
        let mut receiver = event_bus.subscribe();

        event_bus.emit_tool_started("default-test").unwrap();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolStarted { tool_id, .. } => {
                assert_eq!(tool_id, "default-test");
            }
            _ => panic!("Expected ToolStarted"),
        }
    }
}

// ============================================================================
// Event Serialization Tests
// ============================================================================

mod serialization {
    use super::*;

    #[test]
    fn test_forge_event_serialization_tool_started() {
        let event = ForgeEvent::ToolStarted {
            tool_id: "test-tool".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ForgeEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            ForgeEvent::ToolStarted { tool_id, timestamp } => {
                assert_eq!(tool_id, "test-tool");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Deserialization failed"),
        }
    }

    #[test]
    fn test_forge_event_serialization_tool_completed() {
        let event = ForgeEvent::ToolCompleted {
            tool_id: "completed-tool".to_string(),
            duration_ms: 5000,
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ForgeEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            ForgeEvent::ToolCompleted {
                tool_id,
                duration_ms,
                timestamp,
            } => {
                assert_eq!(tool_id, "completed-tool");
                assert_eq!(duration_ms, 5000);
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Deserialization failed"),
        }
    }

    #[test]
    fn test_forge_event_serialization_security_violation() {
        let event = ForgeEvent::SecurityViolationDetected {
            description: "XSS vulnerability".to_string(),
            severity: "high".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ForgeEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            ForgeEvent::SecurityViolationDetected {
                description,
                severity,
                timestamp,
            } => {
                assert_eq!(description, "XSS vulnerability");
                assert_eq!(severity, "high");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Deserialization failed"),
        }
    }

    #[test]
    fn test_forge_event_serialization_custom() {
        let custom_data = json!({"key": "value", "number": 42});
        let event = ForgeEvent::Custom {
            event_type: "custom_type".to_string(),
            data: custom_data.clone(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ForgeEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            ForgeEvent::Custom {
                event_type,
                data,
                timestamp,
            } => {
                assert_eq!(event_type, "custom_type");
                assert_eq!(data, custom_data);
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Deserialization failed"),
        }
    }

    #[test]
    fn test_forge_event_clone() {
        let event = ForgeEvent::ToolStarted {
            tool_id: "clone-test".to_string(),
            timestamp: 1234567890,
        };

        let cloned = event.clone();

        match cloned {
            ForgeEvent::ToolStarted { tool_id, timestamp } => {
                assert_eq!(tool_id, "clone-test");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_forge_event_debug() {
        let event = ForgeEvent::ToolStarted {
            tool_id: "debug-test".to_string(),
            timestamp: 1234567890,
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("ToolStarted"));
        assert!(debug_str.contains("debug-test"));
    }
}

// ============================================================================
// High Volume Tests
// ============================================================================

mod high_volume {
    use super::*;

    #[tokio::test]
    async fn test_many_events_in_sequence() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        // Publish 100 events
        for i in 0..100 {
            event_bus.emit_tool_started(&format!("tool-{}", i)).unwrap();
        }

        // Receive all 100 events
        for i in 0..100 {
            let event = timeout(Duration::from_secs(5), receiver.recv())
                .await
                .expect("Timeout")
                .expect("Failed to receive");

            match event {
                ForgeEvent::ToolStarted { tool_id, .. } => {
                    assert_eq!(tool_id, format!("tool-{}", i));
                }
                _ => panic!("Expected ToolStarted"),
            }
        }
    }

    #[tokio::test]
    async fn test_many_subscribers() {
        let event_bus = EventBus::new();

        // Create 50 subscribers
        let mut receivers: Vec<_> = (0..50).map(|_| event_bus.subscribe()).collect();

        event_bus.emit_tool_started("broadcast-tool").unwrap();

        // All 50 subscribers should receive the event
        for receiver in &mut receivers {
            let event = timeout(Duration::from_secs(5), receiver.recv())
                .await
                .expect("Timeout")
                .expect("Failed to receive");

            match event {
                ForgeEvent::ToolStarted { tool_id, .. } => {
                    assert_eq!(tool_id, "broadcast-tool");
                }
                _ => panic!("Expected ToolStarted"),
            }
        }
    }

    #[tokio::test]
    async fn test_mixed_event_types_high_volume() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        // Publish a mix of event types
        for i in 0..50 {
            match i % 5 {
                0 => event_bus.emit_tool_started(&format!("tool-{}", i)).unwrap(),
                1 => event_bus.emit_tool_completed(&format!("tool-{}", i), i as u64 * 10).unwrap(),
                2 => event_bus.emit_pipeline_started(&format!("pipeline-{}", i)).unwrap(),
                3 => event_bus
                    .emit_pipeline_completed(&format!("pipeline-{}", i), i as u64 * 100)
                    .unwrap(),
                4 => event_bus.emit_custom(&format!("custom-{}", i), json!({"index": i})).unwrap(),
                _ => unreachable!(),
            }
        }

        // Receive all 50 events
        for _ in 0..50 {
            let event = timeout(Duration::from_secs(5), receiver.recv())
                .await
                .expect("Timeout")
                .expect("Failed to receive");

            // Just verify we received something valid
            match event {
                ForgeEvent::ToolStarted { .. }
                | ForgeEvent::ToolCompleted { .. }
                | ForgeEvent::PipelineStarted { .. }
                | ForgeEvent::PipelineCompleted { .. }
                | ForgeEvent::Custom { .. } => {}
                _ => panic!("Unexpected event type"),
            }
        }
    }
}

// ============================================================================
// Timestamp Tests
// ============================================================================

mod timestamps {
    use super::*;

    #[tokio::test]
    async fn test_timestamp_is_current() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        let before = chrono::Utc::now().timestamp();
        event_bus.emit_tool_started("timestamp-test").unwrap();
        let after = chrono::Utc::now().timestamp();

        let event = receiver.recv().await.unwrap();
        match event {
            ForgeEvent::ToolStarted { timestamp, .. } => {
                assert!(timestamp >= before);
                assert!(timestamp <= after);
            }
            _ => panic!("Expected ToolStarted"),
        }
    }

    #[tokio::test]
    async fn test_timestamps_are_monotonic() {
        let event_bus = EventBus::new();
        let mut receiver = event_bus.subscribe();

        // Emit multiple events
        for _ in 0..10 {
            event_bus.emit_tool_started("monotonic-test").unwrap();
        }

        let mut last_timestamp = 0i64;
        for _ in 0..10 {
            let event = receiver.recv().await.unwrap();
            match event {
                ForgeEvent::ToolStarted { timestamp, .. } => {
                    assert!(timestamp >= last_timestamp);
                    last_timestamp = timestamp;
                }
                _ => panic!("Expected ToolStarted"),
            }
        }
    }
}
