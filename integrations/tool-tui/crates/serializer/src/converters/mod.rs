//! Converters from other formats to DX format
//!
//! Supports: JSON, YAML, TOON, TOML → DX
//!
//! All converters apply optimization automatically:
//! - Abbreviated keys (name → n, version → v)
//! - Minimal prefixes (context → c, media → m)
//! - Inline chaining with ^
//! - Compact arrays with |
//! - 2-letter language codes

// JSON, YAML, TOML converters require the converters feature (serde dependencies)
#[cfg(feature = "converters")]
pub mod json;
#[cfg(feature = "converters")]
pub mod toml;
#[cfg(feature = "converters")]
pub mod yaml;

// TOON converter has no external dependencies
pub mod toon;

// Property tests for converters require the converters feature
#[cfg(all(test, feature = "converters"))]
mod converter_props;

#[cfg(feature = "converters")]
pub use json::json_to_dx;
#[cfg(feature = "converters")]
pub use toml::toml_to_dx;
pub use toon::{dx_to_toon, toon_to_dx};
#[cfg(feature = "converters")]
pub use yaml::yaml_to_dx;

/// Common converter trait
pub trait ToDx {
    fn to_dx(&self) -> Result<String, String>;
}

/// Convert any supported format to DX
#[cfg(feature = "converters")]
pub fn convert_to_dx(input: &str, format: &str) -> Result<String, String> {
    match format.to_lowercase().as_str() {
        "json" => json_to_dx(input),
        "yaml" | "yml" => yaml_to_dx(input),
        "toon" => toon_to_dx(input),
        "toml" => toml_to_dx(input),
        _ => Err(format!("Unsupported format: {}", format)),
    }
}

/// Convert any supported format to DX (minimal version without converters feature)
#[cfg(not(feature = "converters"))]
pub fn convert_to_dx(input: &str, format: &str) -> Result<String, String> {
    match format.to_lowercase().as_str() {
        "toon" => toon_to_dx(input),
        _ => Err(format!(
            "Format '{}' requires the 'converters' feature. Only 'toon' is available without it.",
            format
        )),
    }
}
