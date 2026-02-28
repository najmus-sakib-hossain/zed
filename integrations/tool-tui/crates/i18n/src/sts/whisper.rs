//! Whisper-based speech-to-text

#[cfg(feature = "whisper")]
use crate::error::{I18nError, Result};
#[cfg(feature = "whisper")]
use crate::sts::SpeechToText;
#[cfg(feature = "whisper")]
use async_trait::async_trait;
#[cfg(feature = "whisper")]
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[cfg(feature = "whisper")]
pub struct WhisperSTT {
    model_path: String,
    language: Option<String>,
}

#[cfg(feature = "whisper")]
impl WhisperSTT {
    pub fn new(model_path: String, language: Option<String>) -> Self {
        Self {
            model_path,
            language,
        }
    }

    /// Create with GPU acceleration if available
    pub fn new_with_gpu(model_path: String, language: Option<String>) -> Self {
        Self {
            model_path,
            language,
        }
    }
}

#[cfg(feature = "whisper")]
#[async_trait]
impl SpeechToText for WhisperSTT {
    async fn transcribe_file(&self, path: &std::path::Path) -> Result<String> {
        // Enable GPU acceleration
        let mut ctx_params = WhisperContextParameters::default();
        ctx_params.use_gpu(true);

        let ctx = WhisperContext::new_with_params(&self.model_path, ctx_params)
            .map_err(|e| I18nError::Other(format!("Failed to load Whisper model: {}", e)))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Optimize for speed
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_non_speech_tokens(true);
        params.set_no_context(true);
        params.set_single_segment(true); // Process as single segment for speed
        params.set_max_tokens(0); // No token limit
        params.set_audio_ctx(0); // Use default audio context
        params.set_n_threads(num_cpus::get() as i32); // Use all CPU cores

        if let Some(lang) = &self.language {
            params.set_language(Some(lang));
        }

        let mut reader = hound::WavReader::open(path)
            .map_err(|e| I18nError::Other(format!("Failed to open audio file: {}", e)))?;

        let samples: Vec<f32> =
            reader.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect();

        let mut state = ctx
            .create_state()
            .map_err(|e| I18nError::Other(format!("Failed to create state: {}", e)))?;

        state
            .full(params, &samples)
            .map_err(|e| I18nError::Other(format!("Failed to transcribe: {}", e)))?;

        let num_segments = state
            .full_n_segments()
            .map_err(|e| I18nError::Other(format!("Failed to get segments: {}", e)))?;

        let mut result = String::new();
        for i in 0..num_segments {
            let segment = state
                .full_get_segment_text(i)
                .map_err(|e| I18nError::Other(format!("Failed to get segment text: {}", e)))?;
            result.push_str(&segment);
        }

        Ok(result.trim().to_string())
    }

    async fn transcribe_samples(&self, samples: &[f32]) -> Result<String> {
        let ctx =
            WhisperContext::new_with_params(&self.model_path, WhisperContextParameters::default())
                .map_err(|e| I18nError::Other(format!("Failed to load Whisper model: {}", e)))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_non_speech_tokens(true);
        params.set_no_context(true);

        if let Some(lang) = &self.language {
            params.set_language(Some(lang));
        }

        let mut state = ctx
            .create_state()
            .map_err(|e| I18nError::Other(format!("Failed to create state: {}", e)))?;

        state
            .full(params, samples)
            .map_err(|e| I18nError::Other(format!("Failed to transcribe: {}", e)))?;

        let num_segments = state
            .full_n_segments()
            .map_err(|e| I18nError::Other(format!("Failed to get segments: {}", e)))?;

        let mut result = String::new();
        for i in 0..num_segments {
            let segment = state
                .full_get_segment_text(i)
                .map_err(|e| I18nError::Other(format!("Failed to get segment text: {}", e)))?;
            result.push_str(&segment);
        }

        Ok(result.trim().to_string())
    }

    fn get_supported_languages(&self) -> Vec<&'static str> {
        vec![
            "en", "es", "fr", "de", "it", "pt", "nl", "pl", "ru", "zh", "ja", "ko",
        ]
    }
}
