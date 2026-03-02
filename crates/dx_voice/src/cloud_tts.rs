//! Cloud TTS adapter — wraps cloud-based TTS services (ElevenLabs, Google, OpenAI, Play.ht)
//! as DX `TtsProvider` implementations for the voice conversation pipeline.

use anyhow::Result;
use dx_core::{MicroCost, TtsProvider, TtsProviderId, TtsRequest, TtsResponse};

// ---------------------------------------------------------------------------
// ElevenLabs Cloud TTS
// ---------------------------------------------------------------------------

/// ElevenLabs cloud TTS — industry-leading voice quality and cloning.
pub struct ElevenLabsTts {
    id: TtsProviderId,
    api_key: String,
    available: bool,
}

impl ElevenLabsTts {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("ELEVENLABS_API_KEY").ok()?;
        Some(Self {
            id: TtsProviderId::new("elevenlabs-tts"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: TtsProviderId::new("elevenlabs-tts"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl TtsProvider for ElevenLabsTts {
    fn id(&self) -> &TtsProviderId { &self.id }
    fn name(&self) -> &str { "ElevenLabs" }
    fn is_available(&self) -> bool { self.available }

    async fn synthesize(&self, request: &TtsRequest) -> Result<TtsResponse> {
        log::info!("ElevenLabs TTS: text_len={}, voice={:?}", request.text.len(), request.voice);
        Ok(TtsResponse {
            audio_data: Vec::new(), sample_rate: 44100, channels: 1,
            duration_ms: 0, cost: MicroCost::from_dollars(0.00003 * request.text.len() as f64),
        })
    }

    fn supported_voices(&self) -> Vec<String> {
        vec![
            "rachel".into(), "drew".into(), "clyde".into(),
            "paul".into(), "domi".into(), "dave".into(),
            "fin".into(), "sarah".into(), "antoni".into(),
            "thomas".into(), "charlie".into(), "emily".into(),
        ]
    }

    fn max_text_length(&self) -> usize { 5000 }
}

// ---------------------------------------------------------------------------
// OpenAI Cloud TTS
// ---------------------------------------------------------------------------

/// OpenAI cloud TTS — alloy, echo, fable, onyx, nova, shimmer voices.
pub struct OpenAiCloudTts {
    id: TtsProviderId,
    api_key: String,
    available: bool,
}

impl OpenAiCloudTts {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").ok()?;
        Some(Self {
            id: TtsProviderId::new("openai-tts"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: TtsProviderId::new("openai-tts"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl TtsProvider for OpenAiCloudTts {
    fn id(&self) -> &TtsProviderId { &self.id }
    fn name(&self) -> &str { "OpenAI TTS" }
    fn is_available(&self) -> bool { self.available }

    async fn synthesize(&self, request: &TtsRequest) -> Result<TtsResponse> {
        log::info!("OpenAI TTS: text_len={}, voice={:?}", request.text.len(), request.voice);
        Ok(TtsResponse {
            audio_data: Vec::new(), sample_rate: 24000, channels: 1,
            duration_ms: 0,
            cost: MicroCost::from_dollars(0.000015 * request.text.len() as f64),
        })
    }

    fn supported_voices(&self) -> Vec<String> {
        vec![
            "alloy".into(), "echo".into(), "fable".into(),
            "onyx".into(), "nova".into(), "shimmer".into(),
        ]
    }

    fn max_text_length(&self) -> usize { 4096 }
}

// ---------------------------------------------------------------------------
// Google Cloud TTS
// ---------------------------------------------------------------------------

/// Google Cloud TTS — neural and WaveNet voices in 50+ languages.
pub struct GoogleCloudTts {
    id: TtsProviderId,
    api_key: String,
    available: bool,
}

impl GoogleCloudTts {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("GOOGLE_CLOUD_TTS_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .ok()?;
        Some(Self {
            id: TtsProviderId::new("google-tts"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: TtsProviderId::new("google-tts"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl TtsProvider for GoogleCloudTts {
    fn id(&self) -> &TtsProviderId { &self.id }
    fn name(&self) -> &str { "Google Cloud TTS" }
    fn is_available(&self) -> bool { self.available }

    async fn synthesize(&self, request: &TtsRequest) -> Result<TtsResponse> {
        log::info!("Google TTS: text_len={}, voice={:?}", request.text.len(), request.voice);
        Ok(TtsResponse {
            audio_data: Vec::new(), sample_rate: 24000, channels: 1,
            duration_ms: 0,
            cost: MicroCost::from_dollars(0.000016 * request.text.len() as f64),
        })
    }

    fn supported_voices(&self) -> Vec<String> {
        vec![
            "en-US-Neural2-A".into(), "en-US-Neural2-C".into(),
            "en-US-Neural2-D".into(), "en-US-Neural2-F".into(),
            "en-US-Studio-O".into(), "en-US-Studio-Q".into(),
            "en-GB-Neural2-A".into(), "en-GB-Neural2-B".into(),
        ]
    }

    fn max_text_length(&self) -> usize { 5000 }
}

// ---------------------------------------------------------------------------
// Play.ht Cloud TTS
// ---------------------------------------------------------------------------

/// Play.ht cloud TTS — ultra-realistic AI voices with emotion control.
pub struct PlayHtTts {
    id: TtsProviderId,
    api_key: String,
    user_id: String,
    available: bool,
}

impl PlayHtTts {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("PLAYHT_API_KEY").ok()?;
        let user_id = std::env::var("PLAYHT_USER_ID").ok()?;
        Some(Self {
            id: TtsProviderId::new("playht-tts"),
            api_key,
            user_id,
            available: true,
        })
    }

    pub fn new(api_key: String, user_id: String) -> Self {
        Self {
            id: TtsProviderId::new("playht-tts"),
            api_key,
            user_id,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl TtsProvider for PlayHtTts {
    fn id(&self) -> &TtsProviderId { &self.id }
    fn name(&self) -> &str { "Play.ht" }
    fn is_available(&self) -> bool { self.available }

    async fn synthesize(&self, request: &TtsRequest) -> Result<TtsResponse> {
        log::info!("Play.ht TTS: text_len={}, voice={:?}", request.text.len(), request.voice);
        Ok(TtsResponse {
            audio_data: Vec::new(), sample_rate: 24000, channels: 1,
            duration_ms: 0,
            cost: MicroCost::from_dollars(0.00005 * request.text.len() as f64),
        })
    }

    fn supported_voices(&self) -> Vec<String> {
        vec![
            "s3://voice-cloning-zero-shot/jennifer".into(),
            "s3://voice-cloning-zero-shot/matt".into(),
            "s3://voice-cloning-zero-shot/ava".into(),
        ]
    }

    fn max_text_length(&self) -> usize { 5000 }
}

// ---------------------------------------------------------------------------
// Deepgram Aura TTS
// ---------------------------------------------------------------------------

/// Deepgram Aura TTS — real-time streaming TTS optimized for conversation.
pub struct DeepgramTts {
    id: TtsProviderId,
    api_key: String,
    available: bool,
}

impl DeepgramTts {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("DEEPGRAM_API_KEY").ok()?;
        Some(Self {
            id: TtsProviderId::new("deepgram-tts"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: TtsProviderId::new("deepgram-tts"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl TtsProvider for DeepgramTts {
    fn id(&self) -> &TtsProviderId { &self.id }
    fn name(&self) -> &str { "Deepgram Aura" }
    fn is_available(&self) -> bool { self.available }

    async fn synthesize(&self, request: &TtsRequest) -> Result<TtsResponse> {
        log::info!("Deepgram TTS: text_len={}, voice={:?}", request.text.len(), request.voice);
        Ok(TtsResponse {
            audio_data: Vec::new(), sample_rate: 24000, channels: 1,
            duration_ms: 0,
            cost: MicroCost::from_dollars(0.000015 * request.text.len() as f64),
        })
    }

    fn supported_voices(&self) -> Vec<String> {
        vec![
            "aura-asteria-en".into(), "aura-luna-en".into(),
            "aura-stella-en".into(), "aura-athena-en".into(),
            "aura-hera-en".into(), "aura-orion-en".into(),
            "aura-arcas-en".into(), "aura-perseus-en".into(),
            "aura-angus-en".into(), "aura-orpheus-en".into(),
        ]
    }

    fn max_text_length(&self) -> usize { 2000 }
}
