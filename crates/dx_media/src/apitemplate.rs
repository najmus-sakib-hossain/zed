//! APITemplate.io document/image generation adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// APITemplate.io — WYSIWYG/HTML/Markdown to PDF + images with charts.
///
/// Free tier: 50 PDFs/images per month.
/// API key from: apitemplate.io dashboard.
pub struct ApiTemplateProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl ApiTemplateProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::document_providers::apitemplate(),
            api_key: std::env::var("APITEMPLATE_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for ApiTemplateProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "APITemplate.io" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Document] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "apitemplate-pdf".into(),
            name: "APITemplate PDF Generator".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Document,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.05),
                per_second: None,
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: None,
            max_duration_seconds: None,
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("APITemplate.io: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.05))
    }
}
