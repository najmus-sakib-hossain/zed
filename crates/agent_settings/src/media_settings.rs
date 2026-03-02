//! Media Provider Settings — configuration for image/video/audio/3D/live/document generation.
//!
//! This module provides settings for media generation providers in the AI panel.
//! Users can configure their API keys and preferred providers for each media type.

use gpui::App;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings::{RegisterSetting, Settings, SettingsContent};

/// Media provider identifier (matches dx_core::MediaProviderId).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct MediaProviderSelection {
    /// Provider ID (e.g., "openai-image", "elevenlabs", "runway").
    pub provider: String,
    /// Model ID (e.g., "gpt-image-1.5", "eleven_turbo_v2").
    pub model: Option<String>,
}

impl MediaProviderSelection {
    pub fn new(provider: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: None,
        }
    }

    pub fn with_model(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: Some(model.into()),
        }
    }
}

/// Per-provider API key configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct MediaProviderApiKeys {
    // Image providers
    pub openai: Option<String>,
    pub stability_ai: Option<String>,
    pub fal_ai: Option<String>,
    pub replicate: Option<String>,
    pub google: Option<String>,
    pub leonardo_ai: Option<String>,
    pub ideogram: Option<String>,
    pub bria: Option<String>,
    pub deepai: Option<String>,
    pub xai: Option<String>,
    pub huggingface: Option<String>,
    pub together_ai: Option<String>,
    pub wavespeed_ai: Option<String>,
    pub aiml_api: Option<String>,

    // Video providers
    pub runway: Option<String>,
    pub kling_ai: Option<String>,
    pub pika: Option<String>,
    pub luma_ai: Option<String>,
    pub minimax: Option<String>,
    pub synthesia: Option<String>,
    pub heygen: Option<String>,
    pub hailuo_ai: Option<String>,

    // Audio providers
    pub elevenlabs: Option<String>,
    pub play_ht: Option<String>,
    pub cartesia: Option<String>,
    pub deepgram: Option<String>,
    pub fish_audio: Option<String>,
    pub mubert: Option<String>,
    pub suno_ai: Option<String>,
    pub udio: Option<String>,

    // 3D providers
    pub meshy: Option<String>,
    pub tripo_ai: Option<String>,

    // Live avatar providers
    pub d_id: Option<String>,
    pub tavus: Option<String>,
    pub simli: Option<String>,
    pub hedra: Option<String>,

    // Document providers
    pub carbone: Option<String>,
    pub craftmypdf: Option<String>,
    pub pdfshift: Option<String>,
    pub docraptor: Option<String>,
    // QuickChart has a free tier that works without API key
}

/// Media generation settings for the AI panel.
#[derive(Clone, Debug, RegisterSetting)]
pub struct MediaSettings {
    /// Enable media generation features in the AI panel.
    pub enabled: bool,

    /// Default image generation provider.
    pub default_image_provider: Option<MediaProviderSelection>,

    /// Default video generation provider.
    pub default_video_provider: Option<MediaProviderSelection>,

    /// Default audio/TTS provider.
    pub default_audio_provider: Option<MediaProviderSelection>,

    /// Default music generation provider.
    pub default_music_provider: Option<MediaProviderSelection>,

    /// Default 3D generation provider.
    pub default_3d_provider: Option<MediaProviderSelection>,

    /// Default live avatar provider.
    pub default_live_provider: Option<MediaProviderSelection>,

    /// Default document/chart generation provider.
    pub default_document_provider: Option<MediaProviderSelection>,

    /// Favorite media providers for quick access.
    pub favorite_providers: Vec<MediaProviderSelection>,

    /// API keys for media providers.
    /// Note: For security, prefer using environment variables (e.g., OPENAI_API_KEY).
    /// Keys stored here are used as fallback when env vars are not set.
    pub api_keys: MediaProviderApiKeys,

    /// Output directory for generated media.
    pub output_directory: Option<String>,

    /// Auto-save generated media to output directory.
    pub auto_save: bool,

    /// Maximum concurrent media generations.
    pub max_concurrent_generations: usize,

    /// Show cost estimates before generation.
    pub show_cost_estimates: bool,
}

impl Default for MediaSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            default_image_provider: Some(MediaProviderSelection::with_model(
                "openai-image",
                "gpt-image-1.5",
            )),
            default_video_provider: None,
            default_audio_provider: Some(MediaProviderSelection::with_model(
                "elevenlabs",
                "eleven_turbo_v2_5",
            )),
            default_music_provider: None,
            default_3d_provider: None,
            default_live_provider: None,
            default_document_provider: Some(MediaProviderSelection::new("quickchart")),
            favorite_providers: vec![],
            api_keys: MediaProviderApiKeys::default(),
            output_directory: None,
            auto_save: false,
            max_concurrent_generations: 3,
            show_cost_estimates: true,
        }
    }
}

impl Settings for MediaSettings {
    fn from_settings(_content: &SettingsContent) -> Self {
        // For now, return default settings
        // TODO: Add media field to SettingsContent and parse from there
        Self::default()
    }
}

