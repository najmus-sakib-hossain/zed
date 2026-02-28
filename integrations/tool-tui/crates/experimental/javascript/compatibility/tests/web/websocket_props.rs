//! Property tests for WebSocket API.
//!
//! **Feature: dx-js-compatibility, Property 14: WebSocket Message Round-Trip**
//! **Validates: Requirements 12.2, 12.6**
//!
//! *For any* message (text or binary) sent via `ws.send()`, the server SHALL
//! receive the exact same message content and type.

#[cfg(feature = "web-core")]
mod tests {
    use dx_js_compatibility::web::websocket::{CloseFrame, Message, ReadyState};
    use proptest::prelude::*;

    // =========================================================================
    // Property 14: WebSocket Message Round-Trip
    // Validates: Requirements 12.2, 12.6
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 14: Message type conversion preserves content for text messages.
        #[test]
        fn websocket_text_message_preserves_content(text in "\\PC{0,1000}") {
            let msg = Message::Text(text.clone());
            
            // Convert to tungstenite and back
            let tung_msg: tokio_tungstenite::tungstenite::Message = msg.into();
            let back: Message = tung_msg.into();
            
            match back {
                Message::Text(s) => prop_assert_eq!(s, text, "Text content should be preserved"),
                _ => prop_assert!(false, "Message type should remain Text"),
            }
        }

        /// Property 14: Message type conversion preserves content for binary messages.
        #[test]
        fn websocket_binary_message_preserves_content(data in prop::collection::vec(any::<u8>(), 0..1000)) {
            let msg = Message::Binary(data.clone());
            
            // Convert to tungstenite and back
            let tung_msg: tokio_tungstenite::tungstenite::Message = msg.into();
            let back: Message = tung_msg.into();
            
            match back {
                Message::Binary(b) => prop_assert_eq!(b, data, "Binary content should be preserved"),
                _ => prop_assert!(false, "Message type should remain Binary"),
            }
        }

        /// Property 14: Ping message conversion preserves content.
        #[test]
        fn websocket_ping_message_preserves_content(data in prop::collection::vec(any::<u8>(), 0..125)) {
            let msg = Message::Ping(data.clone());
            
            let tung_msg: tokio_tungstenite::tungstenite::Message = msg.into();
            let back: Message = tung_msg.into();
            
            match back {
                Message::Ping(b) => prop_assert_eq!(b, data, "Ping content should be preserved"),
                _ => prop_assert!(false, "Message type should remain Ping"),
            }
        }

        /// Property 14: Pong message conversion preserves content.
        #[test]
        fn websocket_pong_message_preserves_content(data in prop::collection::vec(any::<u8>(), 0..125)) {
            let msg = Message::Pong(data.clone());
            
            let tung_msg: tokio_tungstenite::tungstenite::Message = msg.into();
            let back: Message = tung_msg.into();
            
            match back {
                Message::Pong(b) => prop_assert_eq!(b, data, "Pong content should be preserved"),
                _ => prop_assert!(false, "Message type should remain Pong"),
            }
        }

        /// Property 14: Close frame conversion preserves code and reason.
        #[test]
        fn websocket_close_frame_preserves_content(
            code in 1000u16..5000,
            reason in "[a-zA-Z0-9 ]{0,100}"
        ) {
            let frame = CloseFrame {
                code,
                reason: reason.clone(),
            };
            let msg = Message::Close(Some(frame));
            
            let tung_msg: tokio_tungstenite::tungstenite::Message = msg.into();
            let back: Message = tung_msg.into();
            
            match back {
                Message::Close(Some(f)) => {
                    prop_assert_eq!(f.code, code, "Close code should be preserved");
                    prop_assert_eq!(f.reason, reason, "Close reason should be preserved");
                }
                Message::Close(None) => prop_assert!(false, "Close frame should be preserved"),
                _ => prop_assert!(false, "Message type should remain Close"),
            }
        }
    }

    // =========================================================================
    // ReadyState Tests
    // =========================================================================

    #[test]
    fn ready_state_values_match_spec() {
        // WebSocket spec defines these exact values
        assert_eq!(ReadyState::Connecting as u8, 0);
        assert_eq!(ReadyState::Open as u8, 1);
        assert_eq!(ReadyState::Closing as u8, 2);
        assert_eq!(ReadyState::Closed as u8, 3);
    }

    #[test]
    fn ready_state_equality() {
        assert_eq!(ReadyState::Open, ReadyState::Open);
        assert_ne!(ReadyState::Open, ReadyState::Closed);
    }
}
