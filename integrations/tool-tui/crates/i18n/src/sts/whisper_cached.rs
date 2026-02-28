//! Cached Whisper STT - keeps model loaded in memory

#[cfg(feature = "whisper")]
use crate::error::{I18nError, Result};
#[cfg(feature = "whisper")]
use lazy_static::lazy_static;
#[cfg(feature = "whisper")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "whisper")]
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[cfg(feature = "whisper")]
lazy_static! {
    static ref WHISPER_CONTEXT: Arc<Mutex<Option<WhisperContext>>> = Arc::new(Mutex::new(None));
}

#[cfg(feature = "whisper")]
pub struct CachedWhisperSTT {
    model_path: String,
    language: Option<String>,
}

#[cfg(feature = "whisper")]
impl CachedWhisperSTT {
    pub fn new(model_path: String, language: Option<String>) -> Self {
        Self {
            model_path,
            language,
        }
    }

    /// Get or initialize the cached Whisper context
    fn get_context(&self) -> Result<Arc<Mutex<Option<WhisperContext>>>> {
        let mut ctx_guard = WHISPER_CONTEXT
            .lock()
            .map_err(|e| I18nError::Other(format!("Failed to lock context: {}", e)))?;

        if ctx_guard.is_none() {
            let mut ctx_params = WhisperContextParameters::default();
            ctx_params.use_gpu(true);

            let ctx = WhisperContext::new_with_params(&self.model_path, ctx_params)
                .map_err(|e| I18nError::Other(format!("Failed to load Whisper model: {}", e)))?;

            *ctx_guard = Some(ctx);
        }

        Ok(WHISPER_CONTEXT.clone())
    }

    /// Transcribe audio file with cached model
    pub fn transcribe_file(&self, path: &std::path::Path) -> Result<String> {
        let ctx_arc = self.get_context()?;
        let ctx_guard = ctx_arc
            .lock()
            .map_err(|e| I18nError::Other(format!("Failed to lock context: {}", e)))?;

        let ctx = ctx_guard
            .as_ref()
            .ok_or_else(|| I18nError::Other("Context not initialized".to_string()))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_non_speech_tokens(true);
        params.set_no_context(true);
        params.set_single_segment(true);
        params.set_n_threads(num_cpus::get() as i32);

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
}
