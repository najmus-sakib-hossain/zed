//! Hugging Face Inference API — access open-source media models.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// Hugging Face Inference API — run open-source models (Flux, SD, MusicGen, etc.).
pub struct HuggingFaceProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl HuggingFaceProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::huggingface(),
            api_key: std::env::var("HF_TOKEN")
                .or_else(|_| std::env::var("HUGGINGFACE_API_KEY"))
                .ok(),
        }
    }
}

impl Default for HuggingFaceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for HuggingFaceProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Hugging Face" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image, MediaType::Audio, MediaType::Music, MediaType::ThreeD]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "stabilityai/stable-diffusion-xl-base-1.0".to_string(),
                name: "SDXL Base".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1024, 1024)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "black-forest-labs/FLUX.1-schnell".to_string(),
                name: "FLUX.1 Schnell".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1024, 1024)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "facebook/musicgen-small".to_string(),
                name: "MusicGen Small".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Music,
                pricing: None,
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: Some(30),
            },
            MediaModelInfo {
                id: "stabilityai/triposr".to_string(),
                name: "TripoSR (Image-to-3D)".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::ThreeD,
                pricing: None,
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Hugging Face: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // Free tier with rate limits, paid for dedicated endpoints
        None
    }
}
