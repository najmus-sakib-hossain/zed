//! HTTP/HTTPS Client and Server
//!
//! Native implementation of Node.js http and https modules

use crate::error::{DxError, DxResult};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

/// HTTP module (http and https)
pub struct HttpModule {
    /// Default timeout
    timeout: Duration,
}

impl HttpModule {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
        }
    }

    /// Create HTTP server
    pub fn create_server(&self, handler: Box<dyn Fn(HttpRequest, HttpResponse)>) -> HttpServer {
        HttpServer::new(handler)
    }

    /// Make HTTP GET request
    pub fn get(&self, url: &str) -> DxResult<HttpResponse> {
        self.request("GET", url, None, None)
    }

    /// Make HTTP POST request
    pub fn post(&self, url: &str, body: Option<Vec<u8>>) -> DxResult<HttpResponse> {
        self.request("POST", url, body, None)
    }

    /// Make generic HTTP request
    pub fn request(
        &self,
        method: &str,
        url: &str,
        body: Option<Vec<u8>>,
        headers: Option<HashMap<String, String>>,
    ) -> DxResult<HttpResponse> {
        // Parse URL
        let parsed = parse_url(url)?;

        // Connect to server
        let addr = format!("{}:{}", parsed.host, parsed.port);
        let mut stream = TcpStream::connect(&addr)
            .map_err(|e| DxError::IoError(format!("Connection failed: {}", e)))?;

        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|e| DxError::IoError(e.to_string()))?;

        // Build HTTP request
        let mut request_lines = vec![
            format!("{} {} HTTP/1.1", method, parsed.path),
            format!("Host: {}", parsed.host),
        ];

        // Add headers
        if let Some(hdrs) = headers {
            for (key, value) in hdrs {
                request_lines.push(format!("{}: {}", key, value));
            }
        }

        // Add body if present
        if let Some(ref body_data) = body {
            request_lines.push(format!("Content-Length: {}", body_data.len()));
        }

        request_lines.push(String::new()); // Empty line before body
        let request = request_lines.join("\r\n");

        // Send request
        stream
            .write_all(request.as_bytes())
            .map_err(|e| DxError::IoError(e.to_string()))?;

        if let Some(body_data) = body {
            stream.write_all(&body_data).map_err(|e| DxError::IoError(e.to_string()))?;
        }

        // Read response
        let mut response_data = Vec::new();
        stream
            .read_to_end(&mut response_data)
            .map_err(|e| DxError::IoError(e.to_string()))?;

        // Parse response
        parse_http_response(&response_data)
    }
}

impl Default for HttpModule {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP Server
pub struct HttpServer {
    handler: Box<dyn Fn(HttpRequest, HttpResponse)>,
    listener: Option<TcpListener>,
}

impl HttpServer {
    pub fn new(handler: Box<dyn Fn(HttpRequest, HttpResponse)>) -> Self {
        Self {
            handler,
            listener: None,
        }
    }

    /// Start listening on port
    pub fn listen(&mut self, port: u16) -> DxResult<()> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr)
            .map_err(|e| DxError::IoError(format!("Failed to bind to {}: {}", addr, e)))?;

        self.listener = Some(listener);

        // Accept connections
        self.accept_loop()
    }

    /// Accept connections loop
    fn accept_loop(&self) -> DxResult<()> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| DxError::RuntimeError("Server not listening".to_string()))?;

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    // Read request
                    let mut buffer = vec![0; 8192];
                    let n =
                        stream.read(&mut buffer).map_err(|e| DxError::IoError(e.to_string()))?;

                    let request_data = &buffer[..n];

                    // Parse request
                    if let Ok(request) = parse_http_request(request_data) {
                        // Create response
                        let response = HttpResponse::new();

                        // Call handler
                        (self.handler)(request, response.clone());

                        // Send response
                        let response_bytes = response.to_bytes();
                        stream
                            .write_all(&response_bytes)
                            .map_err(|e| DxError::IoError(e.to_string()))?;
                    }
                }
                Err(e) => {
                    eprintln!("Connection failed: {}", e);
                }
            }
        }

        Ok(())
    }
}

/// HTTP Request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn new(method: String, url: String) -> Self {
        Self {
            method,
            url,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }
}

