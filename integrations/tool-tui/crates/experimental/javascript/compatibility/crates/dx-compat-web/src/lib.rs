//! # dx-compat-web
//!
//! Web Standard APIs compatibility layer implementing WHATWG and W3C specifications.
//!
//! ## Modules
//!
//! - `fetch` - Fetch API implementation
//! - `streams` - WHATWG Streams API
//! - `websocket` - WebSocket API
//! - `url` - URL API

#![warn(missing_docs)]

pub mod fetch;
pub mod streams;
pub mod url;
pub mod websocket;

/// Common error types for Web compatibility.
pub mod error;

pub use error::WebError;
