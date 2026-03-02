//! Hybrid Workflow Executor
//!
//! This module provides a hybrid execution engine that uses native Rust nodes
//! where available and falls back to the n8n sidecar for nodes that haven't
//! been ported to Rust yet.
//!
//! This allows for gradual migration from n8n to pure Rust while maintaining
//! compatibility with all 500+ n8n integrations.

use crate::protocol::{WorkflowDefinition, WorkflowResult};
use crate::rust_nodes::RustNodeRegistry;
use crate::sidecar::N8nSidecar;
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;

/// The Hybrid Workflow Executor
///
/// Executes workflows using native Rust nodes when available, falling back
/// to the n8n sidecar for nodes that haven't been ported yet.
pub struct HybridWorkflowExecutor {
    /// Registry of native Rust nodes
    rust_nodes: RustNodeRegistry,

    /// The n8n sidecar for fallback execution
    n8n_sidecar: Arc<N8nSidecar>,

    /// Execution statistics
    stats: parking_lot::Mutex<ExecutionStats>,
}

/// Statistics about hybrid execution
#[derive(Debug, Default, Clone)]
pub struct ExecutionStats {
    /// Total workflows executed
    pub total_executions: u64,

    /// Workflows executed entirely in Rust
    pub rust_native_executions: u64,

    /// Workflows that required n8n fallback
    pub n8n_fallback_executions: u64,

    /// Total nodes executed in Rust
    pub rust_nodes_executed: u64,

    /// Total nodes executed via n8n
    pub n8n_nodes_executed: u64,

    /// Total execution time in milliseconds (Rust)
    pub rust_execution_time_ms: u64,

    /// Total execution time in milliseconds (n8n)
    pub n8n_execution_time_ms: u64,
}

impl HybridWorkflowExecutor {
    /// Create a new hybrid executor
    pub fn new(n8n_sidecar: Arc<N8nSidecar>) -> Self {
        Self {
            rust_nodes: RustNodeRegistry::new(),
            n8n_sidecar,
            stats: parking_lot::Mutex::new(ExecutionStats::default()),
        }
    }

    /// Create with a custom Rust node registry
    pub fn with_registry(n8n_sidecar: Arc<N8nSidecar>, rust_nodes: RustNodeRegistry) -> Self {
        Self {
            rust_nodes,
            n8n_sidecar,
            stats: parking_lot::Mutex::new(ExecutionStats::default()),
        }
    }

    /// Execute a workflow using the hybrid strategy
    ///
    /// Checks if all nodes have Rust implementations and executes natively if so.
    /// Otherwise, falls back to the n8n sidecar.
    pub async fn execute(
        &self,
        workflow: &WorkflowDefinition,
        input: serde_json::Value,
    ) -> Result<WorkflowResult> {
        let start = Instant::now();

        // Check if all nodes can be executed in Rust
        let all_rust_native = self.can_execute_natively(workflow);

        let result = if all_rust_native {
            log::info!(
                "[Hybrid Executor] Executing workflow '{}' entirely in Rust",
                workflow.name
            );
            self.execute_rust_native(workflow, input).await
        } else {
            let rust_nodes: Vec<&str> = workflow
                .nodes
                .iter()
                .filter(|n| self.rust_nodes.has_node(&n.node_type))
                .map(|n| n.node_type.as_str())
                .collect();

            let n8n_nodes: Vec<&str> = workflow
                .nodes
                .iter()
                .filter(|n| !self.rust_nodes.has_node(&n.node_type))
                .map(|n| n.node_type.as_str())
                .collect();

            log::info!(
                "[Hybrid Executor] Delegating to n8n - Rust nodes: {:?}, n8n nodes: {:?}",
                rust_nodes,
                n8n_nodes
            );

            self.execute_via_n8n(workflow, input).await
        };

        // Update stats
        {
            let mut stats = self.stats.lock();
            stats.total_executions += 1;
            let elapsed = start.elapsed().as_millis() as u64;

            if all_rust_native {
                stats.rust_native_executions += 1;
                stats.rust_execution_time_ms += elapsed;
                stats.rust_nodes_executed += workflow.nodes.len() as u64;
            } else {
                stats.n8n_fallback_executions += 1;
                stats.n8n_execution_time_ms += elapsed;
                stats.n8n_nodes_executed += workflow.nodes.len() as u64;
            }
        }

        result
    }

    /// Check if a workflow can be executed entirely in Rust
    pub fn can_execute_natively(&self, workflow: &WorkflowDefinition) -> bool {
        workflow.nodes.iter().all(|node| {
            // Trigger nodes are always "native" (they just start the workflow)
            node.node_type.contains("Trigger")
                || node.node_type.contains("manualTrigger")
                || self.rust_nodes.has_node(&node.node_type)
        })
    }

