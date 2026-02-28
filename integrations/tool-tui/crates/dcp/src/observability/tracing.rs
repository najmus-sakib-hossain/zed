//! OpenTelemetry tracing integration for DCP server.
//!
//! Provides distributed tracing with request ID propagation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Tracing configuration
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Service name for tracing
    pub service_name: String,
    /// Enable tracing
    pub enabled: bool,
    /// Sample rate (0.0 to 1.0)
    pub sample_rate: f64,
    /// OTLP endpoint (if using OpenTelemetry)
    pub otlp_endpoint: Option<String>,
    /// Include request/response bodies in spans
    pub include_bodies: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "dcp-server".to_string(),
            enabled: true,
            sample_rate: 1.0,
            otlp_endpoint: None,
            include_bodies: false,
        }
    }
}

/// Request ID generator
#[derive(Debug, Default)]
pub struct RequestIdGenerator {
    counter: AtomicU64,
    node_id: u16,
}

impl RequestIdGenerator {
    /// Create a new request ID generator
    pub fn new() -> Self {
        Self::with_node_id(0)
    }

    /// Create with a specific node ID for distributed systems
    pub fn with_node_id(node_id: u16) -> Self {
        Self {
            counter: AtomicU64::new(0),
            node_id,
        }
    }

    /// Generate a new request ID
    /// Format: {timestamp_ms}-{node_id}-{counter}
    pub fn generate(&self) -> String {
        let timestamp =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        let counter = self.counter.fetch_add(1, Ordering::Relaxed);
        format!("{:x}-{:04x}-{:08x}", timestamp, self.node_id, counter)
    }
}

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// Operation completed successfully
    Ok,
    /// Operation failed with an error
    Error,
    /// Operation was cancelled
    Cancelled,
}

/// Span kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// Internal operation
    Internal,
    /// Server handling a request
    Server,
    /// Client making a request
    Client,
    /// Producer sending a message
    Producer,
    /// Consumer receiving a message
    Consumer,
}

/// A tracing span representing a unit of work
#[derive(Debug)]
pub struct Span {
    /// Span name
    pub name: String,
    /// Trace ID
    pub trace_id: String,
    /// Span ID
    pub span_id: String,
    /// Parent span ID
    pub parent_span_id: Option<String>,
    /// Span kind
    pub kind: SpanKind,
    /// Start time
    pub start_time: Instant,
    /// End time (set when span ends)
    pub end_time: Option<Instant>,
    /// Span status
    pub status: SpanStatus,
    /// Attributes
    pub attributes: HashMap<String, SpanValue>,
    /// Events
    pub events: Vec<SpanEvent>,
}

/// Span attribute value
#[derive(Debug, Clone)]
pub enum SpanValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl From<&str> for SpanValue {
    fn from(s: &str) -> Self {
        SpanValue::String(s.to_string())
    }
}

impl From<String> for SpanValue {
    fn from(s: String) -> Self {
        SpanValue::String(s)
    }
}

impl From<i64> for SpanValue {
    fn from(v: i64) -> Self {
        SpanValue::Int(v)
    }
}

impl From<f64> for SpanValue {
    fn from(v: f64) -> Self {
        SpanValue::Float(v)
    }
}

impl From<bool> for SpanValue {
    fn from(v: bool) -> Self {
        SpanValue::Bool(v)
    }
}

/// Span event
#[derive(Debug, Clone)]
pub struct SpanEvent {
    /// Event name
    pub name: String,
    /// Event timestamp
    pub timestamp: Instant,
    /// Event attributes
    pub attributes: HashMap<String, SpanValue>,
}

impl Span {
    /// Create a new span
    pub fn new(name: impl Into<String>, trace_id: String, span_id: String) -> Self {
        Self {
            name: name.into(),
            trace_id,
            span_id,
            parent_span_id: None,
            kind: SpanKind::Internal,
            start_time: Instant::now(),
            end_time: None,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Set parent span ID
    pub fn with_parent(mut self, parent_span_id: String) -> Self {
        self.parent_span_id = Some(parent_span_id);
        self
    }

    /// Set span kind
    pub fn with_kind(mut self, kind: SpanKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set an attribute
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<SpanValue>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Add an event
    pub fn add_event(&mut self, name: impl Into<String>) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: Instant::now(),
            attributes: HashMap::new(),
        });
    }

