//! # vision-select
//!
//! Adaptive two-pass vision token selection: low-detail overview first,
//! then high-detail crops of regions of interest (ROIs).
//!
//! ## Evidence (TOKEN.md 🤷 Unproven)
//! - Concept is real (two-pass: overview → targeted crops)
//! - Grid-based edge density is crude; many important regions have low edges
//! - Adding multiple images (overview + crops) has its own overhead
//! - **Needs real testing before claiming 60-80%**
//! - This implementation is experimental/proof-of-concept
//!
//! STAGE: PrePrompt (priority 8)

use dx_core::*;
use std::sync::Mutex;

/// A region of interest in an image.
#[derive(Debug, Clone)]
pub struct Roi {
    /// X offset (pixels)
    pub x: u32,
    /// Y offset (pixels)  
    pub y: u32,
    /// Width (pixels)
    pub width: u32,
    /// Height (pixels)
    pub height: u32,
    /// Interest score (0.0-1.0)
    pub score: f64,
}

/// Configuration for vision selection.
#[derive(Debug, Clone)]
pub struct VisionSelectConfig {
    /// Grid size for ROI detection
    pub grid_size: u32,
    /// Minimum interest score to include a crop
    pub min_roi_score: f64,
    /// Maximum number of high-detail crop regions
    pub max_crops: usize,
    /// Minimum image tokens before two-pass is worthwhile
    pub min_image_tokens: usize,
    /// Target resolution for ROI crops
    pub crop_size: u32,
}

impl Default for VisionSelectConfig {
    fn default() -> Self {
        Self {
            grid_size: 4,
            min_roi_score: 0.3,
            max_crops: 3,
            min_image_tokens: 400,
            crop_size: 512,
        }
    }
}

pub struct VisionSelectSaver {
    config: VisionSelectConfig,
    report: Mutex<TokenSavingsReport>,
}

impl VisionSelectSaver {
    pub fn new() -> Self {
        Self::with_config(VisionSelectConfig::default())
    }

    pub fn with_config(config: VisionSelectConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Detect regions of interest using edge density + variance analysis.
    /// This is a simple proof-of-concept; production should use a proper detector.
    fn detect_rois(&self, img: &image::DynamicImage) -> Vec<Roi> {
        let gray = img.to_luma8();
        let (w, h) = (gray.width(), gray.height());
        if w < self.config.grid_size || h < self.config.grid_size {
            return vec![];
        }

        let cell_w = w / self.config.grid_size;
        let cell_h = h / self.config.grid_size;
        let mut rois = Vec::new();

        for gy in 0..self.config.grid_size {
            for gx in 0..self.config.grid_size {
                let x0 = gx * cell_w;
                let y0 = gy * cell_h;

                // Calculate edge density and variance for this cell
                let mut edge_count = 0u64;
                let mut sum = 0u64;
                let mut sum_sq = 0u64;
                let mut pixel_count = 0u64;

                for y in y0..(y0 + cell_h).min(h) {
                    for x in x0..(x0 + cell_w).min(w) {
                        let val = gray.get_pixel(x, y)[0] as u64;
                        sum += val;
                        sum_sq += val * val;
                        pixel_count += 1;

                        // Edge detection (simple gradient)
                        if x > 0 {
                            let diff = (val as i64 - gray.get_pixel(x - 1, y)[0] as i64).unsigned_abs();
                            if diff > 25 { edge_count += 1; }
                        }
                        if y > 0 {
                            let diff = (val as i64 - gray.get_pixel(x, y - 1)[0] as i64).unsigned_abs();
                            if diff > 25 { edge_count += 1; }
                        }
                    }
                }

                if pixel_count == 0 { continue; }
                let mean = sum as f64 / pixel_count as f64;
                let variance = (sum_sq as f64 / pixel_count as f64) - mean * mean;
                let edge_density = edge_count as f64 / pixel_count as f64;

                // Score combines edge density and variance (both indicate content)
                let score = (edge_density * 5.0 + (variance.sqrt() / 128.0)).min(1.0);

                if score >= self.config.min_roi_score {
                    rois.push(Roi {
                        x: x0,
                        y: y0,
                        width: cell_w,
                        height: cell_h,
                        score,
                    });
                }
            }
        }

        // Sort by score descending and take top N
        rois.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        rois.truncate(self.config.max_crops);
        rois
    }

    /// Process a single image: create overview + ROI crops.
    fn process_image(&self, img_input: &ImageInput) -> Result<Vec<ImageInput>, SaverError> {
        if img_input.original_tokens < self.config.min_image_tokens {
            return Ok(vec![img_input.clone()]);
        }

        let img = image::load_from_memory(&img_input.data)
            .map_err(|e| SaverError::Failed(format!("Image decode failed: {}", e)))?;

        // 1. Create low-detail overview (85 tokens flat)
        let overview = ImageInput {
            data: img_input.data.clone(),
            mime: img_input.mime.clone(),
            detail: ImageDetail::Low,
            original_tokens: img_input.original_tokens,
            processed_tokens: 85,
        };

        // 2. Detect ROIs and create crops
        let rois = self.detect_rois(&img);
        let mut result = vec![overview];

        for roi in &rois {
            let cropped = img.crop_imm(roi.x, roi.y, roi.width, roi.height);
            let resized = if roi.width > self.config.crop_size || roi.height > self.config.crop_size {
                cropped.resize(
                    self.config.crop_size,
                    self.config.crop_size,
                    image::imageops::FilterType::Lanczos3,
                )
            } else {
                cropped
            };

            let mut buf = std::io::Cursor::new(Vec::new());
            resized.write_to(&mut buf, image::ImageFormat::Jpeg)
                .map_err(|e| SaverError::Failed(format!("JPEG encode: {}", e)))?;

            let crop_tokens = 170 + 85; // 1 tile + base (512×512 crop)
            result.push(ImageInput {
                data: buf.into_inner(),
                mime: "image/jpeg".into(),
                detail: ImageDetail::High,
                original_tokens: 0, // crop, not original
                processed_tokens: crop_tokens,
            });
        }

        Ok(result)
    }
}

#[async_trait::async_trait]
impl TokenSaver for VisionSelectSaver {
    fn name(&self) -> &str { "vision-select" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 8 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.images.iter().map(|i| i.original_tokens).sum();
        let mut new_images = Vec::new();
        let mut processed_count = 0usize;

        for img in &input.images {
            match self.process_image(img) {
                Ok(images) => {
                    if images.len() > 1 { processed_count += 1; }
                    new_images.extend(images);
                }
                Err(_) => new_images.push(img.clone()),
            }
        }

        let tokens_after: usize = new_images.iter().map(|i| i.processed_tokens).sum();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);
        let pct = if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 };

        let report = TokenSavingsReport {
            technique: "vision-select".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: if processed_count > 0 {
                format!(
                    "Two-pass vision on {} images: {} → {} tokens ({:.1}% saved). \
                     EXPERIMENTAL: ROI detection needs real-world validation.",
                    processed_count, tokens_before, tokens_after, pct
                )
            } else {
                "No images suitable for two-pass vision selection.".into()
            },
        };
        *self.report.lock().unwrap() = report;

        Ok(SaverOutput {
            messages: input.messages,
            tools: input.tools,
            images: new_images,
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
    fn test_skip_small_images() {
        let saver = VisionSelectSaver::new();
        let img = ImageInput {
            data: vec![],
            mime: "image/png".into(),
            detail: ImageDetail::High,
            original_tokens: 100,
            processed_tokens: 100,
        };
        let result = saver.process_image(&img).unwrap();
        assert_eq!(result.len(), 1); // returned as-is
    }
}
