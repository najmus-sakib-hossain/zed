//! Audio transcription with Google Gemini and whisper-rs fallback

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct AudioArgs {
    #[command(subcommand)]
    pub command: AudioCommand,
}

#[derive(Subcommand)]
pub enum AudioCommand {
    /// Transcribe audio file to text
    Transcribe {
        /// Path to audio file (WAV, MP3, M4A, etc.)
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Output markdown file path (optional)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Use whisper-rs instead of Google Gemini
        #[arg(long)]
        use_whisper: bool,

        /// Google AI Studio API key (or set GOOGLE_AI_STUDIO_API_KEY env var)
        #[arg(long, env = "GOOGLE_AI_STUDIO_API_KEY")]
        api_key: Option<String>,

        /// Gemini model to use
        #[arg(long, default_value = "gemini-2.0-flash-exp")]
        model: String,
    },

    /// Record audio and transcribe in real-time
    Record {
        /// Duration in seconds (optional, press Ctrl+C to stop)
        #[arg(short, long)]
        duration: Option<u64>,

        /// Output markdown file path (optional)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Use whisper-rs instead of Google Gemini
        #[arg(long)]
        use_whisper: bool,

        /// Google AI Studio API key (or set GOOGLE_AI_STUDIO_API_KEY env var)
        #[arg(long, env = "GOOGLE_AI_STUDIO_API_KEY")]
        api_key: Option<String>,
    },
}

pub async fn execute(args: AudioArgs) -> Result<()> {
    match args.command {
        AudioCommand::Transcribe {
            input,
            output,
            use_whisper,
            api_key,
            model,
        } => transcribe_file(input, output, use_whisper, api_key, model).await,
        AudioCommand::Record {
            duration,
            output,
            use_whisper,
            api_key,
        } => record_and_transcribe(duration, output, use_whisper, api_key).await,
    }
}

async fn transcribe_file(
    input: PathBuf,
    output: Option<PathBuf>,
    use_whisper: bool,
    api_key: Option<String>,
    model: String,
) -> Result<()> {
    use crate::ui::logger;

    if !input.exists() {
        anyhow::bail!("Audio file not found: {}", input.display());
    }

    logger::info(&format!("Transcribing: {}", input.display()));

    let transcript = if use_whisper {
        logger::info("Using whisper-rs for transcription...");
        transcribe_with_whisper(&input).await?
    } else {
        match api_key {
            Some(key) => {
                logger::info(&format!("Using Google Gemini ({}) for transcription...", model));
                match transcribe_with_gemini(&input, &key, &model).await {
                    Ok(text) => text,
                    Err(e) => {
                        logger::warn(&format!(
                            "Gemini failed: {}. Falling back to whisper-rs...",
                            e
                        ));
                        transcribe_with_whisper(&input).await?
                    }
                }
            }
            None => {
                logger::warn("No API key provided. Using whisper-rs...");
                transcribe_with_whisper(&input).await?
            }
        }
    };

    let markdown = format_as_markdown(&transcript);

    if let Some(output_path) = output {
        std::fs::write(&output_path, &markdown).context("Failed to write output file")?;
        logger::success(&format!("Saved to: {}", output_path.display()));
    } else {
        println!("\n{}", markdown);
    }

    Ok(())
}

async fn record_and_transcribe(
    _duration: Option<u64>,
    _output: Option<PathBuf>,
    _use_whisper: bool,
    _api_key: Option<String>,
) -> Result<()> {
    use crate::ui::logger;
    logger::info("Recording feature coming soon!");
    Ok(())
}

pub async fn transcribe_with_gemini(
    audio_path: &PathBuf,
    api_key: &str,
    model: &str,
) -> Result<String> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let audio_data = std::fs::read(audio_path).context("Failed to read audio file")?;

    let audio_base64 = STANDARD.encode(&audio_data);
    let mime_type = detect_audio_mime_type(audio_path)?;

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let request_body = serde_json::json!({
        "contents": [{
            "parts": [
                {
                    "text": "Transcribe this audio file into a professional markdown-formatted prompt. Include proper formatting, headings, and structure."
                },
                {
                    "inline_data": {
                        "mime_type": mime_type,
                        "data": audio_base64
                    }
                }
            ]
        }]
    });

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .context("Failed to send request to Gemini API")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Gemini API error: {}", error_text);
    }

    let response_json: serde_json::Value =
        response.json().await.context("Failed to parse Gemini response")?;

    let text = response_json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .context("Failed to extract text from Gemini response")?
        .to_string();

    Ok(text)
}

async fn transcribe_with_whisper(_audio_path: &PathBuf) -> Result<String> {
    anyhow::bail!("Whisper-rs transcription not yet implemented. Install whisper-rs and implement.")
}

fn detect_audio_mime_type(path: &PathBuf) -> Result<String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let mime_type = match extension.to_lowercase().as_str() {
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "webm" => "audio/webm",
        _ => "audio/mpeg",
    };

    Ok(mime_type.to_string())
}

fn format_as_markdown(text: &str) -> String {
    format!("# Audio Transcription\n\n{}\n\n---\n\n*Transcribed by DX CLI*\n", text.trim())
}
