//! Chrome DevTools Protocol (CDP) server implementation
//!
//! Provides debugging support via the Chrome DevTools Protocol,
//! allowing connection from Chrome DevTools, VS Code, and other CDP clients.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use super::{Debugger, StackFrame};

/// CDP server for debugging JavaScript execution
pub struct CdpServer {
    /// Port to listen on
    port: u16,
    /// Debugger state
    debugger: Arc<Mutex<Debugger>>,
    /// Connected clients
    clients: Arc<Mutex<Vec<CdpClient>>>,
    /// Whether the server is running
    running: Arc<Mutex<bool>>,
}

/// A connected CDP client
struct CdpClient {
    stream: TcpStream,
    id: u64,
}

/// CDP message types
#[derive(Debug, Clone)]
pub enum CdpMessage {
    Request {
        id: u64,
        method: String,
        params: HashMap<String, serde_json::Value>,
    },
    Response {
        id: u64,
        result: serde_json::Value,
    },
    Event {
        method: String,
        params: serde_json::Value,
    },
}

impl CdpServer {
    /// Create a new CDP server on the specified port
    pub fn new(port: u16) -> Self {
        Self {
            port,
            debugger: Arc::new(Mutex::new(Debugger::new())),
            clients: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the CDP server
    pub fn start(&self) -> std::io::Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port))?;

        println!("Debugger listening on ws://127.0.0.1:{}", self.port);
        println!("To debug, open Chrome and navigate to:");
        println!("  chrome://inspect");
        println!("Or connect VS Code debugger to port {}", self.port);

        *self.running.lock().unwrap() = true;

