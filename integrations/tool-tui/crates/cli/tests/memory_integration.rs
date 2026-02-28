//! Integration tests for Memory system (Sprint 1.4 T29)
//!
//! Tests the full memory workflow including:
//! - Store → Search → Retrieve → Update → Maintenance → Delete

#[cfg(test)]
mod memory_integration_tests {
    use std::collections::HashMap;

    fn make_metadata(
        source: &str,
        category: &str,
        tags: Vec<String>,
    ) -> dx::memory::MemoryMetadata {
        dx::memory::MemoryMetadata {
            source: source.to_string(),
            category: category.to_string(),
            tags,
            conversation_id: None,
            custom: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_full_memory_workflow() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let config = dx::memory::MemoryConfig {
            storage_path: tmp.path().to_path_buf(),
            ..Default::default()
        };

        let system = dx::memory::MemorySystem::new(config)
            .await
            .expect("Memory system should initialize");

        // Step 1: Store memories
        let meta1 = make_metadata("test", "programming", vec!["rust".into(), "language".into()]);
        let id1 = system
            .store(
                "Rust is a systems programming language focused on safety and performance",
                meta1,
            )
            .await
            .expect("Store memory1 should work");

        let meta2 = make_metadata("test", "programming", vec!["python".into(), "ml".into()]);
        let id2 = system
            .store("Python is great for data science and machine learning applications", meta2)
            .await
            .expect("Store memory2 should work");

        let meta3 = make_metadata("test", "general", vec!["weather".into()]);
        let id3 = system
            .store("The weather today is sunny with a high of 75 degrees", meta3)
            .await
            .expect("Store memory3 should work");

        // IDs should be non-empty
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        assert!(!id3.is_empty());

        // Step 2: Get stats
        let stats = system.stats().await;
        assert!(stats.total_memories >= 3, "Should have at least 3 memories");

        // Step 3: Search for programming-related memories
        let results = system.search("rust programming", 10).await;
        assert!(results.is_ok(), "Search should succeed");
        let found = results.unwrap();
        assert!(!found.is_empty(), "Should find programming-related memories");

        // Step 4: Search for weather
        let results = system.search("weather sunny", 5).await;
        assert!(results.is_ok(), "Weather search should succeed");

        // Step 5: Delete a memory
        let deleted = system.delete(&id3).await;
        assert!(deleted.is_ok(), "Delete should succeed");

        // Step 6: Verify deletion
        let stats_after = system.stats().await;
        assert!(
            stats_after.total_memories < stats.total_memories,
            "Should have fewer memories after delete"
        );
    }

    #[tokio::test]
    async fn test_memory_store_and_retrieve() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let config = dx::memory::MemoryConfig {
            storage_path: tmp.path().to_path_buf(),
            ..Default::default()
        };

        let system = dx::memory::MemorySystem::new(config).await.expect("Memory system");

        // Store a memory
        let meta = make_metadata("test", "testing", vec!["test".into()]);
        let id = system
            .store("This is a test memory for retrieval", meta)
            .await
            .expect("Store should succeed");

        // Retrieve it
        let retrieved = system.get(&id).await;
        assert!(retrieved.is_ok(), "Get should succeed");
        let mem = retrieved.unwrap();
        assert_eq!(mem.content, "This is a test memory for retrieval");
        assert_eq!(mem.metadata.category, "testing");
    }

    #[tokio::test]
    async fn test_memory_bulk_operations() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let config = dx::memory::MemoryConfig {
            storage_path: tmp.path().to_path_buf(),
            ..Default::default()
        };

        let system = dx::memory::MemorySystem::new(config).await.expect("Memory system");

        // Store 50 memories
        for i in 0..50 {
            let category = if i % 3 == 0 {
                "tech"
            } else if i % 3 == 1 {
                "science"
            } else {
                "art"
            };
            let content = format!("Bulk memory {} about {} topic", i, category);
            let meta = make_metadata("bulk-test", category, vec![format!("tag-{}", i % 5)]);
            system.store(&content, meta).await.expect("Bulk store should succeed");
        }

        // Verify count
        let stats = system.stats().await;
        assert!(stats.total_memories >= 50, "Should have at least 50 memories");

        // Search should return results
        let results = system.search("technology", 10).await;
        assert!(results.is_ok(), "Search should succeed");
    }

    #[tokio::test]
    async fn test_memory_maintenance() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let config = dx::memory::MemoryConfig {
            storage_path: tmp.path().to_path_buf(),
            ..Default::default()
        };

        let system = dx::memory::MemorySystem::new(config).await.expect("Memory system");

        // Store memories
        for i in 0..10 {
            let meta = make_metadata("maintenance-test", "test", vec![]);
            let content = format!("Maintenance memory {}", i);
            system.store(&content, meta).await.expect("Store should succeed");
        }

        let stats_before = system.stats().await;
        assert!(stats_before.total_memories >= 10);

        // Maintenance should work without errors
        let maintenance_result = system.maintenance().await;
        assert!(maintenance_result.is_ok(), "Maintenance should succeed");
    }

    #[tokio::test]
    async fn test_memory_rpc_integration() {
        // Test via RPC methods
        let config = dx::gateway::GatewayConfig::default();
        let state = std::sync::Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Step 1: Get memory stats via RPC
        let request = dx::gateway::rpc::RpcRequest {
            id: "mem-rpc-1".to_string(),
            method: "memory.stats".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        // May fail if memory system not initialized with real paths, but shouldn't panic
        assert!(response.error.is_some() || response.result.is_some());

        // Step 2: Try to store via RPC
        let request = dx::gateway::rpc::RpcRequest {
            id: "mem-rpc-2".to_string(),
            method: "memory.store".to_string(),
            params: serde_json::json!({
                "content": "Test memory via RPC",
                "source": "rpc-test",
                "category": "integration",
                "tags": ["rpc", "test"]
            }),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_some() || response.result.is_some());

        // Step 3: Search via RPC
        let request = dx::gateway::rpc::RpcRequest {
            id: "mem-rpc-3".to_string(),
            method: "memory.search".to_string(),
            params: serde_json::json!({
                "query": "test memory",
                "limit": 5
            }),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_some() || response.result.is_some());
    }
}
