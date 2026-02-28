//! Main gateway server orchestrator

use super::{DiscoveryService, PairingManager};
use anyhow::Result;
use std::sync::Arc;

pub struct GatewayServer {
    port: u16,
    discovery: Arc<DiscoveryService>,
    pairing: Arc<PairingManager>,
}

impl GatewayServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            discovery: Arc::new(DiscoveryService::new(port)),
            pairing: Arc::new(PairingManager::new()),
        }
    }

    pub async fn start(&self) -> Result<()> {
        println!("Starting DX Gateway Server on port {}", self.port);

        // Start mDNS discovery
        self.discovery.start_broadcast().await?;

        // Start cleanup task for expired pairing codes
        let pairing = Arc::clone(&self.pairing);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                pairing.cleanup_expired().await;
            }
        });

        println!("Gateway server started successfully");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        self.discovery.stop().await?;
        println!("Gateway server stopped");
        Ok(())
    }

    pub fn pairing_manager(&self) -> Arc<PairingManager> {
        Arc::clone(&self.pairing)
    }
}
