//! Adobe PDF Services API adapter — HTML/Word/JSON to PDF.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Adobe PDF Services — HTML/Word/JSON to PDF, extract, OCR, combine.
///
/// Free tier: 500 free transactions/month.
/// API key from: developer.adobe.com → Console → Credentials.
pub struct AdobePdfProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl AdobePdfProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::document_providers::adobe_pdf(),
            api_key: std::env::var("ADOBE_PDF_CLIENT_ID").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for AdobePdfProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Adobe PDF Services" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Document] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "html-to-pdf".into(),
                name: "Adobe HTML to PDF".into(),
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
            },
            MediaModelInfo {
                id: "doc-gen".into(),
                name: "Adobe Document Generation".into(),
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
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Adobe PDF: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.05))
    }
}
