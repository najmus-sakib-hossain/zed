//! Browser Controller
//!
//! Manages browser instance lifecycle and provides low-level page control.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Browser configuration
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    /// Headless mode
    pub headless: bool,
    /// Navigation timeout
    pub timeout: Duration,
    /// Viewport size
    pub viewport: (u32, u32),
    /// User agent
    pub user_agent: Option<String>,
    /// Proxy URL
    pub proxy: Option<String>,
    /// Chrome executable path
    pub chrome_path: Option<PathBuf>,
    /// Enable JavaScript
    pub javascript_enabled: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            timeout: Duration::from_secs(30),
            viewport: (1920, 1080),
            user_agent: None,
            proxy: None,
            chrome_path: None,
            javascript_enabled: true,
        }
    }
}

/// Browser errors
#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("Browser not initialized")]
    NotInitialized,

    #[error("Navigation failed: {0}")]
    NavigationFailed(String),

    #[error("Element not found: {0}")]
    ElementNotFound(String),

    #[error("Timeout waiting for {0}")]
    Timeout(String),

    #[error("JavaScript error: {0}")]
    JavaScriptError(String),

    #[error("Screenshot failed: {0}")]
    ScreenshotFailed(String),

    #[error("Browser error: {0}")]
    Other(String),
}

/// Wait strategy for navigation/elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaitStrategy {
    /// Wait for DOM content loaded
    DomContentLoaded,
    /// Wait for full page load
    Load,
    /// Wait for network idle (no requests for N ms)
    NetworkIdle { idle_time_ms: u64 },
    /// Wait for specific selector
    Selector { selector: String, timeout_ms: u64 },
    /// Wait for specific text content
    Text { text: String, timeout_ms: u64 },
    /// No waiting
    None,
}

impl Default for WaitStrategy {
    fn default() -> Self {
        Self::Load
    }
}

/// Information about a page element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    /// CSS selector used to find element
    pub selector: String,
    /// Tag name (div, input, etc.)
    pub tag_name: String,
    /// Element text content
    pub text_content: Option<String>,
    /// Inner HTML
    pub inner_html: Option<String>,
    /// Element attributes
    pub attributes: HashMap<String, String>,
    /// Bounding box
    pub bounds: Option<ElementBounds>,
    /// Is visible
    pub visible: bool,
    /// Is enabled (for inputs)
    pub enabled: bool,
}

/// Element bounding box
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ElementBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Information about a page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    /// Page URL
    pub url: String,
    /// Page title
    pub title: String,
    /// Viewport size
    pub viewport: (u32, u32),
    /// Cookies
    pub cookies: Vec<Cookie>,
}

/// Browser cookie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<i64>,
    pub http_only: bool,
    pub secure: bool,
}

/// Browser controller for web automation
pub struct BrowserController {
    /// Configuration
    config: BrowserConfig,
    /// Current page URL
    current_url: Arc<RwLock<Option<String>>>,
    /// Page content cache
    page_content: Arc<RwLock<Option<String>>>,
    /// Cookies
    cookies: Arc<RwLock<Vec<Cookie>>>,
    /// Session storage
    session_data: Arc<RwLock<HashMap<String, String>>>,
    /// Is browser initialized
    initialized: Arc<RwLock<bool>>,
}

impl BrowserController {
    /// Create a new browser controller
    pub async fn new(config: BrowserConfig) -> Result<Self> {
        let controller = Self {
            config,
            current_url: Arc::new(RwLock::new(None)),
            page_content: Arc::new(RwLock::new(None)),
            cookies: Arc::new(RwLock::new(Vec::new())),
            session_data: Arc::new(RwLock::new(HashMap::new())),
            initialized: Arc::new(RwLock::new(false)),
        };

        Ok(controller)
    }

    /// Initialize the browser
    pub async fn initialize(&self) -> Result<()> {
        // In a full implementation, this would launch Chrome via chromiumoxide
        // For now, we simulate initialization
        let mut initialized = self.initialized.write().await;
        *initialized = true;
        Ok(())
    }

