use serde::{Deserialize, Serialize};

/// AI Provider types - matching Zed's provider ecosystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiProviderKind {
    Anthropic,
    OpenAi,
    GoogleAi,
    Ollama,
    DeepSeek,
    Mistral,
    XAi,
    Bedrock,
    Copilot,
    Vercel,
    Codestral,
    LmStudio,
    OpenRouter,
    Supermaven,
    CloudLlm,
}

impl AiProviderKind {
    pub fn all() -> &'static [AiProviderKind] {
        &[
            AiProviderKind::Anthropic,
            AiProviderKind::OpenAi,
            AiProviderKind::GoogleAi,
            AiProviderKind::Ollama,
            AiProviderKind::DeepSeek,
            AiProviderKind::Mistral,
            AiProviderKind::XAi,
            AiProviderKind::Bedrock,
            AiProviderKind::Copilot,
            AiProviderKind::Vercel,
            AiProviderKind::Codestral,
            AiProviderKind::LmStudio,
            AiProviderKind::OpenRouter,
            AiProviderKind::Supermaven,
            AiProviderKind::CloudLlm,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AiProviderKind::Anthropic => "Anthropic",
            AiProviderKind::OpenAi => "OpenAI",
            AiProviderKind::GoogleAi => "Google AI",
            AiProviderKind::Ollama => "Ollama",
            AiProviderKind::DeepSeek => "DeepSeek",
            AiProviderKind::Mistral => "Mistral",
            AiProviderKind::XAi => "xAI",
            AiProviderKind::Bedrock => "AWS Bedrock",
            AiProviderKind::Copilot => "GitHub Copilot",
            AiProviderKind::Vercel => "Vercel AI",
            AiProviderKind::Codestral => "Codestral",
            AiProviderKind::LmStudio => "LM Studio",
            AiProviderKind::OpenRouter => "OpenRouter",
            AiProviderKind::Supermaven => "Supermaven",
            AiProviderKind::CloudLlm => "Cloud LLM",
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            AiProviderKind::Anthropic => "anthropic",
            AiProviderKind::OpenAi => "openai",
            AiProviderKind::GoogleAi => "google",
            AiProviderKind::Ollama => "ollama",
            AiProviderKind::DeepSeek => "deepseek",
            AiProviderKind::Mistral => "mistral",
            AiProviderKind::XAi => "x_ai",
            AiProviderKind::Bedrock => "bedrock",
            AiProviderKind::Copilot => "copilot",
            AiProviderKind::Vercel => "vercel",
            AiProviderKind::Codestral => "codestral",
            AiProviderKind::LmStudio => "lmstudio",
            AiProviderKind::OpenRouter => "open_router",
            AiProviderKind::Supermaven => "supermaven",
            AiProviderKind::CloudLlm => "cloud_llm",
        }
    }

    /// Whether this provider requires an API key for authentication.
    pub fn needs_api_key(&self) -> bool {
        matches!(
            self,
            AiProviderKind::Anthropic
                | AiProviderKind::OpenAi
                | AiProviderKind::GoogleAi
                | AiProviderKind::DeepSeek
                | AiProviderKind::Mistral
                | AiProviderKind::XAi
                | AiProviderKind::Bedrock
                | AiProviderKind::Vercel
                | AiProviderKind::OpenRouter
                | AiProviderKind::Codestral
        )
    }

    /// Hint text for the API key input field.
    pub fn api_key_hint(&self) -> &'static str {
        match self {
            AiProviderKind::Anthropic => "sk-ant-...",
            AiProviderKind::OpenAi => "sk-...",
            AiProviderKind::GoogleAi => "AI...",
            AiProviderKind::DeepSeek => "sk-...",
            AiProviderKind::Mistral => "...",
            AiProviderKind::XAi => "xai-...",
            AiProviderKind::OpenRouter => "sk-or-...",
            _ => "Enter API key...",
        }
    }
}

/// AI Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModel {
    pub id: String,
    pub name: String,
    pub provider: AiProviderKind,
    pub context_window: usize,
    pub max_output_tokens: Option<usize>,
}

