//! Application builder and startup.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::info;

use crate::routes;
use crate::state::AppState;

/// Build the Axum router with all routes and middleware.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(routes::search_routes())
        .merge(routes::api_routes())
        .merge(routes::autocomplete_routes())
        .merge(routes::opensearch_routes())
        .merge(routes::static_routes())
        .merge(routes::health_routes())
        .layer(
            ServiceBuilder::new()
                // Inbound: stamp a UUID request-id on every request
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                // Structured trace spans per request
                .layer(TraceLayer::new_for_http())
                // Outbound: propagate the request-id to the response headers
                .layer(PropagateRequestIdLayer::x_request_id())
                // Response body gzip compression
                .layer(CompressionLayer::new())
                // Permissive CORS (lock down in production via config)
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

/// Start the server.
pub async fn run(state: Arc<AppState>) -> anyhow::Result<()> {
    let addr = SocketAddr::from((
        state.settings.server.host.parse::<std::net::IpAddr>()?,
        state.settings.server.port,
    ));

    // Display localhost instead of 0.0.0.0 for better UX
    let display_host = if state.settings.server.host == "0.0.0.0" {
        "localhost"
    } else {
        &state.settings.server.host
    };
    info!("Metasearch server listening on http://{}:{}", display_host, state.settings.server.port);

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
