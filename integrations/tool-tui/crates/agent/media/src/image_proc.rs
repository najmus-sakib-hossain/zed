//! Image processing module

use anyhow::Result;
use image::{DynamicImage, GenericImageView, ImageFormat as ImgFmt};
use std::path::Path;

/// Image processing operations
pub struct ImageProcessor;

impl ImageProcessor {
    /// Load an image from file
    pub fn load(path: &Path) -> Result<DynamicImage> {
        let img = image::open(path)?;
        Ok(img)
    }

    /// Load an image from bytes
    pub fn load_from_bytes(data: &[u8]) -> Result<DynamicImage> {
        let img = image::load_from_memory(data)?;
        Ok(img)
    }

    /// Resize image to fit within max dimensions, preserving aspect ratio
    pub fn resize(img: &DynamicImage, max_width: u32, max_height: u32) -> DynamicImage {
        img.resize(max_width, max_height, image::imageops::FilterType::Lanczos3)
    }

    /// Resize to exact dimensions (may distort)
    pub fn resize_exact(img: &DynamicImage, width: u32, height: u32) -> DynamicImage {
        img.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
    }

    /// Create a thumbnail
    pub fn thumbnail(img: &DynamicImage, size: u32) -> DynamicImage {
        img.thumbnail(size, size)
    }

    /// Crop image
    pub fn crop(img: &mut DynamicImage, x: u32, y: u32, width: u32, height: u32) -> DynamicImage {
        img.crop(x, y, width, height)
    }

    /// Get image dimensions
    pub fn dimensions(img: &DynamicImage) -> (u32, u32) {
        img.dimensions()
    }

    /// Save image to file in specified format
    pub fn save(img: &DynamicImage, path: &Path) -> Result<()> {
        img.save(path)?;
        Ok(())
    }

    /// Convert image to PNG bytes
    pub fn to_png_bytes(img: &DynamicImage) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, ImgFmt::Png)?;
        Ok(buf)
    }

    /// Convert image to JPEG bytes with quality (0-100)
    pub fn to_jpeg_bytes(img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
        img.write_with_encoder(encoder)?;
        Ok(buf)
    }

    /// Convert image to WebP bytes
    pub fn to_webp_bytes(img: &DynamicImage) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, ImgFmt::WebP)?;
        Ok(buf)
    }

    /// Convert to base64 data URL
    pub fn to_base64_data_url(img: &DynamicImage, format: &str) -> Result<String> {
        let (bytes, mime) = match format {
            "png" => (Self::to_png_bytes(img)?, "image/png"),
            "jpeg" | "jpg" => (Self::to_jpeg_bytes(img, 85)?, "image/jpeg"),
            "webp" => (Self::to_webp_bytes(img)?, "image/webp"),
            _ => (Self::to_png_bytes(img)?, "image/png"),
        };
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
        Ok(format!("data:{};base64,{}", mime, b64))
    }

    /// Get image metadata
    pub fn metadata(path: &Path) -> Result<super::MediaMetadata> {
        let img = image::open(path)?;
        let (width, height) = img.dimensions();
        let file_size = std::fs::metadata(path)?.len();

        Ok(super::MediaMetadata {
            media_type: Some("image".into()),
            file_size,
            mime_type: Some(super::mime_type(path)),
            width: Some(width),
            height: Some(height),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_resize() {
        // Create a small test image
        let img = DynamicImage::new_rgb8(100, 100);
        assert_eq!(ImageProcessor::dimensions(&img), (100, 100));

        let resized = ImageProcessor::resize(&img, 50, 50);
        let (w, h) = ImageProcessor::dimensions(&resized);
        assert!(w <= 50);
        assert!(h <= 50);
    }

    #[test]
    fn test_thumbnail() {
        let img = DynamicImage::new_rgb8(200, 300);
        let thumb = ImageProcessor::thumbnail(&img, 50);
        let (w, h) = ImageProcessor::dimensions(&thumb);
        assert!(w <= 50 && h <= 50);
    }

    #[test]
    fn test_to_png_bytes() {
        let img = DynamicImage::new_rgb8(10, 10);
        let bytes = ImageProcessor::to_png_bytes(&img).unwrap();
        assert!(!bytes.is_empty());
        // PNG magic bytes
        assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_to_jpeg_bytes() {
        let img = DynamicImage::new_rgb8(10, 10);
        let bytes = ImageProcessor::to_jpeg_bytes(&img, 85).unwrap();
        assert!(!bytes.is_empty());
        // JPEG magic bytes
        assert_eq!(&bytes[0..2], &[0xFF, 0xD8]);
    }
}
