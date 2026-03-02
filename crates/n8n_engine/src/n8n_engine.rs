//! # n8n Workflow Engine Integration for Zed
//!
//! This crate provides a comprehensive integration between Zed's Rust GPUI framework
//! and a forked n8n workflow engine. It implements the "n8n Killer Blueprint" architecture
//! with the following key strategies:
//!
//! 1. **Sidecar Engine Pattern**: Run n8n as a managed child process with IPC
//! 2. **Embedded V8 Routing**: Fast in-process workflow routing and decision making
//! 3. **GPUI WebView Bridge**: Native workflow panel UI integration
//! 4. **Workflow-as-JSON Protocol**: Structured AI ↔ n8n communication
//! 5. **Hybrid Executor**: Gradual migration with Rust-native nodes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │           RUST GPUI ZED (Main Process)                              │
//! │                                                                     │
//! │  ┌────────────────┐  ┌──────────────────┐  ┌───────────────────┐   │
//! │  │ WorkflowPanel  │  │  AiWorkflowRouter│  │  HybridExecutor   │   │
//! │  │ (GPUI WebView) │  │  (Embedded V8)   │  │  (Rust + n8n)     │   │
//! │  └────────┬───────┘  └────────┬─────────┘  └─────────┬─────────┘   │
//! │           │                   │                      │             │
//! │           └───────────────────┼──────────────────────┘             │
//! │                               │                                    │
//! │                    ┌──────────▼──────────┐                         │
//! │                    │   N8nSidecar        │                         │
//! │                    │   (IPC Bridge)      │                         │
//! │                    └──────────┬──────────┘                         │
//! └───────────────────────────────┼────────────────────────────────────┘
//!                                 │ IPC (Named Pipe / Unix Socket)
//! ┌───────────────────────────────▼────────────────────────────────────┐
//! │  n8n EXECUTION SIDECAR (Child Process - Node.js)                   │
//! │  - Custom IPC Bridge Server                                        │
//! │  - WorkflowRunner / WorkflowExecute engine                         │
//! │  - 500+ Node Integrations                                          │
//! └────────────────────────────────────────────────────────────────────┘
//! ```

mod hybrid_executor;
mod protocol;
mod rust_nodes;
mod sidecar;
mod workflow_panel;
mod workflow_router;

pub use hybrid_executor::HybridWorkflowExecutor;
pub use protocol::*;
pub use rust_nodes::{RustHttpRequestNode, RustWorkflowNode};
pub use sidecar::{N8nSidecar, SidecarConfig};
pub use workflow_panel::WorkflowPanel;
pub use workflow_router::AiWorkflowRouter;

use gpui::AppContext;
use std::sync::Arc;

/// Initialize the n8n engine subsystem
pub fn init(cx: &mut AppContext) {
    workflow_panel::init(cx);
}

/// Global n8n engine instance for the application
pub struct N8nEngine {
    pub sidecar: Option<Arc<N8nSidecar>>,
    pub router: Option<Arc<parking_lot::Mutex<AiWorkflowRouter>>>,
    pub executor: Option<Arc<HybridWorkflowExecutor>>,
}

impl N8nEngine {
    pub fn new() -> Self {
        Self {
            sidecar: None,
            router: None,
            executor: None,
        }
    }

    /// Start the n8n engine with the given configuration
    pub async fn start(&mut self, config: SidecarConfig) -> anyhow::Result<()> {
        // Start the n8n sidecar process
        let sidecar = N8nSidecar::spawn(config).await?;
        let sidecar = Arc::new(sidecar);

        // Initialize the workflow router
        let router = AiWorkflowRouter::new()?;
        let router = Arc::new(parking_lot::Mutex::new(router));

        // Create the hybrid executor
        let executor = HybridWorkflowExecutor::new(sidecar.clone());
        let executor = Arc::new(executor);

        self.sidecar = Some(sidecar);
        self.router = Some(router);
        self.executor = Some(executor);

        log::info!("[n8n Engine] Started successfully");
        Ok(())
    }

    /// Execute a workflow using the AI routing pipeline
    pub async fn execute_ai_workflow(
        &self,
        intent: &str,
        parameters: serde_json::Value,
        context: Option<serde_json::Value>,
    ) -> anyhow::Result<WorkflowResult> {
        let router = self
            .router
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Router not initialized"))?;

        let executor = self
            .executor
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Executor not initialized"))?;

        // Step 1: Route the AI decision via embedded V8
        let decision = serde_json::json!({
            "intent": intent,
            "parameters": parameters,
            "context": context.unwrap_or(serde_json::json!({}))
        });

        let route = {
            let mut router_guard = router.lock();
            router_guard.route_ai_decision(decision)?
        };

        // Step 2: Check if we should execute
        let action = route
            .get("action")
            .and_then(|a| a.as_str())
            .unwrap_or("reject");

        if action != "execute" {
            let reason = route
                .get("reason")
                .and_then(|r| r.as_str())
                .unwrap_or("Unknown reason");
            anyhow::bail!("Workflow rejected: {}", reason);
        }

        // Step 3: Build workflow from route
        let workflow: WorkflowDefinition = serde_json::from_value(
            route
                .get("workflow")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No workflow in route"))?,
        )?;

        let input = route
            .get("input_data")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        // Step 4: Execute via hybrid executor
        executor.execute(&workflow, input).await
    }

    /// Shutdown the n8n engine
    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        if let Some(sidecar) = self.sidecar.take() {
            // Try to get exclusive access for shutdown
            if let Ok(mut sidecar) = Arc::try_unwrap(sidecar) {
                sidecar.shutdown().await?;
            }
        }
        self.router.take();
        self.executor.take();
        log::info!("[n8n Engine] Shutdown complete");
        Ok(())
    }
}

impl Default for N8nEngine {
    fn default() -> Self {
        Self::new()
    }
}
