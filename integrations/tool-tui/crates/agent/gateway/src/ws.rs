//! WebSocket connection handling utilities.

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error};

/// Bidirectional WebSocket connection wrapper
pub struct WsConnection {
    tx: mpsc::Sender<String>,
}

impl WsConnection {
    /// Wrap a WebSocket into a managed connection with send/receive channels
    pub fn new(socket: WebSocket) -> (Self, mpsc::Receiver<String>) {
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<String>(256);
        let (inbound_tx, inbound_rx) = mpsc::channel::<String>(256);

        let (mut ws_sender, mut ws_receiver) = socket.split();

        // Forward outbound messages to WebSocket
        tokio::spawn(async move {
            while let Some(msg) = outbound_rx.recv().await {
                if ws_sender.send(WsMessage::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        });

        // Forward WebSocket messages to inbound channel
        tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_receiver.next().await {
                match msg {
                    WsMessage::Text(text) => {
                        if inbound_tx.send(text.to_string()).await.is_err() {
                            break;
                        }
                    }
                    WsMessage::Close(_) => {
                        debug!("WebSocket close received");
                        break;
                    }
                    WsMessage::Ping(_) | WsMessage::Pong(_) => {
                        // Auto-handled
                    }
                    WsMessage::Binary(data) => {
                        if let Ok(text) = String::from_utf8(data.to_vec()) {
                            if inbound_tx.send(text).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        (Self { tx: outbound_tx }, inbound_rx)
    }

    /// Send a message through the WebSocket
    pub async fn send(&self, message: String) -> anyhow::Result<()> {
        self.tx
            .send(message)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send: {}", e))
    }
}
