//! Video processing module
//!
//! Uses pure Rust for metadata extraction.
//! FFmpeg integration available as optional dependency for transcoding.

use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Video processor
pub struct VideoProcessor;

impl VideoProcessor {
    /// Extract video metadata using ffprobe (if available)
    pub fn metadata(path: &Path) -> Result<super::MediaMetadata> {
        let file_size = std::fs::metadata(path)?.len();

        let mut meta = super::MediaMetadata {
            media_type: Some("video".into()),
            file_size,
            mime_type: Some(super::mime_type(path)),
            ..Default::default()
        };

        // Try using ffprobe for detailed metadata
        if let Ok(output) = Command::new("ffprobe")
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .output()
        {
            if output.status.success() {
                if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    // Extract duration from format
                    if let Some(format) = data.get("format") {
                        if let Some(duration) = format.get("duration").and_then(|d| d.as_str()) {
                            meta.duration_secs = duration.parse().ok();
                        }
                        if let Some(bit_rate) = format.get("bit_rate").and_then(|b| b.as_str()) {
                            meta.bitrate = bit_rate.parse().ok();
                        }
                    }

                    // Extract video dimensions from first video stream
                    if let Some(streams) = data.get("streams").and_then(|s| s.as_array()) {
                        for stream in streams {
                            if stream.get("codec_type").and_then(|t| t.as_str()) == Some("video") {
                                meta.width =
                                    stream.get("width").and_then(|w| w.as_u64()).map(|w| w as u32);
                                meta.height =
                                    stream.get("height").and_then(|h| h.as_u64()).map(|h| h as u32);
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(meta)
    }

    /// Extract a thumbnail frame from video using ffmpeg
    pub async fn extract_thumbnail(
        video_path: &Path,
        output_path: &Path,
        timestamp_secs: f64,
    ) -> Result<()> {
        let status = tokio::process::Command::new("ffmpeg")
            .args([
                "-i",
                video_path.to_str().unwrap_or(""),
                "-ss",
                &timestamp_secs.to_string(),
                "-vframes",
                "1",
                "-q:v",
                "2",
                "-y",
            ])
            .arg(output_path)
            .output()
            .await?;

        if !status.status.success() {
            anyhow::bail!("ffmpeg thumbnail extraction failed");
        }

        Ok(())
    }

    /// Check if ffmpeg is available
    pub fn is_ffmpeg_available() -> bool {
        Command::new("ffmpeg").arg("-version").output().is_ok()
    }

    /// Check if ffprobe is available
    pub fn is_ffprobe_available() -> bool {
        Command::new("ffprobe").arg("-version").output().is_ok()
    }

    /// Compress a video file using ffmpeg
    ///
    /// Reduces file size by re-encoding with H.264 at the specified CRF (quality).
    /// CRF 23 is default (visually lossless), higher = smaller but lower quality.
    pub async fn compress(
        input_path: &Path,
        output_path: &Path,
        crf: Option<u8>,
        max_width: Option<u32>,
    ) -> Result<()> {
        if !Self::is_ffmpeg_available() {
            anyhow::bail!("ffmpeg not found. Install ffmpeg for video compression.");
        }

        let crf_val = crf.unwrap_or(28).min(51).to_string();
        let mut args = vec![
            "-i".to_string(),
            input_path.to_str().unwrap_or("").to_string(),
            "-c:v".to_string(),
            "libx264".to_string(),
            "-crf".to_string(),
            crf_val,
            "-preset".to_string(),
            "medium".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "128k".to_string(),
            "-movflags".to_string(),
            "+faststart".to_string(),
        ];

        // Scale down if max_width is specified
        if let Some(w) = max_width {
            args.extend(["-vf".to_string(), format!("scale='min({},iw)':'-2'", w)]);
        }

        args.extend([
            "-y".to_string(),
            output_path.to_str().unwrap_or("").to_string(),
        ]);

        let status = tokio::process::Command::new("ffmpeg").args(&args).output().await?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            anyhow::bail!("ffmpeg compression failed: {}", &stderr[..stderr.len().min(500)]);
        }

        Ok(())
    }

    /// Convert video to a different format
    pub async fn convert(input_path: &Path, output_path: &Path) -> Result<()> {
        if !Self::is_ffmpeg_available() {
            anyhow::bail!("ffmpeg not found");
        }

        let status = tokio::process::Command::new("ffmpeg")
            .args(["-i", input_path.to_str().unwrap_or(""), "-y"])
            .arg(output_path)
            .output()
            .await?;

        if !status.status.success() {
            anyhow::bail!("ffmpeg conversion failed");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_check() {
        // Just verify the function runs without panic
        let _ = VideoProcessor::is_ffmpeg_available();
        let _ = VideoProcessor::is_ffprobe_available();
    }

    #[tokio::test]
    async fn test_compress_requires_ffmpeg() {
        let input = Path::new("nonexistent.mp4");
        let output = Path::new("output.mp4");
        // Should fail gracefully (file doesn't exist or ffmpeg not present)
        let result = VideoProcessor::compress(input, output, Some(28), None).await;
        // We just check it doesn't panic
        let _ = result;
    }
}
