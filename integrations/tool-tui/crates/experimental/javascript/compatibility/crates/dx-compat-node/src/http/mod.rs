//! HTTP and HTTPS server and client compatibility.
//!
//! This module provides Node.js-compatible HTTP/HTTPS server and client implementations.
//! Full implementation uses hyper for maximum performance.

pub mod https;
pub mod server;

pub use https::{
    create_server as create_https_server, HttpsServer, HttpsServerBuilder, TlsOptions,
};
pub use server::{
    create_server as create_http_server, HttpRequestHandler, HttpServer, HttpServerBuilder,
    IncomingMessage, ServerConfig, ServerResponse,
};

use crate::error::{ErrorCode, NodeError, NodeResult};
use bytes::Bytes;
use http::{Method, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

/// HTTP server implementation.
pub struct Server {
    addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Server {
    /// Get the server's listening address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stop the server gracefully.
    pub async fn close(&mut self) -> NodeResult<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        Ok(())
    }
}

/// HTTP request handler function type.
pub type RequestHandler = Arc<dyn Fn(Request, ResponseWriter) + Send + Sync>;

/// Simplified request object for Node.js compatibility.
#[derive(Clone)]
pub struct Request {
    /// HTTP method
    pub method: Method,
    /// Request URL
    pub url: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body
    pub body: Bytes,
}

/// Response writer for streaming responses
pub struct ResponseWriter {
    status: StatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
    response_tx: mpsc::Sender<ResponseData>,
}

struct ResponseData {
    status: StatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl ResponseWriter {
    fn new(response_tx: mpsc::Sender<ResponseData>) -> Self {
        Self {
            status: StatusCode::OK,
            headers: HashMap::new(),
            body: Vec::new(),
            response_tx,
        }
    }

    /// Set response status code.
    pub fn status(&mut self, code: u16) {
        self.status = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
    }

    /// Set a response header.
    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(name.into(), value.into());
    }

    /// Write data to the response body.
    pub fn write(&mut self, data: impl AsRef<[u8]>) {
        self.body.extend_from_slice(data.as_ref());
    }

    /// End the response and send it.
    pub fn end(mut self, data: Option<impl AsRef<[u8]>>) {
        if let Some(d) = data {
            self.body.extend_from_slice(d.as_ref());
        }

        let response_data = ResponseData {
            status: self.status,
            headers: self.headers,
            body: self.body,
        };

        // Send response (ignore error if receiver dropped)
        let _ = self.response_tx.try_send(response_data);
    }
}

/// Simplified response object for Node.js compatibility (legacy API).
pub struct Response {
    status: StatusCode,
    headers: HashMap<String, String>,
}

impl Response {
    /// Create a new response.
    pub fn new() -> Self {
        Self {
            status: StatusCode::OK,
            headers: HashMap::new(),
        }
    }

    /// Set response status code.
    pub fn status(&mut self, code: u16) {
        self.status = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
    }

    /// Set a response header.
    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(name.into(), value.into());
    }

    /// Write data to the response body.
    pub async fn write(&mut self, _data: impl Into<Bytes>) -> NodeResult<()> {
        Ok(())
    }

    /// End the response.
    pub async fn end(&mut self, _data: Option<impl Into<Bytes>>) -> NodeResult<()> {
        Ok(())
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

/// Create an HTTP server with the given request handler.
pub async fn create_server(handler: RequestHandler) -> NodeResult<ServerBuilder> {
    Ok(ServerBuilder { handler })
}

/// Server builder for configuration.
pub struct ServerBuilder {
    handler: RequestHandler,
}

impl ServerBuilder {
    /// Start listening on the given address.
    pub async fn listen(self, addr: impl Into<String>) -> NodeResult<Server> {
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

        // Spawn server task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, _)) => {
                                let handler = handler.clone();
                                tokio::spawn(handle_connection(stream, handler));
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

        Ok(Server {
            addr: actual_addr,
            shutdown_tx: Some(shutdown_tx),
        })
    }
}

async fn handle_connection(stream: TcpStream, handler: RequestHandler) {
    let io = TokioIo::new(stream);

    let service = service_fn(move |req: hyper::Request<Incoming>| {
        let handler = handler.clone();
        async move {
            // Extract request data
            let method = req.method().clone();
            let url = req.uri().to_string();
            let headers: HashMap<String, String> = req
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            // Collect body
            let body = req
                .into_body()
                .collect()
                .await
                .map(|b| b.to_bytes())
                .unwrap_or_else(|_| Bytes::new());

            let request = Request {
                method,
                url,
                headers,
                body,
            };

            // Create response channel
            let (tx, mut rx) = mpsc::channel(1);
            let writer = ResponseWriter::new(tx);

            // Call handler
            handler(request, writer);

            // Wait for response
            let response_data = rx.recv().await.unwrap_or(ResponseData {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                headers: HashMap::new(),
                body: b"Internal Server Error".to_vec(),
            });

            // Build hyper response
            let mut builder = hyper::Response::builder().status(response_data.status);

            for (key, value) in response_data.headers {
                builder = builder.header(key, value);
            }

            builder
                .body(Full::new(Bytes::from(response_data.body)))
                .map_err(std::io::Error::other)
        }
    });

    if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
        eprintln!("Connection error: {}", e);
    }
}

/// HTTP client request function.
pub async fn request(
    url: impl Into<String>,
    options: RequestOptions,
) -> NodeResult<ClientResponse> {
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;

    let url_str = url.into();
    let uri: hyper::Uri = url_str
        .parse()
        .map_err(|e| NodeError::new(ErrorCode::EINVAL, format!("Invalid URL: {}", e)))?;

    let method = options.method.unwrap_or(Method::GET);

    let mut req_builder = hyper::Request::builder().method(method).uri(&uri);

    for (key, value) in &options.headers {
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    let body = options.body.unwrap_or_default();
    let req = req_builder.body(Full::new(body)).map_err(|e| {
        NodeError::new(ErrorCode::UNKNOWN, format!("Failed to build request: {}", e))
    })?;

    let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new()).build_http();

    let response = client
        .request(req)
        .await
        .map_err(|e| NodeError::new(ErrorCode::UNKNOWN, format!("Request failed: {}", e)))?;

    let status = response.status();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body =
        response.collect().await.map(|b| b.to_bytes()).map_err(|e| {
            NodeError::new(ErrorCode::UNKNOWN, format!("Failed to read body: {}", e))
        })?;

    Ok(ClientResponse {
        status,
        headers,
        body,
    })
}

/// HTTP client request options.
#[derive(Default)]
pub struct RequestOptions {
    /// HTTP method (defaults to GET)
    pub method: Option<Method>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body
    pub body: Option<Bytes>,
}

/// HTTP client response.
pub struct ClientResponse {
    /// Response status code
    pub status: StatusCode,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Bytes,
}

impl ClientResponse {
    /// Get response body as string.
    pub fn text(&self) -> NodeResult<String> {
        String::from_utf8(self.body.to_vec())
            .map_err(|e| NodeError::new(ErrorCode::UNKNOWN, format!("Invalid UTF-8: {}", e)))
    }

    /// Get response body as JSON.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> NodeResult<T> {
        serde_json::from_slice(&self.body)
            .map_err(|e| NodeError::new(ErrorCode::UNKNOWN, format!("JSON parse error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let handler = Arc::new(|_req: Request, mut res: ResponseWriter| {
            res.status(200);
            res.set_header("Content-Type", "text/plain");
            res.end(Some("Hello World!"));
        });

        let server = create_server(handler).await.unwrap();
        let mut server = server.listen("127.0.0.1:0").await.unwrap();

        // Server should be listening on a port
        assert!(server.addr().port() > 0);

        // Clean up
        server.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_response_writer() {
        let (tx, mut rx) = mpsc::channel(1);
        let mut writer = ResponseWriter::new(tx);

        writer.status(201);
        writer.set_header("X-Custom", "test");
        writer.write(b"Hello ");
        writer.end(Some("World!"));

        let response = rx.recv().await.unwrap();
        assert_eq!(response.status, StatusCode::CREATED);
        assert_eq!(response.headers.get("X-Custom"), Some(&"test".to_string()));
        assert_eq!(response.body, b"Hello World!");
    }
}
