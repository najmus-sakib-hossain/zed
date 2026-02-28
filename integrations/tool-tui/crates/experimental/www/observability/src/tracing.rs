//! Distributed tracing module with OpenTelemetry integration.
//!
//! This module provides:
//! - OpenTelemetry SDK initialization with OTLP exporter
//! - Configurable sampling rates
//! - Tower middleware layer for automatic request tracing
//! - Trace context propagation through async operations
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_www_observability::{ObservabilityConfig, tracing::init_tracing};
//!
//! let config = ObservabilityConfig::new("my-service")
//!     .with_otlp_endpoint("http://localhost:4317")
//!     .with_sampling_rate(0.5);
//!
//! init_tracing(&config).expect("Failed to initialize tracing");
//!
//! // Use tracing macros as normal
//! tracing::info!("Application started");
//!
//! // On shutdown
//! dx_www_observability::tracing::shutdown_tracing();
//! ```

use crate::ObservabilityConfig;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource, runtime,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
};
use std::sync::atomic::{AtomicBool, Ordering};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Flag to track if tracing has been initialized.
static TRACING_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Errors that can occur during tracing initialization.
#[derive(Debug, thiserror::Error)]
pub enum TracingError {
    /// Failed to initialize the OpenTelemetry tracer.
    #[error("Failed to initialize OpenTelemetry tracer: {0}")]
    InitError(String),

    /// Failed to create OTLP exporter.
    #[error("Failed to create OTLP exporter: {0}")]
    ExporterError(String),

    /// Tracing has already been initialized.
    #[error("Tracing has already been initialized")]
    AlreadyInitialized,

    /// Failed to set global subscriber.
    #[error("Failed to set global subscriber: {0}")]
    SubscriberError(String),

    /// Failed to parse log directive.
    #[error("Failed to parse log directive: {0}")]
    DirectiveError(String),
}

/// Initializes the tracing subsystem with OpenTelemetry.
///
/// This function sets up:
/// - OpenTelemetry SDK with OTLP exporter (if endpoint is configured)
/// - Configurable trace sampling based on `config.sampling_rate`
/// - Trace context propagation for distributed tracing
/// - Integration with the `tracing` crate for seamless instrumentation
///
/// # Arguments
///
/// * `config` - The observability configuration containing tracing settings.
///
/// # Errors
///
/// Returns a `TracingError` if:
/// - Tracing has already been initialized
/// - Failed to create the OTLP exporter
/// - Failed to set the global subscriber
///
/// # Example
///
/// ```rust,ignore
/// use dx_www_observability::{ObservabilityConfig, tracing::init_tracing};
///
/// let config = ObservabilityConfig::new("my-service")
///     .with_otlp_endpoint("http://localhost:4317")
///     .with_sampling_rate(1.0); // Sample all traces
///
/// init_tracing(&config)?;
/// ```
pub fn init_tracing(config: &ObservabilityConfig) -> Result<(), TracingError> {
    // Check if already initialized
    if TRACING_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(TracingError::AlreadyInitialized);
    }

    // Create the base env filter for log levels
    let hyper_directive =
        "hyper=warn".parse().map_err(|e| TracingError::DirectiveError(format!("{e}")))?;
    let tower_directive = "tower_http=debug"
        .parse()
        .map_err(|e| TracingError::DirectiveError(format!("{e}")))?;

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive(hyper_directive)
        .add_directive(tower_directive);

    // Build the subscriber based on whether OTLP is configured
    if let Some(ref endpoint) = config.otlp_endpoint {
        // Initialize OpenTelemetry with OTLP exporter
        let tracer_provider = init_otlp_tracer(config, endpoint)?;

        // Set the global tracer provider for shutdown coordination
        global::set_tracer_provider(tracer_provider.clone());

        // Create the OpenTelemetry layer
        let tracer = tracer_provider.tracer(config.service_name.clone());
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        // Build the full subscriber with OpenTelemetry
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .with(otel_layer)
            .try_init()
            .map_err(|e| TracingError::SubscriberError(e.to_string()))?;
    } else {
        // No OTLP endpoint - just use standard tracing subscriber
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init()
            .map_err(|e| TracingError::SubscriberError(e.to_string()))?;
    }

    tracing::info!(
        service_name = %config.service_name,
        sampling_rate = %config.sampling_rate,
        otlp_enabled = config.otlp_endpoint.is_some(),
        "Tracing initialized"
    );

    Ok(())
}

/// Initializes the OpenTelemetry tracer with OTLP exporter.
fn init_otlp_tracer(
    config: &ObservabilityConfig,
    endpoint: &str,
) -> Result<TracerProvider, TracingError> {
    // Configure the sampler based on sampling rate
    let sampler = create_sampler(config.sampling_rate);

    // Create the resource with service information
    let resource = Resource::new(vec![opentelemetry::KeyValue::new(
        "service.name",
        config.service_name.clone(),
    )]);

    // Build the span exporter directly from the OTLP exporter
    let span_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .build_span_exporter()
        .map_err(|e| TracingError::ExporterError(e.to_string()))?;

    // Build the tracer provider with batch span processor
    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(span_exporter, runtime::Tokio)
        .with_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(sampler)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource),
        )
        .build();

    Ok(tracer_provider)
}

