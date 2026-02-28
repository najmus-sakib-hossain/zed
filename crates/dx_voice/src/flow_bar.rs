//! FlowBar — the compact voice-conversation UI widget
//! (inspired by Wispr Flow, but built into Zed's title bar).

use dx_core::Mood;

/// Which visual state the FlowBar is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FlowBarState {
    /// Bar is hidden.
    Hidden,
    /// Microphone is active, listening for speech.
    Listening,
    /// Transcription in progress.
    Transcribing,
    /// Waiting for LLM response.
    Thinking,
    /// TTS is speaking the response.
    Speaking,
    /// An error occurred.
    Error,
}

impl FlowBarState {
    /// Display label for the current state.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Hidden => "",
            Self::Listening => "Listening…",
            Self::Transcribing => "Transcribing…",
            Self::Thinking => "Thinking…",
            Self::Speaking => "Speaking…",
            Self::Error => "Error",
        }
    }

    /// True when the bar should be visible.
    pub fn is_visible(&self) -> bool {
        !matches!(self, Self::Hidden)
    }
}

/// View-model for the FlowBar widget.
#[derive(Debug, Clone)]
pub struct FlowBarViewModel {
    pub state: FlowBarState,
    pub mood: Mood,
    /// Transcription preview text (partial or final).
    pub transcript_preview: String,
    /// Audio level 0.0–1.0 for visualizer.
    pub audio_level: f32,
    /// Whether the user can interrupt.
    pub interruptible: bool,
}

impl Default for FlowBarViewModel {
    fn default() -> Self {
        Self {
            state: FlowBarState::Hidden,
            mood: Mood::Zen,
            transcript_preview: String::new(),
            audio_level: 0.0,
            interruptible: true,
        }
    }
}

impl FlowBarViewModel {
    pub fn set_listening(&mut self) {
        self.state = FlowBarState::Listening;
        self.transcript_preview.clear();
        self.audio_level = 0.0;
    }

    pub fn set_transcribing(&mut self, preview: &str) {
        self.state = FlowBarState::Transcribing;
        self.transcript_preview = preview.to_string();
    }

    pub fn set_thinking(&mut self) {
        self.state = FlowBarState::Thinking;
    }

    pub fn set_speaking(&mut self) {
        self.state = FlowBarState::Speaking;
    }

    pub fn set_error(&mut self) {
        self.state = FlowBarState::Error;
    }

    pub fn hide(&mut self) {
        self.state = FlowBarState::Hidden;
        self.transcript_preview.clear();
        self.audio_level = 0.0;
    }

    pub fn update_audio_level(&mut self, level: f32) {
        self.audio_level = level.clamp(0.0, 1.0);
    }
}
