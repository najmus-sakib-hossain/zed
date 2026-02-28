//! # vision-compress
//!
//! Downscales images and sets detail level to minimize vision token cost.
//!
//! ## Evidence (TOKEN.md ✅ REAL — backed by OpenAI docs)
//! - "detail": "low" → flat 85 tokens per image regardless of size
//! - "detail": "high" → 170 * tiles + 85 (e.g. 1024×1024 = 765 tokens)
//! - 2048×4096 at high = 1105 tokens; at low = 85 tokens = 92% reduction
//! - **Honest savings: 70-92%** (verified from OpenAI pricing)
//!
//! STAGE: PrePrompt (priority 10)

use dx_core::*;
use image::DynamicImage;
use std::io::Cursor;
use std::sync::Mutex;

/// Configuration for vision compression.
#[derive(Debug, Clone)]
pub struct VisionCompressConfig {
    /// Max dimension (width or height) before downscaling
    pub max_dimension: u32,
    /// Target JPEG quality (1-100) for re-encoding
    pub jpeg_quality: u8,
    /// Force all images to low detail (85 tokens flat)
    pub force_low_detail: bool,
    /// Threshold: images below this many estimated tokens skip compression
    pub skip_below_tokens: usize,
    /// Max image size in bytes after compression
    pub max_bytes: usize,
}

impl Default for VisionCompressConfig {
    fn default() -> Self {
        Self {
            max_dimension: 1024,
            jpeg_quality: 75,
            force_low_detail: false,
            skip_below_tokens: 100,
            max_bytes: 512_000, // 512KB
        }
    }
}

pub struct VisionCompressSaver {
    config: VisionCompressConfig,
    report: Mutex<TokenSavingsReport>,
}

impl VisionCompressSaver {
    pub fn new() -> Self {
        Self::with_config(VisionCompressConfig::default())
    }

