//! Standard I/O transport for MCP compatibility.
//!
//! Provides stdin/stdout handling with line-based JSON-RPC message framing.
//! Supports async operations, graceful shutdown, and stderr diagnostics.

use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Configuration for stdio transport
#[derive(Debug, Clone)]
pub struct StdioConfig {
    /// Maximum message size (default 10MB)
    pub max_message_size: usize,
    /// Whether to flush after each write
    pub auto_flush: bool,
    /// Enable stderr logging
    pub stderr_logging: bool,
    /// Read buffer size
    pub buffer_size: usize,
}

impl Default for StdioConfig {
    fn default() -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10MB
            auto_flush: true,
            stderr_logging: true,
            buffer_size: 4096,
        }
    }
}

/// Line-based message framing for stdio transport
pub struct StdioTransport {
    /// Buffer for reading lines
    read_buffer: String,
    /// Configuration
    config: StdioConfig,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new() -> Self {
        Self::with_config(StdioConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: StdioConfig) -> Self {
        Self {
            read_buffer: String::with_capacity(config.buffer_size),
            config,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set auto-flush behavior
    pub fn with_auto_flush(mut self, auto_flush: bool) -> Self {
        self.config.auto_flush = auto_flush;
        self
    }

    /// Get shutdown handle for external shutdown signaling
    pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    /// Signal shutdown
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown was signaled
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Read a single JSON-RPC message from stdin
    pub fn read_message<R: BufRead>(&mut self, reader: &mut R) -> io::Result<Option<String>> {
        if self.is_shutdown() {
            return Ok(None);
        }

        self.read_buffer.clear();

        let bytes_read = reader.read_line(&mut self.read_buffer)?;
        if bytes_read == 0 {
            // EOF - signal graceful shutdown
            self.log_stderr("EOF received on stdin, initiating shutdown");
            self.shutdown();
            return Ok(None);
        }

        // Trim trailing newline
        let message = self.read_buffer.trim_end().to_string();
        if message.is_empty() {
            return Ok(None);
        }

        Ok(Some(message))
    }

    /// Write a JSON-RPC message to stdout
    pub fn write_message<W: Write>(&self, writer: &mut W, message: &str) -> io::Result<()> {
        writeln!(writer, "{}", message)?;
        if self.config.auto_flush {
            writer.flush()?;
        }
        Ok(())
    }

    /// Read messages from stdin until EOF or shutdown
    pub fn read_all_messages<R: BufRead>(&mut self, reader: &mut R) -> io::Result<Vec<String>> {
        let mut messages = Vec::new();
        while !self.is_shutdown() {
            match self.read_message(reader)? {
                Some(msg) => messages.push(msg),
                None => break,
            }
        }
        Ok(messages)
    }

    /// Log a diagnostic message to stderr
    pub fn log_stderr(&self, message: &str) {
        if self.config.stderr_logging {
            let _ = writeln!(io::stderr(), "[DCP] {}", message);
        }
    }

    /// Log an error to stderr
    pub fn log_error(&self, message: &str) {
        if self.config.stderr_logging {
            let _ = writeln!(io::stderr(), "[DCP ERROR] {}", message);
        }
    }

    /// Log a debug message to stderr
    pub fn log_debug(&self, message: &str) {
        if self.config.stderr_logging {
            let _ = writeln!(io::stderr(), "[DCP DEBUG] {}", message);
        }
    }
}

/// Message framer for handling JSON-RPC over stdio
pub struct MessageFramer {
    /// Accumulated buffer for partial messages
    buffer: Vec<u8>,
    /// Maximum message size
    max_size: usize,
}

impl Default for MessageFramer {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageFramer {
    /// Create a new message framer
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(4096),
            max_size: 10 * 1024 * 1024, // 10MB
        }
    }

    /// Set maximum message size
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }

    /// Feed bytes into the framer and extract complete messages
    pub fn feed(&mut self, data: &[u8]) -> io::Result<Vec<String>> {
        let mut messages = Vec::new();

        for &byte in data {
            if byte == b'\n' {
                // Complete message
                if !self.buffer.is_empty() {
                    let message = String::from_utf8(std::mem::take(&mut self.buffer))
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    let trimmed = message.trim().to_string();
                    if !trimmed.is_empty() {
                        messages.push(trimmed);
                    }
                }
            } else {
                // Check size limit
                if self.buffer.len() >= self.max_size {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Message exceeds maximum size",
                    ));
                }
                self.buffer.push(byte);
            }
        }

