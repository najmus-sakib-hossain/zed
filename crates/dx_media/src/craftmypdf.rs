//! CraftMyPDF — Template-based PDF generation with charts and QR codes.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// CraftMyPDF — Drag-and-drop templates + HTML → PDF/images.
pub struct CraftMyPdfProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl CraftMyPdfProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::document_providers::craftmypdf(),
            api_key: std::env::var("CRAFTMYPDF_API_KEY").ok(),
        }
    }
}

impl Default for CraftMyPdfProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for CraftMyPdfProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "CraftMyPDF" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Document, MediaType::Image]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "template-pdf".to_string(),
                name: "Template → PDF".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Document,
                pricing: None,
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "template-image".to_string(),
                name: "Template → Image".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((2000, 2000)),
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("CraftMyPDF: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // 50 PDFs/images per month free (recurring)
        None
    }
}
