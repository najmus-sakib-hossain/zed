//! Browser controller for Chrome DevTools Protocol automation.

use anyhow::Result;
use chromiumoxide::Page;
use chromiumoxide::browser::{Browser, BrowserConfig as ChromeConfig};
use chromiumoxide::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, CaptureScreenshotParams,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Browser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Show browser window (false = headless)
    #[serde(default)]
    pub headless: bool,
    /// Chrome/Chromium executable path
    pub chrome_path: Option<String>,
    /// Viewport width
    #[serde(default = "default_width")]
    pub viewport_width: u32,
    /// Viewport height
    #[serde(default = "default_height")]
    pub viewport_height: u32,
    /// User agent string
    pub user_agent: Option<String>,
    /// Timeout for page loads in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Extra Chrome arguments
    #[serde(default)]
    pub extra_args: Vec<String>,
    /// Optional browser profile directory (user-data-dir)
    pub profile_dir: Option<String>,
    /// Optional proxy URL, e.g. http://127.0.0.1:8080
    pub proxy_url: Option<String>,
    /// Browser kind for launch strategy
    #[serde(default)]
    pub browser_kind: BrowserKind,
}

/// Browser engine selection.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum BrowserKind {
    /// Auto-detect Chrome/Chromium executable
    #[default]
    Auto,
    /// Google Chrome
    Chrome,
    /// Chromium
    Chromium,
    /// Microsoft Edge (Chromium-based)
    Edge,
}

fn default_width() -> u32 {
    1920
}
fn default_height() -> u32 {
    1080
}
fn default_timeout() -> u64 {
    30
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            chrome_path: None,
            viewport_width: 1920,
            viewport_height: 1080,
            user_agent: None,
            timeout_secs: 30,
            extra_args: vec![],
            profile_dir: None,
            proxy_url: None,
            browser_kind: BrowserKind::Auto,
        }
    }
}

/// Browser cookie model for import/export operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCookie {
    pub name: String,
    pub value: String,
}

/// Result of a page interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult {
    pub url: String,
    pub title: String,
    pub content: Option<String>,
    pub screenshot: Option<Vec<u8>>,
    pub console_logs: Vec<String>,
}

/// Browser controller wrapping chromiumoxide
pub struct BrowserController {
    config: BrowserConfig,
    browser: Option<Arc<Browser>>,
    pages: Arc<Mutex<Vec<Page>>>,
}

impl BrowserController {
    pub fn new(config: BrowserConfig) -> Self {
        Self {
            config,
            browser: None,
            pages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Launch the browser
    pub async fn launch(&mut self) -> Result<()> {
        let mut builder = ChromeConfig::builder();

        if self.config.headless {
            builder = builder.arg("--headless=new");
        }

        builder = builder
            .arg("--disable-gpu")
            .arg("--no-sandbox")
            .arg("--disable-dev-shm-usage")
            .arg(format!(
                "--window-size={},{}",
                self.config.viewport_width, self.config.viewport_height
            ));

        if let Some(ref ua) = self.config.user_agent {
            builder = builder.arg(format!("--user-agent={}", ua));
        }

        if let Some(ref path) = self.config.chrome_path {
            builder = builder.chrome_executable(path);
        } else if let Some(path) = self.default_browser_path() {
            builder = builder.chrome_executable(path);
        }

        if let Some(ref profile_dir) = self.config.profile_dir {
            builder = builder.arg(format!("--user-data-dir={}", profile_dir));
        }

        if let Some(ref proxy_url) = self.config.proxy_url {
            builder = builder.arg(format!("--proxy-server={}", proxy_url));
        }

        for arg in &self.config.extra_args {
            builder = builder.arg(arg);
        }

        let chrome_config = builder.build().map_err(|e| anyhow::anyhow!("{}", e))?;

        let (browser, mut handler) = Browser::launch(chrome_config).await?;

        // Spawn handler for CDP events
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                let _ = event;
            }
        });

