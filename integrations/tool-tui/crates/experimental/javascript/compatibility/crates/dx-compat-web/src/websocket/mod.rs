//! WebSocket API implementation using tokio-tungstenite.
//!
//! This module provides WebSocket client functionality compatible with the
//! Web WebSocket API specification.

use crate::error::{WebError, WebResult};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite};

/// WebSocket ready state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReadyState {
    /// Connecting to the server
    Connecting = 0,
    /// Connection is open and ready
    Open = 1,
    /// Connection is closing
    Closing = 2,
    /// Connection is closed
    Closed = 3,
}

/// WebSocket message type.
#[derive(Debug, Clone)]
pub enum Message {
    /// Text message
    Text(String),
    /// Binary message
    Binary(Vec<u8>),
    /// Ping message
    Ping(Vec<u8>),
    /// Pong message
    Pong(Vec<u8>),
    /// Close message
    Close(Option<CloseFrame>),
}

/// Close frame with code and reason.
#[derive(Debug, Clone)]
pub struct CloseFrame {
    /// Close code
    pub code: u16,
    /// Close reason
    pub reason: String,
}

/// Message event for received messages.
#[derive(Debug, Clone)]
pub struct MessageEvent {
    /// The message data
    pub data: Message,
}

/// Close event when connection closes.
#[derive(Debug, Clone)]
pub struct CloseEvent {
    /// Close code
    pub code: u16,
    /// Close reason
    pub reason: String,
    /// Whether the close was clean
    pub was_clean: bool,
}

impl From<tungstenite::Message> for Message {
    fn from(msg: tungstenite::Message) -> Self {
        match msg {
            tungstenite::Message::Text(s) => Message::Text(s.to_string()),
            tungstenite::Message::Binary(b) => Message::Binary(b.to_vec()),
            tungstenite::Message::Ping(b) => Message::Ping(b.to_vec()),
            tungstenite::Message::Pong(b) => Message::Pong(b.to_vec()),
            tungstenite::Message::Close(frame) => Message::Close(frame.map(|f| CloseFrame {
                code: f.code.into(),
                reason: f.reason.to_string(),
            })),
            tungstenite::Message::Frame(_) => Message::Binary(vec![]),
        }
    }
}

impl From<Message> for tungstenite::Message {
    fn from(msg: Message) -> Self {
        match msg {
            Message::Text(s) => tungstenite::Message::Text(s),
            Message::Binary(b) => tungstenite::Message::Binary(b),
            Message::Ping(b) => tungstenite::Message::Ping(b),
            Message::Pong(b) => tungstenite::Message::Pong(b),
            Message::Close(frame) => {
                tungstenite::Message::Close(frame.map(|f| tungstenite::protocol::CloseFrame {
                    code: tungstenite::protocol::frame::coding::CloseCode::from(f.code),
                    reason: f.reason.into(),
                }))
            }
        }
    }
}

/// WebSocket client implementation.
///
/// Provides a WebSocket client compatible with the Web WebSocket API.
pub struct WebSocket {
    url: String,
    ready_state: Arc<RwLock<ReadyState>>,
    sender: Option<mpsc::Sender<Message>>,
    receiver: Arc<Mutex<mpsc::Receiver<MessageEvent>>>,
    close_receiver: Arc<Mutex<Option<CloseEvent>>>,
}

