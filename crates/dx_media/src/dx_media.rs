//! DX Media — Universal media generation engine.
//!
//! Provides concrete implementations of `MediaProvider` for all supported
//! image, video, audio, music, 3D, live avatar, and document generation providers.
//!
//! ## Provider Coverage (Feb 2026)
//!
//! ### Image (Part 10)
//! - OpenAI (DALL-E 3, GPT-Image), Fal.ai (600+ models), Stability AI (SDXL, SD3.5)
//! - Replicate (200+ community models), Leonardo AI, Ideogram, Google Imagen
//! - Bria (commercial-safe), DeepAI, xAI Grok Image
//! - Aggregators: Hugging Face, Together AI, WaveSpeed AI, AIML API
//!
//! ### Video (Part 11)
//! - Runway Gen-4, Kling AI, Pika, Luma AI Dream Machine, Minimax/Hailuo
//! - Synthesia (avatars), HeyGen, Google Veo, OpenAI Sora
//!
//! ### Audio & Music (Part 12)
//! - ElevenLabs (TTS + Music + SFX), Suno AI (full songs), Udio
//! - Stability Audio, Google TTS, OpenAI TTS
//! - Play.ht, Cartesia, Deepgram, Fish Audio, Mubert
//!
//! ### 3D / AR / VR / XR (Part 13)
//! - Meshy (text/image-to-3D with PBR), Tripo AI, Replicate 3D models
//!
//! ### Live Conversational Avatars (Part 15)
//! - D-ID (real-time streaming), Tavus (CVI), Simli, Hedra, HeyGen Live
//!
//! ### Documents / PDF / Charts (Part 16)
//! - QuickChart (Chart.js → PNG/SVG/PDF), Carbone, CraftMyPDF
//! - PDFShift, DocRaptor (PrinceXML), Local document generation

// ---------------------------------------------------------------------------
// Image providers
// ---------------------------------------------------------------------------
mod bria;
mod deepai;
mod fal_ai;
mod google_imagen;
mod ideogram;
mod leonardo_ai;
mod openai_image;
mod replicate;
mod stability_ai;
mod xai_grok_image;

// Aggregator platforms (one key → many models)
mod aiml_api;
mod huggingface;
mod together_ai;
mod wavespeed_ai;

// ---------------------------------------------------------------------------
// Video providers
// ---------------------------------------------------------------------------
mod google_veo;
mod hailuo_ai;
mod heygen;
mod kling_ai;
mod luma_ai;
mod minimax_video;
mod openai_sora;
mod pika;
mod runway;
mod synthesia;

// ---------------------------------------------------------------------------
// Audio & Music providers
// ---------------------------------------------------------------------------
mod cartesia;
mod deepgram;
mod elevenlabs;
mod fish_audio;
mod google_tts;
mod mubert;
mod openai_tts;
mod play_ht;
mod stability_audio;
mod suno_ai;
mod udio;

// ---------------------------------------------------------------------------
// 3D / AR / VR / XR providers
// ---------------------------------------------------------------------------
mod kaedim;
mod meshy;
mod swiftxr;
mod tripo_ai;
mod world_labs;

// ---------------------------------------------------------------------------
// Live conversational avatar providers
// ---------------------------------------------------------------------------
mod anam_ai;
mod beyond_presence;
mod d_id;
mod deepbrain_ai;
mod hedra;
mod simli;
mod tavus;

// ---------------------------------------------------------------------------
// Document / PDF / Chart providers
// ---------------------------------------------------------------------------
mod adobe_pdf;
mod apitemplate;
mod carbone;
mod craftmypdf;
mod docraptor;
mod document_generator;
mod pdf_co;
mod pdfshift;
mod quickchart;

// ---------------------------------------------------------------------------
// Multi-media orchestration & caching
// ---------------------------------------------------------------------------
mod orchestrator;
mod output_cache;
mod parallel_orchestrator;
mod pdf_renderer;

// ---------------------------------------------------------------------------
// Public exports
// ---------------------------------------------------------------------------

// Image
pub use bria::*;
pub use deepai::*;
pub use fal_ai::*;
pub use google_imagen::*;
pub use ideogram::*;
pub use leonardo_ai::*;
pub use openai_image::*;
pub use replicate::*;
pub use stability_ai::*;
pub use xai_grok_image::*;

// Aggregators
pub use aiml_api::*;
pub use huggingface::*;
pub use together_ai::*;
pub use wavespeed_ai::*;

// Video
pub use google_veo::*;
pub use hailuo_ai::*;
pub use heygen::*;
pub use kling_ai::*;
pub use luma_ai::*;
pub use minimax_video::*;
pub use openai_sora::*;
pub use pika::*;
pub use runway::*;
pub use synthesia::*;

// Audio & Music
pub use cartesia::*;
pub use deepgram::*;
pub use elevenlabs::*;
pub use fish_audio::*;
pub use google_tts::*;
pub use mubert::*;
pub use openai_tts::*;
pub use play_ht::*;
pub use stability_audio::*;
pub use suno_ai::*;
pub use udio::*;

// 3D
pub use kaedim::*;
pub use meshy::*;
pub use swiftxr::*;
pub use tripo_ai::*;
pub use world_labs::*;

// Live avatars
pub use anam_ai::*;
pub use beyond_presence::*;
pub use d_id::*;
pub use deepbrain_ai::*;
pub use hedra::*;
pub use simli::*;
pub use tavus::*;

// Documents
pub use adobe_pdf::*;
pub use apitemplate::*;
pub use carbone::*;
pub use craftmypdf::*;
pub use docraptor::*;
pub use document_generator::*;
pub use pdf_co::*;
pub use pdfshift::*;
pub use quickchart::*;

