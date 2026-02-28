//! mDNS Service Discovery Implementation
//!
//! Real mDNS/Bonjour implementation using mdns-sd crate for:
//! - Service advertisement (server mode)
//! - Service discovery (client mode)
//! - Zero-config local network connectivity

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{RwLock, oneshot};

use crate::gateway::actix_server::GatewayConfig;

/// mDNS service type for DX gateway
pub const SERVICE_TYPE: &str = "_dx._tcp.local.";

/// Default service name
pub const DEFAULT_SERVICE_NAME: &str = "dx-gateway";

/// mDNS service advertiser
pub struct MdnsAdvertiser {
    /// Service name
    service_name: String,
    /// Port number
    port: u16,
    /// TXT records
    txt_records: HashMap<String, String>,
    /// Running flag
    running: Arc<RwLock<bool>>,
    /// Shutdown sender
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl MdnsAdvertiser {
    /// Create a new mDNS advertiser
    pub fn new(config: &GatewayConfig) -> Self {
        let mut txt_records = HashMap::new();
        txt_records.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        txt_records.insert("protocol".to_string(), "dx-gateway-v1".to_string());
        txt_records.insert("auth".to_string(), if false { "required" } else { "none" }.to_string());

        Self {
            service_name: "dx-gateway".to_string().clone(),
            port: config.port,
            txt_records,
            running: Arc::new(RwLock::new(false)),
            shutdown_tx: None,
        }
    }

    /// Start advertising the service
    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        if *self.running.read().await {
            return Ok(());
        }

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let service_name = self.service_name.clone();
        let port = self.port;
        let txt_records = self.txt_records.clone();
        let running = Arc::clone(&self.running);

        *running.write().await = true;

        tokio::spawn(async move {
            run_mdns_advertiser(service_name, port, txt_records, shutdown_rx, running).await;
        });

        tracing::info!(
            "📡 mDNS advertising: {}:{} as {}",
            SERVICE_TYPE,
            self.port,
            self.service_name
        );

        Ok(())
    }

    /// Stop advertising
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        *self.running.write().await = false;
        tracing::info!("mDNS advertising stopped");
    }

    /// Check if advertising
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// Run the mDNS advertiser (platform-specific)
async fn run_mdns_advertiser(
    service_name: String,
    port: u16,
    txt_records: HashMap<String, String>,
    mut shutdown_rx: oneshot::Receiver<()>,
    running: Arc<RwLock<bool>>,
) {
    #[cfg(feature = "mdns")]
    {
        use mdns_sd::{ServiceDaemon, ServiceInfo};

        let mdns = match ServiceDaemon::new() {
            Ok(mdns) => mdns,
            Err(e) => {
                tracing::error!("Failed to create mDNS daemon: {}", e);
                *running.write().await = false;
                return;
            }
        };

        // Build TXT record string
        let txt: Vec<_> = txt_records.iter().map(|(k, v)| format!("{}={}", k, v)).collect();

        let service_info = match ServiceInfo::new(
            SERVICE_TYPE,
            &service_name,
            &format!("{}.local.", hostname::get().unwrap_or_default().to_string_lossy()),
            "",
            port,
            txt.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice(),
        ) {
            Ok(info) => info,
            Err(e) => {
                tracing::error!("Failed to create service info: {}", e);
                *running.write().await = false;
                return;
            }
        };

        if let Err(e) = mdns.register(service_info) {
            tracing::error!("Failed to register mDNS service: {}", e);
            *running.write().await = false;
            return;
        }

        tracing::info!("mDNS service registered: {}", service_name);

        // Wait for shutdown
        let _ = shutdown_rx.await;

        // Unregister service
        let _ = mdns.unregister(&format!("{}.{}", service_name, SERVICE_TYPE));
        let _ = mdns.shutdown();
    }

    #[cfg(not(feature = "mdns"))]
    {
        // Fallback: just log and wait
        tracing::info!(
            "mDNS simulation: {} on port {} (mdns feature not enabled)",
            service_name,
            port
        );

        // Log TXT records
        for (k, v) in &txt_records {
            tracing::debug!("  TXT: {}={}", k, v);
        }

        // Wait for shutdown
        let _ = shutdown_rx.await;
    }

    *running.write().await = false;
}

/// Discovered gateway service
#[derive(Debug, Clone)]
pub struct DiscoveredGateway {
    /// Service name
    pub name: String,
    /// Host addresses
    pub addresses: Vec<IpAddr>,
    /// Port number
    pub port: u16,
    /// TXT records
    pub txt_records: HashMap<String, String>,
    /// Hostname
    pub hostname: String,
}

impl DiscoveredGateway {
    /// Get the primary WebSocket URL
    pub fn ws_url(&self) -> Option<String> {
        self.addresses.first().map(|addr| format!("ws://{}:{}/ws", addr, self.port))
    }

    /// Get the primary HTTP URL
    pub fn http_url(&self) -> Option<String> {
        self.addresses.first().map(|addr| format!("http://{}:{}", addr, self.port))
    }

    /// Get protocol version from TXT records
    pub fn protocol_version(&self) -> Option<&str> {
        self.txt_records.get("protocol").map(|s| s.as_str())
    }

    /// Check if authentication is required
    pub fn auth_required(&self) -> bool {
        self.txt_records.get("auth").map(|s| s == "required").unwrap_or(true)
    }
}

/// mDNS service browser
pub struct MdnsBrowser {
    /// Discovery timeout
    timeout: Duration,
    /// Discovered services
    discovered: Arc<RwLock<Vec<DiscoveredGateway>>>,
}

