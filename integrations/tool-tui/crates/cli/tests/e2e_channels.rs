//! End-to-End tests for messaging channel integrations
//!
//! These tests verify that messaging channels work correctly,
//! including message sending, receiving, and event handling.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc, oneshot};

// ============================================================================
// Mock Channel Infrastructure
// ============================================================================

/// Mock message for testing
#[derive(Debug, Clone)]
pub struct MockMessage {
    pub id: String,
    pub channel_id: String,
    pub content: String,
    pub author: String,
    pub timestamp: u64,
}

impl MockMessage {
    pub fn new(id: &str, channel_id: &str, content: &str, author: &str) -> Self {
        Self {
            id: id.to_string(),
            channel_id: channel_id.to_string(),
            content: content.to_string(),
            author: author.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Mock event for testing
#[derive(Debug, Clone)]
pub enum MockEvent {
    MessageReceived(MockMessage),
    MessageEdited {
        id: String,
        new_content: String,
    },
    MessageDeleted {
        id: String,
    },
    ReactionAdded {
        message_id: String,
        emoji: String,
        user: String,
    },
    TypingStarted {
        channel_id: String,
        user: String,
    },
    Connected,
    Disconnected {
        reason: String,
    },
}

/// Mock channel client
pub struct MockChannelClient {
    name: String,
    connected: bool,
    messages: Vec<MockMessage>,
    event_tx: mpsc::UnboundedSender<MockEvent>,
    event_rx: Option<mpsc::UnboundedReceiver<MockEvent>>,
}

impl MockChannelClient {
    pub fn new(name: &str) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            name: name.to_string(),
            connected: false,
            messages: Vec::new(),
            event_tx: tx,
            event_rx: Some(rx),
        }
    }

    pub fn connect(&mut self) -> Result<(), String> {
        if self.connected {
            return Err("Already connected".to_string());
        }
        self.connected = true;
        let _ = self.event_tx.send(MockEvent::Connected);
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected".to_string());
        }
        self.connected = false;
        let _ = self.event_tx.send(MockEvent::Disconnected {
            reason: "User requested".to_string(),
        });
        Ok(())
    }

    pub fn send_message(&mut self, channel_id: &str, content: &str) -> Result<MockMessage, String> {
        if !self.connected {
            return Err("Not connected".to_string());
        }

        let msg = MockMessage::new(
            &format!("msg_{}", self.messages.len() + 1),
            channel_id,
            content,
            "bot",
        );
        self.messages.push(msg.clone());
        Ok(msg)
    }

    pub fn edit_message(&mut self, message_id: &str, new_content: &str) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected".to_string());
        }

        for msg in &mut self.messages {
            if msg.id == message_id {
                msg.content = new_content.to_string();
                let _ = self.event_tx.send(MockEvent::MessageEdited {
                    id: message_id.to_string(),
                    new_content: new_content.to_string(),
                });
                return Ok(());
            }
        }
        Err("Message not found".to_string())
    }

    pub fn delete_message(&mut self, message_id: &str) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected".to_string());
        }

        let len_before = self.messages.len();
        self.messages.retain(|m| m.id != message_id);

        if self.messages.len() < len_before {
            let _ = self.event_tx.send(MockEvent::MessageDeleted {
                id: message_id.to_string(),
            });
            Ok(())
        } else {
            Err("Message not found".to_string())
        }
    }

    pub fn add_reaction(&self, message_id: &str, emoji: &str) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected".to_string());
        }

        let _ = self.event_tx.send(MockEvent::ReactionAdded {
            message_id: message_id.to_string(),
            emoji: emoji.to_string(),
            user: "bot".to_string(),
        });
        Ok(())
    }

    pub fn simulate_incoming(&self, message: MockMessage) {
        let _ = self.event_tx.send(MockEvent::MessageReceived(message));
    }

    pub fn take_event_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<MockEvent>> {
        self.event_rx.take()
    }

    pub fn messages(&self) -> &[MockMessage] {
        &self.messages
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_channel_connect_disconnect() {
    let mut client = MockChannelClient::new("discord");

    assert!(!client.is_connected());

    client.connect().unwrap();
    assert!(client.is_connected());

    // Double connect should fail
    assert!(client.connect().is_err());

    client.disconnect().unwrap();
    assert!(!client.is_connected());

    // Double disconnect should fail
    assert!(client.disconnect().is_err());
}

#[test]
fn test_send_message() {
    let mut client = MockChannelClient::new("slack");
    client.connect().unwrap();

    let msg = client.send_message("general", "Hello, world!").unwrap();

    assert_eq!(msg.channel_id, "general");
    assert_eq!(msg.content, "Hello, world!");
    assert_eq!(msg.author, "bot");
    assert_eq!(client.messages().len(), 1);
}

#[test]
fn test_send_when_disconnected() {
    let mut client = MockChannelClient::new("telegram");

    // Should fail when not connected
    let result = client.send_message("chat", "Hello");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Not connected");
}

#[test]
fn test_edit_message() {
    let mut client = MockChannelClient::new("discord");
    client.connect().unwrap();

    let msg = client.send_message("general", "Original").unwrap();
    let msg_id = msg.id.clone();

    client.edit_message(&msg_id, "Edited").unwrap();

    assert_eq!(client.messages()[0].content, "Edited");
}

#[test]
fn test_delete_message() {
    let mut client = MockChannelClient::new("slack");
    client.connect().unwrap();

    let msg1 = client.send_message("general", "Message 1").unwrap();
    let msg2 = client.send_message("general", "Message 2").unwrap();

    assert_eq!(client.messages().len(), 2);

    client.delete_message(&msg1.id).unwrap();

    assert_eq!(client.messages().len(), 1);
    assert_eq!(client.messages()[0].id, msg2.id);
}

#[test]
fn test_add_reaction() {
    let mut client = MockChannelClient::new("discord");
    client.connect().unwrap();

    let msg = client.send_message("general", "React to this").unwrap();

    client.add_reaction(&msg.id, "👍").unwrap();
    // Reaction is sent as event, verify no error
}

#[tokio::test]
async fn test_event_streaming() {
    let mut client = MockChannelClient::new("discord");
    let mut rx = client.take_event_receiver().unwrap();

    client.connect().unwrap();

    // Should receive Connected event
    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert!(matches!(event, MockEvent::Connected));

    // Simulate incoming message
    client.simulate_incoming(MockMessage::new("ext_1", "general", "Hello!", "user123"));

    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();

    if let MockEvent::MessageReceived(msg) = event {
        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.author, "user123");
    } else {
        panic!("Expected MessageReceived event");
    }
}

