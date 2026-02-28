//! Unified Media Provider trait — Universe B (Media Generation).
//!
//! Every media provider (50+) implements this trait. Completely separate from
//! Universe A (LLM providers) — different registry, different cost tracking,
//! different API patterns.

use crate::cost::{MediaPricing, MicroCost};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// The type of media being generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Music,
    ThreeD,
    Document,
}

impl MediaType {
    pub fn display_name(&self) -> &'static str {
        match self {
            MediaType::Image => "Image",
            MediaType::Video => "Video",
            MediaType::Audio => "Audio",
            MediaType::Music => "Music",
            MediaType::ThreeD => "3D Model",
            MediaType::Document => "Document",
        }
    }
}

/// Unique identifier for a media provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MediaProviderId(pub Arc<str>);

impl MediaProviderId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for MediaProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Whether a media provider runs locally or in the cloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaProviderLocation {
    Local,
    Cloud,
}

/// A model available from a media provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaModelInfo {
    pub id: String,
    pub name: String,
    pub provider_id: MediaProviderId,
    pub media_type: MediaType,
    pub pricing: Option<MediaPricing>,
    pub supports_streaming: bool,
    pub max_resolution: Option<(u32, u32)>,
    pub max_duration_seconds: Option<u32>,
}

/// Request to generate media.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaGenerationRequest {
    pub model: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub media_type: MediaType,
    /// For images: width x height.
    pub dimensions: Option<(u32, u32)>,
    /// For video/audio: duration in seconds.
    pub duration_seconds: Option<f64>,
    /// Number of outputs to generate.
    pub count: u32,
    /// Style preset (provider-specific).
    pub style: Option<String>,
    /// Seed for reproducibility.
    pub seed: Option<u64>,
    /// Input image for img2img, style transfer, etc. (base64).
    pub input_image: Option<String>,
    /// Input audio for audio-to-audio tasks (base64).
    pub input_audio: Option<String>,
}

/// Progress of a media generation task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaGenerationProgress {
    /// 0.0 to 1.0.
    pub progress: f64,
    /// Current step description.
    pub status: String,
    /// Preview data (partial image bytes, etc.).
    pub preview: Option<Vec<u8>>,
}

/// A generated media output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaOutput {
    pub media_type: MediaType,
    /// Raw bytes of the generated media.
    #[serde(skip)]
    pub data: Vec<u8>,
    /// MIME type (e.g., "image/png", "video/mp4").
    pub mime_type: String,
    /// File extension (e.g., "png", "mp4").
    pub extension: String,
    /// Generation cost.
    pub cost: MicroCost,
    /// Optional local file path if saved.
    pub saved_path: Option<PathBuf>,
    /// Width x Height for images/video.
    pub dimensions: Option<(u32, u32)>,
    /// Duration in seconds for video/audio.
    pub duration_seconds: Option<f64>,
    /// Seed used for generation.
    pub seed: Option<u64>,
}

/// The core trait every media provider must implement.
///
/// This is the heart of Universe B — the unified interface for 50+ media providers.
#[async_trait::async_trait]
pub trait MediaProvider: Send + Sync {
    /// Unique provider identifier.
    fn id(&self) -> &MediaProviderId;

    /// Human-readable provider name.
    fn name(&self) -> &str;

    /// What types of media this provider generates.
    fn supported_media_types(&self) -> &[MediaType];

    /// Whether this provider runs locally or in the cloud.
    fn location(&self) -> MediaProviderLocation;

    /// Whether this provider is currently available.
    fn is_available(&self) -> bool;

    /// List available models from this provider.
    async fn list_models(&self) -> Result<Vec<MediaModelInfo>>;

    /// Generate media from a prompt.
    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>>;

    /// Estimate cost before generating.
    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost>;
}

// ---------------------------------------------------------------------------
// Well-known image providers (Part 10)
// ---------------------------------------------------------------------------

/// Image provider identifiers.
pub mod image_providers {
    use super::MediaProviderId;