impl MediaSettings {
    pub fn enabled(&self, _cx: &App) -> bool {
        self.enabled
    }

    /// Get the API key for a provider, preferring environment variables.
    pub fn get_api_key(&self, provider_id: &str) -> Option<String> {
        // First check environment variables (preferred for security)
        let env_key = Self::provider_env_var(provider_id);
        if let Ok(key) = std::env::var(&env_key) {
            if !key.is_empty() {
                return Some(key);
            }
        }

        // Fall back to settings-stored keys
        match provider_id {
            "openai-image" | "openai-tts" | "openai-sora" => self.api_keys.openai.clone(),
            "stability-ai" | "stability-audio" | "stability-svd" => {
                self.api_keys.stability_ai.clone()
            }
            "fal-ai" => self.api_keys.fal_ai.clone(),
            "replicate" => self.api_keys.replicate.clone(),
            "google-imagen" | "google-veo" | "google-tts" | "google-musicfx" => {
                self.api_keys.google.clone()
            }
            "leonardo-ai" => self.api_keys.leonardo_ai.clone(),
            "ideogram" => self.api_keys.ideogram.clone(),
            "bria" => self.api_keys.bria.clone(),
            "deepai" => self.api_keys.deepai.clone(),
            "xai-grok-image" => self.api_keys.xai.clone(),
            "huggingface" => self.api_keys.huggingface.clone(),
            "together-ai" => self.api_keys.together_ai.clone(),
            "wavespeed-ai" => self.api_keys.wavespeed_ai.clone(),
            "aiml-api" => self.api_keys.aiml_api.clone(),
            "runway" => self.api_keys.runway.clone(),
            "kling-ai" => self.api_keys.kling_ai.clone(),
            "pika" => self.api_keys.pika.clone(),
            "luma-ai" => self.api_keys.luma_ai.clone(),
            "minimax" => self.api_keys.minimax.clone(),
            "synthesia" => self.api_keys.synthesia.clone(),
            "heygen" | "heygen-live" => self.api_keys.heygen.clone(),
            "hailuo-ai" => self.api_keys.hailuo_ai.clone(),
            "elevenlabs" | "elevenlabs-music" | "elevenlabs-sfx" => {
                self.api_keys.elevenlabs.clone()
            }
            "play-ht" => self.api_keys.play_ht.clone(),
            "cartesia" => self.api_keys.cartesia.clone(),
            "deepgram" => self.api_keys.deepgram.clone(),
            "fish-audio" => self.api_keys.fish_audio.clone(),
            "mubert" => self.api_keys.mubert.clone(),
            "suno-ai" => self.api_keys.suno_ai.clone(),
            "udio" => self.api_keys.udio.clone(),
            "meshy" => self.api_keys.meshy.clone(),
            "tripo-ai" => self.api_keys.tripo_ai.clone(),
            "d-id" | "d-id-video" => self.api_keys.d_id.clone(),
            "tavus" => self.api_keys.tavus.clone(),
            "simli" => self.api_keys.simli.clone(),
            "hedra" => self.api_keys.hedra.clone(),
            "carbone" => self.api_keys.carbone.clone(),
            "craftmypdf" => self.api_keys.craftmypdf.clone(),
            "pdfshift" => self.api_keys.pdfshift.clone(),
            "docraptor" => self.api_keys.docraptor.clone(),
            "quickchart" => None, // Works without API key
            _ => None,
        }
    }

    /// Get the environment variable name for a provider.
    fn provider_env_var(provider_id: &str) -> String {
        match provider_id {
            "openai-image" | "openai-tts" | "openai-sora" => "OPENAI_API_KEY".to_string(),
            "stability-ai" | "stability-audio" | "stability-svd" => {
                "STABILITY_API_KEY".to_string()
            }
            "fal-ai" => "FAL_KEY".to_string(),
            "replicate" => "REPLICATE_API_TOKEN".to_string(),
            "google-imagen" | "google-veo" | "google-tts" | "google-musicfx" => {
                "GOOGLE_API_KEY".to_string()
            }
            "leonardo-ai" => "LEONARDO_API_KEY".to_string(),
            "ideogram" => "IDEOGRAM_API_KEY".to_string(),
            "bria" => "BRIA_API_KEY".to_string(),
            "deepai" => "DEEPAI_API_KEY".to_string(),
            "xai-grok-image" => "XAI_API_KEY".to_string(),
            "huggingface" => "HF_TOKEN".to_string(),
            "together-ai" => "TOGETHER_API_KEY".to_string(),
            "wavespeed-ai" => "WAVESPEED_API_KEY".to_string(),
            "aiml-api" => "AIMLAPI_KEY".to_string(),
            "runway" => "RUNWAY_API_KEY".to_string(),
            "kling-ai" => "KLING_API_KEY".to_string(),
            "pika" => "PIKA_API_KEY".to_string(),
            "luma-ai" => "LUMA_API_KEY".to_string(),
            "minimax" => "MINIMAX_API_KEY".to_string(),
            "synthesia" => "SYNTHESIA_API_KEY".to_string(),
            "heygen" | "heygen-live" => "HEYGEN_API_KEY".to_string(),
            "hailuo-ai" => "HAILUO_API_KEY".to_string(),
            "elevenlabs" | "elevenlabs-music" | "elevenlabs-sfx" => {
                "ELEVENLABS_API_KEY".to_string()
            }
            "play-ht" => "PLAYHT_API_KEY".to_string(),
            "cartesia" => "CARTESIA_API_KEY".to_string(),
            "deepgram" => "DEEPGRAM_API_KEY".to_string(),
            "fish-audio" => "FISH_AUDIO_API_KEY".to_string(),
            "mubert" => "MUBERT_API_KEY".to_string(),
            "suno-ai" => "SUNO_API_KEY".to_string(),
            "udio" => "UDIO_API_KEY".to_string(),
            "meshy" => "MESHY_API_KEY".to_string(),
            "tripo-ai" => "TRIPO_API_KEY".to_string(),
            "d-id" | "d-id-video" => "D_ID_API_KEY".to_string(),
            "tavus" => "TAVUS_API_KEY".to_string(),
            "simli" => "SIMLI_API_KEY".to_string(),
            "hedra" => "HEDRA_API_KEY".to_string(),
            "carbone" => "CARBONE_API_KEY".to_string(),
            "craftmypdf" => "CRAFTMYPDF_API_KEY".to_string(),
            "pdfshift" => "PDFSHIFT_API_KEY".to_string(),
            "docraptor" => "DOCRAPTOR_API_KEY".to_string(),
            _ => format!(
                "{}_API_KEY",
                provider_id.to_uppercase().replace('-', "_")
            ),
        }
    }
}

