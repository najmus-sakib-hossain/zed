#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single loaded icon with its SVG body and metadata
#[derive(Debug, Clone)]
pub struct LoadedIcon {
    /// Unique identifier (pack:name format)
    pub id: String,
    /// Display name of the icon
    pub name: String,
    /// The icon pack this belongs to
    pub pack: String,
    /// Source of the icon data
    pub source: IconSource,
    /// SVG body content (the inner path/shape data, not full SVG)
    pub svg_body: String,
    /// Default width (from iconify viewBox)
    pub width: f32,
    /// Default height (from iconify viewBox)
    pub height: f32,
}

impl LoadedIcon {
    /// Build a full SVG string for rendering
    pub fn to_svg_string(&self) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#,
            self.width, self.height, self.width, self.height, self.svg_body
        )
    }
}

/// Where the icon data was loaded from
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IconSource {
    /// Iconify JSON pack from apps/www/public/icons
    WwwIcons,
    /// SVG files from apps/www/public/svgl
    WwwSvgl,
    /// Iconify JSON pack from crates/icon/data
    CrateData,
}

impl std::fmt::Display for IconSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IconSource::WwwIcons => write!(f, "www/icons"),
            IconSource::WwwSvgl => write!(f, "www/svgl"),
            IconSource::CrateData => write!(f, "crate/data"),
        }
    }
}

/// Summary of a loaded icon pack
#[derive(Debug, Clone)]
pub struct IconPackInfo {
    pub prefix: String,
    pub name: String,
    pub total: u32,
    pub source: IconSource,
}

/// Iconify JSON pack format (matches crates/icon/src/types.rs)
#[derive(Deserialize, Serialize, Debug)]
pub struct IconifyPack {
    pub prefix: String,
    #[serde(default)]
    pub info: Option<IconifyPackInfo>,
    pub icons: HashMap<String, IconifyIconData>,
    #[serde(default)]
    pub width: Option<f32>,
    #[serde(default)]
    pub height: Option<f32>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IconifyPackInfo {
    pub name: String,
    #[serde(default)]
    pub total: u32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IconifyIconData {
    pub body: String,
    #[serde(default)]
    pub width: Option<f32>,
    #[serde(default)]
    pub height: Option<f32>,
}
