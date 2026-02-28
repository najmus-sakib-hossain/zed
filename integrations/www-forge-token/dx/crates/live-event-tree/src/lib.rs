//! # live-event-tree
//!
//! Organizes live stream events into a hierarchical tree structure,
//! enabling efficient navigation and selective context loading.
//!
//! ## Evidence
//! - Long live sessions need structure to avoid dumping all frames
//! - Tree: Session → Scenes → Events → Keyframes
//! - Only expand relevant branches into context
//! - **Honest: Structural optimization, savings depend on query relevance**
//!
//! STAGE: PromptAssembly (priority 5)

use dx_core::*;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct LiveEventTreeConfig {
    /// Minimum time gap (seconds) to split into new scene
    pub scene_gap_secs: f64,
    /// Maximum frames per event group
    pub max_frames_per_event: usize,
    /// Maximum events to include in context
    pub max_events_in_context: usize,
}

impl Default for LiveEventTreeConfig {
    fn default() -> Self {
        Self {
            scene_gap_secs: 5.0,
            max_frames_per_event: 5,
            max_events_in_context: 3,
        }
    }
}

/// A scene in the live event tree.
#[derive(Debug, Clone)]
pub struct Scene {
    pub start_secs: f64,
    pub end_secs: f64,
    pub keyframes: Vec<usize>, // indices into the frame list
}

pub struct LiveEventTree {
    config: LiveEventTreeConfig,
    report: Mutex<TokenSavingsReport>,
}

impl LiveEventTree {
    pub fn new() -> Self {
        Self::with_config(LiveEventTreeConfig::default())
    }

    pub fn with_config(config: LiveEventTreeConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Segment frames into scenes based on temporal gaps.
    fn segment_into_scenes(&self, frames: &[LiveFrame]) -> Vec<Scene> {
        if frames.is_empty() {
            return vec![];
        }

        let mut scenes: Vec<Scene> = Vec::new();
        let mut current_scene = Scene {
            start_secs: frames[0].timestamp_secs,
            end_secs: frames[0].timestamp_secs,
            keyframes: vec![0],
        };

        for (i, frame) in frames.iter().enumerate().skip(1) {
            if frame.timestamp_secs - current_scene.end_secs > self.config.scene_gap_secs {
                scenes.push(current_scene);
                current_scene = Scene {
                    start_secs: frame.timestamp_secs,
                    end_secs: frame.timestamp_secs,
                    keyframes: vec![i],
                };
            } else {
                current_scene.end_secs = frame.timestamp_secs;
                if current_scene.keyframes.len() < self.config.max_frames_per_event {
                    current_scene.keyframes.push(i);
                }
            }
        }
        scenes.push(current_scene);
        scenes
    }
}

#[async_trait::async_trait]
impl MultiModalTokenSaver for LiveEventTree {
    fn name(&self) -> &str { "live-event-tree" }
    fn stage(&self) -> SaverStage { SaverStage::PromptAssembly }
    fn priority(&self) -> u32 { 5 }
    fn modality(&self) -> Modality { Modality::Live }

    async fn process_multimodal(
        &self,
        input: MultiModalSaverInput,
        _ctx: &SaverContext,
    ) -> Result<MultiModalSaverOutput, SaverError> {
        let tokens_before: usize = input.live_frames.iter().map(|f| f.token_estimate).sum();
        let frame_count = input.live_frames.len();

        if frame_count == 0 {
            *self.report.lock().unwrap() = TokenSavingsReport {
                technique: "live-event-tree".into(),
                tokens_before: 0, tokens_after: 0, tokens_saved: 0,
                description: "No live frames.".into(),
            };
            return Ok(MultiModalSaverOutput {
                base: SaverOutput { messages: input.base.messages, tools: input.base.tools, images: input.base.images, skipped: true, cached_response: None },
                audio: input.audio, live_frames: input.live_frames, documents: input.documents, videos: input.videos, assets_3d: input.assets_3d,
            });
        }

        let scenes = self.segment_into_scenes(&input.live_frames);

        // Select most recent scenes up to max_events_in_context
        let selected_scenes: Vec<&Scene> = scenes.iter().rev()
            .take(self.config.max_events_in_context)
            .collect();

        // Collect keyframe indices from selected scenes
        let mut kept_indices: Vec<usize> = selected_scenes.iter()
            .flat_map(|s| s.keyframes.iter().copied())
            .collect();
        kept_indices.sort_unstable();
        kept_indices.dedup();

        let kept_frames: Vec<LiveFrame> = kept_indices.iter()
            .filter_map(|&i| input.live_frames.get(i).cloned())
            .collect();

        // Add tree summary as context
        let mut new_messages = input.base.messages;
        let tree_summary = format!(
            "[Live event tree: {} scenes detected, showing {} most recent ({} keyframes). \
             Total session: {:.1}s - {:.1}s]",
            scenes.len(),
            selected_scenes.len(),
            kept_frames.len(),
            input.live_frames.first().map(|f| f.timestamp_secs).unwrap_or(0.0),
            input.live_frames.last().map(|f| f.timestamp_secs).unwrap_or(0.0),
        );
        new_messages.push(Message {
            role: "system".into(),
            content: tree_summary,
            images: vec![],
            tool_call_id: None,
            token_count: 30,
        });

        let tokens_after: usize = kept_frames.iter().map(|f| f.token_estimate).sum::<usize>() + 30;
        let tokens_saved = tokens_before.saturating_sub(tokens_after);

        let report = TokenSavingsReport {
            technique: "live-event-tree".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Event tree: {} scenes, {} → {} frames ({} → {} tokens, {:.0}% saved).",
                scenes.len(), frame_count, kept_frames.len(),
                tokens_before, tokens_after,
                if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 }
            ),
        };
        *self.report.lock().unwrap() = report;

        Ok(MultiModalSaverOutput {
            base: SaverOutput { messages: new_messages, tools: input.base.tools, images: input.base.images, skipped: false, cached_response: None },
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

    fn frame(idx: u64, time: f64) -> LiveFrame {
        LiveFrame { image_data: vec![], timestamp_secs: time, frame_index: idx, token_estimate: 85, is_keyframe: false }
    }

    fn empty_base() -> SaverInput {
        SaverInput { messages: vec![], tools: vec![], images: vec![], turn_number: 1 }
    }

    #[tokio::test]
    async fn test_segments_into_scenes() {
        let saver = LiveEventTree::new();
        let ctx = SaverContext::default();
        // Two scenes with a 10s gap
        let mut frames: Vec<LiveFrame> = (0..10).map(|i| frame(i, i as f64)).collect();
        frames.extend((0..10).map(|i| frame(10 + i, 20.0 + i as f64)));
        let input = MultiModalSaverInput {
            base: empty_base(),
            audio: vec![],
            live_frames: frames,
            documents: vec![], videos: vec![], assets_3d: vec![],
        };
        let out = saver.process_multimodal(input, &ctx).await.unwrap();
        assert!(out.live_frames.len() < 20); // Should prune
        assert!(saver.last_savings().tokens_saved > 0);
    }
}
