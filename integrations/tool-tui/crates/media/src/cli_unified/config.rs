//! Configuration management for media CLI
//!
//! Reads from `dx` config file (no extension) using dx-serializer format

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaConfig {
    /// Base output directory for all downloads
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,

    /// Media-specific output directory
    #[serde(default)]
    pub media_dir: Option<PathBuf>,

    /// Icon-specific output directory
    #[serde(default)]
    pub icon_dir: Option<PathBuf>,

    /// Font-specific output directory
    #[serde(default)]
    pub font_dir: Option<PathBuf>,

    /// Archive output directory
    #[serde(default)]
    pub archive_dir: Option<PathBuf>,

    /// Cache directory for temporary files
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,

    /// Default media provider
    #[serde(default)]
    pub default_media_provider: Option<String>,

    /// Default font provider
    #[serde(default = "default_font_provider")]
    pub default_font_provider: String,

    /// Default font formats
    #[serde(default = "default_font_formats")]
    pub font_formats: Vec<String>,

    /// Default font subsets
    #[serde(default = "default_font_subsets")]
    pub font_subsets: Vec<String>,

    /// Auto-create directories
    #[serde(default = "default_true")]
    pub auto_create_dirs: bool,

    /// Organize downloads by date
    #[serde(default)]
    pub organize_by_date: bool,

    /// Organize downloads by type
    #[serde(default = "default_true")]
    pub organize_by_type: bool,
}

fn default_output_dir() -> PathBuf {
    PathBuf::from("./downloads")
}

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("dx-media")
}

fn default_font_provider() -> String {
    "google".to_string()
}

fn default_font_formats() -> Vec<String> {
    vec!["ttf".to_string(), "woff2".to_string()]
}

fn default_font_subsets() -> Vec<String> {
    vec!["latin".to_string()]
}

fn default_true() -> bool {
    true
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            media_dir: None,
            icon_dir: None,
            font_dir: None,
            archive_dir: None,
            cache_dir: default_cache_dir(),
            default_media_provider: None,
            default_font_provider: default_font_provider(),
            font_formats: default_font_formats(),
            font_subsets: default_font_subsets(),
            auto_create_dirs: true,
            organize_by_date: false,
            organize_by_type: true,
        }
    }
}

impl MediaConfig {
    /// Load config from dx file using dx-serializer
    pub fn load() -> Result<Self> {
        // Try current directory first
        if let Ok(config) = Self::load_from_path("dx") {
            return Ok(config);
        }

        // Try ~/.config/dx/dx
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("dx").join("dx");
            if let Ok(config) = Self::load_from_path(&config_path) {
                return Ok(config);
            }
        }

        // Try home directory
        if let Some(home) = dirs::home_dir() {
            let config_path = home.join("dx");
            if let Ok(config) = Self::load_from_path(&config_path) {
                return Ok(config);
            }
        }

        // Return default if no config found
        Ok(Self::default())
    }

    fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;

        // Parse using dx-serializer human format
        let doc = serializer::human_to_document(&content)?;

        // Extract configuration from document
        let mut config = Self::default();

        // Parse media.cli section
        if let Some(serializer::DxLlmValue::Obj(media)) = doc.context.get("media") {
            if let Some(serializer::DxLlmValue::Obj(cli)) = media.get("cli") {
                if let Some(serializer::DxLlmValue::Str(base_dir)) = cli.get("base_dir") {
                    config.output_dir = PathBuf::from(base_dir);
                }
                if let Some(serializer::DxLlmValue::Bool(auto)) = cli.get("auto_create") {
                    config.auto_create_dirs = *auto;
                }
                if let Some(serializer::DxLlmValue::Bool(by_type)) = cli.get("organize_by_type") {
                    config.organize_by_type = *by_type;
                }
                if let Some(serializer::DxLlmValue::Bool(by_date)) = cli.get("organize_by_date") {
                    config.organize_by_date = *by_date;
                }

                // Parse directories subsection
                if let Some(serializer::DxLlmValue::Obj(dirs)) = cli.get("directories") {
                    if let Some(serializer::DxLlmValue::Str(media_dir)) = dirs.get("media") {
                        config.media_dir = Some(PathBuf::from(media_dir));
                    }
                    if let Some(serializer::DxLlmValue::Str(icons)) = dirs.get("icons") {
                        config.icon_dir = Some(PathBuf::from(icons));
                    }
                    if let Some(serializer::DxLlmValue::Str(fonts)) = dirs.get("fonts") {
                        config.font_dir = Some(PathBuf::from(fonts));
                    }
                    if let Some(serializer::DxLlmValue::Str(archives)) = dirs.get("archives") {
                        config.archive_dir = Some(PathBuf::from(archives));
                    }
                    if let Some(serializer::DxLlmValue::Str(cache)) = dirs.get("cache") {
                        config.cache_dir = PathBuf::from(cache);
                    }
                }

                // Parse providers subsection
                if let Some(serializer::DxLlmValue::Obj(providers)) = cli.get("providers") {
                    if let Some(serializer::DxLlmValue::Str(media_prov)) =
                        providers.get("default_media")
                    {
                        config.default_media_provider = Some(media_prov.clone());
                    }
                    if let Some(serializer::DxLlmValue::Str(font)) = providers.get("default_font") {
                        config.default_font_provider = font.clone();
                    }
                }

                // Parse fonts subsection
                if let Some(serializer::DxLlmValue::Obj(fonts)) = cli.get("fonts") {
                    if let Some(serializer::DxLlmValue::Arr(formats)) = fonts.get("formats") {
                        config.font_formats = formats
                            .iter()
                            .filter_map(|v| {
                                if let serializer::DxLlmValue::Str(s) = v {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                    }
                    if let Some(serializer::DxLlmValue::Arr(subsets)) = fonts.get("subsets") {
                        config.font_subsets = subsets
                            .iter()
                            .filter_map(|v| {
                                if let serializer::DxLlmValue::Str(s) = v {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                    }
                }
            }
        }

        Ok(config)
    }

    /// Get output directory for media downloads
    pub fn get_media_dir(&self) -> PathBuf {
        let dir = if let Some(ref dir) = self.media_dir {
            self.output_dir.join(dir)
        } else if self.organize_by_type {
            self.output_dir.join("media")
        } else {
            self.output_dir.clone()
        };
        dir
    }

    /// Get output directory for icon downloads
    pub fn get_icon_dir(&self) -> PathBuf {
        let dir = if let Some(ref dir) = self.icon_dir {
            self.output_dir.join(dir)
        } else if self.organize_by_type {
            self.output_dir.join("icons")
        } else {
            self.output_dir.clone()
        };
        dir
    }

    /// Get output directory for font downloads
    pub fn get_font_dir(&self) -> PathBuf {
        let dir = if let Some(ref dir) = self.font_dir {
            self.output_dir.join(dir)
        } else if self.organize_by_type {
            self.output_dir.join("fonts")
        } else {
            self.output_dir.clone()
        };
        dir
    }

    /// Get output directory for archive operations
    pub fn get_archive_dir(&self) -> PathBuf {
        let dir = if let Some(ref dir) = self.archive_dir {
            self.output_dir.join(dir)
        } else if self.organize_by_type {
            self.output_dir.join("archives")
        } else {
            self.output_dir.clone()
        };
        dir
    }

    /// Ensure directory exists if auto_create_dirs is enabled
    pub fn ensure_dir(&self, path: &Path) -> Result<()> {
        if self.auto_create_dirs && !path.exists() {
            std::fs::create_dir_all(path)?;
        }
        Ok(())
    }
}
