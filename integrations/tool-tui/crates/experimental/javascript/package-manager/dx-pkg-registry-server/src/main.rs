//! DXRP Registry Server
//!
//! A high-performance binary package registry server implementing the DXRP protocol.
//! Serves .dxp packages over TCP with zero-copy binary transfers.

use anyhow::{Context, Result};
use dashmap::DashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod protocol;
mod storage;

use protocol::*;
use storage::PackageStorage;

/// DXRP Registry Server
pub struct RegistryServer {
    /// Package storage (maps package name+version to .dxp file)
    storage: Arc<PackageStorage>,
    /// In-memory cache for hot packages
    cache: Arc<DashMap<String, Vec<u8>>>,
    /// Server address
    addr: SocketAddr,
}

impl RegistryServer {
    /// Create new registry server
    pub fn new(storage_path: PathBuf, addr: SocketAddr) -> Result<Self> {
        Ok(Self {
            storage: Arc::new(PackageStorage::new(storage_path)?),
            cache: Arc::new(DashMap::new()),
            addr,
        })
    }

    /// Start the server
    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        println!("🚀 DXRP Registry Server listening on {}", self.addr);
        println!("📦 Package storage: {}", self.storage.path().display());
        println!("⚡ Ready to serve binary packages");
        println!();

        loop {
            let (socket, addr) = listener.accept().await?;
            let storage = self.storage.clone();
            let cache = self.cache.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, storage, cache).await {
                    eprintln!("❌ Error handling connection from {}: {}", addr, e);
                }
            });
        }
    }
}

/// Handle a single client connection
async fn handle_connection(
    mut socket: TcpStream,
    storage: Arc<PackageStorage>,
    cache: Arc<DashMap<String, Vec<u8>>>,
) -> Result<()> {
    loop {
        // Read request (32 bytes)
        let mut req_buf = [0u8; 32];
        let n = socket.read(&mut req_buf).await?;

        if n == 0 {
            // Connection closed
            return Ok(());
        }

        if n != 32 {
            // Invalid request size
            let resp = DxrpResponse::error("Invalid request size");
            socket.write_all(&resp.to_bytes()).await?;
            continue;
        }

        // Parse request
        let request = match DxrpRequest::from_bytes(&req_buf) {
            Ok(req) => req,
            Err(e) => {
                let resp = DxrpResponse::error(&format!("Parse error: {}", e));
                socket.write_all(&resp.to_bytes()).await?;
                continue;
            }
        };

        // Handle request
        match request.op {
            DxrpOp::Resolve => {
                // Resolve package name+version to metadata
                handle_resolve(&mut socket, &storage, &request).await?;
            }
            DxrpOp::Download => {
                // Download .dxp package
                handle_download(&mut socket, &storage, &cache, &request).await?;
            }
            DxrpOp::Ping => {
                // Health check
                let resp = DxrpResponse::ok(&b"PONG"[..]);
                socket.write_all(&resp.to_bytes()).await?;
            }
        }
    }
}

/// Handle package resolution
async fn handle_resolve(
    socket: &mut TcpStream,
    storage: &PackageStorage,
    request: &DxrpRequest,
) -> Result<()> {
    // Look up package metadata
    match storage.get_metadata(request.name_hash, request.version).await {
        Ok(metadata) => {
            let payload = bincode::encode_to_vec(&metadata, bincode::config::standard())?;
            let resp = DxrpResponse::ok(&payload);
            socket.write_all(&resp.to_bytes()).await?;
            socket.write_all(&payload).await?;
        }
        Err(e) => {
            let resp = DxrpResponse::error(&format!("Package not found: {}", e));
            socket.write_all(&resp.to_bytes()).await?;
        }
    }
    Ok(())
}

/// Handle package download
async fn handle_download(
    socket: &mut TcpStream,
    storage: &PackageStorage,
    cache: &DashMap<String, Vec<u8>>,
    request: &DxrpRequest,
) -> Result<()> {
    let key = format!("{}:{}", request.name_hash, request.version);

    // Check cache first
    if let Some(data) = cache.get(&key) {
        let resp = DxrpResponse::ok(&data);
        socket.write_all(&resp.to_bytes()).await?;
        socket.write_all(&data).await?;
        return Ok(());
    }

    // Load from storage
    match storage.get_package(request.name_hash, request.version).await {
        Ok(data) => {
            // Cache hot packages (< 5MB)
            if data.len() < 5_000_000 {
                cache.insert(key, data.clone());
            }

            let resp = DxrpResponse::ok(&data);
            socket.write_all(&resp.to_bytes()).await?;
            socket.write_all(&data).await?;
        }
        Err(e) => {
            let resp = DxrpResponse::error(&format!("Package not found: {}", e));
            socket.write_all(&resp.to_bytes()).await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let storage_path = std::env::args().nth(1).unwrap_or_else(|| ".dx-registry".to_string());

    let addr: SocketAddr = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "127.0.0.1:3000".to_string())
        .parse()
        .context("Invalid socket address")?;

    // Create and run server
    let server = RegistryServer::new(PathBuf::from(storage_path), addr)?;
    server.run().await
}
