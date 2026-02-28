//! REST API endpoints

use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    version: String,
}

async fn health() -> impl Responder {
    HttpResponse::Ok().json(StatusResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[derive(Deserialize)]
struct SendMessageRequest {
    channel: String,
    recipient: String,
    message: String,
}

async fn send_message(req: web::Json<SendMessageRequest>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message_id": uuid::Uuid::new_v4().to_string()
    }))
}

async fn list_channels() -> impl Responder {
    HttpResponse::Ok().json(vec![
        "whatsapp", "telegram", "discord", "slack", "signal", "imessage",
    ])
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health))
            .route("/channels", web::get().to(list_channels))
            .route("/messages/send", web::post().to(send_message)),
    );
}
