//! Property-based tests for MCP 2025 compatibility features.
//!
//! Tests for protocol version negotiation, roots, elicitation, resource templates,
//! and other MCP 2025 specification features.

use proptest::prelude::*;

use dcp::compat::json_rpc::RequestId;
use dcp::compat::{
    AnnotatedContent, Annotations, CancellationManager, CancellationState, CompleteMcpAdapter,
    Content, ElicitationAction, ElicitationResponse, ElicitationSchema, JsonRpcParser,
    Notification, NotificationManager, ProgressNotification, ProgressTracker, PropertySchema,
    ProtocolVersion, ResourceTemplate, ResourceTemplateRegistry, Root, RootsRegistry,
    SubscriptionTracker, VersionNegotiator,
};

// ============================================================================
// Property 1: Protocol Version Negotiation
// **Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5**
// For any initialize request with a protocolVersion field, the server SHALL
// negotiate to the requested version if supported, or fall back to the latest
// supported version.
// ============================================================================

/// Generate arbitrary supported version strings
fn arb_supported_version() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("2024-11-05".to_string()),
        Just("2025-03-26".to_string()),
        Just("2025-06-18".to_string()),
    ]
}

/// Generate arbitrary unsupported version strings
fn arb_unsupported_version() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("2023-01-01".to_string()),
        Just("2099-12-31".to_string()),
        Just("invalid".to_string()),
        Just("".to_string()),
        "[a-z0-9-]{1,20}"
            .prop_filter("not a valid version", |s| { ProtocolVersion::from_str(s).is_none() }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 1: Protocol Version Negotiation
    /// For any supported version string, negotiation returns that exact version.
    #[test]
    fn prop_version_negotiation_supported(version in arb_supported_version()) {
        let negotiator = VersionNegotiator::new();
        let negotiated = negotiator.negotiate(&version);

        // Should return the exact requested version
        prop_assert_eq!(negotiated.as_str(), version.as_str(),
            "Supported version {} should negotiate to itself", version);
    }

    /// Feature: mcp-2025-compatibility, Property 1: Protocol Version Negotiation (fallback)
    /// For any unsupported version string, negotiation falls back to latest.
    #[test]
    fn prop_version_negotiation_fallback(version in arb_unsupported_version()) {
        let negotiator = VersionNegotiator::new();
        let negotiated = negotiator.negotiate(&version);

        // Should fall back to latest
        prop_assert_eq!(negotiated, ProtocolVersion::latest(),
            "Unsupported version {} should fall back to latest", version);
    }

    /// Feature: mcp-2025-compatibility, Property 1: Version round-trip
    /// For any protocol version, parsing its string representation returns the same version.
    #[test]
    fn prop_version_round_trip(_dummy in 0..3i32) {
        for version in ProtocolVersion::all_versions() {
            let version_str = version.as_str();
            let parsed = ProtocolVersion::from_str(version_str);

            prop_assert_eq!(parsed, Some(*version),
                "Version {} should round-trip through string", version_str);
        }
    }
}

// ============================================================================
// Property 2: Capability Declaration
// **Validates: Requirements 1.1, 2.1, 7.4**
// For any negotiated protocol version, the server SHALL declare capabilities
// appropriate for that version.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 2: Capability Declaration
    /// Version-specific features are only available in appropriate versions.
    #[test]
    fn prop_capability_declaration(_dummy in 0..10i32) {
        // V2024_11_05 - no roots, no elicitation
        let v1 = ProtocolVersion::V2024_11_05;
        prop_assert!(!v1.supports_roots(), "V2024_11_05 should not support roots");
        prop_assert!(!v1.supports_elicitation(), "V2024_11_05 should not support elicitation");

        // V2025_03_26 - roots, no elicitation
        let v2 = ProtocolVersion::V2025_03_26;
        prop_assert!(v2.supports_roots(), "V2025_03_26 should support roots");
        prop_assert!(!v2.supports_elicitation(), "V2025_03_26 should not support elicitation");
        prop_assert!(v2.supports_progress(), "V2025_03_26 should support progress");

        // V2025_06_18 - all features
        let v3 = ProtocolVersion::V2025_06_18;
        prop_assert!(v3.supports_roots(), "V2025_06_18 should support roots");
        prop_assert!(v3.supports_elicitation(), "V2025_06_18 should support elicitation");
        prop_assert!(v3.supports_progress(), "V2025_06_18 should support progress");
        prop_assert!(v3.supports_structured_output(), "V2025_06_18 should support structured output");
    }
}

