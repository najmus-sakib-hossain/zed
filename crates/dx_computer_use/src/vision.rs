//! Vision analyzer — sends screenshots to vision LLMs for analysis.

use anyhow::Result;
use dx_core::{LlmProvider, LlmRequest, Message, Role};
use std::sync::Arc;

use crate::screenshot::ScreenCapture;

/// Analyzes screenshots using vision-capable LLMs.
pub struct VisionAnalyzer {
    provider: Option<Arc<dyn LlmProvider>>,
}

impl VisionAnalyzer {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn set_provider(&mut self, provider: Arc<dyn LlmProvider>) {
        self.provider = Some(provider);
    }

    /// Analyze a screenshot and describe what's visible.
    pub async fn describe(&self, _capture: &ScreenCapture) -> Result<String> {
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No vision provider configured"))?;

        let request = LlmRequest {
            messages: vec![
                Message {
                    role: Role::System,
                    content: "You are a vision AI analyzing a screenshot. Describe the UI elements visible, their positions, and any text content. Be concise and structured.".into(),
                },
                Message {
                    role: Role::User,
                    content: "[Screenshot would be attached here as base64]".into(),
                },
            ],
            max_tokens: Some(500),
            temperature: Some(0.1),
            model: None,
        };

        let response = provider.complete(&request).await?;
        Ok(response.text)
    }

    /// Find a specific UI element in the screenshot.
    pub async fn find_element(
        &self,
        _capture: &ScreenCapture,
        element_description: &str,
    ) -> Result<Option<(i32, i32)>> {
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No vision provider configured"))?;

        let request = LlmRequest {
            messages: vec![
                Message {
                    role: Role::System,
                    content: "You are a vision AI. Given a screenshot, find the specified UI element and return its center coordinates as JSON: {\"x\": <int>, \"y\": <int>}. If not found, return {\"x\": null, \"y\": null}.".into(),
                },
                Message {
                    role: Role::User,
                    content: format!(
                        "Find this element: {}\n[Screenshot attached]",
                        element_description
                    ),
                },
            ],
            max_tokens: Some(100),
            temperature: Some(0.0),
            model: None,
        };

        let response = provider.complete(&request).await?;

        // Parse coordinates from response
        if let Ok(coords) = serde_json::from_str::<ElementCoords>(&response.text) {
            if let (Some(x), Some(y)) = (coords.x, coords.y) {
                return Ok(Some((x, y)));
            }
        }

        Ok(None)
    }
}

impl Default for VisionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(serde::Deserialize)]
struct ElementCoords {
    x: Option<i32>,
    y: Option<i32>,
}
