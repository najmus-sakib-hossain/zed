//! # dx-core
//!
//! Shared types, traits, and utilities for the dx token-saving pipeline.
//!
//! All 44 token-saving crates depend on this crate and `use dx_core::*`.
//!
//! ## Core Types
//! - [`TokenSaver`] — the main trait all text/image savers implement
//! - [`MultiModalTokenSaver`] — trait for audio/video/document savers
//! - [`SaverInput`] / [`SaverOutput`] — data flowing through the pipeline
//! - [`TokenSavingsReport`] — tracks savings per saver per turn

// ─── Re-export everything for `use dx_core::*` ─────────────────────────────
pub use self::error::SaverError;
pub use self::report::TokenSavingsReport;
pub use self::stage::SaverStage;
pub use self::types::*;
pub use self::traits::{TokenSaver, MultiModalTokenSaver};

// ─── Modules ────────────────────────────────────────────────────────────────

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error, Clone)]
    pub enum SaverError {
        #[error("saver failed: {0}")]
        Failed(String),
        #[error("saver skipped: {0}")]
        Skipped(String),
        #[error("invalid input: {0}")]
        InvalidInput(String),
    }
}

pub mod report {
    use serde::{Deserialize, Serialize};

    /// Tracks token savings for a single saver invocation.
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct TokenSavingsReport {
        /// Name of the saver technique
        pub technique: String,
        /// Tokens before this saver ran
        pub tokens_before: usize,
        /// Tokens after this saver ran
        pub tokens_after: usize,
        /// Tokens saved (tokens_before - tokens_after)
        pub tokens_saved: usize,
        /// Human-readable description of what was saved
        pub description: String,
    }

    impl TokenSavingsReport {
        /// Savings percentage (0.0-100.0)
        pub fn savings_pct(&self) -> f64 {
            if self.tokens_before == 0 {
                return 0.0;
            }
            self.tokens_saved as f64 / self.tokens_before as f64 * 100.0
        }
    }
}

pub mod stage {
    /// Pipeline stage in which a saver runs.
    ///
    /// The order is: CallElimination → PrePrompt → PromptAssembly →
    ///               PreCall → [API] → PostResponse → InterTurn
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum SaverStage {
        /// Stage 1: Try to avoid the API call entirely (cache hits)
        CallElimination,
        /// Stage 2: Process/compress input before building the prompt
        PrePrompt,
        /// Stage 3: Assemble the prompt efficiently (tools, prefix ordering)
        PromptAssembly,
        /// Stage 4: Final checks before sending to the API
        PreCall,
        /// Stage 5: Process the response (truncate outputs, compress)
        PostResponse,
        /// Stage 6: Clean up history between turns
        InterTurn,
    }
}

pub mod types {
    use serde::{Deserialize, Serialize};

    // ── Core message/tool types ──────────────────────────────────────────────

