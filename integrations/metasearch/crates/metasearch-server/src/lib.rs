//! # metasearch-server
//!
//! The Axum-based HTTP server that powers the metasearch web UI and API.

pub mod app;
pub mod cache;
pub mod health;
pub mod middleware;
pub mod orchestrator;
pub mod routes;
pub mod state;
pub mod templates;