    /// Add an event with attributes
    pub fn add_event_with_attributes(
        &mut self,
        name: impl Into<String>,
        attributes: HashMap<String, SpanValue>,
    ) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: Instant::now(),
            attributes,
        });
    }

    /// Set span status
    pub fn set_status(&mut self, status: SpanStatus) {
        self.status = status;
    }

    /// End the span
    pub fn end(&mut self) {
        self.end_time = Some(Instant::now());
    }

    /// Get span duration
    pub fn duration(&self) -> Option<Duration> {
        self.end_time.map(|end| end.duration_since(self.start_time))
    }
}

/// Span ID generator
fn generate_span_id() -> String {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let random = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    format!("{:016x}", id ^ random)
}

/// Trace ID generator
fn generate_trace_id() -> String {
    let timestamp =
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
    let random = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    format!("{:016x}{:016x}", timestamp, random)
}

/// Create a new span
pub fn create_span(name: impl Into<String>) -> Span {
    Span::new(name, generate_trace_id(), generate_span_id())
}

/// Create a child span
pub fn create_child_span(name: impl Into<String>, parent: &Span) -> Span {
    Span::new(name, parent.trace_id.clone(), generate_span_id()).with_parent(parent.span_id.clone())
}

/// Tracer for creating and managing spans
#[derive(Debug)]
pub struct Tracer {
    config: TracingConfig,
    request_id_generator: RequestIdGenerator,
}

impl Tracer {
    /// Create a new tracer
    pub fn new(config: TracingConfig) -> Self {
        Self {
            config,
            request_id_generator: RequestIdGenerator::new(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(TracingConfig::default())
    }

    /// Generate a request ID
    pub fn generate_request_id(&self) -> String {
        self.request_id_generator.generate()
    }

    /// Create a new root span
    pub fn start_span(&self, name: impl Into<String>) -> Span {
        create_span(name)
    }

    /// Create a child span
    pub fn start_child_span(&self, name: impl Into<String>, parent: &Span) -> Span {
        create_child_span(name, parent)
    }

    /// Check if tracing is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get configuration
    pub fn config(&self) -> &TracingConfig {
        &self.config
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Request span for tracking HTTP/RPC requests
pub struct RequestSpan {
    span: Span,
    request_id: String,
}

impl RequestSpan {
    /// Create a new request span
    pub fn new(method: impl Into<String>, request_id: String) -> Self {
        let method = method.into();
        let mut span = create_span(&method);
        span.set_attribute("rpc.method", method);
        span.set_attribute("request.id", request_id.clone());
        span.kind = SpanKind::Server;
        Self { span, request_id }
    }

    /// Get the request ID
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Get the trace ID
    pub fn trace_id(&self) -> &str {
        &self.span.trace_id
    }

    /// Get the span ID
    pub fn span_id(&self) -> &str {
        &self.span.span_id
    }

    /// Set an attribute
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<SpanValue>) {
        self.span.set_attribute(key, value);
    }

    /// Add an event
    pub fn add_event(&mut self, name: impl Into<String>) {
        self.span.add_event(name);
    }

    /// Set status to error
    pub fn set_error(&mut self, error_message: impl Into<String>) {
        self.span.set_status(SpanStatus::Error);
        self.span.set_attribute("error.message", error_message.into());
    }

    /// Finish the span successfully
    pub fn finish(mut self) -> Span {
        self.span.set_status(SpanStatus::Ok);
        self.span.end();
        self.span
    }

    /// Finish the span with an error
    pub fn finish_with_error(mut self, error: impl Into<String>) -> Span {
        self.span.set_status(SpanStatus::Error);
        self.span.set_attribute("error.message", error.into());
        self.span.end();
        self.span
    }

    /// Get the underlying span
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Get mutable access to the underlying span
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }
}

/// Initialize tracing with the given configuration
pub fn init_tracing(config: TracingConfig) -> Tracer {
    Tracer::new(config)
}

/// Span collector for testing and debugging
#[derive(Debug, Default)]
pub struct SpanCollector {
    spans: std::sync::RwLock<Vec<Span>>,
}

impl SpanCollector {
    /// Create a new span collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Collect a span
    pub fn collect(&self, span: Span) {
        self.spans.write().unwrap().push(span);
    }

    /// Get all collected spans
    pub fn spans(&self) -> Vec<Span> {
        self.spans.read().unwrap().clone()
    }

    /// Clear collected spans
    pub fn clear(&self) {
        self.spans.write().unwrap().clear();
    }

    /// Get span count
    pub fn len(&self) -> usize {
        self.spans.read().unwrap().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.spans.read().unwrap().is_empty()
    }
}

impl Clone for Span {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
            parent_span_id: self.parent_span_id.clone(),
            kind: self.kind,
            start_time: self.start_time,
            end_time: self.end_time,
            status: self.status,
            attributes: self.attributes.clone(),
            events: self.events.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_generation() {
        let generator = RequestIdGenerator::new();
        let id1 = generator.generate();
        let id2 = generator.generate();

        assert_ne!(id1, id2);
        assert!(id1.contains('-'));
        assert!(id2.contains('-'));
    }

    #[test]
    fn test_span_creation() {
        let span = create_span("test_operation");

        assert_eq!(span.name, "test_operation");
        assert!(!span.trace_id.is_empty());
        assert!(!span.span_id.is_empty());
        assert!(span.parent_span_id.is_none());
        assert_eq!(span.status, SpanStatus::Ok);
    }

    #[test]
    fn test_child_span() {
        let parent = create_span("parent");
        let child = create_child_span("child", &parent);

        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id.clone()));
        assert_ne!(child.span_id, parent.span_id);
    }

