//! Media validation — size, format, and MIME type checks.
//!
//! Ensures media attachments meet platform-specific limits
//! before they are sent through a channel.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Per-channel limits on media uploads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaLimits {
    /// Maximum image file size in bytes.
    pub max_image_size: u64,
    /// Maximum video file size in bytes.
    pub max_video_size: u64,
    /// Maximum audio file size in bytes.
    pub max_audio_size: u64,
    /// Maximum document / generic file size in bytes.
    pub max_document_size: u64,
    /// Allowed MIME types (empty = allow all).
    pub allowed_formats: Vec<String>,
}

impl Default for MediaLimits {
    fn default() -> Self {
        Self {
            max_image_size: 10 * 1024 * 1024,    // 10 MiB
            max_video_size: 50 * 1024 * 1024,    // 50 MiB
            max_audio_size: 20 * 1024 * 1024,    // 20 MiB
            max_document_size: 50 * 1024 * 1024, // 50 MiB
            allowed_formats: Vec::new(),
        }
    }
}

/// Predefined limits for common platforms.
impl MediaLimits {
    /// Telegram limits (20 MiB images, 50 MiB video/audio).
    pub fn telegram() -> Self {
        Self {
            max_image_size: 10 * 1024 * 1024,
            max_video_size: 50 * 1024 * 1024,
            max_audio_size: 50 * 1024 * 1024,
            max_document_size: 50 * 1024 * 1024,
            allowed_formats: vec![
                "image/jpeg".into(),
                "image/png".into(),
                "image/gif".into(),
                "image/webp".into(),
                "video/mp4".into(),
                "audio/ogg".into(),
                "audio/mpeg".into(),
            ],
        }
    }

    /// Discord limits (25 MiB for non-Nitro).
    pub fn discord() -> Self {
        Self {
            max_image_size: 25 * 1024 * 1024,
            max_video_size: 25 * 1024 * 1024,
            max_audio_size: 25 * 1024 * 1024,
            max_document_size: 25 * 1024 * 1024,
            allowed_formats: Vec::new(),
        }
    }

    /// Slack limits (configurable, default free plan).
    pub fn slack() -> Self {
        Self {
            max_image_size: 1024 * 1024 * 1024, // 1 GiB
            max_video_size: 1024 * 1024 * 1024,
            max_audio_size: 1024 * 1024 * 1024,
            max_document_size: 1024 * 1024 * 1024,
            allowed_formats: Vec::new(),
        }
    }
}

/// Result of a media inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    /// File size in bytes.
    pub size: u64,
    /// Detected format / extension hint.
    pub format: String,
    /// MIME type.
    pub mime_type: String,
}

/// Validate a media blob against the given limits.
///
/// `media_type` is a MIME type string (e.g. `"image/png"`).
pub fn validate_media(data: &[u8], media_type: &str, limits: &MediaLimits) -> Result<()> {
    let size = data.len() as u64;
    let category = media_category(media_type);

    // Size check
    let max_size = match category {
        MediaCategory::Image => limits.max_image_size,
        MediaCategory::Video => limits.max_video_size,
        MediaCategory::Audio => limits.max_audio_size,
        MediaCategory::Document => limits.max_document_size,
    };

    if size > max_size {
        bail!("Media too large: {} bytes (max {} bytes for {})", size, max_size, media_type);
    }

    // Format check
    if !limits.allowed_formats.is_empty()
        && !limits.allowed_formats.iter().any(|f| f.eq_ignore_ascii_case(media_type))
    {
        bail!(
            "Media format '{}' is not allowed. Allowed: {:?}",
            media_type,
            limits.allowed_formats
        );
    }

    Ok(())
}

/// Extract basic info from a media blob.
///
/// Uses magic bytes for detection when possible,
/// otherwise falls back to the supplied `media_type`.
pub fn get_media_info(data: &[u8], media_type: &str) -> MediaInfo {
    let format = detect_format(data)
        .unwrap_or_else(|| media_type.rsplit('/').next().unwrap_or("unknown").to_string());

    MediaInfo {
        size: data.len() as u64,
        format,
        mime_type: media_type.to_string(),
    }
}

/// Broad media category derived from MIME type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MediaCategory {
    Image,
    Video,
    Audio,
    Document,
}

fn media_category(mime: &str) -> MediaCategory {
    let lower = mime.to_lowercase();
    if lower.starts_with("image/") {
        MediaCategory::Image
    } else if lower.starts_with("video/") {
        MediaCategory::Video
    } else if lower.starts_with("audio/") {
        MediaCategory::Audio
    } else {
        MediaCategory::Document
    }
}

/// Simple magic-byte format detector.
fn detect_format(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }
    // PNG
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Some("png".into());
    }
    // JPEG
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpeg".into());
    }
    // GIF
    if data.starts_with(b"GIF8") {
        return Some("gif".into());
    }
    // WebP
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some("webp".into());
    }
    // PDF
    if data.starts_with(b"%PDF") {
        return Some("pdf".into());
    }
    // MP4 / ftyp
    if data.len() >= 8 && &data[4..8] == b"ftyp" {
        return Some("mp4".into());
    }
    // OGG
    if data.starts_with(b"OggS") {
        return Some("ogg".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_media_ok() {
        let limits = MediaLimits::default();
        let data = vec![0u8; 1024]; // 1 KiB
        let result = validate_media(&data, "image/png", &limits);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_media_too_large() {
        let limits = MediaLimits {
            max_image_size: 100,
            ..Default::default()
        };
        let data = vec![0u8; 200];
        let result = validate_media(&data, "image/png", &limits);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_media_format_not_allowed() {
        let limits = MediaLimits {
            allowed_formats: vec!["image/png".into()],
            ..Default::default()
        };
        let data = vec![0u8; 100];
        let result = validate_media(&data, "image/gif", &limits);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_media_format_allowed() {
        let limits = MediaLimits {
            allowed_formats: vec!["image/png".into()],
            ..Default::default()
        };
        let data = vec![0u8; 100];
        let result = validate_media(&data, "image/png", &limits);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_media_empty_formats_allows_all() {
        let limits = MediaLimits::default();
        let data = vec![0u8; 100];
        let result = validate_media(&data, "application/x-custom", &limits);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_media_info() {
        let data = vec![0u8; 512];
        let info = get_media_info(&data, "image/png");
        assert_eq!(info.size, 512);
        assert_eq!(info.mime_type, "image/png");
    }

    #[test]
    fn test_detect_png() {
        let data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
        let info = get_media_info(&data, "image/png");
        assert_eq!(info.format, "png");
    }

    #[test]
    fn test_detect_jpeg() {
        let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        let info = get_media_info(&data, "image/jpeg");
        assert_eq!(info.format, "jpeg");
    }

    #[test]
    fn test_detect_gif() {
        let data = b"GIF89a\x00\x00";
        let info = get_media_info(data, "image/gif");
        assert_eq!(info.format, "gif");
    }

    #[test]
    fn test_video_category() {
        let limits = MediaLimits {
            max_video_size: 100,
            ..Default::default()
        };
        let data = vec![0u8; 200];
        let result = validate_media(&data, "video/mp4", &limits);
        assert!(result.is_err());
    }

    #[test]
    fn test_platform_limits() {
        let tg = MediaLimits::telegram();
        assert!(tg.max_image_size > 0);
        assert!(!tg.allowed_formats.is_empty());

        let dc = MediaLimits::discord();
        assert_eq!(dc.max_image_size, 25 * 1024 * 1024);

        let sl = MediaLimits::slack();
        assert!(sl.max_image_size > 100 * 1024 * 1024);
    }
}
