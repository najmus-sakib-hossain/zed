//! Microsoft Edge TTS implementation

use crate::error::{I18nError, Result};
use crate::tts::base::{TTSConfig, TextToSpeech};
use async_trait::async_trait;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use uuid::Uuid;

/// DRM module for Microsoft Edge TTS authentication
mod drm {
    use super::*;

    const WIN_EPOCH: f64 = 11644473600.0;
    const S_TO_NS: f64 = 1e9;
    const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";

    static mut CLOCK_SKEW_SECONDS: f64 = 0.0;

    pub fn get_unix_timestamp() -> f64 {
        unsafe { chrono::Utc::now().timestamp() as f64 + CLOCK_SKEW_SECONDS }
    }

    pub fn generate_sec_ms_gec() -> String {
        let ticks = get_unix_timestamp();
        let ticks = ticks + WIN_EPOCH;
        let ticks = ticks - (ticks % 300.0); // Round down to nearest 5 minutes
        let ticks = ticks * (S_TO_NS / 100.0); // Convert to 100-nanosecond intervals

        let str_to_hash = format!("{:.0}{}", ticks, TRUSTED_CLIENT_TOKEN);
        let hash = Sha256::digest(str_to_hash.as_bytes());
        format!("{:X}", hash)
    }
}

/// Microsoft Edge TTS
pub struct EdgeTTS {
    config: TTSConfig,
}

impl EdgeTTS {
    /// Create a new Edge TTS instance
    ///
    /// # Arguments
    /// * `voice` - Voice name (e.g., "en-US-AriaNeural")
    ///
    /// # Example
    /// ```no_run
    /// use i18n::tts::EdgeTTS;
    ///
    /// let tts = EdgeTTS::new("en-US-AriaNeural");
    /// ```
    pub fn new(voice: &str) -> Self {
        Self {
            config: TTSConfig {
                voice: voice.to_string(),
                rate: "+0%".to_string(),
                volume: "+0%".to_string(),
                pitch: "+0Hz".to_string(),
            },
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: TTSConfig) -> Self {
        Self { config }
    }

    /// Generate SSML for the given text
    fn mkssml(&self, escaped_text: &str) -> String {
        format!(
            r#"<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>
<voice name='{}'>
<prosody pitch='{}' rate='{}' volume='{}'>
{}
</prosody>
</voice>
</speak>"#,
            self.config.voice,
            self.config.pitch,
            self.config.rate,
            self.config.volume,
            escaped_text
        )
    }

    /// Generate JavaScript-style date string
    fn date_to_string() -> String {
        Utc::now()
            .format("%a %b %d %Y %H:%M:%S GMT+0000 (Coordinated Universal Time)")
            .to_string()
    }

    /// Generate SSML headers and data
    fn ssml_headers_plus_data(&self, request_id: &str, timestamp: &str, ssml: &str) -> String {
        format!(
            "X-RequestId:{}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{}Z\r\nPath:ssml\r\n\r\n{}",
            request_id, timestamp, ssml
        )
    }

    /// Escape text for SSML
    fn escape_text(text: &str) -> String {
        text.replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&apos;")
    }

    /// Split text by byte length (similar to Python implementation)
    fn split_text_by_byte_length(text: &str, max_bytes: usize) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut current_bytes = 0;

        for word in text.split_whitespace() {
            let word_bytes = word.len() + 1; // +1 for space
            if current_bytes + word_bytes > max_bytes && !current.is_empty() {
                result.push(current.trim().to_string());
                current = word.to_string();
                current_bytes = word.len();
            } else {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
                current_bytes += word_bytes;
            }
        }

        if !current.is_empty() {
            result.push(current.trim().to_string());
        }

        result
    }
}

#[async_trait]
impl TextToSpeech for EdgeTTS {
    async fn synthesize(&self, text: &str) -> Result<Vec<u8>> {
        let text = text.trim();
        if text.is_empty() {
            return Err(I18nError::Other("No text to speak".to_string()));
        }

        // Escape and split text
        let escaped_text = Self::escape_text(text);
        let text_chunks = Self::split_text_by_byte_length(&escaped_text, 4096);

        let mut audio_data = Vec::new();

        for chunk in text_chunks {
            let chunk_audio = self.synthesize_chunk(&chunk).await?;
            audio_data.extend(chunk_audio);
        }

        Ok(audio_data)
    }