    pub fn with_config(config: VisionCompressConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Calculate OpenAI token cost for an image based on dimensions and detail.
    pub fn calculate_token_cost(width: u32, height: u32, detail: ImageDetail) -> usize {
        match detail {
            ImageDetail::Low => 85,
            ImageDetail::High | ImageDetail::Auto => {
                // Step 1: Scale so longest side ≤ 2048
                let (w, h) = Self::fit_within(width, height, 2048);
                // Step 2: Scale so shortest side ≤ 768
                let (w, h) = Self::fit_shortest(w, h, 768);
                // Step 3: Count 512×512 tiles
                let tiles_w = (w as f64 / 512.0).ceil() as usize;
                let tiles_h = (h as f64 / 512.0).ceil() as usize;
                let tiles = tiles_w * tiles_h;
                170 * tiles + 85
            }
        }
    }

    fn fit_within(w: u32, h: u32, max: u32) -> (u32, u32) {
        if w <= max && h <= max {
            return (w, h);
        }
        let ratio = max as f64 / w.max(h) as f64;
        ((w as f64 * ratio) as u32, (h as f64 * ratio) as u32)
    }

    fn fit_shortest(w: u32, h: u32, max_short: u32) -> (u32, u32) {
        let short = w.min(h);
        if short <= max_short {
            return (w, h);
        }
        let ratio = max_short as f64 / short as f64;
        ((w as f64 * ratio) as u32, (h as f64 * ratio) as u32)
    }

    /// Compress a single image: downscale + re-encode as JPEG.
    fn compress_image(&self, img: &ImageInput) -> Result<ImageInput, SaverError> {
        // Decode image
        let decoded = image::load_from_memory(&img.data)
            .map_err(|e| SaverError::Failed(format!("image decode: {}", e)))?;

        let (orig_w, orig_h) = (decoded.width(), decoded.height());
        let original_tokens = Self::calculate_token_cost(orig_w, orig_h, ImageDetail::High);

        // Downscale if needed
        let resized = self.resize_if_needed(decoded);
        let (new_w, new_h) = (resized.width(), resized.height());

        // Re-encode as JPEG
        let mut buf = Cursor::new(Vec::new());
        resized.write_to(&mut buf, image::ImageFormat::Jpeg)
            .map_err(|e| SaverError::Failed(format!("jpeg encode: {}", e)))?;
        let compressed_data = buf.into_inner();

        // Decide detail level
        let new_detail = if self.config.force_low_detail {
            ImageDetail::Low
        } else if compressed_data.len() < 50_000 {
            // Small images → low detail is almost free
            ImageDetail::Low
        } else {
            ImageDetail::Auto
        };

        let new_tokens = Self::calculate_token_cost(new_w, new_h, new_detail);

        Ok(ImageInput {
            data: compressed_data,
            mime: "image/jpeg".into(),
            detail: new_detail,
            original_tokens,
            processed_tokens: new_tokens,
        })
    }

    fn resize_if_needed(&self, img: DynamicImage) -> DynamicImage {
        let (w, h) = (img.width(), img.height());
        let max = self.config.max_dimension;
        if w <= max && h <= max {
            return img;
        }
        let ratio = max as f64 / w.max(h) as f64;
        let new_w = (w as f64 * ratio) as u32;
        let new_h = (h as f64 * ratio) as u32;
        img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
    }
}

#[async_trait::async_trait]
impl TokenSaver for VisionCompressSaver {
    fn name(&self) -> &str { "vision-compress" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 10 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let mut total_before = 0usize;
        let mut total_after = 0usize;
        let mut compressed_images = Vec::new();
        let mut images_processed = 0usize;

        // Process standalone images
        for img in &input.images {
            total_before += img.original_tokens;
            if img.original_tokens < self.config.skip_below_tokens {
                total_after += img.original_tokens;
                compressed_images.push(img.clone());
                continue;
            }

            match self.compress_image(img) {
                Ok(compressed) => {
                    total_after += compressed.processed_tokens;
                    images_processed += 1;
                    compressed_images.push(compressed);
                }
                Err(_) => {
                    total_after += img.original_tokens;
                    compressed_images.push(img.clone());
                }
            }
        }

        // Process images embedded in messages
        let mut messages = input.messages;
        for msg in &mut messages {
            let mut new_images = Vec::new();
            for img in &msg.images {
                total_before += img.original_tokens;
                if img.original_tokens < self.config.skip_below_tokens {
                    total_after += img.original_tokens;
                    new_images.push(img.clone());
                    continue;
                }
                match self.compress_image(img) {
                    Ok(compressed) => {
                        total_after += compressed.processed_tokens;
                        images_processed += 1;
                        new_images.push(compressed);
                    }
                    Err(_) => {
                        total_after += img.original_tokens;
                        new_images.push(img.clone());
                    }
                }
            }
            msg.images = new_images;
        }

        let tokens_saved = total_before.saturating_sub(total_after);
        let pct = if total_before > 0 {
            tokens_saved as f64 / total_before as f64 * 100.0
        } else { 0.0 };

        let report = TokenSavingsReport {
            technique: "vision-compress".into(),
            tokens_before: total_before,
            tokens_after: total_after,
            tokens_saved,
            description: if images_processed > 0 {
                format!(
                    "Compressed {} images: {} → {} tokens ({:.1}% saved). Downscale to {}px max, JPEG q{}.",
                    images_processed, total_before, total_after, pct,
                    self.config.max_dimension, self.config.jpeg_quality
                )
            } else {
                "No images to compress.".into()
            },
        };
        *self.report.lock().unwrap() = report;

        Ok(SaverOutput {
            messages,
            tools: input.tools,
            images: compressed_images,
            skipped: false,
            cached_response: None,
        })
    }

    fn last_savings(&self) -> TokenSavingsReport {
        self.report.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_cost_low_detail() {
        assert_eq!(VisionCompressSaver::calculate_token_cost(4096, 4096, ImageDetail::Low), 85);
    }

    #[test]
    fn test_token_cost_high_detail_1024x1024() {
        // 1024×1024 → fits in 2048, shortest is 1024 > 768 → scale to 768×768
        // tiles: ceil(768/512)=2 × ceil(768/512)=2 = 4
        // cost: 170*4 + 85 = 765
        let cost = VisionCompressSaver::calculate_token_cost(1024, 1024, ImageDetail::High);
        assert_eq!(cost, 765);
    }

    #[test]
    fn test_savings_low_vs_high() {
        let high = VisionCompressSaver::calculate_token_cost(2048, 4096, ImageDetail::High);
        let low = VisionCompressSaver::calculate_token_cost(2048, 4096, ImageDetail::Low);
        let savings_pct = (high - low) as f64 / high as f64 * 100.0;
        assert!(savings_pct > 90.0, "Expected >90% savings, got {:.1}%", savings_pct);
    }
}