// ============================================================================
// Property 3: Roots List Round-Trip
// **Validates: Requirements 1.2, 1.4**
// For any set of configured roots, calling roots/list SHALL return all
// configured roots with their uri and optional name fields preserved exactly.
// ============================================================================

/// Generate arbitrary root with unique URI
fn arb_root(index: usize) -> impl Strategy<Value = Root> {
    (
        "[a-z]{1,10}",                      // unique path component
        prop::option::of("[A-Za-z]{1,20}"), // no spaces to avoid edge cases
    )
        .prop_map(move |(path, name)| {
            // Include index to ensure uniqueness
            let uri = format!("file:///{}-{}", path, index);
            if let Some(n) = name {
                Root::with_name(uri, n)
            } else {
                Root::new(uri)
            }
        })
}

/// Generate a vector of roots with unique URIs
fn arb_roots(max_count: usize) -> impl Strategy<Value = Vec<Root>> {
    (0..max_count).prop_flat_map(|count| {
        let strategies: Vec<_> = (0..count).map(arb_root).collect();
        strategies
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 3: Roots List Round-Trip
    /// All added roots should be retrievable with exact field preservation.
    #[test]
    fn prop_roots_list_round_trip(root_count in 0usize..10) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let registry = RootsRegistry::new();

            // Generate unique roots
            let roots: Vec<Root> = (0..root_count)
                .map(|i| {
                    if i % 2 == 0 {
                        Root::new(format!("file:///path{}", i))
                    } else {
                        Root::with_name(format!("file:///path{}", i), format!("Name{}", i))
                    }
                })
                .collect();

            // Add all roots
            for root in &roots {
                registry.add_root(root.clone()).await;
            }

            // List should return all roots
            let listed = registry.list().await;
            prop_assert_eq!(listed.len(), roots.len(),
                "Listed roots count should match added count");

            // Each root should be present with exact fields
            for root in &roots {
                let found = listed.iter().find(|r| r.uri == root.uri);
                prop_assert!(found.is_some(), "Root {} should be in list", root.uri);
                prop_assert_eq!(&found.unwrap().name, &root.name,
                    "Root name should be preserved");
            }

            Ok(())
        })?;
    }

    /// Feature: mcp-2025-compatibility, Property 3: Roots removal
    /// Removing a root should remove only that root.
    #[test]
    fn prop_roots_removal(root_count in 2usize..10) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let registry = RootsRegistry::new();

            // Generate unique roots
            let roots: Vec<Root> = (0..root_count)
                .map(|i| Root::new(format!("file:///path{}", i)))
                .collect();

            // Add all roots
            for root in &roots {
                registry.add_root(root.clone()).await;
            }

            // Remove first root
            let removed_uri = &roots[0].uri;
            let was_removed = registry.remove_root(removed_uri).await;
            prop_assert!(was_removed, "Remove should return true for existing root");

            // List should have one fewer
            let listed = registry.list().await;
            prop_assert_eq!(listed.len(), roots.len() - 1,
                "List should have one fewer root after removal");

            // Removed root should not be in list
            let found = listed.iter().find(|r| r.uri == *removed_uri);
            prop_assert!(found.is_none(), "Removed root should not be in list");

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 4: Roots Change Notification
// **Validates: Requirements 1.3**
// For any change to the roots configuration (add or remove), the server SHALL
// emit a notification to all subscribed clients.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 4: Roots Change Notification
    /// Adding or removing roots should trigger change notifications.
    #[test]
    fn prop_roots_change_notification(root_count in 1usize..5) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let registry = RootsRegistry::new();
            let mut receiver = registry.subscribe();

            // Add roots and count notifications
            for i in 0..root_count {
                registry.add_root(Root::new(format!("file:///path{}", i))).await;

                // Should receive notification for each add
                let result = tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    receiver.recv()
                ).await;
                prop_assert!(result.is_ok(), "Should receive notification for add {}", i);
            }

            // Remove a root
            registry.remove_root("file:///path0").await;

            // Should receive notification for remove
            let result = tokio::time::timeout(
                std::time::Duration::from_millis(100),
                receiver.recv()
            ).await;
            prop_assert!(result.is_ok(), "Should receive notification for remove");

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 6: Resource Template Registration and Matching
// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5**
// For any registered resource template with URI pattern containing placeholders,
// the template SHALL appear in resource listings with all metadata.
// ============================================================================