    /// Execute workflow entirely in Rust
    async fn execute_rust_native(
        &self,
        workflow: &WorkflowDefinition,
        mut data: serde_json::Value,
    ) -> Result<WorkflowResult> {
        let start = Instant::now();
        let execution_id = uuid::Uuid::new_v4().to_string();

        // Simple sequential execution
        // TODO: Implement proper DAG-based parallel execution
        for node in &workflow.nodes {
            // Skip trigger nodes
            if node.node_type.contains("Trigger") || node.node_type.contains("manualTrigger") {
                continue;
            }

            if let Some(rust_node) = self.rust_nodes.get_node(&node.node_type) {
                log::debug!("[Hybrid Executor] Executing Rust node: {}", node.name);

                data = rust_node
                    .execute(data, node.parameters.clone(), serde_json::json!({}))
                    .await?;
            } else {
                // This shouldn't happen if can_execute_natively returned true
                anyhow::bail!(
                    "No Rust implementation for node type: {}",
                    node.node_type
                );
            }
        }

        Ok(WorkflowResult {
            execution_id,
            status: "success".to_string(),
            data,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Execute workflow via n8n sidecar
    async fn execute_via_n8n(
        &self,
        workflow: &WorkflowDefinition,
        input: serde_json::Value,
    ) -> Result<WorkflowResult> {
        self.n8n_sidecar.execute_workflow(workflow, input).await
    }

    /// Execute workflow asynchronously (fire-and-forget)
    pub async fn execute_async(
        &self,
        workflow: &WorkflowDefinition,
        input: serde_json::Value,
    ) -> Result<String> {
        // For async execution, always use n8n sidecar
        // The sidecar handles queuing and background execution
        self.n8n_sidecar
            .execute_workflow_async(workflow, input)
            .await
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> ExecutionStats {
        self.stats.lock().clone()
    }

    /// Get the percentage of workflows that ran natively in Rust
    pub fn get_rust_percentage(&self) -> f64 {
        let stats = self.stats.lock();
        if stats.total_executions == 0 {
            return 0.0;
        }
        (stats.rust_native_executions as f64 / stats.total_executions as f64) * 100.0
    }

    /// List all node types that have Rust implementations
    pub fn list_rust_node_types(&self) -> Vec<&str> {
        self.rust_nodes.list_node_types()
    }

    /// Check if a specific node type has a Rust implementation
    pub fn has_rust_node(&self, node_type: &str) -> bool {
        self.rust_nodes.has_node(node_type)
    }

    /// Reset execution statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = ExecutionStats::default();
    }
}

/// Builder for creating workflows that can be fully executed in Rust
pub struct RustWorkflowBuilder {
    executor: Arc<HybridWorkflowExecutor>,
}

impl RustWorkflowBuilder {
    pub fn new(executor: Arc<HybridWorkflowExecutor>) -> Self {
        Self { executor }
    }

    /// Validate that a workflow can be executed entirely in Rust
    pub fn validate(&self, workflow: &WorkflowDefinition) -> ValidationResult {
        let mut unsupported_nodes = Vec::new();
        let mut supported_nodes = Vec::new();

        for node in &workflow.nodes {
            if node.node_type.contains("Trigger")
                || node.node_type.contains("manualTrigger")
                || self.executor.has_rust_node(&node.node_type)
            {
                supported_nodes.push(node.node_type.clone());
            } else {
                unsupported_nodes.push(node.node_type.clone());
            }
        }

        ValidationResult {
            can_execute_natively: unsupported_nodes.is_empty(),
            supported_nodes,
            unsupported_nodes,
        }
    }
}

/// Result of workflow validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the workflow can be executed entirely in Rust
    pub can_execute_natively: bool,

    /// Nodes that have Rust implementations
    pub supported_nodes: Vec<String>,

    /// Nodes that require n8n fallback
    pub unsupported_nodes: Vec<String>,
}

impl std::fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.can_execute_natively {
            write!(f, "✅ Workflow can be executed entirely in Rust")
        } else {
            write!(
                f,
                "⚠️  Workflow requires n8n fallback for: {:?}",
                self.unsupported_nodes
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::WorkflowBuilder;
    use crate::sidecar::SidecarConfig;

    async fn create_test_executor() -> Arc<HybridWorkflowExecutor> {
        let sidecar = N8nSidecar::spawn(SidecarConfig::default())
            .await
            .expect("Failed to spawn sidecar");
        Arc::new(HybridWorkflowExecutor::new(Arc::new(sidecar)))
    }

    #[tokio::test]
    async fn test_can_execute_natively() {
        let executor = create_test_executor().await;

        // Workflow with only supported nodes
        let workflow = WorkflowBuilder::new("Native Test")
            .add_trigger()
            .add_http_request("API Call", "https://api.example.com", "GET")
            .build();

        assert!(executor.can_execute_natively(&workflow));
    }

    #[tokio::test]
    async fn test_validation() {
        let executor = create_test_executor().await;
        let builder = RustWorkflowBuilder::new(executor.clone());

        let workflow = WorkflowBuilder::new("Mixed Test")
            .add_trigger()
            .add_http_request("API Call", "https://api.example.com", "GET")
            .build();

        let result = builder.validate(&workflow);
        assert!(result.can_execute_natively);
        assert!(result.unsupported_nodes.is_empty());
    }

    #[tokio::test]
    async fn test_stats() {
        let executor = create_test_executor().await;

        let stats = executor.get_stats();
        assert_eq!(stats.total_executions, 0);
        assert_eq!(executor.get_rust_percentage(), 0.0);
    }
}
