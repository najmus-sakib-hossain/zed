//! Property-Based Tests for FastAPI Compatibility
//!
//! Property 7: FastAPI Async and Validation Correctness
//! Validates: Requirements 5.1, 5.2, 5.3, 5.4

use dx_py_fastapi::asgi::ScopeType;
use dx_py_fastapi::asgi::{HttpConnection, LifespanHandler, WebSocketConnection};
use dx_py_fastapi::starlette::HttpMethod;
use dx_py_fastapi::validation::EndpointParameter;
use dx_py_fastapi::{
    AsgiMessage,
    // ASGI
    AsgiScope,
    AsyncGenerator,
    AsyncIterator,
    Coroutine,
    CoroutineResult,
    CoroutineState,
    EndpointValidator,
    // Async runtime
    EventLoop,
    FieldType,
    ModelValidator,
    PydanticField,
    // Pydantic
    PydanticModel,
    // Starlette
    StarletteApp,
    Task,
    // Validation
    TypeValidator,
};

use proptest::prelude::*;
use serde_json::json;

// ============================================================================
// Property 7.1: Pydantic Model Validation Consistency
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Valid data always passes validation
    #[test]
    fn prop_valid_data_passes_validation(
        name in "[a-zA-Z][a-zA-Z0-9_]{0,20}",
        age in 0i64..150i64,
        email in "[a-z]{3,10}@[a-z]{3,10}\\.[a-z]{2,4}",
    ) {
        let mut name_field = PydanticField::new("name", FieldType::String);
        name_field.required = true;
        let mut age_field = PydanticField::new("age", FieldType::Integer);
        age_field.required = true;
        let mut email_field = PydanticField::new("email", FieldType::String);
        email_field.required = true;

        let model = PydanticModel::new("User")
            .field(name_field)
            .field(age_field)
            .field(email_field);

        let mut validator = ModelValidator::new();
        validator.register(model);

        let data = json!({
            "name": name,
            "age": age,
            "email": email,
        });

        let result = validator.validate("User", &data);
        prop_assert!(result.is_ok(), "Valid data should pass validation: {:?}", result);
    }

    /// Property: Missing required fields always fail validation
    #[test]
    fn prop_missing_required_fails(
        name in "[a-zA-Z][a-zA-Z0-9_]{0,20}",
    ) {
        let mut name_field = PydanticField::new("name", FieldType::String);
        name_field.required = true;
        let mut age_field = PydanticField::new("age", FieldType::Integer);
        age_field.required = true;

        let model = PydanticModel::new("User")
            .field(name_field)
            .field(age_field);

        let mut validator = ModelValidator::new();
        validator.register(model);

        // Missing 'age' field
        let data = json!({
            "name": name,
        });

        let result = validator.validate("User", &data);
        prop_assert!(result.is_err(), "Missing required field should fail validation");
    }

    /// Property: Type coercion is consistent
    #[test]
    fn prop_type_coercion_consistent(
        int_str in "[0-9]{1,9}",
    ) {
        let validator = TypeValidator::new();

        let value = serde_json::Value::String(int_str.clone());
        let result = validator.validate(&value, &FieldType::Integer);

        if let Ok(coerced) = result {
            // If coercion succeeds, the result should be a number
            prop_assert!(coerced.is_number(), "Coerced value should be a number");

            // The coerced value should match the original string parsed as int
            let expected: i64 = int_str.parse().unwrap();
            prop_assert_eq!(coerced.as_i64().unwrap(), expected);
        }
    }
}