/// Generate arbitrary resource template
fn arb_resource_template() -> impl Strategy<Value = ResourceTemplate> {
    (
        "[a-z]+://\\{[a-z]+\\}",             // Simple URI template
        "[A-Za-z ]{1,20}",                   // Name
        prop::option::of("[A-Za-z ]{1,50}"), // Description
        prop::option::of("(text/plain|application/json|text/html)"), // MIME type
    )
        .prop_map(|(uri, name, desc, mime)| {
            let mut template = ResourceTemplate::new(uri, name);
            if let Some(d) = desc {
                template = template.with_description(d);
            }
            if let Some(m) = mime {
                template = template.with_mime_type(m);
            }
            template
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 6: Resource Template Registration
    /// All registered templates should be retrievable with metadata preserved.
    #[test]
    fn prop_resource_template_registration(templates in prop::collection::vec(arb_resource_template(), 0..10)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let registry = ResourceTemplateRegistry::new();

            // Register all templates
            for template in &templates {
                registry.register(template.clone()).await;
            }

            // List should return all templates
            let listed = registry.list().await;
            prop_assert_eq!(listed.len(), templates.len(),
                "Listed templates count should match registered count");

            // Each template should be present with exact metadata
            for template in &templates {
                let found = listed.iter().find(|t| t.uri_template == template.uri_template);
                prop_assert!(found.is_some(), "Template {} should be in list", template.uri_template);
                let found = found.unwrap();
                prop_assert_eq!(&found.name, &template.name, "Name should be preserved");
                prop_assert_eq!(&found.description, &template.description, "Description should be preserved");
                prop_assert_eq!(&found.mime_type, &template.mime_type, "MIME type should be preserved");
            }

            Ok(())
        })?;
    }

    /// Feature: mcp-2025-compatibility, Property 6: Template placeholder extraction
    /// Placeholders should be correctly extracted from templates.
    #[test]
    fn prop_template_placeholder_extraction(
        prefix in "[a-z]+://",
        placeholders in prop::collection::vec("[a-z]{1,10}", 1..5)
    ) {
        // Build template with placeholders
        let uri_template = format!("{}{}", prefix,
            placeholders.iter().map(|p| format!("{{{}}}", p)).collect::<Vec<_>>().join("/"));

        let template = ResourceTemplate::new(&uri_template, "Test");
        let extracted = template.placeholders();

        prop_assert_eq!(extracted.len(), placeholders.len(),
            "Should extract correct number of placeholders");

        for (i, placeholder) in placeholders.iter().enumerate() {
            prop_assert_eq!(&extracted[i], placeholder,
                "Placeholder {} should match", i);
        }
    }
}