    /// A single message in the conversation history.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Message {
        /// Role: "system", "user", "assistant", or "tool"
        pub role: String,
        /// Text content of the message
        pub content: String,
        /// Inline images attached to this message
        pub images: Vec<ImageInput>,
        /// Tool call ID (for "tool" role messages)
        pub tool_call_id: Option<String>,
        /// Estimated token count for this message
        pub token_count: usize,
    }

    /// A tool/function schema sent to the LLM.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ToolSchema {
        /// Tool name
        pub name: String,
        /// Tool description (may be minified)
        pub description: String,
        /// JSON schema of the tool's parameters
        pub parameters: serde_json::Value,
        /// Estimated token count for this schema
        pub token_count: usize,
    }

    /// An image to be sent to a vision-capable model.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ImageInput {
        /// Raw image bytes (JPEG, PNG, etc.)
        pub data: Vec<u8>,
        /// MIME type (e.g. "image/jpeg")
        pub mime: String,
        /// Detail level for token cost calculation
        pub detail: ImageDetail,
        /// Token estimate before any compression
        pub original_tokens: usize,
        /// Token estimate after compression
        pub processed_tokens: usize,
    }

    /// Detail level for vision API calls.
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub enum ImageDetail {
        /// 85 tokens flat regardless of size
        Low,
        /// Higher quality, tile-based token cost
        High,
        /// Provider decides based on image size
        Auto,
    }

    // ── Pipeline I/O ──────────────────────────────────────────────────────────

    /// Input to a saver in the pipeline.
    #[derive(Debug, Clone)]
    pub struct SaverInput {
        /// Conversation messages
        pub messages: Vec<Message>,
        /// Tool schemas to include
        pub tools: Vec<ToolSchema>,
        /// Images to include
        pub images: Vec<ImageInput>,
        /// Which turn we are on (1-indexed)
        pub turn_number: usize,
    }

    /// Output from a saver in the pipeline.
    #[derive(Debug, Clone)]
    pub struct SaverOutput {
        /// Processed messages (may be fewer or shorter)
        pub messages: Vec<Message>,
        /// Processed tool schemas (may be fewer)
        pub tools: Vec<ToolSchema>,
        /// Processed images (may be fewer or compressed)
        pub images: Vec<ImageInput>,
        /// If true, the saver determined the API call should be skipped
        pub skipped: bool,
        /// A cached response, if available (only set when skipped = true)
        pub cached_response: Option<String>,
    }

    /// Context passed to each saver's `process` call.
    #[derive(Debug, Clone, Default)]
    pub struct SaverContext {
        /// Natural-language description of the current task
        pub task_description: String,
        /// Current turn number
        pub turn_number: usize,
        /// Target model name (e.g. "gpt-4o", "claude-3-5-sonnet")
        pub model: String,
        /// Token budget remaining
        pub token_budget: Option<usize>,
    }

    // ── Multimodal types ─────────────────────────────────────────────────────

    /// Audio input for the pipeline.
    #[derive(Clone, Debug)]
    pub struct AudioInput {
        /// Raw audio bytes
        pub data: Vec<u8>,
        /// Audio format
        pub format: AudioFormat,
        /// Sample rate in Hz
        pub sample_rate: u32,
        /// Duration in seconds
        pub duration_secs: f64,
        /// Number of channels
        pub channels: u16,
        /// Tokens estimated if sent naively (no compression)
        pub naive_token_estimate: usize,
        /// Tokens after compression
        pub compressed_tokens: usize,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum AudioFormat {
        Wav,
        Mp3,
        Ogg,
        Flac,
        Pcm16,
        Aac,
    }

    /// A single frame from a live/streaming source.
    #[derive(Clone, Debug)]
    pub struct LiveFrame {
        /// Raw image bytes for this frame
        pub image_data: Vec<u8>,
        /// Seconds since stream start
        pub timestamp_secs: f64,
        /// Frame index in the stream
        pub frame_index: u64,
        /// Estimated tokens for this frame
        pub token_estimate: usize,
        /// Whether this frame has been marked as a keyframe
        pub is_keyframe: bool,
    }

    /// A PDF or other document input.
    #[derive(Clone, Debug)]
    pub struct DocumentInput {
        /// Raw document bytes
        pub data: Vec<u8>,
        /// Document type
        pub doc_type: DocumentType,
        /// Page count if known
        pub page_count: Option<usize>,
        /// Tokens if every page were sent as a high-detail image
        pub naive_token_estimate: usize,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum DocumentType {
        Pdf,
        Docx,
        Pptx,
        Xlsx,
        Markdown,
        Html,
        PlainText,
    }

    /// A video input.
    #[derive(Clone, Debug)]
    pub struct VideoInput {
        /// Video source
        pub source: VideoSource,
        /// Duration in seconds
        pub duration_secs: f64,
        /// Frames per second
        pub fps: f64,
        /// Width in pixels
        pub width: u32,
        /// Height in pixels
        pub height: u32,
        /// Tokens if every frame sent at high detail
        pub naive_token_estimate: usize,
    }

    #[derive(Clone, Debug)]
    pub enum VideoSource {
        File(std::path::PathBuf),
        Buffer(Vec<u8>),
        Url(String),
    }

    /// A 3D asset input.
    #[derive(Clone, Debug)]
    pub struct Asset3dInput {
        /// Raw asset bytes
        pub data: Vec<u8>,
        /// Asset format
        pub format: Asset3dFormat,
        /// Number of vertices if known
        pub vertex_count: Option<usize>,
        /// Number of faces if known
        pub face_count: Option<usize>,
        /// Tokens for naive text representation
        pub naive_token_estimate: usize,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Asset3dFormat {
        Gltf,
        Glb,
        Obj,
        Fbx,
        Stl,
        Usdz,
        Ply,
    }

    /// Input to a multimodal saver.
    #[derive(Clone, Debug)]
    pub struct MultiModalSaverInput {
        pub base: SaverInput,
        pub audio: Vec<AudioInput>,
        pub live_frames: Vec<LiveFrame>,
        pub documents: Vec<DocumentInput>,
        pub videos: Vec<VideoInput>,
        pub assets_3d: Vec<Asset3dInput>,
    }

    /// Output from a multimodal saver.
    #[derive(Clone, Debug)]
    pub struct MultiModalSaverOutput {
        pub base: SaverOutput,
        pub audio: Vec<AudioInput>,
        pub live_frames: Vec<LiveFrame>,
        pub documents: Vec<DocumentInput>,
        pub videos: Vec<VideoInput>,
        pub assets_3d: Vec<Asset3dInput>,
    }

    /// Modality category for multimodal savers.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Modality {
        Audio,
        Live,
        Document,
        Video,
        Asset3d,
        CrossModal,
    }
}

pub mod traits {
    use super::{
        error::SaverError,
        report::TokenSavingsReport,
        stage::SaverStage,
        types::{
            SaverInput, SaverOutput, SaverContext,
            MultiModalSaverInput, MultiModalSaverOutput, Modality,
        },
    };

    /// The main trait implemented by all 25 text/image token-saving crates.
    ///
    /// Each saver belongs to a [`SaverStage`] and has a priority within
    /// that stage (lower = runs first). The pipeline calls savers in stage +
    /// priority order.
    #[async_trait::async_trait]
    pub trait TokenSaver: Send + Sync {
        /// Unique name of this saver (used in reports and logs)
        fn name(&self) -> &str;

        /// Pipeline stage this saver operates in
        fn stage(&self) -> SaverStage;

        /// Priority within the stage (lower = runs first)
        fn priority(&self) -> u32;

        /// Process the input and return (potentially modified) output.
        ///
        /// Savers should:
        /// - Only modify what they care about
        /// - Pass everything else through unchanged
        /// - Set `skipped = true` if the API call should be aborted
        /// - Update their internal [`TokenSavingsReport`]
        async fn process(
            &self,
            input: SaverInput,
            ctx: &SaverContext,
        ) -> Result<SaverOutput, SaverError>;

        /// Return the savings report from the last `process` call.
        fn last_savings(&self) -> TokenSavingsReport;
    }

    /// Trait for the 19 multimodal token-saving crates (audio, live,
    /// document, video, 3D).
    #[async_trait::async_trait]
    pub trait MultiModalTokenSaver: Send + Sync {
        /// Unique name of this saver
        fn name(&self) -> &str;

        /// Pipeline stage
        fn stage(&self) -> SaverStage;

        /// Priority within stage
        fn priority(&self) -> u32;

        /// Modality this saver targets
        fn modality(&self) -> Modality;

        /// Process multimodal input
        async fn process_multimodal(
            &self,
            input: MultiModalSaverInput,
            ctx: &SaverContext,
        ) -> Result<MultiModalSaverOutput, SaverError>;

        /// Return the savings report from the last call
        fn last_savings(&self) -> TokenSavingsReport;
    }
}