// ============================================================================
// Property 7.2: ASGI Protocol Correctness
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: HTTP scope always has required fields
    #[test]
    fn prop_http_scope_has_required_fields(
        method in "(GET|POST|PUT|DELETE|PATCH)",
        path in "/[a-z/]{0,50}",
    ) {
        let scope = AsgiScope::http(&method, &path);

        prop_assert_eq!(scope.scope_type, ScopeType::Http);
        prop_assert!(scope.method.is_some());
        prop_assert!(scope.path.is_some());
        prop_assert!(!scope.asgi.version.is_empty());
    }

    /// Property: WebSocket scope has correct type
    #[test]
    fn prop_websocket_scope_correct_type(
        path in "/ws/[a-z]{1,20}",
    ) {
        let scope = AsgiScope::websocket(&path);

        prop_assert_eq!(scope.scope_type, ScopeType::Websocket);
        prop_assert_eq!(scope.scheme, Some("ws".to_string()));
        prop_assert!(scope.method.is_none()); // WebSocket has no method
    }

    /// Property: Headers are case-insensitive
    #[test]
    fn prop_headers_case_insensitive(
        header_name in "[A-Za-z-]{1,20}",
        header_value in "[a-zA-Z0-9]{1,50}",
    ) {
        let scope = AsgiScope::http("GET", "/")
            .with_header(header_name.as_bytes().to_vec(), header_value.as_bytes().to_vec());

        // Should find header regardless of case
        let lower = header_name.to_lowercase();
        let upper = header_name.to_uppercase();

        let found_lower = scope.get_header(lower.as_bytes());
        let found_upper = scope.get_header(upper.as_bytes());

        prop_assert!(found_lower.is_some() || found_upper.is_some());
    }
}

// ============================================================================
// Property 7.3: Async Runtime Correctness
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Coroutine state transitions are valid
    #[test]
    fn prop_coroutine_state_transitions(
        coro_name in "[a-z_]{1,20}",
        result_value in 0i64..1000i64,
    ) {
        let mut coro = Coroutine::new(1, &coro_name);

        // Initial state is Pending
        prop_assert_eq!(coro.state, CoroutineState::Pending);
        prop_assert!(!coro.is_done());

        // After setting result, state is Completed
        coro.set_result(json!(result_value));
        prop_assert_eq!(coro.state, CoroutineState::Completed);
        prop_assert!(coro.is_done());

        // Result is preserved
        if let Some(CoroutineResult::Value(v)) = &coro.result {
            prop_assert_eq!(v.as_i64().unwrap(), result_value);
        } else {
            prop_assert!(false, "Expected value result");
        }
    }

    /// Property: Task cancellation is idempotent
    #[test]
    fn prop_task_cancellation_idempotent(
        task_name in "[a-z_]{1,20}",
    ) {
        let mut task = Task::new(1, &task_name, 1);

        // First cancel succeeds
        let first_cancel = task.cancel();
        prop_assert!(first_cancel);
        prop_assert!(task.done());

        // Second cancel returns false (already done)
        let second_cancel = task.cancel();
        prop_assert!(!second_cancel);

        // State is still cancelled
        prop_assert!(task.cancelled);
    }

    /// Property: Event loop task management is consistent
    #[test]
    fn prop_event_loop_task_management(
        num_tasks in 1usize..10usize,
    ) {
        let mut loop_ = EventLoop::new();

        let mut task_ids = Vec::new();
        for i in 0..num_tasks {
            let coro_id = loop_.create_coroutine(format!("coro_{}", i));
            let task_id = loop_.create_task(coro_id, format!("task_{}", i)).unwrap();
            task_ids.push((coro_id, task_id));
        }

        // All tasks should be pending
        prop_assert_eq!(loop_.pending_tasks().len(), num_tasks);
        prop_assert_eq!(loop_.completed_tasks().len(), 0);

        // Complete half the tasks
        let half = num_tasks / 2;
        for (coro_id, _) in task_ids.iter().take(half) {
            loop_.complete_coroutine(*coro_id, json!(null)).unwrap();
        }

        // Check counts
        prop_assert_eq!(loop_.completed_tasks().len(), half);
        prop_assert_eq!(loop_.pending_tasks().len(), num_tasks - half);
    }
}

