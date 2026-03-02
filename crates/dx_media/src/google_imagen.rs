//! Google Imagen 3 image generation adapter via Gemini API / Vertex AI.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Google Imagen — high-quality image generation via Gemini API.
///
/// Free tier: generous daily quota (no card needed in most regions).
/// API key from: aistudio.google.com/apikey or cloud.google.com/vertex-ai
pub struct GoogleImagenProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl GoogleImagenProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::google_imagen(),
            api_key: std::env::var("GOOGLE_AI_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for GoogleImagenProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Google Imagen" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Image] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "imagen-3.0-generate-002".into(),
                name: "Imagen 3".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.04),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: Some((2048, 2048)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "imagen-3.0-fast-generate-001".into(),
                name: "Imagen 3 Fast".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.02),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: Some((1024, 1024)),
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _api_key = self.api_key.as_deref()
            .ok_or_else(|| anyhow::anyhow!("Google Imagen: API key not configured"))?;

        // POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateImages
        let _ = request;
        Err(anyhow::anyhow!("Google Imagen: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.04))
    }
}
