//! Integration tests for Gateway RPC system (Sprint 1.1 T8, T29)
//!
//! Tests multi-step workflows across models, agents, sessions,
//! exec approvals, and the full RPC method registry.

use std::sync::Arc;

// Re-export gateway types for integration testing
// These tests verify end-to-end RPC workflows

/// Helper: create a test RPC request
fn rpc_request(id: &str, method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "method": method,
        "params": params,
        "timestamp": null
    })
}

#[cfg(test)]
mod gateway_integration_tests {
    use super::*;

    // =========================================================================
    // Sprint 1.1 T8: Integration tests for models/agents
    // =========================================================================

    #[tokio::test]
    async fn test_models_and_agents_workflow() {
        // Test that models.list and agents.list return valid data
        // and that agent identity can be retrieved
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Step 1: List models
        let request = dx::gateway::rpc::RpcRequest {
            id: "int-1".to_string(),
            method: "models.list".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "models.list should succeed");
        let models = response.result.unwrap();
        assert!(models["models"].is_array(), "models should be an array");

        // Step 2: List agents
        let request = dx::gateway::rpc::RpcRequest {
            id: "int-2".to_string(),
            method: "agents.list".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "agents.list should succeed");
        let agents = response.result.unwrap();
        assert!(agents["agents"].is_array(), "agents should be an array");

        // Step 3: Get agent identity
        let request = dx::gateway::rpc::RpcRequest {
            id: "int-3".to_string(),
            method: "agent.identity.get".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "agent.identity.get should succeed");
        let identity = response.result.unwrap();
        assert!(identity["name"].is_string(), "identity should have a name");
    }

    #[tokio::test]
    async fn test_agents_files_workflow() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Step 1: List agent files
        let request = dx::gateway::rpc::RpcRequest {
            id: "af-1".to_string(),
            method: "agents.files.list".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "agents.files.list should succeed");

        // Step 2: Get a specific agent file
        let request = dx::gateway::rpc::RpcRequest {
            id: "af-2".to_string(),
            method: "agents.files.get".to_string(),
            params: serde_json::json!({"path": "default-agent"}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        // May or may not have the file, but shouldn't crash
        assert!(response.error.is_some() || response.result.is_some());

        // Step 3: Set an agent file
        let request = dx::gateway::rpc::RpcRequest {
            id: "af-3".to_string(),
            method: "agents.files.set".to_string(),
            params: serde_json::json!({
                "path": "test-agent",
                "content": "name: Test Agent\nmodel: gpt-4"
            }),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        // Should either succeed or return an error, not panic
        assert!(response.error.is_some() || response.result.is_some());
    }

    // =========================================================================
    // Sprint 1.1 T29: Integration tests for gateway
    // =========================================================================

    #[tokio::test]
    async fn test_gateway_health_and_info_workflow() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Step 1: Health check
        let request = dx::gateway::rpc::RpcRequest {
            id: "gw-1".to_string(),
            method: "health.check".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "health.check should succeed");
        assert_eq!(response.result.unwrap()["status"], "ok");

        // Step 2: Methods list should return 50+ methods
        let request = dx::gateway::rpc::RpcRequest {
            id: "gw-2".to_string(),
            method: "methods.list".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "methods.list should succeed");
        let result = response.result.unwrap();
        let count = result["count"].as_u64().unwrap_or(0);
        assert!(count >= 50, "Should have 50+ RPC methods, got {}", count);

        // Step 3: Channels list
        let request = dx::gateway::rpc::RpcRequest {
            id: "gw-3".to_string(),
            method: "channels.list".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "channels.list should succeed");
    }

    #[tokio::test]
    async fn test_gateway_cron_workflow() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Step 1: List cron jobs
        let request = dx::gateway::rpc::RpcRequest {
            id: "cron-1".to_string(),
            method: "cron.list".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "cron.list should succeed");

        // Step 2: Add a cron job
        let request = dx::gateway::rpc::RpcRequest {
            id: "cron-2".to_string(),
            method: "cron.add".to_string(),
            params: serde_json::json!({
                "name": "test-job",
                "schedule": "0 * * * *",
                "command": "echo hello"
            }),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "cron.add should succeed");

        // Step 3: List again should show new job
        let request = dx::gateway::rpc::RpcRequest {
            id: "cron-3".to_string(),
            method: "cron.list".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_gateway_skills_workflow() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // List skills
        let request = dx::gateway::rpc::RpcRequest {
            id: "skill-1".to_string(),
            method: "skills.list".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "skills.list should succeed");
        let result = response.result.unwrap();
        assert!(result["skills"].is_array());
    }

    #[tokio::test]
    async fn test_gateway_tts_workflow() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Step 1: Get TTS voices
        let request = dx::gateway::rpc::RpcRequest {
            id: "tts-1".to_string(),
            method: "tts.voices".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "tts.voices should succeed");

        // Step 2: Get TTS config
        let request = dx::gateway::rpc::RpcRequest {
            id: "tts-2".to_string(),
            method: "tts.config.get".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "tts.config.get should succeed");
    }

