//! Microsoft Edge TTS Client
//!
//! Free text-to-speech using Edge's TTS service.

use super::{AudioFormat, EdgeTtsConfig, SynthesizedAudio, TtsVoice, VoiceGender};
use crate::error::{DrivenError, Result};
use serde::Deserialize;

/// Edge TTS client (free)
#[derive(Debug, Clone)]
pub struct EdgeTtsClient {
    config: EdgeTtsConfig,
}

/// Edge TTS voice info
#[derive(Debug, Clone, Deserialize)]
struct EdgeVoice {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "ShortName")]
    short_name: String,
    #[serde(rename = "Gender")]
    gender: String,
    #[serde(rename = "Locale")]
    locale: String,
}

impl EdgeTtsClient {
    /// Create a new Edge TTS client
    pub fn new(config: &EdgeTtsConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Edge TTS is always configured (free service)
    pub fn is_configured(&self) -> bool {
        true
    }

    /// Synthesize text to speech
    pub async fn synthesize(&self, text: &str) -> Result<SynthesizedAudio> {
        self.synthesize_with_voice(text, &self.config.voice).await
    }

    /// Synthesize with a specific voice
    pub async fn synthesize_with_voice(&self, text: &str, voice: &str) -> Result<SynthesizedAudio> {
        // Build SSML request
        let ssml = self.build_ssml(text, voice);
        
        // Edge TTS uses WebSocket connection to speech.platform.bing.com
        // For production, we'd use the edge-tts protocol
        // This is a simplified implementation that shells out to edge-tts CLI
        
        let audio_data = self.synthesize_via_cli(&ssml, voice).await?;

        Ok(SynthesizedAudio {
            data: audio_data,
            format: AudioFormat::Mp3,
            duration_ms: None,
            sample_rate: Some(24000),
        })
    }

    /// Build SSML for Edge TTS
    fn build_ssml(&self, text: &str, voice: &str) -> String {
        let rate = if self.config.rate >= 0 {
            format!("+{}%", self.config.rate)
        } else {
            format!("{}%", self.config.rate)
        };

        let pitch = if self.config.pitch >= 0 {
            format!("+{}Hz", self.config.pitch)
        } else {
            format!("{}Hz", self.config.pitch)
        };

        let volume = if self.config.volume >= 0 {
            format!("+{}%", self.config.volume)
        } else {
            format!("{}%", self.config.volume)
        };

        format!(
            r#"<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
                <voice name="{}">
                    <prosody rate="{}" pitch="{}" volume="{}">
                        {}
                    </prosody>
                </voice>
            </speak>"#,
            voice,
            rate,
            pitch,
            volume,
            xml_escape(text)
        )
    }

    /// Synthesize using edge-tts CLI tool
    async fn synthesize_via_cli(&self, _ssml: &str, voice: &str) -> Result<Vec<u8>> {
        use tokio::process::Command;
        use std::process::Stdio;

        // Create temp file for output
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join(format!("dx_tts_{}.mp3", uuid::Uuid::new_v4()));

        // Run edge-tts command
        let output = Command::new("edge-tts")
            .args([
                "--voice", voice,
                "--rate", &format!("{}%", self.config.rate),
                "--pitch", &format!("{}Hz", self.config.pitch),
                "--volume", &format!("{}%", self.config.volume),
                "--write-media", output_path.to_str().unwrap(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| DrivenError::Process(format!("Failed to run edge-tts: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DrivenError::Process(format!("edge-tts failed: {}", stderr)));
        }

        // Read the output file
        let audio_data = tokio::fs::read(&output_path)
            .await
            .map_err(|e| DrivenError::Io(e))?;

        // Clean up temp file
        let _ = tokio::fs::remove_file(&output_path).await;

        Ok(audio_data)
    }

    /// List available voices
    pub async fn list_voices(&self) -> Result<Vec<TtsVoice>> {
        // Return common English voices
        // Full list can be fetched from edge-tts --list-voices
        Ok(vec![
            TtsVoice {
                id: "en-US-AriaNeural".to_string(),
                name: "Aria".to_string(),
                language: "en-US".to_string(),
                gender: Some(VoiceGender::Female),
                description: Some("US English female voice".to_string()),
            },
            TtsVoice {
                id: "en-US-GuyNeural".to_string(),
                name: "Guy".to_string(),
                language: "en-US".to_string(),
                gender: Some(VoiceGender::Male),
                description: Some("US English male voice".to_string()),
            },
            TtsVoice {
                id: "en-US-JennyNeural".to_string(),
                name: "Jenny".to_string(),
                language: "en-US".to_string(),
                gender: Some(VoiceGender::Female),
                description: Some("US English female voice".to_string()),
            },
            TtsVoice {
                id: "en-GB-SoniaNeural".to_string(),
                name: "Sonia".to_string(),
                language: "en-GB".to_string(),
                gender: Some(VoiceGender::Female),
                description: Some("British English female voice".to_string()),
            },
            TtsVoice {
                id: "en-GB-RyanNeural".to_string(),
                name: "Ryan".to_string(),
                language: "en-GB".to_string(),
                gender: Some(VoiceGender::Male),
                description: Some("British English male voice".to_string()),
            },
            TtsVoice {
                id: "en-AU-NatashaNeural".to_string(),
                name: "Natasha".to_string(),
                language: "en-AU".to_string(),
                gender: Some(VoiceGender::Female),
                description: Some("Australian English female voice".to_string()),
            },
        ])
    }
}

/// Escape XML special characters
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("Hello <world>"), "Hello &lt;world&gt;");
        assert_eq!(xml_escape("A & B"), "A &amp; B");
    }

    #[test]
    fn test_build_ssml() {
        let config = EdgeTtsConfig {
            voice: "en-US-AriaNeural".to_string(),
            rate: 0,
            pitch: 0,
            volume: 0,
        };
        let client = EdgeTtsClient::new(&config);
        let ssml = client.build_ssml("Hello", "en-US-AriaNeural");
        assert!(ssml.contains("en-US-AriaNeural"));
        assert!(ssml.contains("Hello"));
    }
}
