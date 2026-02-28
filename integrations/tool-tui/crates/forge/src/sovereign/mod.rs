//! Binary Dawn: Sovereign Orchestration Engine
//!
//! Three critical subsystems:
//! - Sovereign Orchestrator: Tool lifecycle management
//! - Shadow Worker: Background task processing
//! - Traffic Branching: Revolutionary package management

pub mod background;
pub mod orchestrator;
pub mod traffic;

#[cfg(test)]
mod tests;

pub use background::{BackgroundTask, BackgroundWorker};
pub use orchestrator::{DxToolDefinition, Orchestrator, ToolStatus};
pub use traffic::{TrafficLight, TrafficManager};

use std::sync::Arc;
use tokio::sync::RwLock;

/// The God Object: DX Forge Sovereign Engine
pub struct DxForge {
    pub orchestrator: Arc<RwLock<Orchestrator>>,
    pub background_worker: BackgroundWorker,
    pub traffic_manager: TrafficManager,
}

impl DxForge {
    pub async fn new() -> Self {
        println!("⚔️  Initializing DX Forge: Binary Dawn Edition...");
        Self {
            orchestrator: Arc::new(RwLock::new(Orchestrator::new())),
            background_worker: BackgroundWorker::new(),
            traffic_manager: TrafficManager::new(),
        }
    }

    pub async fn run_pipeline(&self, command: &str) -> anyhow::Result<()> {
        match command {
            "install" => {
                self.traffic_manager.install_all_dependencies().await?;
                self.background_worker.enqueue(BackgroundTask::CacheCurrentState).await;
            }
            "build" => {
                let orch = self.orchestrator.read().await;
                orch.ensure_running("dx-style").await?;
                orch.ensure_running("dx-js-runtime").await?;
                orch.execute_tool("dx-js-bundler", &["build"]).await?;
                drop(orch);
                self.background_worker.enqueue(BackgroundTask::SyncToCloudflareR2).await;
            }
            _ => println!("Unknown pipeline command: {}", command),
        }
        Ok(())
    }
}
