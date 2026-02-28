//! # DX Option - Official DX Website
//!
//! Binary-first, zero-parse web application showcasing the DX framework.
//!
//! ## Features
//! - Binary Dawn Animations (20x faster)
//! - 3D WebGL Environment
//! - Binary Forms (10x faster validation)
//! - TailwindCSS-style Query Builder
//! - Turso Database Integration

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub mod animation;
pub mod components;
pub mod db;
pub mod forms;
pub mod pages;
pub mod query_builder;
pub mod state;
pub mod three_d;

// Re-exports
pub use animation::BinaryDawnAnimation;
pub use db::TursoConnection;
pub use forms::BinaryForm;
pub use query_builder::TailwindQuery;
pub use state::AppState;
