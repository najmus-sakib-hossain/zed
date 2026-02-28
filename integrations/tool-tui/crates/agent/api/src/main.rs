use axum::{routing::get, Json, Router};
use dx_core::models::User;
use tracing::info;

async fn health() -> &'static str {
    "OK"
}

async fn get_user() -> Json<User> {
    Json(User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/user", get(get_user));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    info!("Server running on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
