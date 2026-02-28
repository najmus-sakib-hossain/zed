//! # live-token-prune
//!
//! Prunes old live frames from the context to stay within token budget.
//! Implements a sliding window that keeps only the most recent N frames.
//!
//! ## Evidence
//! - Live streams generate frames continuously (30-60 fps)
//! - Context windows fill up fast at 85+ tokens/frame
//! - Must aggressively prune old frames to maintain budget
//! - **Honest: Essential for live use cases, prevents context overflow**
//!
//! STAGE: PreCall (priority 3)

use dx_core::*;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct LiveTokenPruneConfig {
    /// Maximum number of live frames to keep
    pub max_frames: usize,
    /// Maximum total tokens for live frames
    pub max_live_tokens: usize,
    /// Keep frames evenly spaced when pruning (temporal sampling)
    pub temporal_sampling: bool,
}

impl Default for LiveTokenPruneConfig {
    fn default() -> Self {
        Self {
            max_frames: 20,
            max_live_tokens: 5_000,
            temporal_sampling: true,
        }
    }
}

pub struct LiveTokenPrune {
    config: LiveTokenPruneConfig,
    report: Mutex<TokenSavingsReport>,
}

impl LiveTokenPrune {
    pub fn new() -> Self {
        Self::with_config(LiveTokenPruneConfig::default())
    }

    pub fn with_config(config: LiveTokenPruneConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Select frames using temporal sampling (evenly spaced).
    fn temporal_sample(frames: Vec<LiveFrame>, max_count: usize) -> Vec<LiveFrame> {
        if frames.len() <= max_count {
            return frames;
        }
        let step = frames.len() as f64 / max_count as f64;
        let mut selected = Vec::with_capacity(max_count);
        for i in 0..max_count {
            let idx = (i as f64 * step).floor() as usize;
            if idx < frames.len() {
                selected.push(frames[idx].clone());
            }
        }
        // Always include the last frame
        if let Some(last) = frames.last() {
            if selected.last().map_or(true, |s| s.frame_index != last.frame_index) {
                if selected.len() >= max_count {
                    selected.pop();
                }
                selected.push(last.clone());
            }
        }
        selected
    }
}

#[async_trait::async_trait]
impl MultiModalTokenSaver for LiveTokenPrune {
    fn name(&self) -> &str { "live-token-prune" }
    fn stage(&self) -> SaverStage { SaverStage::PreCall }
    fn priority(&self) -> u32 { 3 }
    fn modality(&self) -> Modality { Modality::Live }

    async fn process_multimodal(
        &self,
        input: MultiModalSaverInput,
        _ctx: &SaverContext,
    ) -> Result<MultiModalSaverOutput, SaverError> {
        let tokens_before: usize = input.live_frames.iter().map(|f| f.token_estimate).sum();
        let frame_count = input.live_frames.len();

        if tokens_before <= self.config.max_live_tokens && frame_count <= self.config.max_frames {
            *self.report.lock().unwrap() = TokenSavingsReport {
                technique: "live-token-prune".into(),
                tokens_before, tokens_after: tokens_before, tokens_saved: 0,
                description: format!("Within limits: {} frames, {} tokens.", frame_count, tokens_before),
            };
            return Ok(MultiModalSaverOutput {
                base: SaverOutput { messages: input.base.messages, tools: input.base.tools, images: input.base.images, skipped: true, cached_response: None },
                audio: input.audio, live_frames: input.live_frames, documents: input.documents, videos: input.videos, assets_3d: input.assets_3d,
            });
        }

        // Determine how many frames we can keep
        let avg_tokens_per_frame = if frame_count > 0 { tokens_before / frame_count } else { 85 };
        let max_by_tokens = if avg_tokens_per_frame > 0 {
            self.config.max_live_tokens / avg_tokens_per_frame
        } else {
            self.config.max_frames
        };
        let target_count = self.config.max_frames.min(max_by_tokens).max(1);

        let pruned = if self.config.temporal_sampling {
            Self::temporal_sample(input.live_frames, target_count)
        } else {
            // Keep most recent
            let skip = frame_count.saturating_sub(target_count);
            input.live_frames.into_iter().skip(skip).collect()
        };

        let tokens_after: usize = pruned.iter().map(|f| f.token_estimate).sum();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);

        let report = TokenSavingsReport {
            technique: "live-token-prune".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Pruned live frames: {} → {} frames ({} → {} tokens, {:.0}% saved). \
                 Strategy: {}.",
                frame_count, pruned.len(), tokens_before, tokens_after,
                if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 },
                if self.config.temporal_sampling { "temporal sampling" } else { "keep recent" }
            ),
        };
        *self.report.lock().unwrap() = report;

        Ok(MultiModalSaverOutput {
            base: SaverOutput { messages: input.base.messages, tools: input.base.tools, images: input.base.images, skipped: false, cached_response: None },
            audio: input.audio,
            live_frames: pruned,
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

    fn frame(idx: u64) -> LiveFrame {
        LiveFrame { image_data: vec![0u8; 100], timestamp_secs: idx as f64, frame_index: idx, token_estimate: 85, is_keyframe: false }
    }

    fn empty_base() -> SaverInput {
        SaverInput { messages: vec![], tools: vec![], images: vec![], turn_number: 1 }
    }

    #[tokio::test]
    async fn test_prunes_excess_frames() {
        let config = LiveTokenPruneConfig { max_frames: 5, max_live_tokens: 1000, temporal_sampling: true };
        let saver = LiveTokenPrune::with_config(config);
        let ctx = SaverContext::default();
        let input = MultiModalSaverInput {
            base: empty_base(),
            audio: vec![],
            live_frames: (0..100).map(frame).collect(),
            documents: vec![], videos: vec![], assets_3d: vec![],
        };
        let out = saver.process_multimodal(input, &ctx).await.unwrap();
        assert!(out.live_frames.len() <= 5);
        assert!(saver.last_savings().tokens_saved > 0);
    }
}
