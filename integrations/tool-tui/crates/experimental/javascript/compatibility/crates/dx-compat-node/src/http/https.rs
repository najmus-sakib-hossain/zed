//! HTTPS server implementation with TLS support.
//!
//! This module provides HTTPS server functionality matching Node.js https.createServer()
//! using rustls for TLS encryption.

use crate::error::{ErrorCode, NodeError, NodeResult};
use crate::http::server::{HttpRequestHandler, ResponsePayload, ServerConfig};
use bytes::Bytes;
use http::StatusCode;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use rustls::{Certificate, PrivateKey, ServerConfig as TlsServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::TlsAcceptor;

use super::server::IncomingMessage;

/// TLS configuration options for HTTPS server.
#[derive(Clone)]
pub struct TlsOptions {
    /// Path to the certificate file (PEM format).
    pub cert_path: String,
    /// Path to the private key file (PEM format).
    pub key_path: String,
}

impl TlsOptions {
    /// Create new TLS options with certificate and key paths.
    pub fn new(cert_path: impl Into<String>, key_path: impl Into<String>) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
        }
    }

    /// Load TLS configuration from the specified files.
    pub fn load(&self) -> NodeResult<TlsServerConfig> {
        // Load certificate chain
        let cert_file = File::open(&self.cert_path).map_err(|e| {
            NodeError::new(
                ErrorCode::ENOENT,
                format!("Failed to open certificate file: {}", e),
            )
        })?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs_result = certs(&mut cert_reader).map_err(|e| {
            NodeError::new(ErrorCode::EINVAL, format!("Failed to parse certificates: {}", e))
        })?;
        let certs: Vec<Certificate> = certs_result.into_iter().map(Certificate).collect();

        if certs.is_empty() {
            return Err(NodeError::new(
                ErrorCode::EINVAL,
                "No certificates found in certificate file",
            ));
        }

        // Load private key
        let key_file = File::open(&self.key_path).map_err(|e| {
            NodeError::new(
                ErrorCode::ENOENT,
                format!("Failed to open key file: {}", e),
            )
        })?;
        let mut key_reader = BufReader::new(key_file);

        // Try PKCS8 format first
        let pkcs8_keys = pkcs8_private_keys(&mut key_reader).map_err(|e| {
            NodeError::new(ErrorCode::EINVAL, format!("Failed to parse PKCS8 keys: {}", e))
        })?;

        let key = if !pkcs8_keys.is_empty() {
            PrivateKey(pkcs8_keys[0].clone())
        } else {
            // Reopen file for RSA keys
            let key_file = File::open(&self.key_path).map_err(|e| {
                NodeError::new(
                    ErrorCode::ENOENT,
                    format!("Failed to open key file: {}", e),
                )
            })?;
            let mut key_reader = BufReader::new(key_file);
            let rsa_keys = rsa_private_keys(&mut key_reader).map_err(|e| {
                NodeError::new(ErrorCode::EINVAL, format!("Failed to parse RSA keys: {}", e))
            })?;

            if rsa_keys.is_empty() {
                return Err(NodeError::new(
                    ErrorCode::EINVAL,
                    "No private key found in key file",
                ));
            }
            PrivateKey(rsa_keys[0].clone())
        };

        // Build TLS config
        let config = TlsServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| NodeError::new(ErrorCode::EINVAL, format!("TLS config error: {}", e)))?;

        Ok(config)
    }
}

/// HTTPS server matching Node.js https.Server.
pub struct HttpsServer {
    /// Server listening address.
    addr: SocketAddr,
    /// Shutdown signal sender.
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Server configuration.
    config: ServerConfig,
    /// Whether the server is listening.
    listening: bool,
}

impl HttpsServer {
    /// Get the server's listening address.
    pub fn address(&self) -> SocketAddr {
        self.addr
    }

    /// Check if the server is listening.
    pub fn listening(&self) -> bool {
        self.listening
    }

    /// Get the server configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Close the server gracefully.
    pub async fn close(&mut self) -> NodeResult<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.listening = false;
        Ok(())
    }
}

/// Create an HTTPS server with the given TLS options and request handler.
/// Matches Node.js https.createServer(options, [requestListener])
pub async fn create_server(
    tls_options: TlsOptions,
    handler: HttpRequestHandler,
) -> NodeResult<HttpsServerBuilder> {
    let tls_config = tls_options.load()?;
    Ok(HttpsServerBuilder {
        handler,
        config: ServerConfig::default(),
        tls_config: Arc::new(tls_config),
    })
}