        Ok(messages)
    }

    /// Check if there's a partial message in the buffer
    pub fn has_partial(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Get the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

/// Frame a message for stdio transport (add newline)
pub fn frame_message(message: &str) -> String {
    format!("{}\n", message)
}

/// Unframe a message from stdio transport (remove trailing newline)
pub fn unframe_message(data: &str) -> &str {
    data.trim_end_matches('\n').trim_end_matches('\r')
}

/// Async stdio transport using tokio
#[cfg(feature = "async-stdio")]
pub mod async_transport {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::sync::mpsc;

    /// Async stdio transport
    pub struct AsyncStdioTransport {
        config: StdioConfig,
        shutdown: Arc<AtomicBool>,
    }

    impl AsyncStdioTransport {
        /// Create new async transport
        pub fn new(config: StdioConfig) -> Self {
            Self {
                config,
                shutdown: Arc::new(AtomicBool::new(false)),
            }
        }

        /// Get shutdown handle
        pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
            Arc::clone(&self.shutdown)
        }

        /// Run the transport, returning channels for messages
        pub async fn run(self) -> io::Result<(mpsc::Receiver<String>, mpsc::Sender<String>)> {
            let (in_tx, in_rx) = mpsc::channel(100);
            let (out_tx, mut out_rx) = mpsc::channel::<String>(100);

            let shutdown = Arc::clone(&self.shutdown);
            let config = self.config.clone();

            // Spawn stdin reader
            tokio::spawn(async move {
                let stdin = tokio::io::stdin();
                let mut reader = BufReader::new(stdin);
                let mut line = String::new();

                loop {
                    if shutdown.load(Ordering::SeqCst) {
                        break;
                    }

                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => {
                            // EOF
                            if config.stderr_logging {
                                eprintln!("[DCP] EOF on stdin");
                            }
                            break;
                        }
                        Ok(_) => {
                            let msg = line.trim().to_string();
                            if !msg.is_empty() {
                                if in_tx.send(msg).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            if config.stderr_logging {
                                eprintln!("[DCP ERROR] stdin read error: {}", e);
                            }
                            break;
                        }
                    }
                }
            });

            // Spawn stdout writer
            tokio::spawn(async move {
                let mut stdout = tokio::io::stdout();

                while let Some(msg) = out_rx.recv().await {
                    let framed = format!("{}\n", msg);
                    if let Err(e) = stdout.write_all(framed.as_bytes()).await {
                        eprintln!("[DCP ERROR] stdout write error: {}", e);
                        break;
                    }
                    if let Err(e) = stdout.flush().await {
                        eprintln!("[DCP ERROR] stdout flush error: {}", e);
                        break;
                    }
                }
            });

            Ok((in_rx, out_tx))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_stdio_transport_read_message() {
        let mut transport = StdioTransport::new();
        let input = r#"{"jsonrpc":"2.0","method":"test","id":1}
"#;
        let mut reader = Cursor::new(input);

        let message = transport.read_message(&mut reader).unwrap();
        assert_eq!(message, Some(r#"{"jsonrpc":"2.0","method":"test","id":1}"#.to_string()));
    }

    #[test]
    fn test_stdio_transport_read_eof() {
        let mut transport = StdioTransport::new();
        let mut reader = Cursor::new("");

        let message = transport.read_message(&mut reader).unwrap();
        assert_eq!(message, None);
        assert!(transport.is_shutdown()); // EOF triggers shutdown
    }

    #[test]
    fn test_stdio_transport_write_message() {
        let transport = StdioTransport::new();
        let mut output = Vec::new();

        transport
            .write_message(&mut output, r#"{"jsonrpc":"2.0","result":{},"id":1}"#)
            .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "{\"jsonrpc\":\"2.0\",\"result\":{},\"id\":1}\n"
        );
    }

    #[test]
    fn test_stdio_transport_read_multiple() {
        let mut transport = StdioTransport::new();
        let input = r#"{"id":1}
{"id":2}
{"id":3}
"#;
        let mut reader = Cursor::new(input);

        let messages = transport.read_all_messages(&mut reader).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0], r#"{"id":1}"#);
        assert_eq!(messages[1], r#"{"id":2}"#);
        assert_eq!(messages[2], r#"{"id":3}"#);
    }

    #[test]
    fn test_stdio_transport_shutdown() {
        let transport = StdioTransport::new();
        assert!(!transport.is_shutdown());

        transport.shutdown();
        assert!(transport.is_shutdown());
    }

    #[test]
    fn test_stdio_transport_shutdown_handle() {
        let transport = StdioTransport::new();
        let handle = transport.shutdown_handle();

        assert!(!handle.load(Ordering::SeqCst));
        transport.shutdown();
        assert!(handle.load(Ordering::SeqCst));
    }

    #[test]
    fn test_stdio_config() {
        let config = StdioConfig {
            max_message_size: 1024,
            auto_flush: false,
            stderr_logging: false,
            buffer_size: 2048,
        };

        let transport = StdioTransport::with_config(config);
        assert!(!transport.config.auto_flush);
        assert!(!transport.config.stderr_logging);
    }

    #[test]
    fn test_message_framer_single() {
        let mut framer = MessageFramer::new();
        let messages = framer.feed(b"{\"test\":1}\n").unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "{\"test\":1}");
        assert!(!framer.has_partial());
    }

    #[test]
    fn test_message_framer_multiple() {
        let mut framer = MessageFramer::new();
        let messages = framer.feed(b"{\"a\":1}\n{\"b\":2}\n").unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], "{\"a\":1}");
        assert_eq!(messages[1], "{\"b\":2}");
    }

    #[test]
    fn test_message_framer_partial() {
        let mut framer = MessageFramer::new();

        // First chunk - partial message
        let messages1 = framer.feed(b"{\"partial\":").unwrap();
        assert!(messages1.is_empty());
        assert!(framer.has_partial());

        // Second chunk - complete message
        let messages2 = framer.feed(b"true}\n").unwrap();
        assert_eq!(messages2.len(), 1);
        assert_eq!(messages2[0], "{\"partial\":true}");
        assert!(!framer.has_partial());
    }

    #[test]
    fn test_message_framer_max_size() {
        let mut framer = MessageFramer::new().with_max_size(10);

        let result = framer.feed(b"this is way too long");
        assert!(result.is_err());
    }

    #[test]
    fn test_frame_unframe() {
        let original = r#"{"jsonrpc":"2.0"}"#;
        let framed = frame_message(original);
        let unframed = unframe_message(&framed);

        assert_eq!(unframed, original);
    }
}
