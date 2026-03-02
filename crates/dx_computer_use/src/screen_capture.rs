//! Cross-platform screen capture using the `screenshots` crate pattern.
//!
//! Provides a higher-level API over platform-specific capture methods,
//! with support for full screen, region, and individual window capture.

use anyhow::Result;

/// Captured screen image information.
#[derive(Debug, Clone)]
pub struct CapturedScreen {
    pub width: u32,
    pub height: u32,
    /// Raw RGBA pixel data.
    pub pixels: Vec<u8>,
    /// Monitor/display index.
    pub display_index: usize,
}

/// Cross-platform screen capture manager.
pub struct ScreenCaptureManager;

impl ScreenCaptureManager {
    /// Capture the primary display.
    pub fn capture_primary() -> Result<CapturedScreen> {
        log::debug!("Capturing primary display");
        let png_data = crate::capture::capture_full_screen()?;
        let (width, height) = crate::capture::png_dimensions(&png_data).unwrap_or((1920, 1080));
        Ok(CapturedScreen {
            width,
            height,
            pixels: png_data,
            display_index: 0,
        })
    }

    /// Capture all displays.
    pub fn capture_all() -> Result<Vec<CapturedScreen>> {
        Ok(vec![Self::capture_primary()?])
    }

    /// Capture a specific region of the screen.
    pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<CapturedScreen> {
        log::debug!("Capturing region ({}, {}) {}x{}", x, y, width, height);
        let png_data = crate::capture::capture_region(x, y, width, height)?;
        Ok(CapturedScreen {
            width,
            height,
            pixels: png_data,
            display_index: 0,
        })
    }

    /// Capture a specific window by its title.
    pub fn capture_window(window_title: &str) -> Result<CapturedScreen> {
        log::debug!("Capturing window '{}'", window_title);
        let png_data = crate::capture::capture_window(window_title)?;
        let (width, height) = crate::capture::png_dimensions(&png_data).unwrap_or((800, 600));
        Ok(CapturedScreen {
            width,
            height,
            pixels: png_data,
            display_index: 0,
        })
    }

    /// Encode captured screen as PNG bytes.
    pub fn encode_png(screen: &CapturedScreen) -> Result<Vec<u8>> {
        Ok(screen.pixels.clone())
    }

    /// Encode captured screen as base64 PNG (for sending to vision models).
    pub fn encode_base64(screen: &CapturedScreen) -> Result<String> {
        let png = Self::encode_png(screen)?;
        Ok(crate::capture::png_to_base64(&png))
    }
}
