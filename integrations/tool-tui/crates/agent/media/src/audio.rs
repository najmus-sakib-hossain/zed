//! Audio processing module

use anyhow::Result;
use std::path::Path;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Audio processor
pub struct AudioProcessor;

impl AudioProcessor {
    /// Get audio metadata (duration, sample rate, channels, etc.)
    pub fn metadata(path: &Path) -> Result<super::MediaMetadata> {
        let file = std::fs::File::open(path)?;
        let file_size = file.metadata()?.len();
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let format = probed.format;
        let mut meta = super::MediaMetadata {
            media_type: Some("audio".into()),
            file_size,
            mime_type: Some(super::mime_type(path)),
            ..Default::default()
        };

        // Get track info
        if let Some(track) = format.default_track() {
            let params = &track.codec_params;

            if let Some(sr) = params.sample_rate {
                meta.sample_rate = Some(sr);
            }

            if let Some(ch) = params.channels {
                meta.channels = Some(ch.count() as u32);
            }

            // Calculate duration if time_base and n_frames are available
            if let (Some(tb), Some(frames)) = (params.time_base, params.n_frames) {
                let duration = tb.calc_time(frames);
                meta.duration_secs = Some(duration.seconds as f64 + duration.frac);
            }
        }

        Ok(meta)
    }

    /// Transcribe audio using Whisper API
    pub async fn transcribe(path: &Path, api_key: &str, model: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let file_bytes = tokio::fs::read(path).await?;
        let file_name =
            path.file_name().and_then(|n| n.to_str()).unwrap_or("audio.mp3").to_string();

        let part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name)
            .mime_str("audio/mpeg")?;

        let form = reqwest::multipart::Form::new()
            .text("model", model.to_string())
            .part("file", part);

        let resp = client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await?;

        let body: serde_json::Value = resp.json().await?;
        Ok(body["text"].as_str().unwrap_or("").to_string())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_audio_module_exists() {
        // Basic test ensuring module compiles
        assert!(true);
    }
}
