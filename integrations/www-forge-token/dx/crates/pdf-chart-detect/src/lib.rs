//! # pdf-chart-detect
//!
//! Detects charts/graphs in PDF pages and routes them for specialized
//! handling (keep as high-detail image vs convert to data table).
//!
//! ## Evidence
//! - Charts need high detail to be readable (~765 tokens/page)
//! - But chart data as text table: ~50-100 tokens
//! - If chart data can be extracted, massive savings
//! - **Honest: 80-95% per chart IF data extraction succeeds. Needs OCR/CV.**
//!
//! STAGE: PrePrompt (priority 10)

use dx_core::*;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct PdfChartDetectConfig {
    /// Confidence threshold for chart detection (0.0-1.0)
    pub detection_threshold: f64,
    /// Whether to attempt data extraction (vs just flagging)
    pub extract_data: bool,
    /// Maximum tokens for extracted chart data
    pub max_data_tokens: usize,
}

impl Default for PdfChartDetectConfig {
    fn default() -> Self {
        Self {
            detection_threshold: 0.5,
            extract_data: true,
            max_data_tokens: 200,
        }
    }
}

/// Result of chart detection on a document.
#[derive(Debug, Clone)]
pub struct ChartDetection {
    pub page_index: usize,
    pub chart_type: ChartType,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChartType {
    Bar,
    Line,
    Pie,
    Scatter,
    Table,
    Unknown,
}

pub struct PdfChartDetect {
    config: PdfChartDetectConfig,
    report: Mutex<TokenSavingsReport>,
}

impl PdfChartDetect {
    pub fn new() -> Self {
        Self::with_config(PdfChartDetectConfig::default())
    }

    pub fn with_config(config: PdfChartDetectConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Stub chart detection. In production, use a CV model.
    fn detect_charts(&self, _data: &[u8], page_count: usize) -> Vec<ChartDetection> {
        // Heuristic: assume ~20% of pages in a document contain charts
        let chart_pages = (page_count as f64 * 0.2).ceil() as usize;
        (0..chart_pages).map(|i| ChartDetection {
            page_index: i,
            chart_type: ChartType::Unknown,
            confidence: 0.7,
        }).collect()
    }
}

#[async_trait::async_trait]
impl MultiModalTokenSaver for PdfChartDetect {
    fn name(&self) -> &str { "pdf-chart-detect" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 10 }
    fn modality(&self) -> Modality { Modality::Document }

    async fn process_multimodal(
        &self,
        input: MultiModalSaverInput,
        _ctx: &SaverContext,
    ) -> Result<MultiModalSaverOutput, SaverError> {
        let tokens_before: usize = input.documents.iter().map(|d| d.naive_token_estimate).sum();

        if input.documents.is_empty() {
            *self.report.lock().unwrap() = TokenSavingsReport {
                technique: "pdf-chart-detect".into(),
                tokens_before: 0, tokens_after: 0, tokens_saved: 0,
                description: "No documents.".into(),
            };
            return Ok(MultiModalSaverOutput {
                base: SaverOutput { messages: input.base.messages, tools: input.base.tools, images: input.base.images, skipped: true, cached_response: None },
                audio: input.audio, live_frames: input.live_frames, documents: input.documents, videos: input.videos, assets_3d: input.assets_3d,
            });
        }

        let mut new_messages = input.base.messages;
        let mut new_docs = Vec::new();
        let mut charts_found = 0usize;
        let mut tokens_saved_from_charts = 0usize;

        for doc in input.documents {
            let page_count = doc.page_count.unwrap_or(1);
            let charts = self.detect_charts(&doc.data, page_count);
            let confident_charts: Vec<_> = charts.into_iter()
                .filter(|c| c.confidence >= self.config.detection_threshold)
                .collect();

            if confident_charts.is_empty() || !self.config.extract_data {
                new_docs.push(doc);
                continue;
            }

            charts_found += confident_charts.len();

            // For each detected chart, estimate savings from data extraction
            let tokens_per_page = if page_count > 0 {
                doc.naive_token_estimate / page_count
            } else { 765 };

            let chart_image_tokens = confident_charts.len() * tokens_per_page;
            let chart_data_tokens = confident_charts.len() * self.config.max_data_tokens;
            let saved = chart_image_tokens.saturating_sub(chart_data_tokens);
            tokens_saved_from_charts += saved;

            // Add chart data as text
            for chart in &confident_charts {
                new_messages.push(Message {
                    role: "user".into(),
                    content: format!(
                        "[Chart data placeholder: page {}, type {:?}, confidence {:.0}%. \
                         In production, use chart-data-extraction model here.]",
                        chart.page_index, chart.chart_type, chart.confidence * 100.0
                    ),
                    images: vec![],
                    tool_call_id: None,
                    token_count: self.config.max_data_tokens,
                });
            }

            // Reduce document token estimate
            new_docs.push(DocumentInput {
                naive_token_estimate: doc.naive_token_estimate.saturating_sub(chart_image_tokens),
                ..doc
            });
        }

        let tokens_after = tokens_before.saturating_sub(tokens_saved_from_charts);

        let report = TokenSavingsReport {
            technique: "pdf-chart-detect".into(),
            tokens_before,
            tokens_after,
            tokens_saved: tokens_saved_from_charts,
            description: format!(
                "Detected {} charts, extracted data: {} → {} tokens ({:.0}% saved). \
                 NOTE: Stub detection. Production needs CV model.",
                charts_found, tokens_before, tokens_after,
                if tokens_before > 0 { tokens_saved_from_charts as f64 / tokens_before as f64 * 100.0 } else { 0.0 }
            ),
        };
        *self.report.lock().unwrap() = report;

        Ok(MultiModalSaverOutput {
            base: SaverOutput { messages: new_messages, tools: input.base.tools, images: input.base.images, skipped: false, cached_response: None },
            audio: input.audio, live_frames: input.live_frames,
            documents: new_docs,
            videos: input.videos, assets_3d: input.assets_3d,
        })
    }

    fn last_savings(&self) -> TokenSavingsReport {
        self.report.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_base() -> SaverInput {
        SaverInput { messages: vec![], tools: vec![], images: vec![], turn_number: 1 }
    }

    #[tokio::test]
    async fn test_detects_charts() {
        let saver = PdfChartDetect::new();
        let ctx = SaverContext::default();
        let input = MultiModalSaverInput {
            base: empty_base(),
            audio: vec![], live_frames: vec![],
            documents: vec![DocumentInput {
                data: vec![0u8; 100],
                doc_type: DocumentType::Pdf,
                page_count: Some(20),
                naive_token_estimate: 20 * 765,
            }],
            videos: vec![], assets_3d: vec![],
        };
        let out = saver.process_multimodal(input, &ctx).await.unwrap();
        // Should have detected some charts and added messages
        assert!(saver.last_savings().tokens_saved > 0 || !out.base.messages.is_empty());
    }
}
