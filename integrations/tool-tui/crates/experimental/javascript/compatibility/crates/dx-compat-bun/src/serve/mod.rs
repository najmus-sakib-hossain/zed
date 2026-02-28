//! Bun.serve() HTTP server implementation.
//!
//! High-performance HTTP server using hyper, targeting 400k+ requests/second.

use crate::error::{BunError, BunResult};
use bytes::Bytes;
use http::{Method, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServeOptions<F>
where
    F: Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync + 'static,
{
    /// Port to listen on
    pub port: u16,
    /// Hostname to bind to
    pub hostname: Option<String>,
    /// Unix socket path (Unix only)
    pub unix: Option<PathBuf>,
    /// TLS configuration
    pub tls: Option<TlsConfig>,
    /// Request handler
    pub fetch: F,
}

/// TLS configuration.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to certificate file
    pub cert: PathBuf,
    /// Path to key file
    pub key: PathBuf,
}

/// HTTP Request wrapper.
#[derive(Debug)]
pub struct Request {
    /// HTTP method
    pub method: Method,
    /// Request URL
    pub url: String,
    /// Request headers
    pub headers: Headers,
    /// Request body
    pub body: Option<Bytes>,
}

/// HTTP Response wrapper.
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code
    pub status: StatusCode,
    /// Response headers
    pub headers: Headers,
    /// Response body
    pub body: Bytes,
}

impl Response {
    /// Create a new response with status and body.
    pub fn new(status: StatusCode, body: impl Into<Bytes>) -> Self {
        Self {
            status,
            headers: Headers::new(),
            body: body.into(),
        }
    }

    /// Create a 200 OK response.
    pub fn ok(body: impl Into<Bytes>) -> Self {
        Self::new(StatusCode::OK, body)
    }

    /// Create a JSON response.
    pub fn json(body: impl Into<Bytes>) -> Self {
        let mut response = Self::new(StatusCode::OK, body);
        response.headers.set("content-type", "application/json");
        response
    }

    /// Create a 404 Not Found response.
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "Not Found")
    }

    /// Create a 500 Internal Server Error response.
    pub fn internal_error() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
    }

    /// Set a header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.set(name, value);
        self
    }
}

/// HTTP Headers.
#[derive(Debug, Clone, Default)]
pub struct Headers {
    inner: Vec<(String, String)>,
}

impl Headers {
    /// Create new headers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get header value.
    pub fn get(&self, name: &str) -> Option<&str> {
        let lower = name.to_lowercase();
        self.inner
            .iter()
            .find(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v.as_str())
    }

    /// Set header value.
    pub fn set(&mut self, name: &str, value: &str) {
        let lower = name.to_lowercase();
        if let Some(pos) = self.inner.iter().position(|(k, _)| k.to_lowercase() == lower) {
            self.inner[pos] = (name.to_string(), value.to_string());
        } else {
            self.inner.push((name.to_string(), value.to_string()));
        }
    }

    /// Iterate over headers.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// Server handle.
pub struct Server {
    /// Port the server is listening on
    pub port: u16,
    /// Hostname the server is bound to
    pub hostname: String,
    /// Shutdown signal sender
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl Server {
    /// Stop the server.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    /// Get the server address.
    pub fn address(&self) -> String {
        format!("{}:{}", self.hostname, self.port)
    }
}

/// Type alias for the fetch handler function.
pub type FetchHandler =
    Arc<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>;

/// Simple serve options without generics.
#[derive(Clone)]
pub struct SimpleServeOptions {
    /// Port to listen on
    pub port: u16,
    /// Hostname to bind to
    pub hostname: Option<String>,
    /// Request handler
    pub fetch: FetchHandler,
}

impl SimpleServeOptions {
    /// Create new serve options.
    pub fn new<F>(port: u16, fetch: F) -> Self
    where
        F: Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync + 'static,
    {
        Self {
            port,
            hostname: None,
            fetch: Arc::new(fetch),
        }
    }

    /// Set hostname.
    pub fn hostname(mut self, hostname: &str) -> Self {
        self.hostname = Some(hostname.to_string());
        self
    }
}

/// Create an HTTP server.
///
/// # Example
/// ```ignore
/// use dx_compat_bun::serve::{serve, SimpleServeOptions, Request, Response};
///
/// let options = SimpleServeOptions::new(3000, |req| {
///     Box::pin(async move {
///         Response::ok("Hello, World!")
///     })
/// });
///
/// let server = serve(options).await?;
/// ```
pub async fn serve(options: SimpleServeOptions) -> BunResult<Server> {
    let hostname = options.hostname.unwrap_or_else(|| "0.0.0.0".to_string());
    let addr: SocketAddr = format!("{}:{}", hostname, options.port)
        .parse()
        .map_err(|e| BunError::Server(format!("Invalid address: {}", e)))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| BunError::Server(format!("Failed to bind: {}", e)))?;

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let fetch_handler = options.fetch;

    // Spawn the server task
    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            let io = TokioIo::new(stream);
                            let handler = Arc::clone(&fetch_handler);

                            tokio::spawn(async move {
                                let service = service_fn(move |req: HyperRequest<Incoming>| {
                                    let handler = Arc::clone(&handler);
                                    async move {
                                        let request = convert_request(req).await;
                                        let response = handler(request).await;
                                        Ok::<_, Infallible>(convert_response(response))
                                    }
                                });

                                if let Err(_e) = http1::Builder::new()
                                    .serve_connection(io, service)
                                    .await
                                {
                                    // Connection error, client likely disconnected
                                }
                            });
                        }
                        Err(_e) => {
                            // Accept error
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
        port: options.port,
        hostname,
        shutdown_tx: Some(shutdown_tx),
    })
}

async fn convert_request(req: HyperRequest<Incoming>) -> Request {
    use http_body_util::BodyExt;

    let method = req.method().clone();
    let url = req.uri().to_string();

    let mut headers = Headers::new();
    for (name, value) in req.headers() {
        if let Ok(v) = value.to_str() {
            headers.set(name.as_str(), v);
        }
    }

    let body = match req.into_body().collect().await {
        Ok(collected) => Some(collected.to_bytes()),
        Err(_) => None,
    };

    Request {
        method,
        url,
        headers,
        body,
    }
}

fn convert_response(response: Response) -> HyperResponse<Full<Bytes>> {
    let mut builder = HyperResponse::builder().status(response.status);

    for (name, value) in response.headers.iter() {
        builder = builder.header(name, value);
    }

    builder.body(Full::new(response.body)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_ok() {
        let response = Response::ok("Hello");
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(&response.body[..], b"Hello");
    }

    #[test]
    fn test_response_json() {
        let response = Response::json(r#"{"key": "value"}"#);
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.headers.get("content-type"), Some("application/json"));
    }

    #[test]
    fn test_headers() {
        let mut headers = Headers::new();
        headers.set("Content-Type", "text/plain");
        assert_eq!(headers.get("content-type"), Some("text/plain"));
        assert_eq!(headers.get("CONTENT-TYPE"), Some("text/plain"));
    }

    #[tokio::test]
    async fn test_serve_creates_server() {
        let options =
            SimpleServeOptions::new(0, |_req| Box::pin(async move { Response::ok("test") }));

        // Port 0 lets the OS assign a port
        let result = serve(options).await;
        assert!(result.is_ok());

        let mut server = result.unwrap();
        server.stop();
    }
}
