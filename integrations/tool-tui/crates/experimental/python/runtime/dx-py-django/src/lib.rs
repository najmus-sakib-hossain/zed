//! DX-Py Django Compatibility Layer
//!
//! This crate provides compatibility implementations for Django-required
//! C extensions, including:
//! - JSON parsing (cjson/ujson compatibility)
//! - Password hashing (bcrypt, argon2)
//! - Database adapters (SQLite3, psycopg2)
//! - Template engines (Jinja2/MarkupSafe, Django templates)

pub mod database;
pub mod json;
pub mod password;
pub mod template;

pub use database::{DatabaseAdapter, DatabaseError, PostgresAdapter, SqliteAdapter};
pub use json::{JsonError, JsonParser};
pub use password::{HashAlgorithm, PasswordError, PasswordHasher};
pub use template::{
    escape_html, CompiledTemplate, Context, ContextValue, Markup, TemplateCompiler, TemplateError,
    TemplateRenderer,
};
