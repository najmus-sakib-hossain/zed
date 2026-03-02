//! Metasearch CLI — entry point for the application.

// mimalloc: 2-6x faster than system allocator, critical on musl targets.
// Works on all platforms (Windows, Linux, macOS, Alpine/musl).
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::sync::Arc;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use metasearch_core::config::Settings;
use metasearch_engine::EngineRegistry;
use metasearch_server::{
    app,
    cache::SearchCache,
    health::EngineHealthTracker,
    orchestrator::SearchOrchestrator,
    state::AppState,
    templates::Templates,
};

#[derive(Parser)]
#[command(name = "metasearch")]
#[command(about = "A blazing-fast, privacy-respecting metasearch engine")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8888)]
    port: u16,

    /// Path to templates directory
    #[arg(long, default_value = "templates")]
    templates: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server (default)
    Serve,
    /// List all registered engines
    Engines,
    /// Print the default configuration
    Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("metasearch=info".parse()?))
        .init();

    let cli = Cli::parse();

    // Build settings
    let mut settings = Settings::default();
    settings.server.host = cli.host;
    settings.server.port = cli.port;

    // Build optimized HTTP client with connection pooling
    let http_client = reqwest::Client::builder()
        .user_agent("Metasearch/0.1 (https://github.com/najmus-sakib-hossain/metasearch)")
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(3))
        .pool_max_idle_per_host(50)  // More connections per host
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .tcp_nodelay(true)  // Disable Nagle's algorithm for lower latency
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .http2_adaptive_window(true)  // Better HTTP/2 performance
        .http2_keep_alive_interval(std::time::Duration::from_secs(30))
        .http2_keep_alive_timeout(std::time::Duration::from_secs(10))
        .http2_keep_alive_while_idle(true)
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()?;

    // Register ALL engines using with_defaults() — 200+ engines across all categories
    // Clone the client so we keep one for the registry and one for general use (autocomplete, etc.)
    let shared_client = http_client.clone();
    let registry = EngineRegistry::with_defaults(http_client);
    let engine_count = registry.count();
    let registry = Arc::new(registry);

    // Load templates
    let templates = Templates::new(&cli.templates)?;

    // Build the performance stack
    let cache = SearchCache::new(settings.cache.max_entries, settings.cache.ttl_secs);
    let health = Arc::new(EngineHealthTracker::new());
    let max_engines = settings.search.max_concurrent_engines;
    let orchestrator = Arc::new(SearchOrchestrator::new(
        Arc::clone(&registry),
        cache.clone(),
        Arc::clone(&health),
        max_engines,
    ));

    let state = Arc::new(AppState {
        cache,
        engine_registry: registry,
        templates: Arc::new(templates),
        orchestrator,
        health,
        settings,
        http_client: shared_client,
    });

    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Serve => {
            tracing::info!("Registered {} search engines", engine_count);
            app::run(state).await?;
        }
        Commands::Engines => {
            println!("Registered engines ({}):", engine_count);
            let mut names = state.engine_registry.engine_names();
            names.sort();
            for name in names {
                println!("  - {}", name);
            }
        }
        Commands::Config => {
            println!("{}", serde_json::to_string_pretty(&state.settings)?);
        }
    }

    Ok(())
}
