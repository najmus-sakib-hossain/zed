//! CSS Property Registry Module
//!
//! This module provides a comprehensive database of all CSS properties loaded from
//! DX Serializer format. It enables native CSS property support where any CSS property
//! can be used directly as a class name.
//!
//! ## Features
//!
//! - Complete CSS property database from CSS specification
//! - Property-value pattern matching (e.g., `display-flex`)
//! - Numeric value parsing with units (e.g., `width-100px`)
//! - CSS custom property support (e.g., `--my-var-blue`)
//! - Unknown property warning with fail-open behavior
//!
//! ## Requirements Validated
//!
//! - 1.1: CSS_Property_Registry contains all standard CSS properties
//! - 1.2: Class name matching for CSS property-value patterns
//! - 1.4: CSS values with units support
//! - 1.5: CSS custom properties support
//! - 1.6: Unknown property warning with CSS generation
//! - 1.7: DX Serializer format storage
//! - 1.8: .human file for debugging
//! - 6.1-6.5: CSS property database generation

#![allow(dead_code)]

mod database;
mod generator;
mod registry;

// Re-exports are intentionally not used externally yet - this is infrastructure code
#[allow(unused_imports)]
pub use database::*;
#[allow(unused_imports)]
pub use generator::*;
#[allow(unused_imports)]
pub use registry::*;
