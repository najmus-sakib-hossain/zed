//! Browser Automation System
//!
//! Provides headless and headed browser control for web automation tasks
//! including navigation, interaction, scraping, and screenshot capture.
//!
//! # Architecture
//!
//! The browser system consists of:
//! - [`BrowserController`] - Main browser instance management
//! - [`BrowserActions`] - High-level action execution
//! - [`BrowserConfig`] - Configuration from `browser.sr`
//!
//! # Features
//!
//! - Headless and headed mode toggle
//! - Page navigation with wait strategies
//! - Element interaction (click, type, select)
//! - Screenshot and PDF capture
//! - Content scraping with selectors
//! - Cookie and session management
//! - Variable interpolation in action sequences
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::agent::browser::{BrowserController, BrowserConfig, Action};
//!
//! let config = BrowserConfig::default();
//! let mut browser = BrowserController::new(config).await?;
//!
//! // Navigate and interact
//! browser.navigate("https://example.com").await?;
//! browser.click("#login-button").await?;
//! browser.type_text("#username", "user@example.com").await?;
//!
//! // Take screenshot
//! let screenshot = browser.screenshot().await?;
//! ```

pub mod actions;
pub mod controller;

pub use actions::{Action, ActionResult, ActionSequence, ScrapeResult};
pub use controller::{
    BrowserController, BrowserConfig, BrowserError, ElementInfo, PageInfo, WaitStrategy,
};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Browser configuration loaded from `browser.sr`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSettings {
    /// Enable headless mode (no visible window)
    pub headless: bool,
    /// Default navigation timeout in milliseconds
    pub timeout_ms: u64,
    /// Default viewport width
    pub viewport_width: u32,
    /// Default viewport height
    pub viewport_height: u32,
    /// User agent string
    pub user_agent: Option<String>,
    /// Proxy server URL
    pub proxy: Option<String>,
    /// Downloads directory
    pub downloads_dir: Option<PathBuf>,
    /// Enable JavaScript
    pub javascript_enabled: bool,
    /// Block images for faster loading
    pub block_images: bool,
    /// Block ads and trackers
    pub block_ads: bool,
    /// Custom Chrome/Chromium path
    pub chrome_path: Option<PathBuf>,
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            headless: true,
            timeout_ms: 30_000,
            viewport_width: 1920,
            viewport_height: 1080,
            user_agent: None,
            proxy: None,
            downloads_dir: None,
            javascript_enabled: true,
            block_images: false,
            block_ads: false,
            chrome_path: None,
        }
    }
}

impl BrowserSettings {
    /// Load from configuration file
    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            // Parse using DX Serializer (simplified for now)
            let _ = content;
            Ok(Self::default())
        } else {
            Ok(Self::default())
        }
    }

    /// Load from default location
    pub fn load_default() -> Self {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx")
            .join("browser.sr");

        Self::load(&config_path).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = BrowserSettings::default();
        assert!(settings.headless);
        assert_eq!(settings.timeout_ms, 30_000);
        assert!(settings.javascript_enabled);
    }
}
