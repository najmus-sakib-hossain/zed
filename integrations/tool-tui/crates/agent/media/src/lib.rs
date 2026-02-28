//! DX Agent Media Pipeline
//!
//! Pure Rust media processing for images, audio, video, and PDFs.
//! Used by channels for media handling and by the agent for content analysis.

pub mod audio;
pub mod image_proc;
pub mod pdf;
pub mod video;

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Media type detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Image(ImageFormat),
    Audio(AudioFormat),
    Video(VideoFormat),
    Pdf,
    Unknown,
}

/// Image formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Png,
    Jpeg,
    WebP,
    Gif,
    Bmp,
    Svg,
    Avif,
}

/// Audio formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    Wav,
    Ogg,
    Flac,
    Aac,
    M4a,
    Opus,
}

/// Video formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFormat {
    Mp4,
    Webm,
    Mkv,
    Avi,
    Mov,
}

/// Media metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub media_type: Option<String>,
    pub file_size: u64,
    pub mime_type: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_secs: Option<f64>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub page_count: Option<u32>,
}

/// Detect media type from file extension
pub fn detect_media_type(path: &Path) -> MediaType {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_lowercase().as_str() {
        "png" => MediaType::Image(ImageFormat::Png),
        "jpg" | "jpeg" => MediaType::Image(ImageFormat::Jpeg),
        "webp" => MediaType::Image(ImageFormat::WebP),
        "gif" => MediaType::Image(ImageFormat::Gif),
        "bmp" => MediaType::Image(ImageFormat::Bmp),
        "svg" => MediaType::Image(ImageFormat::Svg),
        "avif" => MediaType::Image(ImageFormat::Avif),
        "mp3" => MediaType::Audio(AudioFormat::Mp3),
        "wav" => MediaType::Audio(AudioFormat::Wav),
        "ogg" => MediaType::Audio(AudioFormat::Ogg),
        "flac" => MediaType::Audio(AudioFormat::Flac),
        "aac" => MediaType::Audio(AudioFormat::Aac),
        "m4a" => MediaType::Audio(AudioFormat::M4a),
        "opus" => MediaType::Audio(AudioFormat::Opus),
        "mp4" => MediaType::Video(VideoFormat::Mp4),
        "webm" => MediaType::Video(VideoFormat::Webm),
        "mkv" => MediaType::Video(VideoFormat::Mkv),
        "avi" => MediaType::Video(VideoFormat::Avi),
        "mov" => MediaType::Video(VideoFormat::Mov),
        "pdf" => MediaType::Pdf,
        _ => MediaType::Unknown,
    }
}

/// Get mime type for a file
pub fn mime_type(path: &Path) -> String {
    mime_guess::from_path(path).first_or_octet_stream().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_media_type() {
        assert_eq!(
            detect_media_type(&PathBuf::from("photo.jpg")),
            MediaType::Image(ImageFormat::Jpeg)
        );
        assert_eq!(
            detect_media_type(&PathBuf::from("song.mp3")),
            MediaType::Audio(AudioFormat::Mp3)
        );
        assert_eq!(
            detect_media_type(&PathBuf::from("clip.mp4")),
            MediaType::Video(VideoFormat::Mp4)
        );
        assert_eq!(detect_media_type(&PathBuf::from("doc.pdf")), MediaType::Pdf);
        assert_eq!(detect_media_type(&PathBuf::from("file.xyz")), MediaType::Unknown);
    }

    #[test]
    fn test_mime_type() {
        assert_eq!(mime_type(&PathBuf::from("photo.jpg")), "image/jpeg");
        assert_eq!(mime_type(&PathBuf::from("song.mp3")), "audio/mpeg");
    }
}
