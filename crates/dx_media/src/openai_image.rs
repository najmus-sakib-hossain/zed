//! OpenAI image generation provider — DALL-E 3, GPT-Image-1.5.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider,
    MediaProviderId, MediaProviderLocation, MediaType, MicroCost,
};

/// OpenAI image provider configuration.
#[derive(Debug, Clone)]
pub struct OpenAiImageConfig {
    pub api_key: String,
    pub base_url: String,
}

impl Default for OpenAiImageConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".into(),
        }
    }
}

/// OpenAI image generation provider.
pub struct OpenAiImageProvider {
    id: MediaProviderId,
    config: OpenAiImageConfig,
}

impl OpenAiImageProvider {
    pub fn new(config: OpenAiImageConfig) -> Self {
        Self {
            id: dx_core::image_providers::openai(),
            config,
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for OpenAiImageProvider {
    fn id(&self) -> &MediaProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "OpenAI Images"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image]
    }

    fn location(&self) -> MediaProviderLocation {
        MediaProviderLocation::Cloud
    }

    fn is_available(&self) -> bool {
        !self.config.api_key.is_empty()
    }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "dall-e-3".into(),
                name: "DALL-E 3".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1792, 1024)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "gpt-image-1".into(),
                name: "GPT Image 1".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((2048, 2048)),
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let (w, h) = request.dimensions.unwrap_or((1024, 1024));
        let size = format!("{}x{}", w, h);

        let body = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "n": request.count,
            "size": size,
            "response_format": "b64_json",
        });

        log::info!("OpenAI image generate: {:?}", body);

        Ok(vec![MediaOutput {
            media_type: MediaType::Image,
            data: Vec::new(),
            mime_type: "image/png".into(),
            extension: "png".into(),
            cost: MicroCost::from_dollars(0.04),
            saved_path: None,
            dimensions: Some((w, h)),
            duration_seconds: None,
            seed: None,
        }])
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.04))
    }
}