    #[test]
    fn test_span_attributes() {
        let mut span = create_span("test");
        span.set_attribute("key1", "value1");
        span.set_attribute("key2", 42i64);
        span.set_attribute("key3", 3.14f64);
        span.set_attribute("key4", true);

        assert_eq!(span.attributes.len(), 4);
    }

    #[test]
    fn test_span_events() {
        let mut span = create_span("test");
        span.add_event("event1");
        span.add_event("event2");

        assert_eq!(span.events.len(), 2);
        assert_eq!(span.events[0].name, "event1");
        assert_eq!(span.events[1].name, "event2");
    }

    #[test]
    fn test_span_duration() {
        let mut span = create_span("test");
        std::thread::sleep(std::time::Duration::from_millis(1));
        span.end();

        let duration = span.duration().unwrap();
        assert!(duration.as_micros() > 0);
    }

    #[test]
    fn test_request_span() {
        let request_id = "test-request-123".to_string();
        let mut span = RequestSpan::new("tools/call", request_id.clone());

        assert_eq!(span.request_id(), "test-request-123");
        span.set_attribute("tool.name", "my_tool");

        let finished = span.finish();
        assert_eq!(finished.status, SpanStatus::Ok);
        assert!(finished.end_time.is_some());
    }

    #[test]
    fn test_tracer() {
        let tracer = Tracer::with_defaults();

        let request_id = tracer.generate_request_id();
        assert!(!request_id.is_empty());

        let span = tracer.start_span("test");
        assert_eq!(span.name, "test");
    }

    #[test]
    fn test_span_collector() {
        let collector = SpanCollector::new();

        let mut span1 = create_span("span1");
        span1.end();
        collector.collect(span1);

        let mut span2 = create_span("span2");
        span2.end();
        collector.collect(span2);

        assert_eq!(collector.len(), 2);

        let spans = collector.spans();
        assert_eq!(spans[0].name, "span1");
        assert_eq!(spans[1].name, "span2");
    }
}
