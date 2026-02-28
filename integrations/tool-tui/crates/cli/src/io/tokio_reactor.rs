//! Tokio-based reactor implementation (fallback for all platforms)

use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::process::Stdio;

use tokio::fs;
use tokio::process::Command;
use tokio::sync::mpsc;

use super::reactor::{BoxFuture, ProcessOutput, Reactor, Response, WatchEvent};

/// Tokio-based reactor implementation
///
/// This is the fallback implementation that works on all platforms
/// using Tokio's async runtime.
pub struct TokioReactor;

impl TokioReactor {
    /// Create a new Tokio reactor
    pub fn new() -> Self {
        Self
    }
}

impl Default for TokioReactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Reactor for TokioReactor {
    fn read_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, io::Result<Vec<u8>>> {
        Box::pin(async move { fs::read(path).await })
    }

    fn write_file<'a>(&'a self, path: &'a Path, data: &'a [u8]) -> BoxFuture<'a, io::Result<()>> {
        Box::pin(async move {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(path, data).await
        })
    }

    fn spawn_process<'a>(
        &'a self,
        cmd: &'a str,
        args: &'a [&'a str],
    ) -> BoxFuture<'a, io::Result<ProcessOutput>> {
        Box::pin(async move {
            let output = Command::new(cmd)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            Ok(ProcessOutput {
                status: output.status,
                stdout: output.stdout,
                stderr: output.stderr,
            })
        })
    }

    fn watch_dir<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxFuture<'a, io::Result<mpsc::Receiver<WatchEvent>>> {
        let path = path.to_path_buf();
        Box::pin(async move {
            use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

            let (tx, rx) = mpsc::channel(100);

            let mut watcher = RecommendedWatcher::new(
                move |res: Result<notify::Event, notify::Error>| {
                    if let Ok(event) = res {
                        let watch_event = match event.kind {
                            notify::EventKind::Create(_) => {
                                event.paths.first().map(|p| WatchEvent::Create(p.clone()))
                            }
                            notify::EventKind::Modify(_) => {
                                event.paths.first().map(|p| WatchEvent::Modify(p.clone()))
                            }
                            notify::EventKind::Remove(_) => {
                                event.paths.first().map(|p| WatchEvent::Delete(p.clone()))
                            }
                            _ => None,
                        };

                        if let Some(evt) = watch_event {
                            let _ = tx.blocking_send(evt);
                        }
                    }
                },
                Config::default(),
            )
            .map_err(io::Error::other)?;

            watcher.watch(&path, RecursiveMode::Recursive).map_err(io::Error::other)?;

            // Keep watcher alive by leaking it (in production, you'd manage this differently)
            std::mem::forget(watcher);

            Ok(rx)
        })
    }

    fn http_get<'a>(&'a self, url: &'a str) -> BoxFuture<'a, io::Result<Response>> {
        Box::pin(async move {
            // For now, use a simple implementation
            // In production, you'd use reqwest or similar
            let url_parsed: url::Url =
                url.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

            let host = url_parsed
                .host_str()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No host in URL"))?;

            let port = url_parsed.port().unwrap_or(if url_parsed.scheme() == "https" {
                443
            } else {
                80
            });

            // Simple HTTP/1.1 GET request
            let request = format!(
                "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: dx-cli/1.0\r\n\r\n",
                url_parsed.path(),
                host
            );

            let stream = tokio::net::TcpStream::connect((host, port)).await?;

            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let mut stream = stream;
            stream.write_all(request.as_bytes()).await?;

            let mut response = Vec::new();
            stream.read_to_end(&mut response).await?;

            // Parse response (simplified)
            let response_str = String::from_utf8_lossy(&response);
            let mut lines = response_str.lines();

            let status_line = lines.next().unwrap_or("HTTP/1.1 500 Error");
            let status: u16 = status_line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(500);

            let mut headers = HashMap::new();

            for line in lines {
                if line.is_empty() {
                    break;
                }
                if let Some((key, value)) = line.split_once(": ") {
                    headers.insert(key.to_lowercase(), value.to_string());
                }
            }

            // Find body after headers
            let body = if let Some(pos) = response_str.find("\r\n\r\n") {
                response[pos + 4..].to_vec()
            } else {
                Vec::new()
            };

            Ok(Response {
                status,
                headers,
                body,
            })
        })
    }

    fn http_post<'a>(
        &'a self,
        url: &'a str,
        body: &'a [u8],
    ) -> BoxFuture<'a, io::Result<Response>> {
        Box::pin(async move {
            let url_parsed: url::Url =
                url.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

            let host = url_parsed
                .host_str()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No host in URL"))?;

            let port = url_parsed.port().unwrap_or(if url_parsed.scheme() == "https" {
                443
            } else {
                80
            });

            let request = format!(
                "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nConnection: close\r\nUser-Agent: dx-cli/1.0\r\n\r\n",
                url_parsed.path(),
                host,
                body.len()
            );

            let stream = tokio::net::TcpStream::connect((host, port)).await?;

            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let mut stream = stream;
            stream.write_all(request.as_bytes()).await?;
            stream.write_all(body).await?;

            let mut response = Vec::new();
            stream.read_to_end(&mut response).await?;

            let response_str = String::from_utf8_lossy(&response);
            let status_line = response_str.lines().next().unwrap_or("HTTP/1.1 500 Error");
            let status: u16 = status_line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(500);

            let mut headers = HashMap::new();
            for line in response_str.lines().skip(1) {
                if line.is_empty() {
                    break;
                }
                if let Some((key, value)) = line.split_once(": ") {
                    headers.insert(key.to_lowercase(), value.to_string());
                }
            }

            let body = if let Some(pos) = response_str.find("\r\n\r\n") {
                response[pos + 4..].to_vec()
            } else {
                Vec::new()
            };

            Ok(Response {
                status,
                headers,
                body,
            })
        })
    }
}
