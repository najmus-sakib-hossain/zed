//! Observability module for DCP server.
//!
//! Provides Prometheus metrics, OpenTelemetry tracing, and structured logging.

mod logging;
mod metrics;
mod tracing;

pub use logging::{
    LogConfig, LogEntry, LogEntryBuilder, LogFormat, LogLevel, LogValue, StructuredLogger,
};
pub use metrics::{
    Counter, Histogram, LabeledCounter, LabeledHistogram, MetricsConfig, PrometheusMetrics,
    RequestMetrics,
};
pub use tracing::{
    create_span, init_tracing, RequestIdGenerator, RequestSpan, Span, SpanCollector, SpanEvent,
    SpanKind, SpanStatus, SpanValue, Tracer, TracingConfig,
};
