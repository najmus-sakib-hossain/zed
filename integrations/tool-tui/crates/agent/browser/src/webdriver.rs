//! Optional WebDriver-based browser controller using fantoccini.
//!
//! This is an alternative backend for environments where CDP is not suitable.

use anyhow::Result;
use fantoccini::{Client, ClientBuilder, Locator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDriverConfig {
    /// WebDriver endpoint, e.g. http://localhost:4444
    pub endpoint: String,
}

impl Default for WebDriverConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4444".to_string(),
        }
    }
}

pub struct WebDriverController {
    client: Option<Client>,
    config: WebDriverConfig,
}

impl WebDriverController {
    pub fn new(config: WebDriverConfig) -> Self {
        Self {
            client: None,
            config,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let client = ClientBuilder::native().connect(&self.config.endpoint).await?;
        self.client = Some(client);
        Ok(())
    }

    pub async fn goto(&self, url: &str) -> Result<()> {
        let client =
            self.client.as_ref().ok_or_else(|| anyhow::anyhow!("WebDriver not connected"))?;
        client.goto(url).await?;
        Ok(())
    }

    pub async fn click_css(&self, selector: &str) -> Result<()> {
        let client =
            self.client.as_ref().ok_or_else(|| anyhow::anyhow!("WebDriver not connected"))?;
        client.find(Locator::Css(selector)).await?.click().await?;
        Ok(())
    }

    pub async fn source(&self) -> Result<String> {
        let client =
            self.client.as_ref().ok_or_else(|| anyhow::anyhow!("WebDriver not connected"))?;
        Ok(client.source().await?)
    }

    pub async fn close(mut self) -> Result<()> {
        if let Some(client) = self.client.take() {
            client.close().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_webdriver_config() {
        let cfg = WebDriverConfig::default();
        assert_eq!(cfg.endpoint, "http://localhost:4444");
    }
}