// Orchestration
pub use orchestrator::*;
pub use output_cache::*;
pub use parallel_orchestrator::{execute_parallel, estimate_cost, validate_request};
pub use pdf_renderer::{render_pdf, render_svg, render_xlsx};

use dx_core::DxProviderRegistry;
use gpui::App;
use std::sync::Arc;

/// Initialize the media generation subsystem and register all available providers.
pub fn init(_cx: &mut App) {
    log::info!("DX Media engine initialized with 50+ provider types across 6 media categories");
}

/// Register all media providers into the DX provider registry.
pub fn register_media_providers(registry: &DxProviderRegistry) {
    // ---------------------------------------------------------------------------
    // Image providers
    // ---------------------------------------------------------------------------
    registry.register_media_provider(Arc::new(OpenAiImageProvider::new()));
    registry.register_media_provider(Arc::new(FalAiProvider::new()));
    registry.register_media_provider(Arc::new(StabilityAiProvider::new()));
    registry.register_media_provider(Arc::new(ReplicateProvider::new()));
    registry.register_media_provider(Arc::new(GoogleImagenProvider::new()));
    registry.register_media_provider(Arc::new(LeonardoAiProvider::new()));
    registry.register_media_provider(Arc::new(IdeogramProvider::new()));
    registry.register_media_provider(Arc::new(BriaProvider::new()));
    registry.register_media_provider(Arc::new(DeepAiProvider::new()));
    registry.register_media_provider(Arc::new(XaiGrokImageProvider::new()));

    // Aggregator platforms (one key → many models)
    registry.register_media_provider(Arc::new(HuggingFaceProvider::new()));
    registry.register_media_provider(Arc::new(TogetherAiProvider::new()));
    registry.register_media_provider(Arc::new(WaveSpeedAiProvider::new()));
    registry.register_media_provider(Arc::new(AimlApiProvider::new()));

    // ---------------------------------------------------------------------------
    // Video providers
    // ---------------------------------------------------------------------------
    registry.register_media_provider(Arc::new(RunwayProvider::new()));
    registry.register_media_provider(Arc::new(KlingAiProvider::new()));
    registry.register_media_provider(Arc::new(PikaProvider::new()));
    registry.register_media_provider(Arc::new(LumaAiVideoProvider::new()));
    registry.register_media_provider(Arc::new(MinimaxVideoProvider::new()));
    registry.register_media_provider(Arc::new(SynthesiaProvider::new()));
    registry.register_media_provider(Arc::new(HeygenProvider::new()));
    registry.register_media_provider(Arc::new(GoogleVeoProvider::new()));
    registry.register_media_provider(Arc::new(OpenAiSoraProvider::new()));
    registry.register_media_provider(Arc::new(HailuoAiProvider::new()));

    // ---------------------------------------------------------------------------
    // Audio & Music providers
    // ---------------------------------------------------------------------------
    registry.register_media_provider(Arc::new(ElevenLabsProvider::new()));
    registry.register_media_provider(Arc::new(SunoAiProvider::new()));
    registry.register_media_provider(Arc::new(UdioProvider::new()));
    registry.register_media_provider(Arc::new(StabilityAudioProvider::new()));
    registry.register_media_provider(Arc::new(OpenAiTtsProvider::new()));
    registry.register_media_provider(Arc::new(GoogleTtsProvider::new()));
    registry.register_media_provider(Arc::new(PlayHtProvider::new()));
    registry.register_media_provider(Arc::new(CartesiaProvider::new()));
    registry.register_media_provider(Arc::new(DeepgramProvider::new()));
    registry.register_media_provider(Arc::new(FishAudioProvider::new()));
    registry.register_media_provider(Arc::new(MubertProvider::new()));

    // ---------------------------------------------------------------------------
    // 3D / AR / VR / XR providers
    // ---------------------------------------------------------------------------
    registry.register_media_provider(Arc::new(MeshyProvider::new()));
    registry.register_media_provider(Arc::new(TripoAiProvider::new()));
    registry.register_media_provider(Arc::new(KaedimProvider::new()));
    registry.register_media_provider(Arc::new(SwiftXrProvider::new()));
    registry.register_media_provider(Arc::new(WorldLabsProvider::new()));

    // ---------------------------------------------------------------------------
    // Live conversational avatar providers
    // ---------------------------------------------------------------------------
    registry.register_media_provider(Arc::new(DIdProvider::new()));
    registry.register_media_provider(Arc::new(TavusProvider::new()));
    registry.register_media_provider(Arc::new(SimliProvider::new()));
    registry.register_media_provider(Arc::new(HedraProvider::new()));
    registry.register_media_provider(Arc::new(AnamAiProvider::new()));
    registry.register_media_provider(Arc::new(BeyondPresenceProvider::new()));
    registry.register_media_provider(Arc::new(DeepbrainAiProvider::new()));

    // ---------------------------------------------------------------------------
    // Document / PDF / Chart providers
    // ---------------------------------------------------------------------------
    registry.register_media_provider(Arc::new(QuickChartProvider::new()));
    registry.register_media_provider(Arc::new(CarboneProvider::new()));
    registry.register_media_provider(Arc::new(CraftMyPdfProvider::new()));
    registry.register_media_provider(Arc::new(PdfShiftProvider::new()));
    registry.register_media_provider(Arc::new(DocRaptorProvider::new()));
    registry.register_media_provider(Arc::new(AdobePdfProvider::new()));
    registry.register_media_provider(Arc::new(ApiTemplateProvider::new()));
    registry.register_media_provider(Arc::new(PdfCoProvider::new()));

    log::info!("DX Media: registered 55+ media providers across 7 categories");
}
