//! Icon Component System for DX-WWW
//!
//! This module provides compile-time icon resolution for the DX-WWW framework.
//! Icons are resolved at compile time using the dx-icon library, enabling:
//! - Zero runtime overhead for icon loading
//! - Tree-shaking of unused icons
//! - Type-safe icon references
//! - Integration with 297,000+ icons from Iconify and SVGL
//!
//! # Usage
//!
//! ```rust,ignore
//! use dx_www_framework_core::icon;
//!
//! // Basic icon usage
//! let home_icon = icon!("heroicons:home");
//!
//! // With size specification
//! let large_icon = icon!("mdi:home", size = 32);
//!
//! // With color
//! let colored_icon = icon!("lucide:star", color = "#FFD700");
//! ```
//!
//! # Component Syntax
//!
//! In DX-WWW components, icons can be used with the following syntax:
//! ```html
//! <dx-icon name="heroicons:home" />
//! <dx-icon name="mdi:home" size="32" />
//! <dx-icon name="lucide:star" color="#FFD700" />
//! ```
//!
//! # Build-Time Processing
//!
//! The IconProcessor (task 5.2) scans component trees for icon usage and:
//! 1. Extracts all icon references
//! 2. Resolves icons via dx-icon library
//! 3. Inlines optimized SVG at compile time
//! 4. Tree-shakes unused icons from the final bundle

mod component;
mod macros;
mod parser;

#[cfg(test)]
mod tests;

pub use component::IconComponent;
pub use parser::{extract_icon_names, extract_icons_by_set, parse_icon_components};
