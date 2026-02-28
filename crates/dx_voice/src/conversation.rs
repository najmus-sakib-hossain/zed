//! Voice conversation loop: listen → transcribe → LLM → TTS → play.
//!
//! Integrates SttEngine + TtsManager + LLM provider into a real-time
//! voice conversation experience that feels like talking to a friend.

use anyhow::Result;
use dx_core::{LlmMessage, LlmProvider, LlmRequest, LlmRole, TtsRequest};
use std::sync::Arc;

use crate::stt_engine::SttEngine;
use crate::tts_manager::TtsManager;

/// State of the conversation loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConversationState {
    /// Idle — not actively in a conversation.
    Idle,
    /// Listening for user speech.
    Listening,
    /// Transcribing user's audio.
    Transcribing,
    /// Waiting for LLM response.
    Thinking,
    /// Speaking the LLM's response.
    Speaking,
    /// Paused by user.
    Paused,
    /// An error occurred.
    Error,
}

/// A single turn in the conversation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationTurn {
    pub role: LlmRole,
    pub text: String,
    pub audio_duration_seconds: Option<f64>,
    pub timestamp: std::time::SystemTime,
}

/// Configuration for voice conversation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationConfig {
    /// System prompt for the LLM.
    pub system_prompt: String,
    /// Voice ID for TTS playback.
    pub voice_id: String,
    /// Sample rate for audio.
    pub sample_rate: u32,
    /// Max conversation turns to keep in context window.
    pub max_context_turns: usize,
    /// Whether to auto-detect end of speech.
    pub auto_detect_silence: bool,
    /// Silence threshold in seconds before ending listening.
    pub silence_threshold_secs: f64,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            system_prompt: "You are a helpful voice assistant. Keep responses concise and conversational.".into(),
            voice_id: "en_US-lessac-medium".into(),
            sample_rate: 22050,
            max_context_turns: 20,
            auto_detect_silence: true,
            silence_threshold_secs: 1.5,
        }
    }
}

/// Orchestrates the voice conversation loop.
pub struct VoiceConversation {
    state: ConversationState,
    config: ConversationConfig,
    history: Vec<ConversationTurn>,
    stt: Arc<parking_lot::Mutex<SttEngine>>,
    tts: Arc<parking_lot::Mutex<TtsManager>>,
    llm: Arc<dyn LlmProvider>,
}

impl VoiceConversation {
    pub fn new(
        config: ConversationConfig,
        stt: Arc<parking_lot::Mutex<SttEngine>>,
        tts: Arc<parking_lot::Mutex<TtsManager>>,
        llm: Arc<dyn LlmProvider>,
    ) -> Self {
        Self {
            state: ConversationState::Idle,
            config,
            history: Vec::new(),
            stt,
            tts,
            llm,
        }
    }

    #[inline]
    pub fn state(&self) -> ConversationState {
        self.state
    }

    pub fn history(&self) -> &[ConversationTurn] {
        &self.history
    }

    /// Start the conversation — transitions to Listening state.
    pub fn start(&mut self) -> Result<()> {
        if self.state != ConversationState::Idle && self.state != ConversationState::Paused {
            return Err(anyhow::anyhow!(
                "Cannot start from state {:?}",
                self.state
            ));
        }
        self.state = ConversationState::Listening;
        self.stt.lock().start_listening()?;
        Ok(())
    }

    /// Stop the conversation.
    pub fn stop(&mut self) -> Result<()> {
        let _ = self.stt.lock().stop_listening();
        self.state = ConversationState::Idle;
        Ok(())
    }

    /// Pause the conversation.
    pub fn pause(&mut self) {
        if self.state != ConversationState::Idle {
            let _ = self.stt.lock().stop_listening();
            self.state = ConversationState::Paused;
        }
    }

    /// Feed raw audio data from the microphone.
    pub fn feed_audio(&self, samples: &[f32]) -> Result<()> {
        self.stt.lock().feed_audio(samples)
    }

    /// Process a full turn: transcribe → LLM → TTS.
    /// Returns the assistant's spoken text and audio data.
    pub async fn process_turn(&mut self) -> Result<(String, Vec<f32>)> {
        // 1. Transcribe
        self.state = ConversationState::Transcribing;
        let transcription = {
            let mut stt = self.stt.lock();
            stt.stop_listening()?;
            stt.transcribe().await?
        };

        if transcription.text.trim().is_empty() {
            self.state = ConversationState::Listening;
            self.stt.lock().start_listening()?;
            return Err(anyhow::anyhow!("Empty transcription"));
        }

        // Record user turn
        self.history.push(ConversationTurn {
            role: LlmRole::User,
            text: transcription.text.clone(),
            audio_duration_seconds: Some(transcription.duration_seconds),
            timestamp: std::time::SystemTime::now(),
        });

        // 2. LLM inference
        self.state = ConversationState::Thinking;
        let messages = self.build_messages();
        let llm_request = LlmRequest {
            messages,
            max_tokens: Some(300),
            temperature: Some(0.7),
            model: String::new(),
            top_p: None,
            stop_sequences: Vec::new(),
            stream: false,
        };

        let response = self.llm.complete(&llm_request).await?;
        let assistant_text = response.content.clone();

        // Record assistant turn
        self.history.push(ConversationTurn {
            role: LlmRole::Assistant,
            text: assistant_text.clone(),
            audio_duration_seconds: None,
            timestamp: std::time::SystemTime::now(),
        });

        // 3. TTS
        self.state = ConversationState::Speaking;
        let tts_request = TtsRequest {
            text: assistant_text.clone(),
            voice_id: self.config.voice_id.clone(),
            sample_rate: self.config.sample_rate,
            speed: 1.0,
        };

        let tts_output = {
            let mgr = self.tts.lock();
            mgr.speak(&tts_request).await?
        };

        // Update duration on the assistant turn
        if let Some(last) = self.history.last_mut() {
            last.audio_duration_seconds = Some(tts_output.duration_seconds);
        }

        // 4. Resume listening
        self.state = ConversationState::Listening;
        self.stt.lock().start_listening()?;

        Ok((assistant_text, tts_output.audio_data))
    }

    /// Build LLM message list from conversation history.
    fn build_messages(&self) -> Vec<LlmMessage> {
        let mut messages = Vec::new();
        messages.push(LlmMessage {
            role: LlmRole::System,
            content: self.config.system_prompt.clone(),
            images: Vec::new(),
        });

        // Limit to max_context_turns
        let start = if self.history.len() > self.config.max_context_turns {
            self.history.len() - self.config.max_context_turns
        } else {
            0
        };

        for turn in &self.history[start..] {
            messages.push(LlmMessage {
                role: turn.role.clone(),
                content: turn.text.clone(),
                images: Vec::new(),
            });
        }

        messages
    }

    /// Clear conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}
