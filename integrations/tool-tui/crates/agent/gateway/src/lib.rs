//! # DX Agent Gateway
//!
//! Production-ready WebSocket gateway server built on axum + tokio-tungstenite.
//! Provides session management, authentication, rate limiting, health checks,
//! and structured logging.
//!
//! ## Features
//! - WebSocket server with 150k-250k msg/sec throughput
//! - SQLite-backed session persistence
//! - JWT token authentication
//! - TOML-based configuration
//! - Health check endpoints
//! - Structured logging with file rotation

pub mod audit;
pub mod auth;
pub mod config;
pub mod health;
pub mod logging;
pub mod rate_limiter;
pub mod secrets;
pub mod server;
pub mod session_store;
pub mod web;
pub mod ws;

pub use audit::AuditLogger;
pub use config::GatewayConfig;
pub use rate_limiter::RateLimiter;
pub use secrets::SecretStore;
pub use server::GatewayServer;
pub use session_store::SessionStore;
