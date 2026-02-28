use serde::{Deserialize, Serialize};

/// AI Settings - configuration for AI features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub stream_response: bool,
    pub use_tools: bool,
    pub mcp_enabled: bool,
    pub acp_enabled: bool,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 4096,
            top_p: 1.0,
            stream_response: true,
            use_tools: true,
            mcp_enabled: false,
            acp_enabled: false,
        }
    }
}

impl AiSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = top_p.clamp(0.0, 1.0);
        self
    }

    pub fn toggle_stream(&mut self) {
        self.stream_response = !self.stream_response;
    }

    pub fn toggle_tools(&mut self) {
        self.use_tools = !self.use_tools;
    }

    pub fn toggle_mcp(&mut self) {
        self.mcp_enabled = !self.mcp_enabled;
    }

    pub fn toggle_acp(&mut self) {
        self.acp_enabled = !self.acp_enabled;
    }
}
