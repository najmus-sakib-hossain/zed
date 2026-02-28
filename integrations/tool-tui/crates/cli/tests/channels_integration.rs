//! Integration tests for messaging channels

#[cfg(test)]
mod tests {
    use dx_cli::channels::{ChannelCredentials, CredentialsStore, MessageQueue, QueuedMessage};

    #[tokio::test]
    async fn test_credentials_store() {
        let mut store = CredentialsStore::default();

        let mut creds = ChannelCredentials::new("test".to_string());
        creds.add("token".to_string(), "test_token".to_string());

        store.set("test".to_string(), creds);

        let retrieved = store.get("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().get("token").unwrap(), "test_token");
    }

    #[tokio::test]
    async fn test_message_queue() {
        let queue = MessageQueue::new();

        let msg =
            QueuedMessage::new("telegram".to_string(), "user123".to_string(), "Hello".to_string());

        queue.enqueue(msg).await;
        assert_eq!(queue.size().await, 1);

        let dequeued = queue.dequeue().await;
        assert!(dequeued.is_some());
        assert_eq!(queue.size().await, 0);
    }

    #[tokio::test]
    async fn test_message_retry() {
        let queue = MessageQueue::new();

        let msg =
            QueuedMessage::new("telegram".to_string(), "user123".to_string(), "Hello".to_string());

        queue.enqueue(msg.clone()).await;
        let mut msg = queue.dequeue().await.unwrap();

        // Simulate failures
        for _ in 0..3 {
            queue.mark_failed(msg.clone()).await;
            msg = queue.dequeue().await.unwrap();
        }

        // Final failure should move to failed list
        queue.mark_failed(msg).await;
        assert_eq!(queue.size().await, 0);
        assert_eq!(queue.failed_count().await, 1);
    }
}
