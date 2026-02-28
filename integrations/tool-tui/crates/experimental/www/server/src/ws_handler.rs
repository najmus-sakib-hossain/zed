// WebSocket handler for dx-sync realtime features

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use std::sync::Arc;

use crate::ecosystem::EcosystemState;

/// Handle WebSocket upgrade
pub async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<Arc<EcosystemState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(mut socket: WebSocket, state: Arc<EcosystemState>) {
    // Send welcome message
    let _ = socket.send(Message::Binary(vec![0xA0, 0x00])).await; // SYNC_SUBSCRIBE opcode

    // Handle messages
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Binary(data)) => {
                handle_binary_message(&data, &state, &mut socket).await;
            }
            Ok(Message::Text(text)) => {
                // Text messages not supported in binary protocol
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
            Ok(Message::Close(_)) => {
                break;
            }
            _ => {}
        }
    }
}

/// Handle binary message
async fn handle_binary_message(data: &[u8], state: &Arc<EcosystemState>, socket: &mut WebSocket) {
    if data.is_empty() {
        return;
    }

    let opcode = data[0];

    match opcode {
        0xA0 => {
            // SYNC_SUBSCRIBE
            if data.len() < 2 {
                return;
            }
            let channel_id = data[1];

            // Subscribe to channel
            #[cfg(feature = "sync")]
            if let Some(ref manager) = state.channel_manager {
                manager.subscribe(channel_id, socket_id()).await;
            }

            // Send ACK
            let _ = socket.send(Message::Binary(vec![0xA4, channel_id])).await;
        }

        0xA1 => {
            // SYNC_UNSUBSCRIBE
            if data.len() < 2 {
                return;
            }
            let channel_id = data[1];

            // Unsubscribe from channel
            #[cfg(feature = "sync")]
            if let Some(ref manager) = state.channel_manager {
                manager.unsubscribe(channel_id, socket_id()).await;
            }
        }

        0xA2 => {
            // SYNC_MESSAGE
            if data.len() < 3 {
                return;
            }
            let channel_id = data[1];
            let message = &data[2..];

            // Broadcast to channel
            #[cfg(feature = "sync")]
            if let Some(ref manager) = state.channel_manager {
                manager.broadcast(channel_id, message).await;
            }
        }

        _ => {
            // Unknown opcode
        }
    }
}

/// Get socket ID (placeholder)
fn socket_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SOCKET_COUNTER: AtomicU64 = AtomicU64::new(0);
    SOCKET_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_id() {
        let id1 = socket_id();
        let id2 = socket_id();
        assert_ne!(id1, id2);
    }
}
