//! Configuration module for dx-style
//!
//! This module provides configuration loading from DX Serializer format files.
//! It supports both the new .sr format and legacy .toml files (via dx-serializer's converter).

pub mod rebuild_config;

#[allow(unused_imports)]
pub use rebuild_config::{RebuildConfig, RebuildConfigBuilder};

use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct PathsConfig {
    pub html_dir: String,
    pub index_file: String,
    pub css_file: String,
    #[serde(default)]
    pub style_dir: Option<String>,
    #[serde(default)]
    pub cache_dir: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WatchConfig {
    pub debounce_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub paths: PathsConfig,
    pub watch: Option<WatchConfig>,
    #[serde(default)]
    pub format: Option<FormatConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FormatConfig {
    #[serde(default = "FormatConfig::default_delay")]
    pub delay_ms: u64,
    #[serde(default = "FormatConfig::default_interval")]
    pub interval_ms: u64,
    #[serde(default)]
    pub force_write: bool,
    #[serde(default = "FormatConfig::default_debounce")]
    pub debounce_ms: u64,
}

impl FormatConfig {
    fn default_delay() -> u64 {
        10_000
    }
    fn default_interval() -> u64 {
        10_000
    }
    fn default_debounce() -> u64 {
        1_000
    }
}

impl Config {
    /// Load configuration from DX Serializer format file
    /// Tries .sr first, then falls back to .toml (via dx-serializer's converter)
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Try .sr format first (new DX Serializer format)
        let sr_path = ".dx/config.sr";
        if std::path::Path::new(sr_path).exists() {
            let content = fs::read_to_string(sr_path)?;
            return Self::from_sr_content(&content);
        }

        // Fall back to .toml format (legacy) using dx-serializer's converter
        let toml_path = ".dx/config.toml";
        if std::path::Path::new(toml_path).exists() {
            let content = fs::read_to_string(toml_path)?;
            // Use dx-serializer's toml_to_dx converter
            let sr_content = serializer::toml_to_dx(&content)
                .map_err(|e| format!("Failed to convert TOML to DX: {}", e))?;
            return Self::from_sr_content(&sr_content);
        }

        Err("No configuration file found (.dx/config.sr or .dx/config.toml)".into())
    }

    /// Parse configuration from DX Serializer format content
    fn from_sr_content(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Parse DX format into a structured config
        // DX format uses key=value syntax with sections
        let mut paths = PathsConfig {
            html_dir: String::new(),
            index_file: String::new(),
            css_file: String::new(),
            style_dir: None,
            cache_dir: None,
        };
        let mut watch = WatchConfig { debounce_ms: None };
        let mut format = FormatConfig {
            delay_ms: FormatConfig::default_delay(),
            interval_ms: FormatConfig::default_interval(),
            force_write: false,
            debounce_ms: FormatConfig::default_debounce(),
        };

        let mut current_section = String::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle section headers
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].trim().trim_matches('"').to_string();
                continue;
            }

            // Handle key=value pairs
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().trim_matches('"');
                let value = line[eq_pos + 1..].trim().trim_matches('"');

                match current_section.as_str() {
                    "paths" => match key {
                        "html_dir" => paths.html_dir = value.to_string(),
                        "index_file" => paths.index_file = value.to_string(),
                        "css_file" => paths.css_file = value.to_string(),
                        "style_dir" => paths.style_dir = Some(value.to_string()),
                        "cache_dir" => paths.cache_dir = Some(value.to_string()),
                        _ => {}
                    },
                    "watch" => {
                        if key == "debounce_ms" {
                            watch.debounce_ms = value.parse().ok();
                        }
                    }
                    "format" => match key {
                        "delay_ms" => {
                            format.delay_ms = value.parse().unwrap_or(FormatConfig::default_delay())
                        }
                        "interval_ms" => {
                            format.interval_ms =
                                value.parse().unwrap_or(FormatConfig::default_interval())
                        }
                        "force_write" => format.force_write = value == "true",
                        "debounce_ms" => {
                            format.debounce_ms =
                                value.parse().unwrap_or(FormatConfig::default_debounce())
                        }
                        _ => {}
                    },
                    _ => {
                        // Handle top-level keys or keys with section prefix
                        if key.starts_with("paths.") || key.contains("|paths|") {
                            let subkey = key
                                .trim_start_matches("paths.")
                                .split('|')
                                .next_back()
                                .unwrap_or(key);
                            match subkey {
                                "html_dir" => paths.html_dir = value.to_string(),
                                "index_file" => paths.index_file = value.to_string(),
                                "css_file" => paths.css_file = value.to_string(),
                                "style_dir" => paths.style_dir = Some(value.to_string()),
                                "cache_dir" => paths.cache_dir = Some(value.to_string()),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        Ok(Config {
            paths,
            watch: Some(watch),
            format: Some(format),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            paths: PathsConfig {
                html_dir: ".".into(),
                index_file: "index.html".into(),
                css_file: "style.css".into(),
                style_dir: Some(".dx/style".into()),
                cache_dir: Some(".dx/cache".into()),
            },
            watch: Some(WatchConfig {
                debounce_ms: Some(250),
            }),
            format: Some(FormatConfig {
                delay_ms: FormatConfig::default_delay(),
                interval_ms: FormatConfig::default_interval(),
                force_write: false,
                debounce_ms: FormatConfig::default_debounce(),
            }),
        }
    }
}

impl Config {
    pub fn resolved_style_dir(&self) -> &str {
        self.paths.style_dir.as_deref().unwrap_or(".dx/style")
    }
    pub fn resolved_cache_dir(&self) -> &str {
        self.paths.cache_dir.as_deref().unwrap_or(".dx/cache")
    }
    pub fn format_delay_ms(&self) -> u64 {
        self.format
            .as_ref()
            .map(|f| f.delay_ms)
            .unwrap_or(FormatConfig::default_delay())
    }
    pub fn format_interval_ms(&self) -> u64 {
        self.format
            .as_ref()
            .map(|f| f.interval_ms)
            .unwrap_or(FormatConfig::default_interval())
    }
    pub fn format_force_write(&self) -> bool {
        self.format.as_ref().map(|f| f.force_write).unwrap_or(false)
    }
    pub fn format_debounce_ms(&self) -> u64 {
        self.format
            .as_ref()
            .map(|f| f.debounce_ms)
            .unwrap_or(FormatConfig::default_debounce())
    }
}
