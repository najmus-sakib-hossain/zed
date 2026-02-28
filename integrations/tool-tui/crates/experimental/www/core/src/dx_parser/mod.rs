//! # DX Format Parsers
//!
//! Parsers for DX-WWW file formats (.pg, .cp, .lyt, dx config)
//!
//! ## File Formats
//!
//! - `.pg` - Page files (route handlers with template and logic)
//! - `.cp` - Component files (reusable UI components)
//! - `.lyt` - Layout files (page wrappers with slots)
//! - `dx` - Configuration file (no extension)
//!
//! ## Example
//!
//! ```rust,ignore
//! use dx_core::dx_parser::{parse_dx_file, BlockType};
//!
//! let source = std::fs::read_to_string("pages/index.pg")?;
//! let ast = parse_dx_file(&source, Some(BlockType::Page))?;
//!
//! for class in &ast.css_classes {
//!     println!("CSS class: {}", class);
//! }
//! ```

pub mod dx_format;

pub use dx_format::*;