/// HTTP Response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn new() -> Self {
        Self {
            status_code: 200,
            status_text: "OK".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Set status code
    pub fn status(&mut self, code: u16) {
        self.status_code = code;
        self.status_text = match code {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            400 => "Bad Request",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Unknown",
        }
        .to_string();
    }

    /// Set header
    pub fn set_header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }

    /// Write body
    pub fn write(&mut self, data: Vec<u8>) {
        self.body.extend(data);
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut response = Vec::new();

        // Status line
        let status_line = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text);
        response.extend(status_line.as_bytes());

        // Headers
        for (key, value) in &self.headers {
            let header = format!("{}: {}\r\n", key, value);
            response.extend(header.as_bytes());
        }

        // Content-Length
        let content_length = format!("Content-Length: {}\r\n", self.body.len());
        response.extend(content_length.as_bytes());

        // Empty line
        response.extend(b"\r\n");

        // Body
        response.extend(&self.body);

        response
    }
}

impl Default for HttpResponse {
    fn default() -> Self {
        Self::new()
    }
}

/// Parsed URL
struct ParsedUrl {
    host: String,
    port: u16,
    path: String,
}

/// Parse URL
fn parse_url(url: &str) -> DxResult<ParsedUrl> {
    // Simple URL parsing: http://host:port/path
    let url = url.trim_start_matches("http://").trim_start_matches("https://");

    let (host_port, path) = if let Some(pos) = url.find('/') {
        (&url[..pos], &url[pos..])
    } else {
        (url, "/")
    };

    let (host, port) = if let Some(pos) = host_port.find(':') {
        let host = &host_port[..pos];
        let port = host_port[pos + 1..]
            .parse::<u16>()
            .map_err(|_| DxError::RuntimeError("Invalid port".to_string()))?;
        (host.to_string(), port)
    } else {
        (host_port.to_string(), 80)
    };

    Ok(ParsedUrl {
        host,
        port,
        path: path.to_string(),
    })
}

/// Parse HTTP response
fn parse_http_response(data: &[u8]) -> DxResult<HttpResponse> {
    let response_str = String::from_utf8_lossy(data);
    let mut lines = response_str.lines();

    // Parse status line
    let status_line = lines
        .next()
        .ok_or_else(|| DxError::RuntimeError("Invalid HTTP response".to_string()))?;

    let parts: Vec<&str> = status_line.split_whitespace().collect();
    let status_code = parts.get(1).and_then(|s| s.parse::<u16>().ok()).unwrap_or(200);

    // Parse headers
    let mut headers = HashMap::new();
    let mut body_start = 0;

    for (i, line) in lines.enumerate() {
        if line.is_empty() {
            body_start = i + 2; // After status line and headers
            break;
        }

        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_string();
            let value = line[pos + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    // Extract body
    let body = if body_start > 0 {
        let body_str = response_str.lines().skip(body_start).collect::<Vec<_>>().join("\n");
        body_str.as_bytes().to_vec()
    } else {
        Vec::new()
    };

    Ok(HttpResponse {
        status_code,
        status_text: "OK".to_string(),
        headers,
        body,
    })
}

/// Parse HTTP request
fn parse_http_request(data: &[u8]) -> DxResult<HttpRequest> {
    let request_str = String::from_utf8_lossy(data);
    let mut lines = request_str.lines();

    // Parse request line
    let request_line = lines
        .next()
        .ok_or_else(|| DxError::RuntimeError("Invalid HTTP request".to_string()))?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let method = parts.first().unwrap_or(&"GET").to_string();
    let url = parts.get(1).unwrap_or(&"/").to_string();

    // Parse headers
    let mut headers = HashMap::new();
    let mut body_start = 0;

    for (i, line) in lines.enumerate() {
        if line.is_empty() {
            body_start = i + 2;
            break;
        }

        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_string();
            let value = line[pos + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    // Extract body
    let body = if body_start > 0 {
        let body_str = request_str.lines().skip(body_start).collect::<Vec<_>>().join("\n");
        body_str.as_bytes().to_vec()
    } else {
        Vec::new()
    };

    Ok(HttpRequest {
        method,
        url,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url() {
        let parsed = parse_url("http://example.com:8080/path").unwrap();
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 8080);
        assert_eq!(parsed.path, "/path");
    }

    #[test]
    fn test_http_response() {
        let mut response = HttpResponse::new();
        response.status(404);
        response.set_header("Content-Type".to_string(), "text/plain".to_string());
        response.write(b"Not Found".to_vec());

        let bytes = response.to_bytes();
        assert!(!bytes.is_empty());
    }
}
