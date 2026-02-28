//! Fal.ai media provider — 600+ models, fastest inference.
//!
//! Fal.ai is an API aggregator providing access to image, video, and audio models.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider,
    MediaProviderId, MediaProviderLocation, MediaType, MicroCost,
};
use std::sync::Arc;

/// Fal.ai provider configuration.
#[derive(Debug, Clone)]
pub struct FalAiConfig {
    pub api_key: String,
    pub base_url: String,
}

impl Default for FalAiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://fal.run".into(),
        }
    }
}

/// Fal.ai media provider — supports 600+ image/video/audio models.
pub struct FalAiProvider {
    id: MediaProviderId,
    config: FalAiConfig,
    available: bool,
}

impl FalAiProvider {
    pub fn new(config: FalAiConfig) -> Self {
        let available = !config.api_key.is_empty();
        Self {
            id: dx_core::image_providers::fal_ai(),
            config,
            available,
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for FalAiProvider {
    fn id(&self) -> &MediaProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "Fal.ai"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image, MediaType::Video, MediaType::Audio]
    }

    fn location(&self) -> MediaProviderLocation {
        MediaProviderLocation::Cloud
    }

    fn is_available(&self) -> bool {
        self.available
    }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        // Return some well-known models. Full list comes from the API.
        Ok(vec![
            MediaModelInfo {
                id: "fal-ai/flux/dev".into(),
                name: "FLUX.1 Dev".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((2048, 2048)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "fal-ai/flux/schnell".into(),
                name: "FLUX.1 Schnell".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1536, 1536)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "fal-ai/stable-diffusion-xl".into(),
                name: "Stable Diffusion XL".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1024, 1024)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "fal-ai/recraft-v3".into(),
                name: "Recraft V3".into(),
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
        let _url = format!("{}/{}", self.config.base_url, request.model);

        // Build the request body.
        let mut body = serde_json::json!({
            "prompt": request.prompt,
        });

        if let Some(neg) = &request.negative_prompt {
            body["negative_prompt"] = serde_json::Value::String(neg.clone());
        }

        if let Some((w, h)) = request.dimensions {
            body["image_size"] = serde_json::json!({ "width": w, "height": h });
        }

        if let Some(seed) = request.seed {
            body["seed"] = serde_json::Value::Number(serde_json::Number::from(seed));
        }

        body["num_images"] = serde_json::Value::Number(serde_json::Number::from(request.count));

        log::info!("Fal.ai generate request for model={}: {:?}", request.model, body);

        // Placeholder: actual HTTP call would go here via http_client.
        // For now, return an empty result indicating the provider is wired.
        Ok(vec![MediaOutput {
            media_type: request.media_type,
            data: Vec::new(),
            mime_type: "image/png".into(),
            extension: "png".into(),
            cost: MicroCost::from_dollars(0.03),
            saved_path: None,
            dimensions: request.dimensions,
            duration_seconds: None,
            seed: request.seed,
        }])
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.03))
    }
}