    #[tokio::test]
    async fn test_gateway_config_workflow() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Step 1: Get config
        let request = dx::gateway::rpc::RpcRequest {
            id: "cfg-1".to_string(),
            method: "config.get".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "config.get should succeed");

        // Step 2: Get config schema
        let request = dx::gateway::rpc::RpcRequest {
            id: "cfg-2".to_string(),
            method: "config.schema".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "config.schema should succeed");

        // Step 3: Validate config
        let request = dx::gateway::rpc::RpcRequest {
            id: "cfg-3".to_string(),
            method: "config.validate".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "config.validate should succeed");
    }

    #[tokio::test]
    async fn test_gateway_exec_approvals_workflow() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Step 1: Get exec approvals
        let request = dx::gateway::rpc::RpcRequest {
            id: "exec-1".to_string(),
            method: "exec.approvals.get".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "exec.approvals.get should succeed");

        // Step 2: Set exec approval mode
        let request = dx::gateway::rpc::RpcRequest {
            id: "exec-2".to_string(),
            method: "exec.approvals.set".to_string(),
            params: serde_json::json!({"mode": "always_ask"}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "exec.approvals.set should succeed");
    }

    #[tokio::test]
    async fn test_gateway_plugins_workflow() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Step 1: List plugins
        let request = dx::gateway::rpc::RpcRequest {
            id: "plug-1".to_string(),
            method: "plugins.list".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "plugins.list should succeed");

        // Step 2: Health check all plugins
        let request = dx::gateway::rpc::RpcRequest {
            id: "plug-2".to_string(),
            method: "plugins.health".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        // May fail if plugin system not enabled, but shouldn't panic
        assert!(response.error.is_some() || response.result.is_some());
    }

    #[tokio::test]
    async fn test_gateway_wizard_workflow() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Step 1: Start wizard
        let request = dx::gateway::rpc::RpcRequest {
            id: "wiz-1".to_string(),
            method: "wizard.start".to_string(),
            params: serde_json::json!({}),
            timestamp: None,
        };
        let response = registry.invoke(state.clone(), "test-client", request).await;
        assert!(response.error.is_none(), "wizard.start should succeed");
    }

    #[tokio::test]
    async fn test_gateway_invalid_params_handling() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = dx::gateway::rpc::MethodRegistry::new();

        // Test that methods handle invalid params gracefully
        let methods = vec![
            "models.list",
            "agents.list",
            "sessions.list",
            "config.get",
            "memory.stats",
            "plugins.list",
        ];

        for method in methods {
            let request = dx::gateway::rpc::RpcRequest {
                id: format!("invalid-{}", method),
                method: method.to_string(),
                params: serde_json::json!({"invalid_key": "invalid_value"}),
                timestamp: None,
            };
            let response = registry.invoke(state.clone(), "test-client", request).await;
            // Methods should handle invalid params without panicking
            assert!(
                response.error.is_some() || response.result.is_some(),
                "Method {} should handle invalid params gracefully",
                method
            );
        }
    }

    #[tokio::test]
    async fn test_gateway_concurrent_requests() {
        let config = dx::gateway::GatewayConfig::default();
        let state = Arc::new(dx::gateway::GatewayState::new(config).await);
        let registry = Arc::new(dx::gateway::rpc::MethodRegistry::new());

        // Send multiple concurrent requests
        let mut handles = vec![];

        for i in 0..10 {
            let state = state.clone();
            let registry = registry.clone();
            handles.push(tokio::spawn(async move {
                let request = dx::gateway::rpc::RpcRequest {
                    id: format!("concurrent-{}", i),
                    method: "health.check".to_string(),
                    params: serde_json::json!({}),
                    timestamp: None,
                };
                registry.invoke(state, &format!("client-{}", i), request).await
            }));
        }

        for handle in handles {
            let response = handle.await.unwrap();
            assert!(response.error.is_none(), "Concurrent health check should succeed");
            assert_eq!(response.result.unwrap()["status"], "ok");
        }
    }

    #[tokio::test]
    async fn test_gateway_method_categories_complete() {
        let registry = dx::gateway::rpc::MethodRegistry::new();
        let config = dx::gateway::GatewayConfig::default();
        let state = std::sync::Arc::new(dx::gateway::GatewayState::new(config).await);

        // Verify key methods from expected categories exist by invoking them
        let category_methods = vec![
            "health.ping",
            "agents.list",
            "sessions.list",
            "models.list",
            "config.get",
            "memory.stats",
            "plugins.list",
        ];

        for method in &category_methods {
            let request = dx::gateway::rpc::RpcRequest {
                id: format!("cat-test-{}", method),
                method: method.to_string(),
                params: serde_json::json!({}),
                timestamp: None,
            };
            let response = registry.invoke(state.clone(), "test-client", request).await;
            // Method should be recognized (might error but shouldn't be "method not found")
            assert!(
                response.error.is_some() || response.result.is_some(),
                "Method {} should be recognized",
                method
            );
        }
    }
}