    pub fn openai() -> MediaProviderId { MediaProviderId::new("openai-image") }
    pub fn fal_ai() -> MediaProviderId { MediaProviderId::new("fal-ai") }
    pub fn stability_ai() -> MediaProviderId { MediaProviderId::new("stability-ai") }
    pub fn replicate() -> MediaProviderId { MediaProviderId::new("replicate") }
    pub fn google_imagen() -> MediaProviderId { MediaProviderId::new("google-imagen") }
    pub fn midjourney() -> MediaProviderId { MediaProviderId::new("midjourney") }
    pub fn adobe_firefly() -> MediaProviderId { MediaProviderId::new("adobe-firefly") }
    pub fn deepseek_janus() -> MediaProviderId { MediaProviderId::new("deepseek-janus") }
    pub fn black_forest_labs() -> MediaProviderId { MediaProviderId::new("black-forest-labs") }
    pub fn recraft() -> MediaProviderId { MediaProviderId::new("recraft") }
    pub fn ideogram() -> MediaProviderId { MediaProviderId::new("ideogram") }
    pub fn local_sdxl() -> MediaProviderId { MediaProviderId::new("local-sdxl") }
}

// ---------------------------------------------------------------------------
// Well-known video providers (Part 11)
// ---------------------------------------------------------------------------

pub mod video_providers {
    use super::MediaProviderId;

    pub fn runway() -> MediaProviderId { MediaProviderId::new("runway") }
    pub fn kling_ai() -> MediaProviderId { MediaProviderId::new("kling-ai") }
    pub fn pika() -> MediaProviderId { MediaProviderId::new("pika") }
    pub fn luma_ai() -> MediaProviderId { MediaProviderId::new("luma-ai") }
    pub fn google_veo() -> MediaProviderId { MediaProviderId::new("google-veo") }
    pub fn openai_sora() -> MediaProviderId { MediaProviderId::new("openai-sora") }
    pub fn minimax() -> MediaProviderId { MediaProviderId::new("minimax") }
    pub fn synthesia() -> MediaProviderId { MediaProviderId::new("synthesia") }
    pub fn heygen() -> MediaProviderId { MediaProviderId::new("heygen") }
}

// ---------------------------------------------------------------------------
// Well-known music providers (Part 12)
// ---------------------------------------------------------------------------

pub mod music_providers {
    use super::MediaProviderId;

    pub fn suno_ai() -> MediaProviderId { MediaProviderId::new("suno-ai") }
    pub fn udio() -> MediaProviderId { MediaProviderId::new("udio") }
    pub fn stability_audio() -> MediaProviderId { MediaProviderId::new("stability-audio") }
    pub fn musicgen() -> MediaProviderId { MediaProviderId::new("musicgen") }
    pub fn google_musicfx() -> MediaProviderId { MediaProviderId::new("google-musicfx") }
    pub fn aiva() -> MediaProviderId { MediaProviderId::new("aiva") }
    pub fn mubert() -> MediaProviderId { MediaProviderId::new("mubert") }
}

// ---------------------------------------------------------------------------
// Well-known 3D providers (Part 13)
// ---------------------------------------------------------------------------

pub mod threed_providers {
    use super::MediaProviderId;

    pub fn meshy() -> MediaProviderId { MediaProviderId::new("meshy") }
    pub fn tripo_ai() -> MediaProviderId { MediaProviderId::new("tripo-ai") }
    pub fn luma_genie() -> MediaProviderId { MediaProviderId::new("luma-genie") }
    pub fn stability_triposr() -> MediaProviderId { MediaProviderId::new("stability-triposr") }
    pub fn openai_shape() -> MediaProviderId { MediaProviderId::new("openai-shap-e") }
    pub fn csm() -> MediaProviderId { MediaProviderId::new("csm") }
    pub fn kaedim() -> MediaProviderId { MediaProviderId::new("kaedim") }
    pub fn rodin_ai() -> MediaProviderId { MediaProviderId::new("rodin-ai") }
    pub fn local_triposr() -> MediaProviderId { MediaProviderId::new("local-triposr") }
}