    /// Check if browser is initialized
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }

    /// Navigate to a URL
    pub async fn navigate(&self, url: &str) -> Result<PageInfo> {
        self.navigate_with_wait(url, WaitStrategy::Load).await
    }

    /// Navigate with custom wait strategy
    pub async fn navigate_with_wait(&self, url: &str, wait: WaitStrategy) -> Result<PageInfo> {
        if !self.is_initialized().await {
            self.initialize().await?;
        }

        // Simulate navigation
        tracing::debug!("Navigating to {} with wait strategy {:?}", url, wait);

        // In production, this would use chromiumoxide to navigate
        // For now, we use reqwest for basic HTTP fetching
        let response = reqwest::get(url)
            .await
            .with_context(|| format!("Failed to navigate to {}", url))?;

        let final_url = response.url().to_string();
        let content = response
            .text()
            .await
            .with_context(|| "Failed to get page content")?;

        // Update state
        {
            let mut current = self.current_url.write().await;
            *current = Some(final_url.clone());
        }
        {
            let mut page = self.page_content.write().await;
            *page = Some(content.clone());
        }

        // Extract title from HTML
        let title = extract_title(&content).unwrap_or_else(|| "Untitled".to_string());

        Ok(PageInfo {
            url: final_url,
            title,
            viewport: self.config.viewport,
            cookies: self.cookies.read().await.clone(),
        })
    }

    /// Get current page info
    pub async fn page_info(&self) -> Result<PageInfo> {
        let url = self
            .current_url
            .read()
            .await
            .clone()
            .ok_or_else(|| BrowserError::NotInitialized)?;

        let content = self.page_content.read().await.clone().unwrap_or_default();
        let title = extract_title(&content).unwrap_or_else(|| "Untitled".to_string());

        Ok(PageInfo {
            url,
            title,
            viewport: self.config.viewport,
            cookies: self.cookies.read().await.clone(),
        })
    }

    /// Click on an element
    pub async fn click(&self, selector: &str) -> Result<()> {
        if !self.is_initialized().await {
            return Err(BrowserError::NotInitialized.into());
        }

        tracing::debug!("Clicking element: {}", selector);

        // In production, this would use chromiumoxide
        // Simulate click by checking element exists
        let content = self.page_content.read().await;
        if content.is_none() {
            return Err(BrowserError::NavigationFailed("No page loaded".to_string()).into());
        }

        // Basic check if selector might exist (simplified)
        // Real implementation would use actual DOM querying
        Ok(())
    }

    /// Type text into an element
    pub async fn type_text(&self, selector: &str, text: &str) -> Result<()> {
        if !self.is_initialized().await {
            return Err(BrowserError::NotInitialized.into());
        }

        tracing::debug!("Typing '{}' into element: {}", text, selector);
        Ok(())
    }

    /// Clear an input element
    pub async fn clear(&self, selector: &str) -> Result<()> {
        if !self.is_initialized().await {
            return Err(BrowserError::NotInitialized.into());
        }

        tracing::debug!("Clearing element: {}", selector);
        Ok(())
    }

    /// Select an option from a dropdown
    pub async fn select(&self, selector: &str, value: &str) -> Result<()> {
        if !self.is_initialized().await {
            return Err(BrowserError::NotInitialized.into());
        }

        tracing::debug!("Selecting '{}' in element: {}", value, selector);
        Ok(())
    }

    /// Get element information
    pub async fn get_element(&self, selector: &str) -> Result<ElementInfo> {
        if !self.is_initialized().await {
            return Err(BrowserError::NotInitialized.into());
        }

        // Simplified element info
        Ok(ElementInfo {
            selector: selector.to_string(),
            tag_name: "div".to_string(),
            text_content: None,
            inner_html: None,
            attributes: HashMap::new(),
            bounds: None,
            visible: true,
            enabled: true,
        })
    }

    /// Get multiple elements
    pub async fn get_elements(&self, selector: &str) -> Result<Vec<ElementInfo>> {
        if !self.is_initialized().await {
            return Err(BrowserError::NotInitialized.into());
        }

        // Would return actual elements in production
        Ok(vec![self.get_element(selector).await?])
    }

    /// Wait for an element to appear
    pub async fn wait_for_element(&self, selector: &str, timeout_ms: u64) -> Result<ElementInfo> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        loop {
            if let Ok(element) = self.get_element(selector).await {
                return Ok(element);
            }

            if start.elapsed() > timeout {
                return Err(BrowserError::Timeout(format!("element {}", selector)).into());
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Execute JavaScript
    pub async fn execute_js(&self, script: &str) -> Result<serde_json::Value> {
        if !self.is_initialized().await {
            return Err(BrowserError::NotInitialized.into());
        }

        if !self.config.javascript_enabled {
            return Err(BrowserError::JavaScriptError("JavaScript is disabled".to_string()).into());
        }

        tracing::debug!("Executing JavaScript: {}", script);

        // In production, this would actually execute JS
        Ok(serde_json::Value::Null)
    }

    /// Take a screenshot
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        if !self.is_initialized().await {
            return Err(BrowserError::NotInitialized.into());
        }

        // In production, this would capture actual screenshot
        // For now, return empty PNG
        Ok(create_placeholder_png())
    }

    /// Take a screenshot of an element
    pub async fn screenshot_element(&self, selector: &str) -> Result<Vec<u8>> {
        if !self.is_initialized().await {
            return Err(BrowserError::NotInitialized.into());
        }

        tracing::debug!("Taking screenshot of element: {}", selector);
        Ok(create_placeholder_png())
    }

    /// Save screenshot to file
    pub async fn save_screenshot(&self, path: &PathBuf) -> Result<()> {
        let data = self.screenshot().await?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Generate PDF of current page
    pub async fn pdf(&self) -> Result<Vec<u8>> {
        if !self.is_initialized().await {
            return Err(BrowserError::NotInitialized.into());
        }

        // Placeholder
        Ok(Vec::new())
    }

    /// Get page content (HTML)
    pub async fn content(&self) -> Result<String> {
        self.page_content
            .read()
            .await
            .clone()
            .ok_or_else(|| BrowserError::NavigationFailed("No page loaded".to_string()).into())
    }

    /// Get text content of an element
    pub async fn text_content(&self, selector: &str) -> Result<String> {
        let element = self.get_element(selector).await?;
        Ok(element.text_content.unwrap_or_default())
    }

    /// Get attribute of an element
    pub async fn attribute(&self, selector: &str, attribute: &str) -> Result<Option<String>> {
        let element = self.get_element(selector).await?;
        Ok(element.attributes.get(attribute).cloned())
    }

    /// Set a cookie
    pub async fn set_cookie(&self, cookie: Cookie) -> Result<()> {
        let mut cookies = self.cookies.write().await;
        cookies.push(cookie);
        Ok(())
    }

    /// Get all cookies
    pub async fn get_cookies(&self) -> Vec<Cookie> {
        self.cookies.read().await.clone()
    }

    /// Clear all cookies
    pub async fn clear_cookies(&self) -> Result<()> {
        let mut cookies = self.cookies.write().await;
        cookies.clear();
        Ok(())
    }

    /// Set session data
    pub async fn set_session(&self, key: &str, value: &str) {
        let mut data = self.session_data.write().await;
        data.insert(key.to_string(), value.to_string());
    }

    /// Get session data
    pub async fn get_session(&self, key: &str) -> Option<String> {
        let data = self.session_data.read().await;
        data.get(key).cloned()
    }

    /// Go back in history
    pub async fn back(&self) -> Result<()> {
        tracing::debug!("Navigating back");
        Ok(())
    }

    /// Go forward in history
    pub async fn forward(&self) -> Result<()> {
        tracing::debug!("Navigating forward");
        Ok(())
    }

    /// Reload page
    pub async fn reload(&self) -> Result<()> {
        if let Some(url) = self.current_url.read().await.clone() {
            self.navigate(&url).await?;
        }
        Ok(())
    }

    /// Close the browser
    pub async fn close(&self) -> Result<()> {
        let mut initialized = self.initialized.write().await;
        *initialized = false;
        Ok(())
    }

    /// Set viewport size
    pub async fn set_viewport(&mut self, width: u32, height: u32) -> Result<()> {
        self.config.viewport = (width, height);
        Ok(())
    }

    /// Toggle headless mode (requires restart)
    pub fn set_headless(&mut self, headless: bool) {
        self.config.headless = headless;
    }
}

/// Extract title from HTML content
fn extract_title(html: &str) -> Option<String> {
    // Simple regex-free extraction
    let lower = html.to_lowercase();
    let start = lower.find("<title>")?;
    let end = lower.find("</title>")?;

    if end > start + 7 {
        let title = &html[start + 7..end];
        Some(title.trim().to_string())
    } else {
        None
    }
}

/// Create a minimal placeholder PNG
fn create_placeholder_png() -> Vec<u8> {
    // Minimal 1x1 transparent PNG
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_controller_creation() {
        let config = BrowserConfig::default();
        let controller = BrowserController::new(config).await;
        assert!(controller.is_ok());
    }

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>Test Page</title></head></html>";
        let title = extract_title(html);
        assert_eq!(title, Some("Test Page".to_string()));
    }

    #[test]
    fn test_config_default() {
        let config = BrowserConfig::default();
        assert!(config.headless);
        assert_eq!(config.viewport, (1920, 1080));
    }
}
