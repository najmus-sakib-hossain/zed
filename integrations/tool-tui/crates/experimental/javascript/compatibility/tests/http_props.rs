//! Property tests for HTTP server functionality.
//!
//! This file contains property-based tests for the HTTP server implementation,
//! validating correctness properties defined in the design document.
//!
//! Feature: production-readiness

#[cfg(feature = "node-core")]
mod tests {
    use bytes::Bytes;
    use dx_js_compatibility::node::http::{
        create_http_server, HttpRequestHandler, ServerConfig, ServerResponse,
    };
    use proptest::prelude::*;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    // =========================================================================
    // Property 14: HTTP Response Lifecycle
    // Validates: Requirements 4.4, 4.5, 4.6
    // For any response, writeHead() SHALL set headers, write() SHALL send body
    // chunks, and end() SHALL complete the response.
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 14: ServerResponse correctly sets status code.
        /// For any valid status code, writeHead() SHALL set it correctly.
        #[test]
        fn response_sets_status_code(status_code in 200u16..500) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (tx, mut rx) = mpsc::channel(1);
                let mut response = ServerResponse::new(tx);

                response.write_head(status_code, None);
                response.end(None::<&str>);

                let payload = rx.recv().await.unwrap();
                prop_assert_eq!(payload.status.as_u16(), status_code, "Status code should match");
                Ok(())
            });
            result?;
        }

        /// Property 14: ServerResponse correctly sets headers.
        /// For any valid header, setHeader() SHALL set it correctly.
        #[test]
        fn response_sets_headers(
            header_name in "[a-z]{1,20}",
            header_value in "[a-zA-Z0-9_-]{1,50}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (tx, mut rx) = mpsc::channel(1);
                let mut response = ServerResponse::new(tx);

                response.set_header(&header_name, &header_value);
                response.write_head(200, None);
                response.end(None::<&str>);

                let payload = rx.recv().await.unwrap();
                prop_assert_eq!(
                    payload.headers.get(&header_name),
                    Some(&header_value),
                    "Header should be set"
                );
                Ok(())
            });
            result?;
        }

        /// Property 14: ServerResponse correctly accumulates body chunks.
        /// For any sequence of write() calls, end() SHALL combine all chunks.
        #[test]
        fn response_accumulates_body_chunks(
            chunks in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..100), 1..5)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (tx, mut rx) = mpsc::channel(1);
                let mut response = ServerResponse::new(tx);

                // Write all chunks
                let mut expected_body: Vec<u8> = Vec::new();
                for chunk in &chunks {
                    response.write(Bytes::from(chunk.clone()));
                    expected_body.extend(chunk);
                }
                response.end(None::<&str>);

                let payload = rx.recv().await.unwrap();
                prop_assert_eq!(payload.body, expected_body, "Body should contain all chunks");
                Ok(())
            });
            result?;
        }

        /// Property 14: ServerResponse end() with data appends to body.
        /// For any data passed to end(), it SHALL be appended to the body.
        #[test]
        fn response_end_appends_data(
            initial_data in prop::collection::vec(any::<u8>(), 0..100),
            final_data in prop::collection::vec(any::<u8>(), 1..100)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let (tx, mut rx) = mpsc::channel(1);
                let mut response = ServerResponse::new(tx);

                if !initial_data.is_empty() {
                    response.write(Bytes::from(initial_data.clone()));
                }
                response.end(Some(Bytes::from(final_data.clone())));

                let payload = rx.recv().await.unwrap();
                let mut expected = initial_data;
                expected.extend(&final_data);
                prop_assert_eq!(payload.body, expected, "Body should include end() data");
                Ok(())
            });
            result?;
        }
    }

    // =========================================================================
    // Additional Unit Tests
    // =========================================================================

    #[tokio::test]
    async fn test_server_config_defaults() {
        let config = ServerConfig::default();
        assert_eq!(config.keep_alive_timeout, 5);
        assert_eq!(config.max_header_size, 8192);
        assert!(config.keep_alive);
    }

    #[tokio::test]
    async fn test_http_server_creation_and_close() {
        let handler: HttpRequestHandler = Arc::new(|_req, mut res| {
            res.write_head(200, None);
            res.end(Some("OK"));
        });

        let server = create_http_server(handler).await.unwrap();
        let mut server = server.listen("127.0.0.1:0").await.unwrap();

        assert!(server.listening());
        let port = server.address().port();
        assert!(port > 0);

        server.close().await.unwrap();
        assert!(!server.listening());
    }

    #[tokio::test]
    async fn test_response_headers_sent_flag() {
        let (tx, _rx) = mpsc::channel(1);
        let mut response = ServerResponse::new(tx);

        assert!(!response.headers_sent());
        response.set_header("X-Test", "value");
        assert!(!response.headers_sent());

        response.write_head(200, None);
        assert!(response.headers_sent());
    }

    #[tokio::test]
    async fn test_response_get_header() {
        let (tx, _rx) = mpsc::channel(1);
        let mut response = ServerResponse::new(tx);

        response.set_header("Content-Type", "application/json");
        assert_eq!(response.get_header("content-type"), Some("application/json"));
        assert_eq!(response.get_header("Content-Type"), Some("application/json"));
    }

    #[tokio::test]
    async fn test_response_remove_header() {
        let (tx, _rx) = mpsc::channel(1);
        let mut response = ServerResponse::new(tx);

        response.set_header("X-Test", "value");
        assert_eq!(response.get_header("x-test"), Some("value"));

        response.remove_header("x-test");
        assert_eq!(response.get_header("x-test"), None);
    }

    #[tokio::test]
    async fn test_response_status_code() {
        let (tx, _rx) = mpsc::channel(1);
        let mut response = ServerResponse::new(tx);

        assert_eq!(response.status_code(), 200); // Default
        response.set_status_code(404);
        assert_eq!(response.status_code(), 404);
    }

    #[tokio::test]
    async fn test_response_finished_flag() {
        let (tx, mut rx) = mpsc::channel(1);
        let mut response = ServerResponse::new(tx);

        assert!(!response.finished());
        response.write(&b"test"[..]);
        assert!(!response.finished());

        response.end(None::<&str>);
        // After end(), the response is consumed, so we check via the channel
        let payload = rx.recv().await.unwrap();
        assert_eq!(payload.body, b"test");
    }

    #[tokio::test]
    async fn test_multiple_headers() {
        let (tx, mut rx) = mpsc::channel(1);
        let mut response = ServerResponse::new(tx);

        response.set_header("Content-Type", "text/html");
        response.set_header("X-Custom-1", "value1");
        response.set_header("X-Custom-2", "value2");
        response.write_head(200, None);
        response.end(None::<&str>);

        let payload = rx.recv().await.unwrap();
        assert_eq!(payload.headers.get("content-type"), Some(&"text/html".to_string()));
        assert_eq!(payload.headers.get("x-custom-1"), Some(&"value1".to_string()));
        assert_eq!(payload.headers.get("x-custom-2"), Some(&"value2".to_string()));
    }
}