/// HTTPS server builder for configuration.
pub struct HttpsServerBuilder {
    handler: HttpRequestHandler,
    config: ServerConfig,
    tls_config: Arc<TlsServerConfig>,
}

impl HttpsServerBuilder {
    /// Set Keep-Alive timeout.
    pub fn keep_alive_timeout(mut self, seconds: u64) -> Self {
        self.config.keep_alive_timeout = seconds;
        self
    }

    /// Enable or disable Keep-Alive.
    pub fn keep_alive(mut self, enabled: bool) -> Self {
        self.config.keep_alive = enabled;
        self
    }

    /// Set maximum header size.
    pub fn max_header_size(mut self, size: usize) -> Self {
        self.config.max_header_size = size;
        self
    }

    /// Start listening on the given address.
    pub async fn listen(self, addr: impl Into<String>) -> NodeResult<HttpsServer> {
        let addr_str = addr.into();
        let addr: SocketAddr = addr_str
            .parse()
            .map_err(|e| NodeError::new(ErrorCode::EINVAL, format!("Invalid address: {}", e)))?;

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| NodeError::new(ErrorCode::EADDRINUSE, format!("Failed to bind: {}", e)))?;

        let actual_addr = listener.local_addr().map_err(|e| {
            NodeError::new(ErrorCode::UNKNOWN, format!("Failed to get address: {}", e))
        })?;

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let handler = self.handler;
        let config = self.config.clone();
        let tls_acceptor = TlsAcceptor::from(self.tls_config);

        // Spawn server task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _)) => {
                                let handler = handler.clone();
                                let config = config.clone();
                                let acceptor = tls_acceptor.clone();
                                tokio::spawn(handle_https_connection(stream, acceptor, handler, config));
                            }
                            Err(e) => {
                                eprintln!("Accept error: {}", e);
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        break;
                    }
                }
            }
        });

        Ok(HttpsServer {
            addr: actual_addr,
            shutdown_tx: Some(shutdown_tx),
            config: self.config,
            listening: true,
        })
    }
}

/// Handle an HTTPS connection with TLS.
async fn handle_https_connection(
    stream: tokio::net::TcpStream,
    acceptor: TlsAcceptor,
    handler: HttpRequestHandler,
    config: ServerConfig,
) {
    // Perform TLS handshake
    let tls_stream = match acceptor.accept(stream).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("TLS handshake error: {}", e);
            return;
        }
    };

    let io = TokioIo::new(tls_stream);

    let service = service_fn(move |req: hyper::Request<Incoming>| {
        let handler = handler.clone();
        async move {
            // Extract request parts
            let method = req.method().clone();
            let uri = req.uri().clone();
            let version = req.version();
            let headers = req.headers().clone();

            // Collect body
            let body = req
                .into_body()
                .collect()
                .await
                .map(|b| b.to_bytes())
                .unwrap_or_else(|_| Bytes::new());

            // Create IncomingMessage
            let incoming = IncomingMessage::from_hyper(method, &uri, version, &headers, body);

            // Create response channel
            let (tx, mut rx) = mpsc::channel(1);
            let response = super::server::ServerResponse::new(tx);

            // Call handler
            handler(incoming, response);

            // Wait for response
            let payload = rx.recv().await.unwrap_or(ResponsePayload {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                headers: HashMap::new(),
                body: b"Internal Server Error".to_vec(),
            });

            // Build hyper response
            let mut builder = hyper::Response::builder().status(payload.status);

            for (key, value) in payload.headers {
                builder = builder.header(key, value);
            }

            builder
                .body(Full::new(Bytes::from(payload.body)))
                .map_err(std::io::Error::other)
        }
    });

    let mut conn = http1::Builder::new();

    if config.keep_alive {
        conn.keep_alive(true);
    }

    if let Err(e) = conn.serve_connection(io, service).await {
        if !e.to_string().contains("connection") {
            eprintln!("HTTPS connection error: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_options_creation() {
        let options = TlsOptions::new("cert.pem", "key.pem");
        assert_eq!(options.cert_path, "cert.pem");
        assert_eq!(options.key_path, "key.pem");
    }

    #[test]
    fn test_tls_options_load_missing_cert() {
        let options = TlsOptions::new("/nonexistent/cert.pem", "/nonexistent/key.pem");
        let result = options.load();
        assert!(result.is_err());
    }
}