        self.browser = Some(Arc::new(browser));
        info!("Browser launched (headless={})", self.config.headless);
        Ok(())
    }

    fn default_browser_path(&self) -> Option<String> {
        match self.config.browser_kind {
            BrowserKind::Auto | BrowserKind::Chrome => None,
            BrowserKind::Chromium => Some("chromium".to_string()),
            BrowserKind::Edge => {
                if cfg!(target_os = "windows") {
                    Some("C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe".to_string())
                } else if cfg!(target_os = "macos") {
                    Some(
                        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"
                            .to_string(),
                    )
                } else {
                    Some("microsoft-edge".to_string())
                }
            }
        }
    }

    /// Navigate to a URL and return page content
    pub async fn navigate(&self, url: &str) -> Result<PageResult> {
        let browser =
            self.browser.as_ref().ok_or_else(|| anyhow::anyhow!("Browser not launched"))?;

        let page = browser.new_page(url).await?;

        // Wait for page to load
        page.wait_for_navigation().await?;

        let title = page.get_title().await?.unwrap_or_default();
        let current_url = page.url().await?.map(|u| u.to_string()).unwrap_or_default();

        // Get page text content
        let content = page.evaluate("document.body.innerText").await?.into_value::<String>().ok();

        // Store the page
        self.pages.lock().await.push(page);

        Ok(PageResult {
            url: current_url,
            title,
            content,
            screenshot: None,
            console_logs: vec![],
        })
    }

    /// Take a screenshot of the current page
    pub async fn screenshot(&self, page_index: usize) -> Result<Vec<u8>> {
        let pages = self.pages.lock().await;
        let page = pages
            .get(page_index)
            .ok_or_else(|| anyhow::anyhow!("No page at index {}", page_index))?;

        let params =
            CaptureScreenshotParams::builder().format(CaptureScreenshotFormat::Png).build();

        let screenshot = page.execute(params).await?;
        let data =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &screenshot.data)?;

        Ok(data)
    }

    /// Execute JavaScript on a page
    pub async fn execute_js(&self, page_index: usize, script: &str) -> Result<serde_json::Value> {
        let pages = self.pages.lock().await;
        let page = pages
            .get(page_index)
            .ok_or_else(|| anyhow::anyhow!("No page at index {}", page_index))?;

        let result = page.evaluate(script).await?;
        Ok(result.into_value()?)
    }

    /// Click an element by CSS selector
    pub async fn click(&self, page_index: usize, selector: &str) -> Result<()> {
        let pages = self.pages.lock().await;
        let page = pages
            .get(page_index)
            .ok_or_else(|| anyhow::anyhow!("No page at index {}", page_index))?;

        let element = page.find_element(selector).await?;
        element.click().await?;
        Ok(())
    }

    /// Type text into an element
    pub async fn type_text(&self, page_index: usize, selector: &str, text: &str) -> Result<()> {
        let pages = self.pages.lock().await;
        let page = pages
            .get(page_index)
            .ok_or_else(|| anyhow::anyhow!("No page at index {}", page_index))?;

        let element = page.find_element(selector).await?;
        element.click().await?;
        element.type_str(text).await?;
        Ok(())
    }

    /// Get text content of an element
    pub async fn get_text(&self, page_index: usize, selector: &str) -> Result<String> {
        let pages = self.pages.lock().await;
        let page = pages
            .get(page_index)
            .ok_or_else(|| anyhow::anyhow!("No page at index {}", page_index))?;

        let element = page.find_element(selector).await?;
        let text = element.inner_text().await?.unwrap_or_default();
        Ok(text)
    }

    /// Get page HTML
    pub async fn get_html(&self, page_index: usize) -> Result<String> {
        let pages = self.pages.lock().await;
        let page = pages
            .get(page_index)
            .ok_or_else(|| anyhow::anyhow!("No page at index {}", page_index))?;

        let html = page.content().await?;
        Ok(html)
    }

    /// Close a specific page
    pub async fn close_page(&self, page_index: usize) -> Result<()> {
        let mut pages = self.pages.lock().await;
        if page_index < pages.len() {
            let page = pages.remove(page_index);
            page.close().await?;
        }
        Ok(())
    }

    /// Set a single cookie on a page using JS.
    pub async fn set_cookie(
        &self,
        page_index: usize,
        name: &str,
        value: &str,
        path: Option<&str>,
    ) -> Result<()> {
        let escaped_name = serde_json::to_string(name)?;
        let escaped_value = serde_json::to_string(value)?;
        let escaped_path = serde_json::to_string(path.unwrap_or("/"))?;
        let script = format!(
            "document.cookie = {n} + '=' + {v} + '; path=' + {p}; true;",
            n = escaped_name,
            v = escaped_value,
            p = escaped_path
        );
        let _ = self.execute_js(page_index, &script).await?;
        Ok(())
    }

    /// Get cookies from a page by parsing document.cookie.
    pub async fn get_cookies(&self, page_index: usize) -> Result<Vec<BrowserCookie>> {
        let value = self
            .execute_js(page_index, "document.cookie")
            .await?
            .as_str()
            .unwrap_or_default()
            .to_string();

        let mut cookies = Vec::new();
        for item in value.split(';') {
            let part = item.trim();
            if part.is_empty() {
                continue;
            }
            let mut iter = part.splitn(2, '=');
            let name = iter.next().unwrap_or_default().trim();
            let val = iter.next().unwrap_or_default().trim();
            if !name.is_empty() {
                cookies.push(BrowserCookie {
                    name: name.to_string(),
                    value: val.to_string(),
                });
            }
        }
        Ok(cookies)
    }

    /// Clear all cookies in the current document context.
    pub async fn clear_cookies(&self, page_index: usize) -> Result<()> {
        let script = r#"
            document.cookie.split(';').forEach(function(c) {
              document.cookie = c.replace(/^ +/, '')
                .replace(/=.*/, '=;expires=' + new Date(0).toUTCString() + ';path=/');
            });
            true;
        "#;
        let _ = self.execute_js(page_index, script).await?;
        Ok(())
    }

    /// Persist page cookies to a JSON file.
    pub async fn save_cookies_to_file(&self, page_index: usize, file: &str) -> Result<()> {
        let cookies = self.get_cookies(page_index).await?;
        let bytes = serde_json::to_vec_pretty(&cookies)?;
        std::fs::write(file, bytes)?;
        Ok(())
    }

    /// Load cookies from a JSON file and apply on the page.
    pub async fn load_cookies_from_file(&self, page_index: usize, file: &str) -> Result<()> {
        let data = std::fs::read(file)?;
        let cookies: Vec<BrowserCookie> = serde_json::from_slice(&data)?;
        for c in cookies {
            self.set_cookie(page_index, &c.name, &c.value, Some("/")).await?;
        }
        Ok(())
    }

    /// Export basic session metadata for diagnostics.
    pub async fn session_metadata(&self, page_index: usize) -> Result<HashMap<String, String>> {
        let mut map = HashMap::new();
        map.insert("page_index".into(), page_index.to_string());
        map.insert("headless".into(), self.config.headless.to_string());
        map.insert("browser_kind".into(), format!("{:?}", self.config.browser_kind));
        map.insert("proxy".into(), self.config.proxy_url.clone().unwrap_or_default());
        let cookies = self.get_cookies(page_index).await?;
        map.insert("cookie_count".into(), cookies.len().to_string());
        Ok(map)
    }

    /// Get the number of open pages
    pub async fn page_count(&self) -> usize {
        self.pages.lock().await.len()
    }

    /// Close the browser
    pub async fn close(&mut self) -> Result<()> {
        if let Some(_browser) = self.browser.take() {
            info!("Browser closed");
        }
        self.pages.lock().await.clear();
        Ok(())
    }
}

impl Drop for BrowserController {
    fn drop(&mut self) {
        if self.browser.is_some() {
            warn!("BrowserController dropped without calling close()");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_config_default() {
        let config = BrowserConfig::default();
        assert!(config.headless);
        assert_eq!(config.viewport_width, 1920);
        assert_eq!(config.viewport_height, 1080);
        assert!(config.profile_dir.is_none());
        assert!(config.proxy_url.is_none());
    }

    #[test]
    fn test_browser_controller_new() {
        let config = BrowserConfig::default();
        let controller = BrowserController::new(config);
        assert!(controller.browser.is_none());
    }

    #[test]
    fn test_cookie_model_serde() {
        let cookie = BrowserCookie {
            name: "sid".into(),
            value: "abc".into(),
        };
        let text = serde_json::to_string(&cookie).unwrap();
        assert!(text.contains("sid"));
    }
}