/// Creates a sampler based on the configured sampling rate.
///
/// - Rate of 0.0: Never sample (`AlwaysOff`)
/// - Rate of 1.0: Always sample (`AlwaysOn`)
/// - Rate between 0.0 and 1.0: Probabilistic sampling
fn create_sampler(sampling_rate: f64) -> Sampler {
    if sampling_rate <= 0.0 {
        Sampler::AlwaysOff
    } else if sampling_rate >= 1.0 {
        Sampler::AlwaysOn
    } else {
        Sampler::TraceIdRatioBased(sampling_rate)
    }
}

/// Shuts down the tracing subsystem gracefully.
///
/// This function:
/// - Flushes any pending spans to the exporter
/// - Shuts down the OpenTelemetry provider
///
/// Should be called during application shutdown to ensure all traces are exported.
///
/// # Example
///
/// ```rust,ignore
/// // During application shutdown
/// dx_www_observability::tracing::shutdown_tracing();
/// ```
pub fn shutdown_tracing() {
    // Use the global shutdown which handles flushing and cleanup
    global::shutdown_tracer_provider();
    tracing::info!("Tracing shutdown complete");
}

/// Creates a Tower middleware layer for automatic HTTP request tracing.
///
/// This layer:
/// - Creates a span for each incoming HTTP request
/// - Records request metadata (method, URI, version)
/// - Records response metadata (status code, latency)
/// - Propagates trace context through the request lifecycle
///
/// # Returns
///
/// A `TraceLayer` that can be added to a Tower/Axum service stack.
///
/// # Example
///
/// ```rust,ignore
/// use axum::{Router, routing::get};
/// use dx_www_observability::tracing::tracing_layer;
///
/// let app = Router::new()
///     .route("/", get(handler))
///     .layer(tracing_layer());
/// ```
#[must_use]
pub fn tracing_layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
> {
    TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO))
}

/// Creates a Tower middleware layer with custom span configuration.
///
/// This variant allows customizing the span level and what information is recorded.
///
/// # Arguments
///
/// * `level` - The tracing level for request/response events
///
/// # Returns
///
/// A configured `TraceLayer` for HTTP request tracing.
///
/// # Example
///
/// ```rust,ignore
/// use tracing::Level;
/// use dx_www_observability::tracing::tracing_layer_with_level;
///
/// // Use DEBUG level for more verbose tracing
/// let layer = tracing_layer_with_level(Level::DEBUG);
/// ```
#[must_use]
pub fn tracing_layer_with_level(
    level: Level,
) -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
> {
    TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(level))
        .on_request(DefaultOnRequest::new().level(level))
        .on_response(DefaultOnResponse::new().level(level))
}

/// Configuration for request tracing spans.
///
/// This struct allows fine-grained control over what information is captured
/// in request tracing spans.
#[derive(Debug, Clone)]
pub struct TracingLayerConfig {
    /// The tracing level for span events.
    pub level: Level,
    /// Whether to include request headers in the span.
    pub include_headers: bool,
    /// Whether to record the request body size.
    pub record_body_size: bool,
}

impl Default for TracingLayerConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            include_headers: false,
            record_body_size: true,
        }
    }
}

impl TracingLayerConfig {
    /// Creates a new `TracingLayerConfig` with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the tracing level.
    #[must_use]
    pub const fn with_level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Enables or disables header recording.
    #[must_use]
    pub const fn with_headers(mut self, include: bool) -> Self {
        self.include_headers = include;
        self
    }

    /// Enables or disables body size recording.
    #[must_use]
    pub const fn with_body_size(mut self, record: bool) -> Self {
        self.record_body_size = record;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sampler_always_off() {
        let sampler = create_sampler(0.0);
        assert!(matches!(sampler, Sampler::AlwaysOff));
    }

    #[test]
    fn test_create_sampler_always_on() {
        let sampler = create_sampler(1.0);
        assert!(matches!(sampler, Sampler::AlwaysOn));
    }

    #[test]
    fn test_create_sampler_probabilistic() {
        let sampler = create_sampler(0.5);
        assert!(matches!(sampler, Sampler::TraceIdRatioBased(_)));
    }

    #[test]
    fn test_create_sampler_negative_rate() {
        // Negative rates should be treated as AlwaysOff
        let sampler = create_sampler(-0.1);
        assert!(matches!(sampler, Sampler::AlwaysOff));
    }

    #[test]
    fn test_create_sampler_rate_above_one() {
        // Rates above 1.0 should be treated as AlwaysOn
        let sampler = create_sampler(1.5);
        assert!(matches!(sampler, Sampler::AlwaysOn));
    }

    #[test]
    fn test_tracing_layer_config_default() {
        let config = TracingLayerConfig::default();
        assert_eq!(config.level, Level::INFO);
        assert!(!config.include_headers);
        assert!(config.record_body_size);
    }

    #[test]
    fn test_tracing_layer_config_builder() {
        let config = TracingLayerConfig::new()
            .with_level(Level::DEBUG)
            .with_headers(true)
            .with_body_size(false);

        assert_eq!(config.level, Level::DEBUG);
        assert!(config.include_headers);
        assert!(!config.record_body_size);
    }

    #[test]
    fn test_tracing_layer_creation() {
        // Verify that tracing_layer() returns a valid layer
        let _layer = tracing_layer();
    }

    #[test]
    fn test_tracing_layer_with_level_creation() {
        // Verify that tracing_layer_with_level() returns a valid layer
        let _layer = tracing_layer_with_level(Level::DEBUG);
    }
}
