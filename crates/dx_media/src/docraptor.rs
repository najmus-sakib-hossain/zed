//! DocRaptor — PrinceXML engine for pixel-perfect PDF generation.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// DocRaptor — HTML/CSS → PDF with best typography engine.
pub struct DocRaptorProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl DocRaptorProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::document_providers::docraptor(),
            api_key: std::env::var("DOCRAPTOR_API_KEY").ok(),
        }
    }
}

impl Default for DocRaptorProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for DocRaptorProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "DocRaptor" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Document]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "prince-pdf".to_string(),
                name: "PrinceXML PDF".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Document,
                pricing: None,
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("DocRaptor: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // Unlimited test (watermarked) + limited free tier
        None
    }
}
