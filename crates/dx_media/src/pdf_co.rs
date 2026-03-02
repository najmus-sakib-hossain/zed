//! PDF.co all-in-one document API adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// PDF.co — HTML→PDF, merge, edit, chart generation from data.
///
/// Free credits on signup (~10k operations).
/// API key from: pdf.co dashboard.
pub struct PdfCoProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl PdfCoProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::document_providers::pdf_co(),
            api_key: std::env::var("PDF_CO_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for PdfCoProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "PDF.co" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Document] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "html-to-pdf".into(),
                name: "PDF.co HTML to PDF".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Document,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.02),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "add-chart".into(),
                name: "PDF.co Add Chart".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Document,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.02),
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
        Err(anyhow::anyhow!("PDF.co: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.02))
    }
}
