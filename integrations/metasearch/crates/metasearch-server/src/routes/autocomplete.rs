//! Autocomplete / search-suggestion endpoint.
//!
//! GET /autocomplete?q=<query>
//!
//! Returns a JSON tuple `[query, [suggestion, …]]` compatible with
//! the OpenSearch Suggestions extension and metasearch2's autocomplete
//! format so the browser autocomplete widget "just works".
//!
//! Suggestions are fetched from Google's public Firefox-autocomplete API.

use std::{collections::HashMap, sync::Arc};

use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
};
use reqwest::Client;
use serde_json::Value;
use tracing::error;

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/autocomplete", get(autocomplete_handler))
}

async fn autocomplete_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let query = params
        .get("q")
        .cloned()
        .unwrap_or_default()
        .replace('\n', " ");

    if query.trim().is_empty() {
        return (StatusCode::OK, Json(serde_json::json!([query, []]))).into_response();
    }

    match fetch_suggestions(&state.http_client, &query).await {
        Ok(suggestions) => {
            (StatusCode::OK, Json(serde_json::json!([query, suggestions]))).into_response()
        }
        Err(e) => {
            error!("Autocomplete fetch failed for {:?}: {}", query, e);
            (StatusCode::OK, Json(serde_json::json!([query, []]))).into_response()
        }
    }
}

/// Fetch suggestions from Google's Firefox autocomplete endpoint.
async fn fetch_suggestions(client: &Client, query: &str) -> anyhow::Result<Vec<String>> {
    let url = reqwest::Url::parse_with_params(
        "https://suggestqueries.google.com/complete/search",
        &[
            ("output", "firefox"),
            ("client", "firefox"),
            ("hl", "US-en"),
            ("q", query),
        ],
    )?;

    let body = client
        .get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (compatible; Metasearch/1.0; +https://github.com/metasearch)",
        )
        .send()
        .await?
        .text()
        .await?;

    // Response format: ["query", ["suggestion1", "suggestion2", ...]]
    let parsed: Vec<Value> = serde_json::from_str(&body)?;
    let suggestions = parsed
        .into_iter()
        .nth(1)
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    Ok(suggestions)
}