        let debugger = Arc::clone(&self.debugger);
        let clients = Arc::clone(&self.clients);
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            for stream in listener.incoming() {
                if !*running.lock().unwrap() {
                    break;
                }

                match stream {
                    Ok(stream) => {
                        let debugger = Arc::clone(&debugger);
                        let clients = Arc::clone(&clients);

                        thread::spawn(move || {
                            if let Err(e) = handle_client(stream, debugger, clients) {
                                eprintln!("Client error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Connection error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the CDP server
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }

    /// Get the debugger instance
    pub fn debugger(&self) -> Arc<Mutex<Debugger>> {
        Arc::clone(&self.debugger)
    }

    /// Notify all clients of a debugger event
    pub fn notify_paused(&self, frame: &StackFrame, reason: &str) {
        let event = serde_json::json!({
            "method": "Debugger.paused",
            "params": {
                "callFrames": [{
                    "callFrameId": "0",
                    "functionName": frame.function_name,
                    "location": {
                        "scriptId": "1",
                        "lineNumber": frame.line,
                        "columnNumber": frame.column
                    },
                    "url": frame.file,
                    "scopeChain": []
                }],
                "reason": reason,
                "hitBreakpoints": []
            }
        });

        self.broadcast_event(event);
    }

    /// Notify all clients that execution resumed
    pub fn notify_resumed(&self) {
        let event = serde_json::json!({
            "method": "Debugger.resumed",
            "params": {}
        });

        self.broadcast_event(event);
    }

    fn broadcast_event(&self, event: serde_json::Value) {
        let clients = self.clients.lock().unwrap();
        for client in clients.iter() {
            let _ = send_cdp_message(&client.stream, &event);
        }
    }
}

/// Handle a connected CDP client
fn handle_client(
    mut stream: TcpStream,
    debugger: Arc<Mutex<Debugger>>,
    clients: Arc<Mutex<Vec<CdpClient>>>,
) -> std::io::Result<()> {
    // Perform WebSocket handshake
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();

    // Read HTTP request headers
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line.is_empty() {
            break;
        }
        request.push_str(&line);
    }

    // Extract WebSocket key
    let ws_key = extract_websocket_key(&request);

    if let Some(key) = ws_key {
        // Send WebSocket handshake response
        let accept_key = compute_websocket_accept(&key);
        let response = format!(
            "HTTP/1.1 101 Switching Protocols\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Accept: {}\r\n\r\n",
            accept_key
        );
        stream.write_all(response.as_bytes())?;
        stream.flush()?;

        // Add client to list
        let client_id = {
            let mut clients = clients.lock().unwrap();
            let id = clients.len() as u64;
            clients.push(CdpClient {
                stream: stream.try_clone()?,
                id,
            });
            id
        };

        // Handle WebSocket messages
        loop {
            match read_websocket_frame(&mut stream) {
                Ok(Some(message)) => {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&message) {
                        let response = handle_cdp_request(&json, &debugger);
                        if let Some(resp) = response {
                            send_cdp_message(&stream, &resp)?;
                        }
                    }
                }
                Ok(None) => {
                    // Connection closed
                    break;
                }
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        // Remove client from list
        let mut clients = clients.lock().unwrap();
        clients.retain(|c| c.id != client_id);
    }

    Ok(())
}

/// Handle a CDP request and return a response
fn handle_cdp_request(
    request: &serde_json::Value,
    debugger: &Arc<Mutex<Debugger>>,
) -> Option<serde_json::Value> {
    let id = request.get("id")?.as_u64()?;
    let method = request.get("method")?.as_str()?;
    let params = request.get("params").cloned().unwrap_or(serde_json::json!({}));

    let result = match method {
        // Debugger domain
        "Debugger.enable" => {
            serde_json::json!({
                "debuggerId": "dx-js-debugger"
            })
        }
        "Debugger.disable" => {
            serde_json::json!({})
        }
        "Debugger.setBreakpointByUrl" => {
            let line = params.get("lineNumber").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let column = params.get("columnNumber").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let url = params.get("url").and_then(|v| v.as_str()).unwrap_or("");

            let mut dbg = debugger.lock().unwrap();
            dbg.set_breakpoint(url.to_string(), line, column);

            serde_json::json!({
                "breakpointId": format!("{}:{}:{}", url, line, column),
                "locations": [{
                    "scriptId": "1",
                    "lineNumber": line,
                    "columnNumber": column
                }]
            })
        }
        "Debugger.removeBreakpoint" => {
            let bp_id = params.get("breakpointId").and_then(|v| v.as_str()).unwrap_or("");
            let parts: Vec<&str> = bp_id.rsplitn(3, ':').collect();
            if parts.len() >= 2 {
                if let Ok(line) = parts[1].parse::<usize>() {
                    let file = parts[2..].join(":");
                    let mut dbg = debugger.lock().unwrap();
                    dbg.remove_breakpoint(&file, line);
                }
            }
            serde_json::json!({})
        }
        "Debugger.resume" => {
            let mut dbg = debugger.lock().unwrap();
            dbg.resume();
            serde_json::json!({})
        }
        "Debugger.pause" => {
            let mut dbg = debugger.lock().unwrap();
            dbg.pause();
            serde_json::json!({})
        }
        "Debugger.stepOver" => {
            let mut dbg = debugger.lock().unwrap();
            dbg.step_over();
            serde_json::json!({})
        }
        "Debugger.stepInto" => {
            let mut dbg = debugger.lock().unwrap();
            dbg.step_into();
            serde_json::json!({})
        }
        "Debugger.stepOut" => {
            let mut dbg = debugger.lock().unwrap();
            dbg.step_out();
            serde_json::json!({})
        }
        "Debugger.getScriptSource" => {
            // Return script source - would need to track loaded scripts
            serde_json::json!({
                "scriptSource": "// Script source not available"
            })
        }

        // Runtime domain
        "Runtime.enable" => {
            serde_json::json!({})
        }
        "Runtime.disable" => {
            serde_json::json!({})
        }
        "Runtime.evaluate" => {
            let _expression = params.get("expression").and_then(|v| v.as_str()).unwrap_or("");
            // Would evaluate expression in current context
            serde_json::json!({
                "result": {
                    "type": "undefined"
                }
            })
        }
        "Runtime.getProperties" => {
            let dbg = debugger.lock().unwrap();
            let vars = dbg.get_variables();
            let properties: Vec<serde_json::Value> = vars
                .iter()
                .map(|(name, value)| {
                    serde_json::json!({
                        "name": name,
                        "value": {
                            "type": "string",
                            "value": value
                        },
                        "writable": true,
                        "configurable": true,
                        "enumerable": true
                    })
                })
                .collect();

            serde_json::json!({
                "result": properties
            })
        }

        // Profiler domain
        "Profiler.enable" => serde_json::json!({}),
        "Profiler.disable" => serde_json::json!({}),

        // HeapProfiler domain
        "HeapProfiler.enable" => serde_json::json!({}),
        "HeapProfiler.disable" => serde_json::json!({}),

        // Default response for unknown methods
        _ => {
            eprintln!("Unknown CDP method: {}", method);
            serde_json::json!({})
        }
    };

    Some(serde_json::json!({
        "id": id,
        "result": result
    }))
}

/// Extract WebSocket key from HTTP request
fn extract_websocket_key(request: &str) -> Option<String> {
    for line in request.lines() {
        if line.to_lowercase().starts_with("sec-websocket-key:") {
            return Some(line.split(':').nth(1)?.trim().to_string());
        }
    }
    None
}

/// Compute WebSocket accept key
fn compute_websocket_accept(key: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use sha1::{Digest, Sha1};

    let magic = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined = format!("{}{}", key, magic);
    let hash = Sha1::digest(combined.as_bytes());
    STANDARD.encode(hash)
}

/// Read a WebSocket frame
fn read_websocket_frame(stream: &mut TcpStream) -> std::io::Result<Option<String>> {
    let mut header = [0u8; 2];
    if stream.read_exact(&mut header).is_err() {
        return Ok(None);
    }

    let _fin = (header[0] & 0x80) != 0;
    let opcode = header[0] & 0x0F;
    let masked = (header[1] & 0x80) != 0;
    let mut payload_len = (header[1] & 0x7F) as usize;

    // Handle extended payload length
    if payload_len == 126 {
        let mut ext = [0u8; 2];
        stream.read_exact(&mut ext)?;
        payload_len = u16::from_be_bytes(ext) as usize;
    } else if payload_len == 127 {
        let mut ext = [0u8; 8];
        stream.read_exact(&mut ext)?;
        payload_len = u64::from_be_bytes(ext) as usize;
    }

    // Read mask if present
    let mask = if masked {
        let mut m = [0u8; 4];
        stream.read_exact(&mut m)?;
        Some(m)
    } else {
        None
    };

    // Read payload
    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload)?;

    // Unmask if needed
    if let Some(mask) = mask {
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
    }

    // Handle different opcodes
    match opcode {
        0x1 => {
            // Text frame
            Ok(Some(String::from_utf8_lossy(&payload).to_string()))
        }
        0x8 => {
            // Close frame
            Ok(None)
        }
        0x9 => {
            // Ping - send pong
            send_websocket_frame(stream, 0xA, &payload)?;
            read_websocket_frame(stream)
        }
        _ => {
            // Other frames - ignore
            read_websocket_frame(stream)
        }
    }
}

/// Send a WebSocket frame
fn send_websocket_frame(stream: &TcpStream, opcode: u8, payload: &[u8]) -> std::io::Result<()> {
    let mut stream = stream.try_clone()?;
    let mut frame = Vec::new();

    // FIN + opcode
    frame.push(0x80 | opcode);

    // Payload length
    if payload.len() < 126 {
        frame.push(payload.len() as u8);
    } else if payload.len() < 65536 {
        frame.push(126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }

    frame.extend_from_slice(payload);
    stream.write_all(&frame)?;
    stream.flush()
}

/// Send a CDP message over WebSocket
fn send_cdp_message(stream: &TcpStream, message: &serde_json::Value) -> std::io::Result<()> {
    let json = serde_json::to_string(message).unwrap_or_default();
    send_websocket_frame(stream, 0x1, json.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_accept_key() {
        // Test vector from RFC 6455
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = compute_websocket_accept(key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}
