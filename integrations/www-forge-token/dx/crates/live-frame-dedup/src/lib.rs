//! # live-frame-dedup
//!
//! Deduplicates consecutive frames in live/streaming video feeds
//! by detecting static or near-static scenes.
//!
//! ## Evidence
//! - Screen share / webcam often has 90%+ identical consecutive frames
//! - At 85 tokens/frame (low detail), dropping duplicates saves massively
//! - Even crude pixel-diff can catch static screens
//! - **Honest: 50-95% frame reduction depending on content dynamism**
//!
//! STAGE: PrePrompt (priority 2)

use dx_core::*;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct LiveFrameDedupConfig {
    /// Maximum pixel difference ratio (0.0-1.0) to consider frames identical
    pub similarity_threshold: f64,
    /// Minimum interval between kept frames (seconds)
    pub min_frame_interval_secs: f64,
    /// Always keep every Nth frame regardless of similarity
    pub force_keep_every_n: usize,
}

impl Default for LiveFrameDedupConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.02, // 2% pixel difference
            min_frame_interval_secs: 1.0,
            force_keep_every_n: 30,
        }
    }
}

pub struct LiveFrameDedup {
    config: LiveFrameDedupConfig,
    report: Mutex<TokenSavingsReport>,
}

impl LiveFrameDedup {
    pub fn new() -> Self {
        Self::with_config(LiveFrameDedupConfig::default())
    }

    pub fn with_config(config: LiveFrameDedupConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Compute a simple byte-level difference ratio between two frames.
    fn frame_diff_ratio(a: &[u8], b: &[u8]) -> f64 {
        if a.is_empty() || b.is_empty() || a.len() != b.len() {
            return 1.0; // Different sizes = different frames
        }
        let diff_count = a.iter().zip(b.iter())
            .filter(|(x, y)| (**x as i16 - **y as i16).unsigned_abs() > 10)
            .count();
        diff_count as f64 / a.len() as f64
    }
}

#[async_trait::async_trait]
impl MultiModalTokenSaver for LiveFrameDedup {
    fn name(&self) -> &str { "live-frame-dedup" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 2 }
    fn modality(&self) -> Modality { Modality::Live }

    async fn process_multimodal(
        &self,
        input: MultiModalSaverInput,
        _ctx: &SaverContext,
    ) -> Result<MultiModalSaverOutput, SaverError> {
        let tokens_before: usize = input.live_frames.iter().map(|f| f.token_estimate).sum();
        let frame_count = input.live_frames.len();

        if frame_count <= 1 {
            *self.report.lock().unwrap() = TokenSavingsReport {
                technique: "live-frame-dedup".into(),
                tokens_before, tokens_after: tokens_before, tokens_saved: 0,
                description: format!("Only {} frame(s), nothing to dedup.", frame_count),
            };
            return Ok(MultiModalSaverOutput {
                base: SaverOutput { messages: input.base.messages, tools: input.base.tools, images: input.base.images, skipped: true, cached_response: None },
                audio: input.audio, live_frames: input.live_frames, documents: input.documents, videos: input.videos, assets_3d: input.assets_3d,
            });
        }

        let mut kept_frames: Vec<LiveFrame> = Vec::new();
        let mut last_kept_data: Option<Vec<u8>> = None;
        let mut last_kept_time: f64 = -f64::INFINITY;

        for (i, frame) in input.live_frames.into_iter().enumerate() {
            let force_keep = i % self.config.force_keep_every_n == 0;
            let time_ok = (frame.timestamp_secs - last_kept_time) >= self.config.min_frame_interval_secs;

            if force_keep || frame.is_keyframe {
                last_kept_data = Some(frame.image_data.clone());
                last_kept_time = frame.timestamp_secs;
                kept_frames.push(frame);
                continue;
            }

            if !time_ok {
                continue; // Too soon since last kept frame
            }

            let is_different = match &last_kept_data {
                Some(prev) => Self::frame_diff_ratio(prev, &frame.image_data) > self.config.similarity_threshold,
                None => true,
            };

            if is_different {
                last_kept_data = Some(frame.image_data.clone());
                last_kept_time = frame.timestamp_secs;
                kept_frames.push(frame);
            }
        }

        let tokens_after: usize = kept_frames.iter().map(|f| f.token_estimate).sum();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);

        let report = TokenSavingsReport {
            technique: "live-frame-dedup".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Deduped live frames: {} → {} frames ({} → {} tokens, {:.0}% saved). \
                 Threshold: {:.1}% pixel diff, min interval: {:.1}s.",
                frame_count, kept_frames.len(), tokens_before, tokens_after,
                if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 },
                self.config.similarity_threshold * 100.0,
                self.config.min_frame_interval_secs
            ),
        };
        *self.report.lock().unwrap() = report;

        Ok(MultiModalSaverOutput {
            base: SaverOutput { messages: input.base.messages, tools: input.base.tools, images: input.base.images, skipped: false, cached_response: None },
            audio: input.audio,
            live_frames: kept_frames,
            documents: input.documents,
            videos: input.videos,
            assets_3d: input.assets_3d,
        })
    }

    fn last_savings(&self) -> TokenSavingsReport {
        self.report.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(idx: u64, time: f64, data: &[u8]) -> LiveFrame {
        LiveFrame { image_data: data.to_vec(), timestamp_secs: time, frame_index: idx, token_estimate: 85, is_keyframe: false }
    }

    fn empty_base() -> SaverInput {
        SaverInput { messages: vec![], tools: vec![], images: vec![], turn_number: 1 }
    }

    #[tokio::test]
    async fn test_dedup_identical_frames() {
        let saver = LiveFrameDedup::new();
        let ctx = SaverContext::default();
        let same_data = vec![100u8; 1000];
        let input = MultiModalSaverInput {
            base: empty_base(),
            audio: vec![],
            live_frames: (0..60).map(|i| frame(i, i as f64 * 0.5, &same_data)).collect(),
            documents: vec![], videos: vec![], assets_3d: vec![],
        };
        let out = saver.process_multimodal(input, &ctx).await.unwrap();
        // Most identical frames should be dropped
        assert!(out.live_frames.len() < 30);
        assert!(saver.last_savings().tokens_saved > 0);
    }
}