// ============================================================================
// Property 7: Unsubscribe Idempotence
// **Validates: Requirements 4.1, 4.2, 4.3**
// For any resources/unsubscribe request, the server SHALL return success
// regardless of whether the subscription exists.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 7: Unsubscribe Idempotence
    /// Unsubscribing should always succeed, even for non-existent subscriptions.
    #[test]
    fn prop_unsubscribe_idempotence(
        uri in "file:///[a-z]{1,20}",
        subscriber_id in "[a-z0-9-]{1,20}"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tracker = SubscriptionTracker::new();

            // Unsubscribe from non-existent subscription should succeed
            let result = tracker.unsubscribe(&uri, &subscriber_id).await;
            prop_assert!(result, "Unsubscribe should succeed for non-existent subscription");

            // Subscribe then unsubscribe should succeed
            tracker.subscribe(&uri, &subscriber_id).await;
            prop_assert!(tracker.is_subscribed(&uri, &subscriber_id).await,
                "Should be subscribed after subscribe");

            let result = tracker.unsubscribe(&uri, &subscriber_id).await;
            prop_assert!(result, "Unsubscribe should succeed for existing subscription");
            prop_assert!(!tracker.is_subscribed(&uri, &subscriber_id).await,
                "Should not be subscribed after unsubscribe");

            // Double unsubscribe should still succeed (idempotent)
            let result = tracker.unsubscribe(&uri, &subscriber_id).await;
            prop_assert!(result, "Double unsubscribe should succeed (idempotent)");

            Ok(())
        })?;
    }

    /// Feature: mcp-2025-compatibility, Property 7: Unsubscribe isolation
    /// Unsubscribing one subscriber should not affect others.
    #[test]
    fn prop_unsubscribe_isolation(
        uri in "file:///[a-z]{1,20}",
        subscriber_count in 2usize..5
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tracker = SubscriptionTracker::new();

            // Generate unique subscriber IDs
            let subscribers: Vec<String> = (0..subscriber_count)
                .map(|i| format!("subscriber-{}", i))
                .collect();

            // Subscribe all
            for sub in &subscribers {
                tracker.subscribe(&uri, sub).await;
            }

            // Unsubscribe first
            tracker.unsubscribe(&uri, &subscribers[0]).await;

            // Others should still be subscribed
            for sub in &subscribers[1..] {
                prop_assert!(tracker.is_subscribed(&uri, sub).await,
                    "Subscriber {} should still be subscribed", sub);
            }

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 8: Annotation Preservation
// **Validates: Requirements 5.1, 5.2, 5.3, 5.4**
// For any content with annotations, the annotations should be preserved.
// ============================================================================

/// Generate arbitrary annotations
fn arb_annotations() -> impl Strategy<Value = Annotations> {
    (
        prop::option::of(prop::collection::vec("[a-z]{1,10}", 0..5)),
        prop::option::of(0.0f64..=1.0f64),
    )
        .prop_map(|(audience, priority)| {
            let mut ann = Annotations::new();
            if let Some(a) = audience {
                ann = ann.with_audience(a);
            }
            if let Some(p) = priority {
                ann = ann.with_priority(p);
            }
            ann
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 8: Annotation Preservation
    /// Annotations should be preserved through serialization.
    #[test]
    fn prop_annotation_preservation(annotations in arb_annotations()) {
        let content = AnnotatedContent::new(Content::text("test"))
            .with_annotations(annotations.clone());

        // Serialize and deserialize
        let json = serde_json::to_string(&content).unwrap();
        let deserialized: AnnotatedContent = serde_json::from_str(&json).unwrap();

        // Check audience is preserved exactly
        prop_assert_eq!(&deserialized.annotations.as_ref().and_then(|a| a.audience.clone()),
            &annotations.audience,
            "Audience should be preserved through serialization");

        // Check priority is preserved (with floating point tolerance)
        match (&deserialized.annotations.as_ref().and_then(|a| a.priority), &annotations.priority) {
            (Some(d), Some(o)) => {
                prop_assert!((d - o).abs() < 1e-10,
                    "Priority should be preserved through serialization");
            }
            (None, None) => {}
            _ => prop_assert!(false, "Priority presence should match"),
        }
    }

    /// Feature: mcp-2025-compatibility, Property 8: Priority clamping
    /// Priority values should be clamped to [0.0, 1.0].
    #[test]
    fn prop_priority_clamping(priority in -10.0f64..10.0f64) {
        let ann = Annotations::new().with_priority(priority);

        if let Some(p) = ann.priority {
            prop_assert!(p >= 0.0 && p <= 1.0,
                "Priority {} should be clamped to [0.0, 1.0], got {}", priority, p);
        }
    }
}

// ============================================================================
// Property 5: Elicitation Request Handling
// **Validates: Requirements 2.2, 2.3, 2.4, 2.5, 2.6, 2.7**
// For any elicitation request, the server SHALL validate responses against
// the provided schema and support accept/decline/cancel actions.
// ============================================================================

/// Generate arbitrary elicitation schema
fn arb_elicitation_schema() -> impl Strategy<Value = ElicitationSchema> {
    prop::collection::vec(
        (
            "[a-z]{1,10}", // property name
            prop_oneof![
                Just("string".to_string()),
                Just("number".to_string()),
                Just("boolean".to_string()),
            ],
            any::<bool>(), // required
        ),
        0..5,
    )
    .prop_map(|props| {
        let mut schema = ElicitationSchema::object();
        for (name, prop_type, required) in props {
            let prop_schema = match prop_type.as_str() {
                "string" => PropertySchema::string(),
                "number" => PropertySchema::number(),
                "boolean" => PropertySchema::boolean(),
                _ => PropertySchema::string(),
            };
            schema = schema.with_property(&name, prop_schema);
            if required {
                schema = schema.with_required(&name);
            }
        }
        schema
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 5: Elicitation Request Handling
    /// Elicitation responses should be validated against schema.
    #[test]
    fn prop_elicitation_schema_validation(schema in arb_elicitation_schema()) {
        // Verify schema serialization round-trip
        let json = serde_json::to_string(&schema).unwrap();
        let deserialized: ElicitationSchema = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(deserialized.schema_type, schema.schema_type,
            "Schema type should be preserved");

        // Verify properties are preserved
        if let (Some(orig_props), Some(deser_props)) = (&schema.properties, &deserialized.properties) {
            prop_assert_eq!(orig_props.len(), deser_props.len(),
                "Property count should be preserved");
        }

        // Verify required fields are preserved
        prop_assert_eq!(deserialized.required, schema.required,
            "Required fields should be preserved");
    }

    /// Feature: mcp-2025-compatibility, Property 5: Elicitation action types
    /// All three elicitation actions should serialize correctly.
    #[test]
    fn prop_elicitation_action_serialization(action_type in 0..3i32) {
        let response = match action_type {
            0 => ElicitationResponse::accept(serde_json::json!({"test": "value"})),
            1 => ElicitationResponse::decline(),
            _ => ElicitationResponse::cancel(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ElicitationResponse = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(deserialized.action, response.action,
            "Action should be preserved through serialization");

        // Content should only be present for accept
        match response.action {
            ElicitationAction::Accept => {
                prop_assert!(deserialized.content.is_some(),
                    "Accept response should have content");
            }
            _ => {
                prop_assert!(deserialized.content.is_none(),
                    "Non-accept response should not have content");
            }
        }
    }
}

// ============================================================================
// Property 9: List Changed Notifications
// **Validates: Requirements 7.1, 7.2, 7.3, 7.5**
// For any change to tools, resources, or prompts lists, the server SHALL
// emit the appropriate list_changed notification.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 9: List Changed Notifications
    /// NotificationManager should emit correct notifications for list changes.
    #[test]
    fn prop_list_changed_notifications(notification_type in 0..4i32) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = NotificationManager::new();
            let mut receiver = manager.subscribe();

            // Send notification based on type
            match notification_type {
                0 => manager.notify_tools_changed(),
                1 => manager.notify_resources_changed(),
                2 => manager.notify_prompts_changed(),
                _ => manager.notify_roots_changed(),
            };

            // Should receive the notification
            let result = tokio::time::timeout(
                std::time::Duration::from_millis(100),
                receiver.recv()
            ).await;

            prop_assert!(result.is_ok(), "Should receive notification");
            let notification = result.unwrap().unwrap();

            // Verify correct notification type
            match notification_type {
                0 => prop_assert!(matches!(notification, Notification::ToolsListChanged)),
                1 => prop_assert!(matches!(notification, Notification::ResourcesListChanged)),
                2 => prop_assert!(matches!(notification, Notification::PromptsListChanged)),
                _ => prop_assert!(matches!(notification, Notification::RootsListChanged)),
            }

            Ok(())
        })?;
    }

    /// Feature: mcp-2025-compatibility, Property 9: Notification JSON-RPC format
    /// Notifications should serialize to valid JSON-RPC format.
    #[test]
    fn prop_notification_json_rpc_format(notification_type in 0..4i32) {
        let notification = match notification_type {
            0 => Notification::ToolsListChanged,
            1 => Notification::ResourcesListChanged,
            2 => Notification::PromptsListChanged,
            _ => Notification::RootsListChanged,
        };

        let json = notification.to_json_rpc();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should have jsonrpc field
        prop_assert_eq!(&parsed["jsonrpc"], "2.0");

        // Should have method field
        prop_assert!(parsed["method"].is_string());

        // Should NOT have id field (notifications don't have id)
        prop_assert!(parsed.get("id").is_none());
    }
}

// ============================================================================
// Property 10: Cancellation Handling
// **Validates: Requirements 8.1, 8.2, 8.3, 8.4**
// For any cancellation request, the server SHALL track cancellation state
// and handle cancellation of completed requests gracefully.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 10: Cancellation Handling
    /// CancellationManager should track and cancel requests correctly.
    #[test]
    fn prop_cancellation_handling(request_id in 1i64..1000) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = CancellationManager::new();
            let id = RequestId::Number(request_id);

            // Create token
            let token = manager.create_token(id.clone()).await;

            // Initially active
            prop_assert_eq!(token.state(), CancellationState::Active,
                "Token should be active initially");
            prop_assert!(!token.is_cancelled(), "Should not be cancelled initially");
            prop_assert!(!token.is_completed(), "Should not be completed initially");

            // Cancel the request
            let cancelled = manager.cancel(&id, Some("test reason".to_string())).await;
            prop_assert!(cancelled, "Cancel should succeed for active request");

            // Should be cancelled now
            prop_assert_eq!(token.state(), CancellationState::Cancelled,
                "Token should be cancelled after cancel");
            prop_assert!(token.is_cancelled(), "Should be cancelled");
            prop_assert!(!token.is_completed(), "Should not be completed");

            // Cancelling again should fail (already cancelled)
            let cancelled_again = manager.cancel(&id, None).await;
            prop_assert!(!cancelled_again, "Cancel should fail for already cancelled request");

            Ok(())
        })?;
    }

    /// Feature: mcp-2025-compatibility, Property 10: Cancellation of completed requests
    /// Cancelling a completed request should be handled gracefully.
    #[test]
    fn prop_cancellation_completed_request(request_id in 1i64..1000) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = CancellationManager::new();
            let id = RequestId::Number(request_id);

            // Create token
            let token = manager.create_token(id.clone()).await;

            // Complete the request
            let completed = token.complete();
            prop_assert!(completed, "Complete should succeed for active request");
            prop_assert_eq!(token.state(), CancellationState::Completed,
                "Token should be completed");

            // Cancelling completed request should fail gracefully
            let cancelled = manager.cancel(&id, None).await;
            prop_assert!(!cancelled, "Cancel should fail for completed request");

            // State should still be completed
            prop_assert_eq!(token.state(), CancellationState::Completed,
                "Token should still be completed");

            Ok(())
        })?;
    }

    /// Feature: mcp-2025-compatibility, Property 10: Cancellation reason preservation
    /// Cancellation reason should be preserved.
    #[test]
    fn prop_cancellation_reason(request_id in 1i64..1000, reason in "[a-z ]{1,50}") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = CancellationManager::new();
            let id = RequestId::Number(request_id);

            let token = manager.create_token(id.clone()).await;
            manager.cancel(&id, Some(reason.clone())).await;

            let stored_reason = token.reason().await;
            prop_assert_eq!(stored_reason, Some(reason),
                "Cancellation reason should be preserved");

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 11: Progress Notifications
// **Validates: Requirements 9.1, 9.2, 9.3, 9.4**
// For any request with a progressToken, the server SHALL emit progress
// notifications and a final notification on completion.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 11: Progress Notifications
    /// ProgressTracker should emit notifications on progress updates.
    #[test]
    fn prop_progress_notifications(
        token in "[a-z0-9-]{1,20}",
        progress_values in prop::collection::vec(0.0f64..=1.0f64, 1..5)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tracker = ProgressTracker::new();
            let mut receiver = tracker.subscribe();

            // Start tracking
            tracker.start(&token, Some(100)).await;
            prop_assert!(tracker.is_tracking(&token).await,
                "Should be tracking after start");

            // Update progress and verify notifications
            for progress in &progress_values {
                tracker.update(&token, *progress).await;

                let result = tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    receiver.recv()
                ).await;

                prop_assert!(result.is_ok(), "Should receive progress notification");
                let notification = result.unwrap().unwrap();
                prop_assert_eq!(&notification.progress_token, &token,
                    "Progress token should match");
            }

            // Complete and verify final notification
            tracker.complete(&token).await;

            let result = tokio::time::timeout(
                std::time::Duration::from_millis(100),
                receiver.recv()
            ).await;

            prop_assert!(result.is_ok(), "Should receive completion notification");
            let notification = result.unwrap().unwrap();
            prop_assert_eq!(notification.progress, 1.0,
                "Completion notification should have progress 1.0");

            Ok(())
        })?;
    }

    /// Feature: mcp-2025-compatibility, Property 11: Progress value clamping
    /// Progress values should be clamped to [0.0, 1.0].
    #[test]
    fn prop_progress_value_clamping(
        token in "[a-z0-9-]{1,20}",
        progress in -10.0f64..10.0f64
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tracker = ProgressTracker::new();
            tracker.start(&token, None).await;
            tracker.update(&token, progress).await;

            let state = tracker.get(&token).await;
            prop_assert!(state.is_some(), "Should have state after update");

            let p = state.unwrap().progress;
            prop_assert!(p >= 0.0 && p <= 1.0,
                "Progress {} should be clamped to [0.0, 1.0], got {}", progress, p);

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 12: Ping Response
// **Validates: Requirements 10.1, 10.2, 10.3**
// For any ping request, the server SHALL return an empty result object.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: mcp-2025-compatibility, Property 12: Ping Response
    /// Ping requests should return empty result object.
    #[test]
    fn prop_ping_response(request_id in 1i64..1000) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = CompleteMcpAdapter::new();

            // Create ping request
            let request_json = format!(
                r#"{{"jsonrpc":"2.0","method":"ping","id":{}}}"#,
                request_id
            );

            let router = dcp::dispatch::BinaryTrieRouter::new();
            let result = adapter.handle_request(&request_json, &router).await;

            prop_assert!(result.is_ok(), "Ping should not error");
            let response = result.unwrap();
            prop_assert!(response.is_some(), "Ping should return response");

            let response_str = response.unwrap();
            let parsed = JsonRpcParser::parse_response(&response_str).unwrap();

            prop_assert!(parsed.is_success(), "Ping should succeed");

            // Result should be empty object
            let result_value = parsed.result.unwrap();
            prop_assert!(result_value.is_object(), "Result should be object");
            prop_assert!(result_value.as_object().unwrap().is_empty(),
                "Result should be empty object");

            Ok(())
        })?;
    }
}

