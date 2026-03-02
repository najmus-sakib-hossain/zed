//! JSON API routes for programmatic access.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ApiSearchParams {
    pub q: String,
    pub format: Option<String>,
    pub categories: Option<String>,
    pub language: Option<String>,
    pub page: Option<u32>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/search", get(api_search))
        .route("/api/v1/engines", get(api_engines))
        .route("/api/v1/config", get(api_config))
}

async fn api_search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ApiSearchParams>,
) -> Json<serde_json::Value> {
    use metasearch_core::category::SearchCategory;
    use metasearch_core::query::SearchQuery;
    use crate::cache::SearchCache;

    let category = match params.categories.as_deref() {
        Some("images") => SearchCategory::Images,
        Some("videos") => SearchCategory::Videos,
        Some("news") => SearchCategory::News,
        Some("science") => SearchCategory::Science,
        Some("music") => SearchCategory::Music,
        Some("files") => SearchCategory::Files,
        Some("it") => SearchCategory::IT,
        Some("social") => SearchCategory::SocialMedia,
        Some("maps") => SearchCategory::Maps,
        _ => SearchCategory::General,
    };

    let search_query = SearchQuery {
        query: params.q.clone(),
        categories: vec![category],
        language: params.language.clone(),
        safe_search: 1,
        page: params.page.unwrap_or(1),
        time_range: None,
        engines: Vec::new(),
    };

    let cache_key = SearchCache::cache_key(
        &params.q,
        params.categories.as_deref().unwrap_or("general"),
        params.page.unwrap_or(1),
        params.language.as_deref().unwrap_or("auto"),
    );

    let response = state.orchestrator.search(&search_query, &cache_key).await;

    Json(serde_json::json!({
        "query": response.query,
        "results": response.results,
        "number_of_results": response.number_of_results,
        "engines_used": response.engines_used,
        "engines_failed": response.engines_failed,
        "search_time_ms": response.search_time_ms
    }))
}

async fn api_engines(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let names = state.engine_registry.engine_names();
    Json(serde_json::json!({ "engines": names }))
}

async fn api_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "safe_search": state.settings.search.safe_search,
        "default_language": state.settings.search.default_language,
        "image_proxy": state.settings.server.image_proxy,
    }))
}