    fn get_supported_languages(&self) -> Vec<&'static str> {
        vec![
            "en-US", "en-GB", "en-AU", "en-CA", "en-IN", "es-ES", "es-MX", "fr-FR", "fr-CA",
            "de-DE", "it-IT", "pt-BR", "pt-PT", "ru-RU", "ja-JP", "ko-KR", "zh-CN", "zh-TW",
            "ar-SA", "hi-IN",
        ]
    }
}

impl EdgeTTS {
    async fn synthesize_chunk(&self, text: &str) -> Result<Vec<u8>> {
        let connection_id = Uuid::new_v4().to_string();
        let sec_ms_gec = drm::generate_sec_ms_gec();
        let sec_ms_gec_version = format!("1-{}", crate::tts::constants::EDGE_TTS_CHROMIUM_VERSION);

        let wss_url = format!(
            "{}?TrustedClientToken={}&ConnectionId={}&Sec-MS-GEC={}&Sec-MS-GEC-Version={}",
            crate::tts::constants::EDGE_TTS_WSS_URL,
            crate::tts::constants::EDGE_TTS_TRUSTED_CLIENT_TOKEN,
            connection_id,
            sec_ms_gec,
            sec_ms_gec_version
        );

        let (ws_stream, _) = connect_async(&wss_url)
            .await
            .map_err(|e| I18nError::Other(format!("WebSocket connection failed: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();

        // Send speech config
        let config_message = format!(
            "X-Timestamp:{}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{}",
            Self::date_to_string(),
            r#"{"context":{"synthesis":{"audio":{"metadataoptions":{"sentenceBoundaryEnabled":"true","wordBoundaryEnabled":"false"},"outputFormat":"audio-24khz-48kbitrate-mono-mp3"}}}}"#
        );

        write
            .send(Message::Text(config_message))
            .await
            .map_err(|e| I18nError::Other(format!("Failed to send config: {}", e)))?;

        // Send SSML
        let request_id = Uuid::new_v4().to_string();
        let timestamp = Self::date_to_string();
        let ssml = self.mkssml(text);
        let ssml_message = self.ssml_headers_plus_data(&request_id, &timestamp, &ssml);

        write
            .send(Message::Text(ssml_message))
            .await
            .map_err(|e| I18nError::Other(format!("Failed to send SSML: {}", e)))?;

        let mut audio_received = false;
        let mut audio_data = Vec::new();

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text_msg)) => {
                    let text_bytes = text_msg.as_bytes();
                    if let Some(header_end) = text_msg.find("\r\n\r\n") {
                        let headers_str = &text_msg[..header_end];
                        let _data = &text_bytes[header_end + 4..];

                        let headers: HashMap<&str, &str> = headers_str
                            .lines()
                            .filter_map(|line| {
                                let mut parts = line.splitn(2, ':');
                                Some((parts.next()?.trim(), parts.next()?.trim()))
                            })
                            .collect();

                        if let Some(path) = headers.get("Path") {
                            match *path {
                                "audio.metadata" => {
                                    // Parse metadata if needed
                                }
                                "turn.end" => {
                                    // Turn ended, can send next chunk
                                    break;
                                }
                                "response" | "turn.start" => {
                                    // Continue
                                }
                                _ => {
                                    return Err(I18nError::Other(format!(
                                        "Unknown path: {}",
                                        path
                                    )));
                                }
                            }
                        }
                    }
                }
                Ok(Message::Binary(binary_msg)) => {
                    if binary_msg.len() < 2 {
                        continue;
                    }

                    let header_length = u16::from_be_bytes([binary_msg[0], binary_msg[1]]) as usize;
                    if header_length > binary_msg.len() {
                        continue;
                    }

                    let headers_str = std::str::from_utf8(&binary_msg[2..2 + header_length])
                        .map_err(|e| I18nError::Other(format!("Invalid headers: {}", e)))?;

                    let headers: HashMap<&str, &str> = headers_str
                        .lines()
                        .filter_map(|line| {
                            let mut parts = line.splitn(2, ':');
                            Some((parts.next()?.trim(), parts.next()?.trim()))
                        })
                        .collect();

                    if headers.get("Path") == Some(&"audio") {
                        let data_start = 2 + header_length;
                        if data_start < binary_msg.len() {
                            let audio_chunk = &binary_msg[data_start..];
                            if !audio_chunk.is_empty() {
                                audio_data.extend_from_slice(audio_chunk);
                                audio_received = true;
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => return Err(I18nError::Other(format!("WebSocket error: {}", e))),
                _ => {}
            }
        }

        if !audio_received {
            return Err(I18nError::Other("No audio received from Edge TTS".to_string()));
        }

        Ok(audio_data)
    }
}