#[tokio::test]
async fn test_message_flow() {
    let mut client = MockChannelClient::new("slack");
    let mut rx = client.take_event_receiver().unwrap();

    client.connect().unwrap();

    // Consume Connected event
    let _ = rx.recv().await;

    // Send message
    let msg = client.send_message("general", "Outgoing").unwrap();

    // Edit message
    client.edit_message(&msg.id, "Edited").unwrap();

    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();

    if let MockEvent::MessageEdited { id, new_content } = event {
        assert_eq!(id, msg.id);
        assert_eq!(new_content, "Edited");
    } else {
        panic!("Expected MessageEdited event");
    }

    // Delete message
    client.delete_message(&msg.id).unwrap();

    let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert!(matches!(event, MockEvent::MessageDeleted { .. }));
}

/// Test multi-channel management
#[test]
fn test_multi_channel_manager() {
    struct ChannelManager {
        channels: HashMap<String, MockChannelClient>,
    }

    impl ChannelManager {
        fn new() -> Self {
            Self {
                channels: HashMap::new(),
            }
        }

        fn add_channel(&mut self, name: &str) {
            self.channels.insert(name.to_string(), MockChannelClient::new(name));
        }

        fn connect_all(&mut self) -> Vec<Result<(), String>> {
            self.channels.values_mut().map(|c| c.connect()).collect()
        }

        fn broadcast(&mut self, message: &str) -> Vec<Result<MockMessage, String>> {
            self.channels
                .values_mut()
                .map(|c| c.send_message("broadcast", message))
                .collect()
        }

        fn disconnect_all(&mut self) -> Vec<Result<(), String>> {
            self.channels.values_mut().map(|c| c.disconnect()).collect()
        }
    }

    let mut manager = ChannelManager::new();
    manager.add_channel("discord");
    manager.add_channel("slack");
    manager.add_channel("telegram");

    let connect_results = manager.connect_all();
    assert!(connect_results.iter().all(|r| r.is_ok()));

    let broadcast_results = manager.broadcast("Hello everyone!");
    assert!(broadcast_results.iter().all(|r| r.is_ok()));

    let disconnect_results = manager.disconnect_all();
    assert!(disconnect_results.iter().all(|r| r.is_ok()));
}