impl AiModel {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        provider: AiProviderKind,
        context_window: usize,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            provider,
            context_window,
            max_output_tokens: None,
        }
    }

    pub fn with_max_output(mut self, max_output: usize) -> Self {
        self.max_output_tokens = Some(max_output);
        self
    }
}

/// AI Provider instance
#[derive(Debug, Clone)]
pub struct AiProvider {
    pub kind: AiProviderKind,
    pub models: Vec<AiModel>,
    pub enabled: bool,
}

impl AiProvider {
    pub fn new(kind: AiProviderKind) -> Self {
        let models = Self::default_models_for(kind);
        Self {
            kind,
            models,
            enabled: true,
        }
    }

    fn default_models_for(kind: AiProviderKind) -> Vec<AiModel> {
        match kind {
            AiProviderKind::Anthropic => vec![
                AiModel::new("claude-sonnet-4-20250514", "Claude Sonnet 4", kind, 200_000)
                    .with_max_output(64_000),
                AiModel::new("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet", kind, 200_000)
                    .with_max_output(8192),
                AiModel::new("claude-3-5-haiku-20241022", "Claude 3.5 Haiku", kind, 200_000)
                    .with_max_output(8192),
                AiModel::new("claude-3-opus-20240229", "Claude 3 Opus", kind, 200_000)
                    .with_max_output(4096),
            ],
            AiProviderKind::OpenAi => vec![
                AiModel::new("gpt-4o", "GPT-4o", kind, 128_000).with_max_output(16_384),
                AiModel::new("gpt-4o-mini", "GPT-4o Mini", kind, 128_000).with_max_output(16_384),
                AiModel::new("o1", "o1", kind, 200_000).with_max_output(100_000),
                AiModel::new("o3-mini", "o3-mini", kind, 200_000).with_max_output(100_000),
                AiModel::new("gpt-4-turbo", "GPT-4 Turbo", kind, 128_000).with_max_output(4096),
            ],
            AiProviderKind::GoogleAi => vec![
                AiModel::new("gemini-2.0-flash", "Gemini 2.0 Flash", kind, 1_000_000)
                    .with_max_output(8192),
                AiModel::new("gemini-1.5-pro", "Gemini 1.5 Pro", kind, 2_000_000)
                    .with_max_output(8192),
                AiModel::new("gemini-1.5-flash", "Gemini 1.5 Flash", kind, 1_000_000)
                    .with_max_output(8192),
            ],
            AiProviderKind::DeepSeek => vec![
                AiModel::new("deepseek-chat", "DeepSeek Chat", kind, 64_000).with_max_output(8192),
                AiModel::new("deepseek-reasoner", "DeepSeek Reasoner", kind, 64_000)
                    .with_max_output(8192),
            ],
            AiProviderKind::Mistral => vec![
                AiModel::new("mistral-large-latest", "Mistral Large", kind, 128_000)
                    .with_max_output(4096),
                AiModel::new("mistral-small-latest", "Mistral Small", kind, 32_000)
                    .with_max_output(4096),
            ],
            AiProviderKind::XAi => {
                vec![AiModel::new("grok-2", "Grok 2", kind, 131_072).with_max_output(4096)]
            }
            AiProviderKind::Ollama => vec![
                AiModel::new("llama3.1", "Llama 3.1", kind, 128_000).with_max_output(4096),
                AiModel::new("mistral", "Mistral (local)", kind, 32_000).with_max_output(4096),
                AiModel::new("qwen2.5-coder", "Qwen 2.5 Coder", kind, 32_000).with_max_output(4096),
            ],
            AiProviderKind::LmStudio => {
                vec![AiModel::new("local-model", "Local Model", kind, 4096).with_max_output(4096)]
            }
            AiProviderKind::OpenRouter => vec![
                AiModel::new("anthropic/claude-sonnet-4", "Claude Sonnet 4 (OR)", kind, 200_000)
                    .with_max_output(64_000),
                AiModel::new("openai/gpt-4o", "GPT-4o (OR)", kind, 128_000).with_max_output(16_384),
            ],
            _ => vec![],
        }
    }

    pub fn get_model(&self, model_id: &str) -> Option<&AiModel> {
        self.models.iter().find(|m| m.id == model_id)
    }
}
