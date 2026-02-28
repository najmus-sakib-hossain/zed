//! Google Speech Recognition (Free API) implementation

use crate::error::{I18nError, Result};
use crate::sts::base::SpeechToText;
use async_trait::async_trait;
use std::path::Path;

const GOOGLE_SPEECH_API_URL: &str = "http://www.google.com/speech-api/v2/recognize";
// NOTE: Google API key is currently not working, use Whisper instead
// const DEFAULT_API_KEY: &str = "AIzaSyBOti4mM-6x9WDnZIjIeyEU21OpBXqWBgw";
const DEFAULT_API_KEY: &str = "";

/// Google Speech Recognition using free API
pub struct GoogleSTT {
    api_key: String,
    language: String,
    client: reqwest::Client,
    timeout: std::time::Duration,
}

impl GoogleSTT {
    /// Create a new Google STT instance
    ///
    /// # Arguments
    /// * `language` - Language code (e.g., "en-US", "es-ES")
    /// * `api_key` - Optional API key (REQUIRED - default key is disabled)
    ///
    /// # Example
    /// ```no_run
    /// use dx_i18n::sts::GoogleSTT;
    ///
    /// // NOTE: Google API is currently not working, use WhisperSTT or AutoSTT instead
    /// let stt = GoogleSTT::new("en-US", Some("your-api-key".to_string()));
    /// ```
    pub fn new(language: impl Into<String>, api_key: Option<String>) -> Self {
        let timeout = std::time::Duration::from_secs(5);
        Self {
            api_key: api_key.unwrap_or_else(|| DEFAULT_API_KEY.to_string()),
            language: language.into(),
            client: reqwest::Client::builder()
                .timeout(timeout)
                .connect_timeout(std::time::Duration::from_secs(3))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            timeout,
        }
    }

    /// Set custom timeout
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self.client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        self
    }

    /// Convert audio samples to FLAC format
    fn samples_to_flac(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        // Create WAV in memory first
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut wav_buffer = std::io::Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut wav_buffer, spec)
            .map_err(|e| I18nError::Other(format!("Failed to create WAV writer: {}", e)))?;

        for &sample in samples {
            let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer
                .write_sample(sample_i16)
                .map_err(|e| I18nError::Other(format!("Failed to write sample: {}", e)))?;
        }

        writer
            .finalize()
            .map_err(|e| I18nError::Other(format!("Failed to finalize WAV: {}", e)))?;

        let wav_data = wav_buffer.into_inner();

        // Try to convert to FLAC using ffmpeg
        match Command::new("ffmpeg")
            .args(&[
                "-f",
                "wav",
                "-i",
                "pipe:0",
                "-f",
                "flac",
                "-compression_level",
                "0",
                "pipe:1",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(&wav_data).map_err(|e| {
                        I18nError::Other(format!("Failed to write to ffmpeg: {}", e))
                    })?;
                }

                let output = child
                    .wait_with_output()
                    .map_err(|e| I18nError::Other(format!("Failed to wait for ffmpeg: {}", e)))?;

                if output.status.success() && !output.stdout.is_empty() {
                    return Ok(output.stdout);
                }
            }
            Err(_) => {
                // ffmpeg not available, return WAV data
                eprintln!(
                    "Warning: ffmpeg not found, using WAV format (may not work with Google API)"
                );
            }
        }

        // Fallback to WAV if FLAC conversion fails
        Ok(wav_data)
    }

    /// Load and convert audio file to appropriate format
    fn load_audio_file(path: &Path) -> Result<(Vec<f32>, u32)> {
        let reader = hound::WavReader::open(path)
            .map_err(|e| I18nError::Other(format!("Failed to open audio file: {}", e)))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader
                .into_samples::<f32>()
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| I18nError::Other(format!("Failed to read samples: {}", e)))?,
            hound::SampleFormat::Int => {
                let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .into_samples::<i32>()
                    .map(|s| s.map(|v| v as f32 / max_val))
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| I18nError::Other(format!("Failed to read samples: {}", e)))?
            }
        };

        // Convert stereo to mono if needed
        let samples = if spec.channels == 2 {
            samples.chunks(2).map(|chunk| (chunk[0] + chunk[1]) / 2.0).collect()
        } else {
            samples
        };

        Ok((samples, sample_rate))
    }
}

#[async_trait]
impl SpeechToText for GoogleSTT {
    async fn transcribe_file(&self, path: &Path) -> Result<String> {
        let (samples, _sample_rate) = Self::load_audio_file(path)?;
        self.transcribe_samples(&samples).await
    }

    async fn transcribe_samples(&self, samples: &[f32]) -> Result<String> {
        // Limit to 15 seconds max for faster processing
        let max_samples = 16000 * 15;
        let samples = if samples.len() > max_samples {
            &samples[..max_samples]
        } else {
            samples
        };

        let sample_rate = 16000u32;
        let audio_data = Self::samples_to_flac(samples, sample_rate)?;

        let content_type = if audio_data.starts_with(b"fLaC") {
            format!("audio/x-flac; rate={}", sample_rate)
        } else {
            format!("audio/l16; rate={}", sample_rate)
        };

        let url = format!(
            "{}?client=chromium&lang={}&key={}&pFilter=0",
            GOOGLE_SPEECH_API_URL, self.language, self.api_key
        );

        let response = tokio::time::timeout(
            self.timeout,
            self.client
                .post(&url)
                .header("Content-Type", content_type)
                .body(audio_data)
                .send(),
        )
        .await
        .map_err(|_| I18nError::Other("Google API timeout".to_string()))??;

        if !response.status().is_success() {
            return Err(I18nError::ServerError {
                code: response.status().as_u16(),
                message: format!("Google API error: {}", response.status()),
            });
        }

        let response_text = response.text().await?;

        for line in response_text.lines().rev() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(result) = json.get("result").and_then(|r| r.as_array()) {
                    if !result.is_empty() {
                        if let Some(alternative) = result[0]
                            .get("alternative")
                            .and_then(|a| a.as_array())
                            .and_then(|a| a.first())
                        {
                            if let Some(text) =
                                alternative.get("transcript").and_then(|t| t.as_str())
                            {
                                return Ok(text.to_string());
                            }
                        }
                    }
                }
            }
        }

        Err(I18nError::TranslationNotFound("No transcription found".to_string()))
    }

    fn get_supported_languages(&self) -> Vec<&'static str> {
        vec![
            "en-US", "en-GB", "en-AU", "en-CA", "en-IN", "es-ES", "es-MX", "fr-FR", "fr-CA",
            "de-DE", "it-IT", "pt-BR", "pt-PT", "ru-RU", "ja-JP", "ko-KR", "zh-CN", "zh-TW",
            "ar-SA", "hi-IN", "nl-NL", "pl-PL", "tr-TR", "sv-SE", "da-DK", "no-NO", "fi-FI",
        ]
    }
}