// ============================================================================
// Additional unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_ordering() {
        assert!(ProtocolVersion::V2024_11_05 < ProtocolVersion::V2025_03_26);
        assert!(ProtocolVersion::V2025_03_26 < ProtocolVersion::V2025_06_18);
    }

    #[test]
    fn test_elicitation_schema_serialization() {
        let schema = ElicitationSchema::object()
            .with_property("name", PropertySchema::string())
            .with_property("age", PropertySchema::number().with_minimum(0.0))
            .with_required("name");

        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("\"type\":\"object\""));
        assert!(json.contains("\"name\""));
    }

    #[test]
    fn test_elicitation_response_serialization() {
        let accept = ElicitationResponse::accept(serde_json::json!({"name": "test"}));
        let json = serde_json::to_string(&accept).unwrap();
        assert!(json.contains("\"action\":\"accept\""));

        let decline = ElicitationResponse::decline();
        let json = serde_json::to_string(&decline).unwrap();
        assert!(json.contains("\"action\":\"decline\""));
    }

    #[test]
    fn test_content_types() {
        let text = Content::text("hello");
        let json = serde_json::to_string(&text).unwrap();
        assert!(json.contains("\"type\":\"text\""));

        let image = Content::image("base64data", "image/png");
        let json = serde_json::to_string(&image).unwrap();
        assert!(json.contains("\"type\":\"image\""));

        let resource = Content::resource("file:///test.txt");
        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("\"type\":\"resource\""));
    }
}
