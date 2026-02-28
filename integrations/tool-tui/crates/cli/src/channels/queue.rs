//! Message queue system for reliable message delivery

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedMessage {
    pub id: String,
    pub channel: String,
    pub recipient: String,
    pub content: String,
    pub timestamp: i64,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl QueuedMessage {
    pub fn new(channel: String, recipient: String, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channel,
            recipient,
            content,
            timestamp: chrono::Utc::now().timestamp(),
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Message queue with retry logic
pub struct MessageQueue {
    queue: Arc<Mutex<VecDeque<QueuedMessage>>>,
    failed: Arc<Mutex<Vec<QueuedMessage>>>,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            failed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add message to queue
    pub async fn enqueue(&self, message: QueuedMessage) {
        let mut queue = self.queue.lock().await;
        queue.push_back(message);
    }

    /// Get next message from queue
    pub async fn dequeue(&self) -> Option<QueuedMessage> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }

    /// Mark message as failed
    pub async fn mark_failed(&self, mut message: QueuedMessage) {
        if message.can_retry() {
            message.increment_retry();
            let mut queue = self.queue.lock().await;
            queue.push_back(message);
        } else {
            let mut failed = self.failed.lock().await;
            failed.push(message);
        }
    }

    /// Get queue size
    pub async fn size(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }

    /// Get failed messages count
    pub async fn failed_count(&self) -> usize {
        let failed = self.failed.lock().await;
        failed.len()
    }

    /// Clear queue
    pub async fn clear(&self) {
        let mut queue = self.queue.lock().await;
        queue.clear();
    }

    /// Get all failed messages
    pub async fn get_failed(&self) -> Vec<QueuedMessage> {
        let failed = self.failed.lock().await;
        failed.clone()
    }

    /// Retry all failed messages
    pub async fn retry_failed(&self) {
        let mut failed = self.failed.lock().await;
        let mut queue = self.queue.lock().await;

        for mut msg in failed.drain(..) {
            msg.retry_count = 0;
            queue.push_back(msg);
        }
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_queue() {
        let queue = MessageQueue::new();

        let msg =
            QueuedMessage::new("telegram".to_string(), "user123".to_string(), "Hello".to_string());

        queue.enqueue(msg.clone()).await;
        assert_eq!(queue.size().await, 1);

        let dequeued = queue.dequeue().await;
        assert!(dequeued.is_some());
        assert_eq!(queue.size().await, 0);
    }

    #[tokio::test]
    async fn test_retry_logic() {
        let queue = MessageQueue::new();

        let msg =
            QueuedMessage::new("telegram".to_string(), "user123".to_string(), "Hello".to_string());

        queue.enqueue(msg.clone()).await;
        let mut msg = queue.dequeue().await.unwrap();

        // Fail 3 times
        for _ in 0..3 {
            queue.mark_failed(msg.clone()).await;
            msg = queue.dequeue().await.unwrap();
        }

        // 4th failure should move to failed list
        queue.mark_failed(msg).await;
        assert_eq!(queue.size().await, 0);
        assert_eq!(queue.failed_count().await, 1);
    }
}
