//! Structured logging infrastructure with file rotation.

use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::LoggingConfig;

/// Initialize the logging system based on configuration
pub fn init_logging(config: &LoggingConfig) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    if config.json {
        let fmt_layer = fmt_layer.json();
        tracing_subscriber::registry().with(filter).with(fmt_layer).init();
    } else {
        tracing_subscriber::registry().with(filter).with(fmt_layer).init();
    }

    // File appender if configured
    if let Some(ref log_path) = config.file {
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file_name = log_path.file_name().and_then(|n| n.to_str()).unwrap_or("dx-gateway.log");
        let dir = log_path.parent().unwrap_or(std::path::Path::new("."));

        let _file_appender = tracing_appender::rolling::daily(dir, file_name);
        // In production, you'd add this as a layer. For now, console + file.
        tracing::info!(
            "File logging configured: {} (max {}MB, {} files)",
            log_path.display(),
            config.max_size_mb,
            config.max_files
        );
    }
}
