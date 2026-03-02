//! Search routes — the main web UI endpoints.
//!
//! Handles homepage rendering, search execution, and result display.
//! Search fan-out is delegated to `SearchOrchestrator`, which uses
//! `FuturesUnordered` for streaming aggregation, a coalescing cache,
//! and per-engine adaptive timeouts with circuit-breaker logic.

use std::sync::Arc;
use std::time::Instant;

use axum::{
    Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use metasearch_core::category::SearchCategory;
use metasearch_core::query::SearchQuery;
use metasearch_core::result::SearchResult;
use serde::{Deserialize, Serialize};
use tera::Context;

use crate::cache::SearchCache;
use crate::state::AppState;

#[derive(Deserialize, Default)]
pub struct SearchParams {
    pub q: Option<String>,
    pub category: Option<String>,
    pub language: Option<String>,
    pub page: Option<u32>,
    pub safe_search: Option<u8>,
    pub time_range: Option<String>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(index))
        .route("/search", get(search))
}

/// Render the homepage.
async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let context = Context::new();
    match state.templates.render("index.html", &context) {
        Ok(html) => Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            Html(format!(
                r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Metasearch</title></head>
                <body style="background:#0a0a1a;color:#fff;font-family:sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;">
                <div style="text-align:center"><h1>Metasearch</h1><p>Template error: {}</p>
                <p>Make sure you're running from the project root directory.</p></div></body></html>"#,
                e
            ))
        }
    }
}

/// Serializable search result for templates.
#[derive(Serialize, Clone)]
struct TemplateResult {
    title: String,
    url: String,
    content: String,
    engine: String,
    thumbnail: Option<String>,
    published_date: Option<String>,
    category: String,
    metadata: serde_json::Value,
}

/// Map string category to SearchCategory enum.
fn parse_category(cat: &str) -> SearchCategory {
    match cat {
        "images" => SearchCategory::Images,
        "videos" => SearchCategory::Videos,
        "news" => SearchCategory::News,
        "science" => SearchCategory::Science,
        "music" => SearchCategory::Music,
        "files" => SearchCategory::Files,
        "it" => SearchCategory::IT,
        "social" => SearchCategory::SocialMedia,
        "maps" => SearchCategory::Maps,
        _ => SearchCategory::General,
    }
}

/// Execute search via the orchestrator and render results.
async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Html<String> {
    let query_text = params.q.clone().unwrap_or_default();
    let category_str = params.category.clone().unwrap_or_else(|| "general".to_string());
    let page = params.page.unwrap_or(1);
    let language = params.language.clone();

    // If no query, render homepage
    if query_text.trim().is_empty() {
        let context = Context::new();
        return match state.templates.render("index.html", &context) {
            Ok(html) => Html(html),
            Err(_) => Html("<meta http-equiv='refresh' content='0;url=/'/>".to_string()),
        };
    }

    let start_time = Instant::now();
    let active_category = parse_category(&category_str);

    // Build search query
    let search_query = SearchQuery {
        query: query_text.clone(),
        categories: vec![active_category],
        language: language.clone(),
        safe_search: params.safe_search.unwrap_or(1),
        page,
        time_range: params.time_range.clone(),
        engines: Vec::new(),
    };

    // Build cache key
    let cache_key = SearchCache::cache_key(
        &query_text,
        &category_str,
        page,
        language.as_deref().unwrap_or("auto"),
    );

    // ── Delegate entirely to the orchestrator ────────────────────────────────
    // orchestrator.search() handles:
    //   1. Cache hit/coalesce → returns Arc<SearchResponse> in < 1 ms
    //   2. Cache miss → FuturesUnordered fan-out, health-tracked, FxHashMap dedup
    let response = state.orchestrator.search(&search_query, &cache_key).await;
    // Heuristic: if search completed faster than 5 ms it was served from cache
    let search_time_ms = start_time.elapsed().as_millis();
    let from_cache = search_time_ms < 5;

    // ── Convert to template format ────────────────────────────────────────────
    let template_results: Vec<TemplateResult> = response
        .results
        .iter()
        .map(result_to_template)
        .collect();

    let number_of_results = template_results.len();
    let total_pages = ((number_of_results as f64) / 10.0).ceil().max(1.0) as u32;

    let mut context = build_context(
        &query_text,
        &category_str,
        page,
        &template_results,
        number_of_results,
        search_time_ms,
        &response.engines_used,
        total_pages,
        &language,
    );
    context.insert("from_cache", &from_cache);
    context.insert("engines_failed", &response.engines_failed);

    match state.templates.render("results.html", &context) {
        Ok(html) => Html(html),
        Err(e) => render_error(&e.to_string()),
    }
}

/// Convert a SearchResult to a TemplateResult.
fn result_to_template(r: &SearchResult) -> TemplateResult {
    TemplateResult {
        title: r.title.clone(),
        url: r.url.clone(),
        content: strip_html_tags(&r.content),
        engine: r.engine.clone(),
        thumbnail: r.thumbnail.clone(),
        published_date: r.published_date.map(|d| d.format("%b %d, %Y").to_string()),
        category: r.category.clone(),
        metadata: r.metadata.clone(),
    }
}

/// Strip HTML tags from a string (Wikipedia snippets etc. contain raw HTML).
fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build the Tera context for the results template.
#[allow(clippy::too_many_arguments)]
fn build_context(
    query: &str,
    category: &str,
    page: u32,
    results: &[TemplateResult],
    number_of_results: usize,
    search_time_ms: u128,
    engines_used: &[String],
    total_pages: u32,
    language: &Option<String>,
) -> Context {
    let mut context = Context::new();
    context.insert("query", query);
    context.insert("category", category);
    context.insert("page", &page);
    context.insert("results", results);
    context.insert("number_of_results", &number_of_results);
    context.insert("search_time_ms", &(search_time_ms as u64));
    context.insert("engines_used", engines_used);
    context.insert("total_pages", &total_pages);
    context.insert("safe_search", &"Moderate");
    context.insert(
        "language",
        &language.clone().unwrap_or_else(|| "Auto".to_string()),
    );
    context
}

/// Render an error page.
fn render_error(msg: &str) -> Html<String> {
    tracing::error!("Template error: {}", msg);
    Html(format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Error</title></head>
        <body style="background:#0a0a1a;color:#fff;font-family:sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;">
        <div style="text-align:center"><h1>Search Error</h1><p>{}</p>
        <a href="/" style="color:#3b82f6;">Back to Home</a></div></body></html>"#,
        msg
    ))
}
