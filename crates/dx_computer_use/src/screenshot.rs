//! Screenshot capture and encoding.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A captured screenshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenCapture {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Raw RGBA pixel data.
    #[serde(skip)]
    pub pixels: Vec<u8>,
    /// PNG-encoded data for transmission.
    pub png_data: Option<Vec<u8>>,
    /// Base64-encoded PNG for LLM vision APIs.
    pub base64_png: Option<String>,
}

impl ScreenCapture {
    /// Capture the entire screen (uses platform-native capture).
    pub fn capture_full() -> Result<Self> {
        log::info!("Capturing full screen");
        let png_data = crate::capture::capture_full_screen()?;
        let (width, height) = crate::capture::png_dimensions(&png_data).unwrap_or((1920, 1080));
        Ok(Self {
            width,
            height,
            pixels: Vec::new(),
            png_data: Some(png_data),
            base64_png: None,
        })
    }

    /// Capture a region of the screen.
    pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<Self> {
        log::info!(
            "Capturing region ({}, {}) {}x{}",
            x,
            y,
            width,
            height
        );
        let png_data = crate::capture::capture_region(x, y, width, height)?;
        Ok(Self {
            width,
            height,
            pixels: Vec::new(),
            png_data: Some(png_data),
            base64_png: None,
        })
    }

    /// Encode to PNG.
    pub fn encode_png(&mut self) -> Result<()> {
        // Placeholder — real implementation uses png crate
        log::info!("Encoding screenshot {}x{} to PNG", self.width, self.height);
        self.png_data = Some(Vec::new());
        Ok(())
    }

    /// Encode to base64 PNG for vision APIs.
    pub fn encode_base64(&mut self) -> Result<()> {
        if self.png_data.is_none() {
            self.encode_png()?;
        }
        if let Some(ref data) = self.png_data {
            self.base64_png = Some(crate::capture::png_to_base64(data));
        }
        Ok(())
    }
}