impl WebSocket {
    /// Create a new WebSocket connection.
    ///
    /// # Arguments
    /// * `url` - The WebSocket URL to connect to (ws:// or wss://)
    ///
    /// # Returns
    /// A new WebSocket instance in the Open state if successful.
    pub async fn new(url: &str) -> WebResult<Self> {
        let ready_state = Arc::new(RwLock::new(ReadyState::Connecting));
        let (msg_tx, msg_rx) = mpsc::channel::<MessageEvent>(100);
        let (send_tx, mut send_rx) = mpsc::channel::<Message>(100);
        let close_event: Arc<Mutex<Option<CloseEvent>>> = Arc::new(Mutex::new(None));

        // Connect to WebSocket server
        let (ws_stream, _response) = connect_async(url)
            .await
            .map_err(|e| WebError::WebSocket(format!("Connection failed: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();

        // Update ready state to Open
        *ready_state.write().await = ReadyState::Open;

        let ready_state_clone = Arc::clone(&ready_state);
        let close_event_clone = Arc::clone(&close_event);

        // Spawn task to handle incoming messages
        tokio::spawn(async move {
            while let Some(result) = read.next().await {
                match result {
                    Ok(msg) => {
                        let message: Message = msg.into();
                        if let Message::Close(frame) = &message {
                            *ready_state_clone.write().await = ReadyState::Closed;
                            let mut close_guard = close_event_clone.lock().await;
                            *close_guard = Some(CloseEvent {
                                code: frame.as_ref().map(|f| f.code).unwrap_or(1000),
                                reason: frame
                                    .as_ref()
                                    .map(|f| f.reason.clone())
                                    .unwrap_or_default(),
                                was_clean: true,
                            });
                            break;
                        }
                        let _ = msg_tx.send(MessageEvent { data: message }).await;
                    }
                    Err(_) => {
                        *ready_state_clone.write().await = ReadyState::Closed;
                        break;
                    }
                }
            }
        });

        // Spawn task to handle outgoing messages
        tokio::spawn(async move {
            while let Some(msg) = send_rx.recv().await {
                let tung_msg: tungstenite::Message = msg.into();
                if write.send(tung_msg).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            url: url.to_string(),
            ready_state,
            sender: Some(send_tx),
            receiver: Arc::new(Mutex::new(msg_rx)),
            close_receiver: close_event,
        })
    }

    /// Send a message through the WebSocket.
    ///
    /// # Arguments
    /// * `message` - The message to send (Text or Binary)
    pub async fn send(&self, message: Message) -> WebResult<()> {
        let state = *self.ready_state.read().await;
        if state != ReadyState::Open {
            return Err(WebError::WebSocket(format!("WebSocket is not open (state: {:?})", state)));
        }

        if let Some(sender) = &self.sender {
            sender
                .send(message)
                .await
                .map_err(|e| WebError::WebSocket(format!("Send failed: {}", e)))?;
        }
        Ok(())
    }

    /// Send a text message.
    pub async fn send_text(&self, text: &str) -> WebResult<()> {
        self.send(Message::Text(text.to_string())).await
    }

    /// Send a binary message.
    pub async fn send_binary(&self, data: Vec<u8>) -> WebResult<()> {
        self.send(Message::Binary(data)).await
    }

    /// Receive the next message.
    ///
    /// Returns None if the connection is closed.
    pub async fn recv(&self) -> Option<MessageEvent> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }

    /// Close the WebSocket connection.
    ///
    /// # Arguments
    /// * `code` - Optional close code (default: 1000)
    /// * `reason` - Optional close reason
    pub async fn close(&self, code: Option<u16>, reason: Option<&str>) -> WebResult<()> {
        let mut state = self.ready_state.write().await;
        if *state == ReadyState::Closed || *state == ReadyState::Closing {
            return Ok(());
        }

        *state = ReadyState::Closing;

        if let Some(sender) = &self.sender {
            let close_frame = CloseFrame {
                code: code.unwrap_or(1000),
                reason: reason.unwrap_or("").to_string(),
            };
            let _ = sender.send(Message::Close(Some(close_frame))).await;
        }

        *state = ReadyState::Closed;
        Ok(())
    }

    /// Get the current ready state.
    pub async fn ready_state(&self) -> ReadyState {
        *self.ready_state.read().await
    }

    /// Get the URL of the WebSocket.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Check if the WebSocket is open.
    pub async fn is_open(&self) -> bool {
        *self.ready_state.read().await == ReadyState::Open
    }

    /// Get the close event if the connection was closed.
    pub async fn close_event(&self) -> Option<CloseEvent> {
        self.close_receiver.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ready_state_values() {
        assert_eq!(ReadyState::Connecting as u8, 0);
        assert_eq!(ReadyState::Open as u8, 1);
        assert_eq!(ReadyState::Closing as u8, 2);
        assert_eq!(ReadyState::Closed as u8, 3);
    }

    #[test]
    fn test_message_conversion() {
        let text_msg = Message::Text("hello".to_string());
        let tung_msg: tungstenite::Message = text_msg.into();
        assert!(matches!(tung_msg, tungstenite::Message::Text(_)));

        let binary_msg = Message::Binary(vec![1, 2, 3]);
        let tung_msg: tungstenite::Message = binary_msg.into();
        assert!(matches!(tung_msg, tungstenite::Message::Binary(_)));
    }

    #[test]
    fn test_close_frame() {
        let frame = CloseFrame {
            code: 1000,
            reason: "Normal closure".to_string(),
        };
        assert_eq!(frame.code, 1000);
        assert_eq!(frame.reason, "Normal closure");
    }
}
