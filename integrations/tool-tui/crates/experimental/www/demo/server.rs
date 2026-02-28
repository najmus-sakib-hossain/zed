mod htip_generator;

use axum::{Router, extract::Path, http::StatusCode, response::IntoResponse, routing::get};
use std::net::SocketAddr;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/todo", get(todo_app))
        .route("/demo/:name", get(demo_page))
        .route("/htip/:name", get(serve_htip))
        .route("/dx_www_client.wasm", get(serve_wasm))
        .route("/styles.binary", get(serve_binary_css))
        .nest_service("/static", ServeDir::new("demo"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("DX-WWW Demo Server");
    println!("Listening on http://{}", addr);
    println!();
    println!("Available demos:");
    println!("  http://localhost:3000/         - Main demo");
    println!("  http://localhost:3000/todo     - Todo app (full features)");
    println!("  http://localhost:3000/demo/counter");
    println!("  http://localhost:3000/demo/todo");
    println!("  http://localhost:3000/demo/dashboard");
    println!();
    println!("Optimized for 100/100/100/100 Lighthouse scores");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> impl IntoResponse {
    let html = include_str!("demo_full.html");

    (
        StatusCode::OK,
        [
            ("content-type", "text/html; charset=utf-8"),
            ("cache-control", "public, max-age=3600"),
            ("x-content-type-options", "nosniff"),
            ("x-frame-options", "DENY"),
            ("x-xss-protection", "1; mode=block"),
            ("referrer-policy", "strict-origin-when-cross-origin"),
        ],
        html,
    )
}

async fn todo_app() -> impl IntoResponse {
    let html = include_str!("todo.html");

    (
        StatusCode::OK,
        [
            ("content-type", "text/html; charset=utf-8"),
            ("cache-control", "public, max-age=3600"),
            ("x-content-type-options", "nosniff"),
            ("x-frame-options", "DENY"),
            ("x-xss-protection", "1; mode=block"),
            ("referrer-policy", "strict-origin-when-cross-origin"),
        ],
        html,
    )
}

async fn demo_page(Path(name): Path<String>) -> impl IntoResponse {
    let html = match name.as_str() {
        "counter" | "todo" | "dashboard" => include_str!("demo_full.html"),
        _ => return (StatusCode::NOT_FOUND, [("content-type", "text/html")], "Demo not found"),
    };

    (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], html)
}

async fn serve_htip(Path(name): Path<String>) -> impl IntoResponse {
    let htip_data = match name.as_str() {
        "counter" => htip_generator::generate_counter_htip(),
        "todo" => htip_generator::generate_todo_htip(),
        "dashboard" => htip_generator::generate_dashboard_htip(),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                [
                    ("content-type", "text/plain"),
                    ("cache-control", "no-cache"),
                    ("x-content-type-options", "nosniff"),
                ],
                vec![],
            );
        }
    };

    (
        StatusCode::OK,
        [
            ("content-type", "application/octet-stream"),
            ("cache-control", "public, max-age=31536000, immutable"),
            ("x-content-type-options", "nosniff"),
        ],
        htip_data,
    )
}

async fn serve_wasm() -> impl IntoResponse {
    let wasm_bytes = std::fs::read("crates/www/demo/dx_www_client.wasm").unwrap_or_else(|_| vec![]);

    (
        StatusCode::OK,
        [
            ("content-type", "application/wasm"),
            ("cache-control", "public, max-age=31536000, immutable"),
            ("x-content-type-options", "nosniff"),
        ],
        wasm_bytes,
    )
}

async fn serve_binary_css() -> impl IntoResponse {
    // Development: Use DX Serializer format (1.8KB, fast round-trip)
    // Production: Use DXOB format (511B compressed, 40% smaller)
    let css_bytes = std::fs::read("styles.binary").unwrap_or_else(|e| {
        eprintln!("Failed to read binary CSS: {}", e);
        vec![]
    });

    (
        StatusCode::OK,
        [
            ("content-type", "application/octet-stream"),
            ("cache-control", "no-cache, no-store, must-revalidate"),
            ("x-content-type-options", "nosniff"),
        ],
        css_bytes,
    )
}
