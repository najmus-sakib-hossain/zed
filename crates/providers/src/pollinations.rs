/// Pollinations.ai — truly keyless LLM provider.
///
/// Endpoint: `https://text.pollinations.ai/openai` (OpenAI-compatible)
/// Authentication: None required
/// Models: 1 (openai-fast, 20B reasoning model)

pub const POLLINATIONS_API: &str = "https://text.pollinations.ai/openai";

pub const POLLINATIONS_MODELS: [PollinationsModelDescriptor; 1] = [PollinationsModelDescriptor {
    id: "openai-fast",
    display_name: "OpenAI Fast",
    max_tokens: 131_000,
    max_output_tokens: 32_768,
}];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollinationsModelDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub max_tokens: u64,
    pub max_output_tokens: u64,
}