/// Test rate limiting
#[tokio::test]
async fn test_rate_limiting() {
    use std::time::Instant;

    struct RateLimiter {
        window: Duration,
        max_requests: usize,
        requests: Vec<Instant>,
    }

    impl RateLimiter {
        fn new(window: Duration, max_requests: usize) -> Self {
            Self {
                window,
                max_requests,
                requests: Vec::new(),
            }
        }

        fn can_proceed(&mut self) -> bool {
            let now = Instant::now();

            // Remove old requests
            self.requests.retain(|&t| now.duration_since(t) < self.window);

            if self.requests.len() < self.max_requests {
                self.requests.push(now);
                true
            } else {
                false
            }
        }
    }

    let mut limiter = RateLimiter::new(Duration::from_millis(100), 3);

    // Should allow first 3 requests
    assert!(limiter.can_proceed());
    assert!(limiter.can_proceed());
    assert!(limiter.can_proceed());

    // 4th should be blocked
    assert!(!limiter.can_proceed());

    // Wait for window to expire
    tokio::time::sleep(Duration::from_millis(120)).await;

    // Should allow again
    assert!(limiter.can_proceed());
}

/// Test message queuing
#[test]
fn test_message_queue() {
    struct MessageQueue {
        queue: Vec<(String, String)>, // (channel, message)
        max_size: usize,
    }

    impl MessageQueue {
        fn new(max_size: usize) -> Self {
            Self {
                queue: Vec::new(),
                max_size,
            }
        }

        fn enqueue(&mut self, channel: &str, message: &str) -> Result<(), &'static str> {
            if self.queue.len() >= self.max_size {
                return Err("Queue full");
            }
            self.queue.push((channel.to_string(), message.to_string()));
            Ok(())
        }

        fn dequeue(&mut self) -> Option<(String, String)> {
            if self.queue.is_empty() {
                None
            } else {
                Some(self.queue.remove(0))
            }
        }

        fn len(&self) -> usize {
            self.queue.len()
        }

        fn is_empty(&self) -> bool {
            self.queue.is_empty()
        }
    }

    let mut queue = MessageQueue::new(3);

    assert!(queue.is_empty());

    queue.enqueue("general", "Message 1").unwrap();
    queue.enqueue("general", "Message 2").unwrap();
    queue.enqueue("general", "Message 3").unwrap();

    assert!(queue.enqueue("general", "Message 4").is_err());

    let (ch, msg) = queue.dequeue().unwrap();
    assert_eq!(ch, "general");
    assert_eq!(msg, "Message 1");

    assert_eq!(queue.len(), 2);
}

/// Test webhook verification
#[test]
fn test_webhook_signature_verification() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn compute_signature(secret: &str, payload: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        secret.hash(&mut hasher);
        payload.hash(&mut hasher);
        hasher.finish()
    }

    fn verify_webhook(secret: &str, payload: &str, signature: u64) -> bool {
        compute_signature(secret, payload) == signature
    }

    let secret = "webhook_secret_123";
    let payload = r#"{"event": "message", "data": "hello"}"#;

    let signature = compute_signature(secret, payload);

    assert!(verify_webhook(secret, payload, signature));
    assert!(!verify_webhook("wrong_secret", payload, signature));
    assert!(!verify_webhook(secret, "tampered payload", signature));
}

/// Test channel configuration
#[test]
fn test_channel_config() {
    #[derive(Debug)]
    struct ChannelConfig {
        name: String,
        enabled: bool,
        token: Option<String>,
        webhook_url: Option<String>,
        default_channel: Option<String>,
        rate_limit: Option<(usize, Duration)>,
    }

    impl ChannelConfig {
        fn from_sr(content: &str) -> Option<Self> {
            let mut name = None;
            let mut enabled = true;
            let mut token = None;
            let mut webhook_url = None;
            let mut default_channel = None;

            for line in content.lines() {
                let line = line.trim();
                if let Some(val) = line.strip_prefix("name = \"").and_then(|s| s.strip_suffix('"'))
                {
                    name = Some(val.to_string());
                } else if let Some(val) = line.strip_prefix("enabled = ") {
                    enabled = val == "true";
                } else if let Some(val) =
                    line.strip_prefix("token = \"").and_then(|s| s.strip_suffix('"'))
                {
                    token = Some(val.to_string());
                } else if let Some(val) =
                    line.strip_prefix("webhook_url = \"").and_then(|s| s.strip_suffix('"'))
                {
                    webhook_url = Some(val.to_string());
                } else if let Some(val) =
                    line.strip_prefix("default_channel = \"").and_then(|s| s.strip_suffix('"'))
                {
                    default_channel = Some(val.to_string());
                }
            }

            Some(Self {
                name: name?,
                enabled,
                token,
                webhook_url,
                default_channel,
                rate_limit: None,
            })
        }
    }

    let config = r#"
        name = "discord"
        enabled = true
        token = "BOT_TOKEN_HERE"
        default_channel = "general"
    "#;

    let parsed = ChannelConfig::from_sr(config).unwrap();
    assert_eq!(parsed.name, "discord");
    assert!(parsed.enabled);
    assert_eq!(parsed.token.unwrap(), "BOT_TOKEN_HERE");
    assert_eq!(parsed.default_channel.unwrap(), "general");
}
