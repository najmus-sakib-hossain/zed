//! QuickChart.io — Chart.js to PNG/SVG/PDF chart images.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// QuickChart — Generate chart images from Chart.js config.
pub struct QuickChartProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl QuickChartProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::document_providers::quickchart(),
            // API key optional for community tier (high limits)
            api_key: std::env::var("QUICKCHART_API_KEY").ok(),
        }
    }
}

impl Default for QuickChartProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for QuickChartProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "QuickChart" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Document, MediaType::Image]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    
    // QuickChart works without API key (high free limits)
    fn is_available(&self) -> bool { true }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "chart-to-png".to_string(),
                name: "Chart.js → PNG".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((2000, 2000)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "chart-to-svg".to_string(),
                name: "Chart.js → SVG".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "chart-to-pdf".to_string(),
                name: "Chart.js → PDF".to_string(),
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
        Err(anyhow::anyhow!("QuickChart: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // Free community tier (~100k/month)
        Some(MicroCost(0))
    }
}