impl MdnsBrowser {
    /// Create a new browser
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            discovered: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Discover gateway services on the network
    pub async fn discover(&self) -> Vec<DiscoveredGateway> {
        #[cfg(feature = "mdns")]
        {
            use mdns_sd::ServiceDaemon;

            let mdns = match ServiceDaemon::new() {
                Ok(mdns) => mdns,
                Err(e) => {
                    tracing::error!("Failed to create mDNS daemon: {}", e);
                    return Vec::new();
                }
            };

            let receiver = match mdns.browse(SERVICE_TYPE) {
                Ok(rx) => rx,
                Err(e) => {
                    tracing::error!("Failed to browse mDNS: {}", e);
                    return Vec::new();
                }
            };

            let discovered = Arc::clone(&self.discovered);
            let timeout = self.timeout;

            tokio::spawn(async move {
                let deadline = tokio::time::Instant::now() + timeout;

                while tokio::time::Instant::now() < deadline {
                    match tokio::time::timeout(Duration::from_millis(100), async {
                        receiver.recv()
                    })
                    .await
                    {
                        Ok(Ok(event)) => {
                            use mdns_sd::ServiceEvent;
                            match event {
                                ServiceEvent::ServiceResolved(info) => {
                                    let gateway = DiscoveredGateway {
                                        name: info.get_fullname().to_string(),
                                        addresses: info.get_addresses().iter().copied().collect(),
                                        port: info.get_port(),
                                        txt_records: info
                                            .get_properties()
                                            .iter()
                                            .filter_map(|p| {
                                                p.val_str()
                                                    .map(|v| (p.key().to_string(), v.to_string()))
                                            })
                                            .collect(),
                                        hostname: info.get_hostname().to_string(),
                                    };
                                    discovered.write().await.push(gateway);
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }

                let _ = mdns.stop_browse(SERVICE_TYPE);
                let _ = mdns.shutdown();
            })
            .await
            .ok();
        }

        self.discovered.read().await.clone()
    }

    /// Find the first available gateway
    pub async fn find_one(&self) -> Option<DiscoveredGateway> {
        self.discover().await.into_iter().next()
    }

    /// Clear discovered services
    pub async fn clear(&self) {
        self.discovered.write().await.clear();
    }
}

/// Advertise the gateway service via mDNS
pub async fn advertise_mdns(config: &GatewayConfig) -> Result<(), anyhow::Error> {
    let mut advertiser = MdnsAdvertiser::new(config);
    advertiser.start().await?;

    // Keep running until shutdown
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        if !advertiser.is_running().await {
            break;
        }
    }

    Ok(())
}

/// Discover gateway services on the local network
pub async fn discover_gateways(timeout: Duration) -> Vec<DiscoveredGateway> {
    let browser = MdnsBrowser::new(timeout);
    browser.discover().await
}

/// Service info for mDNS registration
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    /// Service type
    pub service_type: String,
    /// Port number
    pub port: u16,
    /// TXT records
    pub txt_records: HashMap<String, String>,
}

impl ServiceInfo {
    /// Create service info for DX gateway
    pub fn for_gateway(config: &GatewayConfig) -> Self {
        let mut txt_records = HashMap::new();
        txt_records.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        txt_records.insert("protocol".to_string(), "dx-gateway-v1".to_string());
        txt_records.insert("auth".to_string(), if false { "required" } else { "none" }.to_string());

        Self {
            name: "dx-gateway".to_string().clone(),
            service_type: SERVICE_TYPE.to_string(),
            port: config.port,
            txt_records,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovered_gateway_url() {
        let gateway = DiscoveredGateway {
            name: "test-gateway._dx._tcp.local.".to_string(),
            addresses: vec!["192.168.1.100".parse().unwrap()],
            port: 31337,
            txt_records: HashMap::new(),
            hostname: "test-gateway.local.".to_string(),
        };

        assert_eq!(gateway.ws_url(), Some("ws://192.168.1.100:31337/ws".to_string()));
        assert_eq!(gateway.http_url(), Some("http://192.168.1.100:31337".to_string()));
    }

    #[test]
    fn test_auth_required() {
        let mut txt_records = HashMap::new();
        txt_records.insert("auth".to_string(), "required".to_string());

        let gateway = DiscoveredGateway {
            name: "test".to_string(),
            addresses: vec![],
            port: 31337,
            txt_records,
            hostname: "test.local.".to_string(),
        };

        assert!(gateway.auth_required());

        let mut txt_records = HashMap::new();
        txt_records.insert("auth".to_string(), "none".to_string());

        let gateway = DiscoveredGateway {
            name: "test".to_string(),
            addresses: vec![],
            port: 31337,
            txt_records,
            hostname: "test.local.".to_string(),
        };

        assert!(!gateway.auth_required());
    }

    #[test]
    fn test_service_info() {
        let config = GatewayConfig::default();
        let info = ServiceInfo::for_gateway(&config);

        assert_eq!(info.name, "dx-gateway");
        assert_eq!(info.service_type, SERVICE_TYPE);
        assert_eq!(info.port, 31337);
        assert!(info.txt_records.contains_key("version"));
        assert!(info.txt_records.contains_key("protocol"));
    }

    #[tokio::test]
    async fn test_mdns_advertiser_creation() {
        let config = GatewayConfig::default();
        let advertiser = MdnsAdvertiser::new(&config);

        assert!(!advertiser.is_running().await);
    }
}