// ============================================================================
// Property 7.4: Async Iterator and Generator Correctness
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Async iterator yields all items in order
    #[test]
    fn prop_async_iterator_yields_all(
        items in prop::collection::vec(0i64..1000i64, 0..20),
    ) {
        let json_items: Vec<serde_json::Value> = items.iter().map(|&i| json!(i)).collect();
        let mut iter = AsyncIterator::new("test_iter", json_items.clone());

        let mut collected = Vec::new();
        while let Some(item) = iter.next_item() {
            collected.push(item);
        }

        prop_assert_eq!(collected.len(), items.len());
        prop_assert!(iter.exhausted);

        // Items should be in order
        for (i, item) in collected.iter().enumerate() {
            prop_assert_eq!(item.as_i64().unwrap(), items[i]);
        }
    }

    /// Property: Async generator tracks yielded values
    #[test]
    fn prop_async_generator_tracks_yields(
        yields in prop::collection::vec(0i64..1000i64, 1..10),
    ) {
        let mut gen = AsyncGenerator::new("test_gen");

        for &y in &yields {
            gen.yield_value(json!(y));
        }

        let yielded = gen.get_yielded();
        prop_assert_eq!(yielded.len(), yields.len());

        for (i, y) in yielded.iter().enumerate() {
            prop_assert_eq!(y.as_i64().unwrap(), yields[i]);
        }
    }

    /// Property: Closed generator rejects sends
    #[test]
    fn prop_closed_generator_rejects_send(
        gen_name in "[a-z_]{1,20}",
    ) {
        let mut gen = AsyncGenerator::new(&gen_name);

        // Can send before close
        prop_assert!(gen.send(json!(1)).is_ok());

        gen.close();

        // Cannot send after close
        prop_assert!(gen.send(json!(2)).is_err());
    }
}

// ============================================================================
// Property 7.5: Starlette Routing Correctness
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Route matching is deterministic
    #[test]
    fn prop_route_matching_deterministic(
        path_segment in "[a-z]{1,10}",
    ) {
        let path = format!("/{}", path_segment);
        let app = StarletteApp::new()
            .route(&path, "endpoint", vec![HttpMethod::Get])
            .unwrap();

        // Same path should always match
        let result1 = app.match_route(&path, HttpMethod::Get);
        let result2 = app.match_route(&path, HttpMethod::Get);

        prop_assert!(result1.is_ok());
        prop_assert!(result2.is_ok());

        let (route1, _) = result1.unwrap();
        let (route2, _) = result2.unwrap();
        prop_assert_eq!(&route1.endpoint, &route2.endpoint);
    }

    /// Property: Path parameters are extracted correctly
    #[test]
    fn prop_path_params_extracted(
        param_value in "[a-z0-9]{1,20}",
    ) {
        let app = StarletteApp::new()
            .route("/users/{id}", "user_detail", vec![HttpMethod::Get])
            .unwrap();

        let path = format!("/users/{}", param_value);
        let (_, params) = app.match_route(&path, HttpMethod::Get).unwrap();

        prop_assert_eq!(params.get("id"), Some(&param_value));
    }

    /// Property: Method filtering works correctly
    #[test]
    fn prop_method_filtering(
        path in "/[a-z]{1,10}",
    ) {
        let app = StarletteApp::new()
            .route(&path, "endpoint", vec![HttpMethod::Get, HttpMethod::Post])
            .unwrap();

        // GET and POST should match
        prop_assert!(app.match_route(&path, HttpMethod::Get).is_ok());
        prop_assert!(app.match_route(&path, HttpMethod::Post).is_ok());

        // DELETE should not match
        prop_assert!(app.match_route(&path, HttpMethod::Delete).is_err());
    }
}

