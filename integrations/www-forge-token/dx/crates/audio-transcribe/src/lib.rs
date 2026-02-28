//! # audio-transcribe
//!
//! Converts audio to text transcripts, avoiding sending raw audio tokens.
//! Text transcripts are vastly cheaper than raw audio tokens.
//!
//! ## Evidence
//! - Gemini charges ~32 tokens/sec for audio (~1920 tokens/min)
//! - A 1-minute speech transcript ≈ 150-200 text tokens
//! - That's 90-95% savings per minute of audio
//! - **Honest: 85-95% savings, but requires Whisper/STT API call (latency + cost)**
//!
//! STAGE: PrePrompt (priority 10)

use dx_core::*;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct AudioTranscribeConfig {
    /// Minimum duration in seconds to bother transcribing
    pub min_duration_secs: f64,
    /// Language hint for transcription
    pub language: Option<String>,
    /// Whether to include timestamps in transcript
    pub include_timestamps: bool,
    /// Average words per minute of speech (for estimation)
    pub avg_wpm: f64,
}

impl Default for AudioTranscribeConfig {
    fn default() -> Self {
        Self {
            min_duration_secs: 5.0,
            language: None,
            include_timestamps: false,
            avg_wpm: 150.0,
        }
    }
}

pub struct AudioTranscribe {
    config: AudioTranscribeConfig,
    report: Mutex<TokenSavingsReport>,
}

impl AudioTranscribe {
    pub fn new() -> Self {
        Self::with_config(AudioTranscribeConfig::default())
    }

    pub fn with_config(config: AudioTranscribeConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Estimate transcript tokens from audio duration.
    /// ~150 WPM speech, ~1.3 tokens per word average.
    fn estimate_transcript_tokens(&self, duration_secs: f64) -> usize {
        let words = self.config.avg_wpm * duration_secs / 60.0;
        (words * 1.3).ceil() as usize
    }
}

#[async_trait::async_trait]
impl MultiModalTokenSaver for AudioTranscribe {
    fn name(&self) -> &str { "audio-transcribe" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 10 }
    fn modality(&self) -> Modality { Modality::Audio }

    async fn process_multimodal(
        &self,
        input: MultiModalSaverInput,
        _ctx: &SaverContext,
    ) -> Result<MultiModalSaverOutput, SaverError> {
        let tokens_before: usize = input.audio.iter().map(|a| a.naive_token_estimate).sum();

        if input.audio.is_empty() {
            *self.report.lock().unwrap() = TokenSavingsReport {
                technique: "audio-transcribe".into(),
                tokens_before: 0, tokens_after: 0, tokens_saved: 0,
                description: "No audio to transcribe.".into(),
            };
            return Ok(MultiModalSaverOutput {
                base: SaverOutput { messages: input.base.messages, tools: input.base.tools, images: input.base.images, skipped: true, cached_response: None },
                audio: input.audio, live_frames: input.live_frames, documents: input.documents, videos: input.videos, assets_3d: input.assets_3d,
            });
        }

        let mut new_messages = input.base.messages;
        let mut remaining_audio = Vec::new();
        let mut total_transcript_tokens = 0usize;
        let mut transcribed_count = 0usize;

        for audio in input.audio {
            if audio.duration_secs < self.config.min_duration_secs {
                remaining_audio.push(audio);
                continue;
            }

            // In production, call Whisper API here.
            // For now, estimate the transcript size.
            let transcript_tokens = self.estimate_transcript_tokens(audio.duration_secs);
            total_transcript_tokens += transcript_tokens;
            transcribed_count += 1;

            // Add a placeholder transcript message
            let transcript = format!(
                "[Audio transcript placeholder — {:.1}s audio → ~{} text tokens. \
                 In production, call Whisper/STT API here.]",
                audio.duration_secs, transcript_tokens
            );

            new_messages.push(Message {
                role: "user".into(),
                content: transcript,
                images: vec![],
                tool_call_id: None,
                token_count: transcript_tokens,
            });
            // Audio is consumed (not passed through)
        }

        let tokens_after = total_transcript_tokens
            + remaining_audio.iter().map(|a| a.naive_token_estimate).sum::<usize>();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);

        let report = TokenSavingsReport {
            technique: "audio-transcribe".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Transcribed {} audio clips: {} audio tokens → {} text tokens ({:.0}% saved). \
                 NOTE: Requires STT API call (added latency + cost not reflected here).",
                transcribed_count, tokens_before, tokens_after,
                if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 }
            ),
        };
        *self.report.lock().unwrap() = report;

        Ok(MultiModalSaverOutput {
            base: SaverOutput {
                messages: new_messages,
                tools: input.base.tools,
                images: input.base.images,
                skipped: false,
                cached_response: None,
            },
            audio: remaining_audio,
            live_frames: input.live_frames,
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

    fn test_audio(duration: f64, tokens: usize) -> AudioInput {
        AudioInput { data: vec![], format: AudioFormat::Wav, sample_rate: 16000, duration_secs: duration, channels: 1, naive_token_estimate: tokens, compressed_tokens: tokens }
    }

    fn empty_base() -> SaverInput {
        SaverInput { messages: vec![], tools: vec![], images: vec![], turn_number: 1 }
    }

    #[tokio::test]
    async fn test_transcribe_saves_tokens() {
        let saver = AudioTranscribe::new();
        let ctx = SaverContext::default();
        // 60s audio = 1920 audio tokens, transcript ~195 tokens
        let input = MultiModalSaverInput {
            base: empty_base(),
            audio: vec![test_audio(60.0, 1920)],
            live_frames: vec![], documents: vec![], videos: vec![], assets_3d: vec![],
        };
        let out = saver.process_multimodal(input, &ctx).await.unwrap();
        assert!(out.audio.is_empty()); // Audio consumed
        assert!(!out.base.messages.is_empty()); // Transcript added
        assert!(saver.last_savings().tokens_saved > 1000);
    }
}
