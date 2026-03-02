//! Quality-based TTS routing.
//!
//! Routes TTS requests to the optimal provider based on the use case:
//! - Short UI responses → fast local Piper (lowest latency)
//! - Long narration → Chatterbox (highest quality local)
//! - Premium quality → cloud provider (ElevenLabs, Fish Audio, etc.)
//! - Cost-sensitive → cheapest available provider

use dx_core::cost::MicroCost;
use dx_core::tts_provider::TtsRequest;

/// Quality routing decision.
#[derive(Debug, Clone)]
pub struct RouteDecision {
    /// The recommended provider ID.
    pub provider_id: String,
    /// Why this provider was chosen.
    pub reason: RouteReason,
    /// Estimated cost for this request.
    pub estimated_cost: MicroCost,
    /// Estimated latency in milliseconds.
    pub estimated_latency_ms: u64,
}

/// Reason for choosing a particular provider.
#[derive(Debug, Clone)]
pub enum RouteReason {
    /// Chosen for lowest latency (short UI responses).
    LowestLatency,
    /// Chosen for highest quality (long-form narration).
    HighestQuality,
    /// Chosen for lowest cost.
    CheapestAvailable,
    /// Chosen because it's the only available option.
    OnlyAvailable,
    /// User explicitly requested this provider.
    UserPreference,
    /// Fallback after primary failed.
    Fallback,
}

/// Request context for routing decisions.
#[derive(Debug, Clone)]
pub struct RouteContext {
    /// Length of text to synthesize.
    pub text_length: usize,
    /// Whether this is a UI notification (short, needs low latency).
    pub is_ui_notification: bool,
    /// Whether user is in a real-time conversation.
    pub is_conversation: bool,
    /// Maximum acceptable cost.
    pub max_cost: Option<MicroCost>,
    /// Preferred quality level.
    pub quality: QualityLevel,
    /// Whether we have internet connectivity.
    pub has_internet: bool,
}

/// Desired quality level for TTS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityLevel {
    /// Fastest possible (Piper tiny).
    Fast,
    /// Good enough for conversation (Piper medium / Kokoro).
    Standard,
    /// High quality (Chatterbox / cloud).
    High,
    /// Best available (premium cloud like ElevenLabs).
    Premium,
}

/// Route a TTS request to the best provider.
pub fn route_tts_request(request: &TtsRequest, context: &RouteContext) -> RouteDecision {
    // Decision tree:
    // 1. No internet → must use local
    // 2. UI notification (< 50 chars) → Piper tiny (fastest)
    // 3. Conversation mode → Piper medium or Kokoro (low latency)
    // 4. Long text (> 500 chars) + High quality → Chatterbox or cloud
    // 5. Premium quality → ElevenLabs or Fish Audio
    // 6. Cost-sensitive → cheapest available

    let _ = request;

    if !context.has_internet {
        return RouteDecision {
            provider_id: if context.text_length < 100 {
                "piper".to_string()
            } else {
                "chatterbox".to_string()
            },
            reason: RouteReason::OnlyAvailable,
            estimated_cost: MicroCost::ZERO,
            estimated_latency_ms: 50,
        };
    }

    if context.is_ui_notification || context.text_length < 50 {
        return RouteDecision {
            provider_id: "piper".to_string(),
            reason: RouteReason::LowestLatency,
            estimated_cost: MicroCost::ZERO,
            estimated_latency_ms: 20,
        };
    }

    if context.is_conversation {
        return RouteDecision {
            provider_id: "piper".to_string(),
            reason: RouteReason::LowestLatency,
            estimated_cost: MicroCost::ZERO,
            estimated_latency_ms: 40,
        };
    }

    match context.quality {
        QualityLevel::Fast => RouteDecision {
            provider_id: "piper".to_string(),
            reason: RouteReason::LowestLatency,
            estimated_cost: MicroCost::ZERO,
            estimated_latency_ms: 20,
        },
        QualityLevel::Standard => RouteDecision {
            provider_id: "chatterbox".to_string(),
            reason: RouteReason::HighestQuality,
            estimated_cost: MicroCost::ZERO,
            estimated_latency_ms: 100,
        },
        QualityLevel::High => RouteDecision {
            provider_id: "fish_audio".to_string(),
            reason: RouteReason::HighestQuality,
            estimated_cost: MicroCost(context.text_length as u64 * 60),
            estimated_latency_ms: 200,
        },
        QualityLevel::Premium => RouteDecision {
            provider_id: "elevenlabs".to_string(),
            reason: RouteReason::HighestQuality,
            estimated_cost: MicroCost(context.text_length as u64 * 300),
            estimated_latency_ms: 300,
        },
    }
}
