//! Screenshot utilities for browser control.

use serde::{Deserialize, Serialize};

/// Screenshot format
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
    Webp,
}

/// Screenshot options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotOptions {
    /// Format (default: PNG)
    pub format: ScreenshotFormat,
    /// JPEG/WebP quality (1-100)
    pub quality: Option<u8>,
    /// Full page screenshot
    pub full_page: bool,
    /// Clip region
    pub clip: Option<ClipRegion>,
    /// Scale factor
    pub device_scale_factor: Option<f64>,
}

/// Region to clip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            format: ScreenshotFormat::Png,
            quality: None,
            full_page: false,
            clip: None,
            device_scale_factor: None,
        }
    }
}
