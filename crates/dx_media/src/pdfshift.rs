//! PDFShift — HTML/URL → high-fidelity PDF.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// PDFShift — Convert HTML or URLs to pixel-perfect PDFs.
pub struct PdfShiftProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl PdfShiftProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::document_providers::pdfshift(),
            api_key: std::env::var("PDFSHIFT_API_KEY").ok(),
        }
    }
}

impl Default for PdfShiftProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for PdfShiftProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "PDFShift" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Document]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "html-to-pdf".to_string(),
                name: "HTML → PDF".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Document,
                pricing: None,
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "url-to-pdf".to_string(),
                name: "URL → PDF".to_string(),
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
        Err(anyhow::anyhow!("PDFShift: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // 50 PDFs/month free (permanent)
        None
    }
}
