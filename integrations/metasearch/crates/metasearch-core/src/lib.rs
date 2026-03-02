//! # metasearch-core
//!
//! Core types, traits, and result models for the metasearch engine.
//! This crate is engine-agnostic and defines the shared contract
//! that all search engines must implement.

pub mod category;
pub mod config;
pub mod engine;
pub mod error;
pub mod query;
pub mod ranking;
pub mod result;

pub use config::Settings;
pub use engine::{EngineMetadata, SearchEngine};
pub use error::MetasearchError;
pub use query::SearchQuery;
pub use result::SearchResult;