// ============================================================================
// Property 7.6: Endpoint Validation Correctness
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Path parameters are validated correctly
    #[test]
    fn prop_path_param_validation(
        id_value in "[0-9]{1,9}",
    ) {
        let mut validator = EndpointValidator::new();
        validator.add_parameter(EndpointParameter::path("id", FieldType::Integer));

        let mut path_params = std::collections::HashMap::new();
        path_params.insert("id".to_string(), id_value.clone());

        let result = validator.validate_request(
            &path_params,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
            None,
        );

        prop_assert!(result.valid);

        let expected: i64 = id_value.parse().unwrap();
        prop_assert_eq!(result.values.get("id").unwrap().as_i64().unwrap(), expected);
    }

    /// Property: Query parameters with defaults work correctly
    #[test]
    fn prop_query_param_defaults(
        default_value in 1i64..100i64,
    ) {
        let mut validator = EndpointValidator::new();
        validator.add_parameter(
            EndpointParameter::query("page", FieldType::Integer)
                .with_default(json!(default_value))
        );

        // Without providing the parameter
        let result = validator.validate_request(
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
            None,
        );

        prop_assert!(result.valid);
        prop_assert_eq!(result.values.get("page").unwrap().as_i64().unwrap(), default_value);
    }

    /// Property: Required parameters without values fail
    #[test]
    fn prop_required_param_fails_without_value(
        param_name in "[a-z]{1,10}",
    ) {
        let mut validator = EndpointValidator::new();
        validator.add_parameter(EndpointParameter::path(&param_name, FieldType::String));

        let result = validator.validate_request(
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
            None,
        );

        prop_assert!(!result.valid);
        prop_assert!(!result.errors.is_empty());
    }
}

// ============================================================================
// Property 7.7: ASGI Lifecycle Correctness
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Lifespan handler state is consistent
    #[test]
    fn prop_lifespan_state_consistent(
        state_key in "[a-z_]{1,10}",
        state_value in 0i64..1000i64,
    ) {
        let mut handler = LifespanHandler::new();

        // Not ready initially
        prop_assert!(!handler.is_ready());

        // Set state
        handler.set_state(&state_key, json!(state_value));

        // State is retrievable
        let retrieved = handler.get_state(&state_key).unwrap();
        prop_assert_eq!(retrieved.as_i64().unwrap(), state_value);

        // After startup, is ready
        handler.handle(&AsgiMessage::LifespanStartup).unwrap();
        prop_assert!(handler.is_ready());
    }

    /// Property: HTTP connection state machine is valid
    #[test]
    fn prop_http_connection_state_machine(
        body_content in "[a-zA-Z0-9]{0,100}",
        status_code in 200u16..600u16,
    ) {
        let scope = AsgiScope::http("POST", "/api/test");
        let mut conn = HttpConnection::new(scope).unwrap();

        // Receive body
        conn.receive_body(body_content.as_bytes().to_vec(), false);
        prop_assert_eq!(conn.get_request_body(), body_content.as_bytes());

        // Start response
        conn.start_response(status_code, vec![]).unwrap();
        prop_assert_eq!(conn.get_response_status(), Some(status_code));

        // Cannot start response twice
        prop_assert!(conn.start_response(200, vec![]).is_err());

        // Send body
        conn.send_body(b"response".to_vec(), false).unwrap();
        prop_assert!(conn.is_complete());
    }

    /// Property: WebSocket connection state machine is valid
    #[test]
    fn prop_websocket_connection_state_machine(
        message_text in "[a-zA-Z0-9 ]{1,100}",
    ) {
        let scope = AsgiScope::websocket("/ws/test");
        let mut conn = WebSocketConnection::new(scope).unwrap();

        // Cannot send before accept
        prop_assert!(conn.send(AsgiMessage::websocket_text(&message_text)).is_err());

        // Accept connection
        conn.accept(None, vec![]);
        prop_assert!(conn.is_open());

        // Can send after accept
        prop_assert!(conn.send(AsgiMessage::websocket_text(&message_text)).is_ok());

        // Close connection
        conn.close(Some(1000), None);
        prop_assert!(!conn.is_open());

        // Cannot send after close
        prop_assert!(conn.send(AsgiMessage::websocket_text("test")).is_err());
    }
}