/// All available media providers grouped by type.
pub struct AvailableMediaProviders;

impl AvailableMediaProviders {
    /// Image generation providers.
    pub fn image() -> &'static [(&'static str, &'static str)] {
        &[
            ("openai-image", "OpenAI (GPT-Image, DALL-E)"),
            ("fal-ai", "Fal.ai (600+ models)"),
            ("stability-ai", "Stability AI (SDXL, SD3.5)"),
            ("replicate", "Replicate (200+ models)"),
            ("google-imagen", "Google Imagen 3"),
            ("leonardo-ai", "Leonardo.ai"),
            ("ideogram", "Ideogram (best text-in-image)"),
            ("bria", "Bria.ai (commercial-safe)"),
            ("deepai", "DeepAI"),
            ("xai-grok-image", "xAI Grok Image"),
            ("huggingface", "Hugging Face"),
            ("together-ai", "Together AI"),
            ("wavespeed-ai", "WaveSpeed AI"),
            ("aiml-api", "AIML API"),
        ]
    }

    /// Video generation providers.
    pub fn video() -> &'static [(&'static str, &'static str)] {
        &[
            ("runway", "Runway Gen-4"),
            ("kling-ai", "Kling AI 2.0"),
            ("pika", "Pika Labs"),
            ("luma-ai", "Luma Dream Machine"),
            ("google-veo", "Google Veo 3"),
            ("openai-sora", "OpenAI Sora 2"),
            ("minimax", "Minimax"),
            ("synthesia", "Synthesia (avatars)"),
            ("heygen", "HeyGen"),
            ("hailuo-ai", "Hailuo AI"),
        ]
    }

    /// Audio/TTS providers.
    pub fn audio() -> &'static [(&'static str, &'static str)] {
        &[
            ("elevenlabs", "ElevenLabs (TTS + SFX)"),
            ("openai-tts", "OpenAI TTS"),
            ("google-tts", "Google Cloud TTS"),
            ("play-ht", "Play.ht"),
            ("cartesia", "Cartesia (low-latency)"),
            ("deepgram", "Deepgram Aura-2"),
            ("fish-audio", "Fish Audio"),
        ]
    }

    /// Music generation providers.
    pub fn music() -> &'static [(&'static str, &'static str)] {
        &[
            ("elevenlabs-music", "ElevenLabs Music"),
            ("suno-ai", "Suno AI"),
            ("udio", "Udio"),
            ("stability-audio", "Stability Audio"),
            ("mubert", "Mubert"),
        ]
    }

    /// 3D/AR/VR generation providers.
    pub fn threed() -> &'static [(&'static str, &'static str)] {
        &[
            ("meshy", "Meshy (text/image-to-3D)"),
            ("tripo-ai", "Tripo AI"),
        ]
    }

    /// Live conversational avatar providers.
    pub fn live() -> &'static [(&'static str, &'static str)] {
        &[
            ("d-id", "D-ID (real-time streaming)"),
            ("tavus", "Tavus (CVI)"),
            ("heygen-live", "HeyGen Live"),
            ("simli", "Simli"),
            ("hedra", "Hedra"),
        ]
    }

    /// Document/chart generation providers.
    pub fn document() -> &'static [(&'static str, &'static str)] {
        &[
            ("quickchart", "QuickChart (free charts)"),
            ("carbone", "Carbone.io"),
            ("craftmypdf", "CraftMyPDF"),
            ("pdfshift", "PDFShift"),
            ("docraptor", "DocRaptor"),
        ]
    }
}
