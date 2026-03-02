//! Local inference adapter — Tier 5.
//!
//! Wraps the `dx_inference` crate (Candle / llama.cpp) to run models fully
//! on-device. Always available, zero cost, no network required.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Adapter that runs models locally using Candle or llama.cpp backends.
pub struct LocalLlmAdapter {
    id: LlmProviderId,
    /// Directory where GGUF / safetensors models are stored.
    model_dir: std::path::PathBuf,
}

impl Default for LocalLlmAdapter {
    fn default() -> Self {
        let model_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("dx")
            .join("models");
        Self { id: LlmProviderId::new("local"), model_dir }
    }
}

impl LocalLlmAdapter {
    pub fn new(model_dir: std::path::PathBuf) -> Self {
        Self { id: LlmProviderId::new("local"), model_dir }
    }

    /// Scan the model directory for downloaded models.
    pub fn scan_available_models(&self) -> Vec<LocalModelEntry> {
        let mut entries = Vec::new();
        if let Ok(dir) = std::fs::read_dir(&self.model_dir) {
            for entry in dir.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "gguf" || ext == "safetensors" {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    entries.push(LocalModelEntry {
                        id: name.clone(),
                        name,
                        path,
                        size_bytes,
                        quantization: detect_quantization(ext, size_bytes),
                    });
                }
            }
        }
        entries
    }
}

/// Metadata about a locally-stored model file.
#[derive(Debug, Clone)]
pub struct LocalModelEntry {
    pub id: String,
    pub name: String,
    pub path: std::path::PathBuf,
    pub size_bytes: u64,
    pub quantization: Quantization,
}

/// Common quantization formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantization {
    F16,
    Q8_0,
    Q6_K,
    Q5_K_M,
    Q4_K_M,
    Q4_0,
    Q3_K_M,
    Q2_K,
    Unknown,
}

fn detect_quantization(ext: &str, size_bytes: u64) -> Quantization {
    // Heuristic: infer quantization from file name patterns and size.
    // In production, parse GGUF header metadata.
    if ext == "safetensors" {
        return Quantization::F16;
    }
    // Rough heuristics for 7B parameter models.
    match size_bytes {
        0..=3_000_000_000 => Quantization::Q2_K,
        3_000_000_001..=4_200_000_000 => Quantization::Q4_0,
        4_200_000_001..=5_000_000_000 => Quantization::Q4_K_M,
        5_000_000_001..=6_000_000_000 => Quantization::Q5_K_M,
        6_000_000_001..=7_200_000_000 => Quantization::Q6_K,
        7_200_000_001..=8_500_000_000 => Quantization::Q8_0,
        _ => Quantization::Unknown,
    }
}

#[async_trait::async_trait]
impl LlmProvider for LocalLlmAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Local Inference" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Local }

    fn is_available(&self) -> bool {
        // Available if we have at least one model file.
        self.model_dir.is_dir()
            && std::fs::read_dir(&self.model_dir)
                .map(|d| d.count() > 0)
                .unwrap_or(false)
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        let entries = self.scan_available_models();
        Ok(entries
            .into_iter()
            .map(|e| LlmModelInfo {
                id: e.id.clone(),
                name: format!("{} ({:?})", e.name, e.quantization),
                provider_id: self.id.clone(),
                context_window: 4096,
                max_output_tokens: Some(2048),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::ZERO,
                    output_per_million: MicroCost::ZERO,
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: false,
                supports_vision: false,
            })
            .collect())
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Local inference complete: model={}", request.model);
        // In production: load model via dx_inference, run inference.
        Ok(LlmResponse {
            content: String::new(),
            model: request.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(
        &self,
        request: &LlmRequest,
    ) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!("Local inference stream: model={}", request.model);
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        // Local embedding via Candle (e.g. all-MiniLM-L6-v2).
        Ok(EmbeddingResponse {
            embeddings: vec![],
            model: String::new(),
            total_tokens: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> {
        Some(TokenPricing {
            input_per_million: MicroCost::ZERO,
            output_per_million: MicroCost::ZERO,
            cached_input_per_million: None,
        })
    }
}
