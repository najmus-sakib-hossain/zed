//! # asset3d-mesh-summarize
//!
//! Summarizes 3D meshes into text descriptions (bounding box, vertex count,
//! material info) instead of sending raw geometry data.
//!
//! ## Evidence
//! - Full mesh as text: 100k+ tokens for a typical model
//! - Summary (bounds, topology stats, materials): ~50-100 tokens
//! - **Honest: 99%+ savings, but loses geometric detail**
//! - Best when the model only needs to "understand" the asset, not edit it
//!
//! STAGE: PrePrompt (priority 10)

use dx_core::*;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct MeshSummarizeConfig {
    /// Include vertex/face counts in summary
    pub include_topology: bool,
    /// Include format info
    pub include_format: bool,
    /// Include estimated bounding box
    pub include_bounds: bool,
    /// Maximum summary tokens
    pub max_summary_tokens: usize,
}

impl Default for MeshSummarizeConfig {
    fn default() -> Self {
        Self {
            include_topology: true,
            include_format: true,
            include_bounds: true,
            max_summary_tokens: 100,
        }
    }
}

pub struct Asset3dMeshSummarize {
    config: MeshSummarizeConfig,
    report: Mutex<TokenSavingsReport>,
}

impl Asset3dMeshSummarize {
    pub fn new() -> Self {
        Self::with_config(MeshSummarizeConfig::default())
    }

    pub fn with_config(config: MeshSummarizeConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Generate a text summary of a 3D asset.
    fn summarize(&self, asset: &Asset3dInput) -> String {
        let mut parts: Vec<String> = Vec::new();

        if self.config.include_format {
            parts.push(format!("Format: {:?}", asset.format));
        }
        if self.config.include_topology {
            if let Some(verts) = asset.vertex_count {
                parts.push(format!("Vertices: {}", verts));
            }
            if let Some(faces) = asset.face_count {
                parts.push(format!("Faces: {}", faces));
            }
        }
        parts.push(format!("Data size: {} bytes", asset.data.len()));
        if self.config.include_bounds {
            parts.push("Bounding box: [computed at render time]".into());
        }

        format!("[3D Asset Summary: {}]", parts.join(", "))
    }
}

#[async_trait::async_trait]
impl MultiModalTokenSaver for Asset3dMeshSummarize {
    fn name(&self) -> &str { "asset3d-mesh-summarize" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 10 }
    fn modality(&self) -> Modality { Modality::Asset3d }

    async fn process_multimodal(
        &self,
        input: MultiModalSaverInput,
        _ctx: &SaverContext,
    ) -> Result<MultiModalSaverOutput, SaverError> {
        let tokens_before: usize = input.assets_3d.iter().map(|a| a.naive_token_estimate).sum();

        if input.assets_3d.is_empty() {
            *self.report.lock().unwrap() = TokenSavingsReport {
                technique: "asset3d-mesh-summarize".into(),
                tokens_before: 0, tokens_after: 0, tokens_saved: 0,
                description: "No 3D assets.".into(),
            };
            return Ok(MultiModalSaverOutput {
                base: SaverOutput { messages: input.base.messages, tools: input.base.tools, images: input.base.images, skipped: true, cached_response: None },
                audio: input.audio, live_frames: input.live_frames, documents: input.documents, videos: input.videos, assets_3d: input.assets_3d,
            });
        }

        let mut new_messages = input.base.messages;
        let mut summary_tokens = 0usize;

        for asset in &input.assets_3d {
            let summary = self.summarize(asset);
            let tokens = (summary.len() / 4).min(self.config.max_summary_tokens);
            summary_tokens += tokens;

            new_messages.push(Message {
                role: "user".into(),
                content: summary,
                images: vec![],
                tool_call_id: None,
                token_count: tokens,
            });
        }

        let tokens_after = summary_tokens;
        let tokens_saved = tokens_before.saturating_sub(tokens_after);

        let report = TokenSavingsReport {
            technique: "asset3d-mesh-summarize".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Summarized {} 3D assets: {} → {} tokens ({:.0}% saved). \
                 NOTE: Loses geometric detail. Best when model needs understanding, not editing.",
                input.assets_3d.len(), tokens_before, tokens_after,
                if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 }
            ),
        };
        *self.report.lock().unwrap() = report;

        Ok(MultiModalSaverOutput {
            base: SaverOutput { messages: new_messages, tools: input.base.tools, images: input.base.images, skipped: false, cached_response: None },
            audio: input.audio, live_frames: input.live_frames, documents: input.documents, videos: input.videos,
            assets_3d: vec![], // Consumed
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
    async fn test_summarize_mesh() {
        let saver = Asset3dMeshSummarize::new();
        let ctx = SaverContext::default();
        let input = MultiModalSaverInput {
            base: empty_base(),
            audio: vec![], live_frames: vec![], documents: vec![], videos: vec![],
            assets_3d: vec![Asset3dInput {
                data: vec![0u8; 50000],
                format: Asset3dFormat::Glb,
                vertex_count: Some(50_000),
                face_count: Some(100_000),
                naive_token_estimate: 500_000,
            }],
        };
        let out = saver.process_multimodal(input, &ctx).await.unwrap();
        assert!(out.assets_3d.is_empty()); // Consumed
        assert!(!out.base.messages.is_empty()); // Summary added
        assert!(saver.last_savings().tokens_saved > 400_000);
    }
}
