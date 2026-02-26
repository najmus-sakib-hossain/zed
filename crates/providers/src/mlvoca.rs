/// mlvoca.com — truly keyless LLM provider (Ollama-compatible API).
///
/// Endpoint: `https://mlvoca.com/api/generate`
/// Authentication: None required
/// Models: 2 (TinyLlama 1.1B, DeepSeek R1 1.5B)

pub const MLVOCA_API: &str = "https://mlvoca.com";

pub const MLVOCA_MODELS: [MlvocaModelDescriptor; 2] = [
    MlvocaModelDescriptor {
        id: "tinyllama",
        display_name: "TinyLlama",
        max_tokens: 131_000,
        max_output_tokens: 32_768,
    },
    MlvocaModelDescriptor {
        id: "deepseek-r1:1.5b",
        display_name: "DeepSeek R1 1.5B",
        max_tokens: 131_000,
        max_output_tokens: 32_768,
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MlvocaModelDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub max_tokens: u64,
    pub max_output_tokens: u64,
}
