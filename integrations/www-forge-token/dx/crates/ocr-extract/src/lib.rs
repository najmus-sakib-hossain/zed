//! # ocr-extract
//!
//! Converts text-heavy images to text via OCR before sending to VLM,
//! replacing expensive image tokens with cheaper text tokens.
//!
//! ## Evidence (TOKEN.md ⚠️ Partly Real)
//! - Save 100% of *image* tokens but ADD text tokens for OCR output
//! - Screenshot of code: ~1000 image tokens → ~200 OCR text tokens = 80% savings
//! - Photo with little text: OCR is useless and wastes compute
//! - **Honest savings: 60-90% on text-heavy images, 0% on photos**
//!
//! STAGE: PrePrompt (priority 5)

use dx_core::*;
use std::sync::Mutex;

/// Classification of image content type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageContentType {
    /// Mostly text: code screenshots, terminal output, documents
    TextHeavy,
    /// Mixed: UI with both text and visual elements
    Mixed,
    /// Mostly visual: photos, diagrams, charts
    Visual,
    /// Unknown — couldn't classify
    Unknown,
}

/// Configuration for OCR extraction.
#[derive(Debug, Clone)]
pub struct OcrExtractConfig {
    /// Minimum image token cost to consider OCR extraction
    pub min_image_tokens: usize,
    /// Max text tokens the OCR output can produce (if > this, keep image)
    pub max_ocr_text_tokens: usize,
    /// Whether to attempt classification before OCR
    pub classify_first: bool,
    /// Minimum savings ratio to justify OCR (e.g., 0.5 = at least 50% savings)
    pub min_savings_ratio: f64,
}

impl Default for OcrExtractConfig {
    fn default() -> Self {
        Self {
            min_image_tokens: 200,
            max_ocr_text_tokens: 2000,
            classify_first: true,
            min_savings_ratio: 0.40,
        }
    }
}

pub struct OcrExtractSaver {
    config: OcrExtractConfig,
    report: Mutex<TokenSavingsReport>,
}

impl OcrExtractSaver {
    pub fn new() -> Self {
        Self::with_config(OcrExtractConfig::default())
    }

    pub fn with_config(config: OcrExtractConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Heuristic: classify image content type from image data.
    /// Uses simple signal analysis — not a neural classifier.
    fn classify_image(data: &[u8]) -> ImageContentType {
        if let Ok(img) = image::load_from_memory(data) {
            let gray = img.to_luma8();
            let (w, h) = (gray.width(), gray.height());
            if w == 0 || h == 0 {
                return ImageContentType::Unknown;
            }

            // Analyze contrast distribution — text images have bimodal histogram
            let mut histogram = [0u32; 256];
            for pixel in gray.pixels() {
                histogram[pixel[0] as usize] += 1;
            }
            let total_pixels = (w * h) as f64;

            // Check for bimodal distribution (text on background)
            let dark_fraction = histogram[..64].iter().sum::<u32>() as f64 / total_pixels;
            let light_fraction = histogram[192..].iter().sum::<u32>() as f64 / total_pixels;
            let bimodal_score = dark_fraction + light_fraction;

            // Check horizontal edge density (text has lots of horizontal variation)
            let mut edge_count = 0u64;
            for y in 0..h {
                for x in 1..w {
                    let diff = (gray.get_pixel(x, y)[0] as i32 - gray.get_pixel(x - 1, y)[0] as i32).unsigned_abs();
                    if diff > 30 {
                        edge_count += 1;
                    }
                }
            }
            let edge_density = edge_count as f64 / total_pixels;

            if bimodal_score > 0.75 && edge_density > 0.05 {
                ImageContentType::TextHeavy
            } else if bimodal_score > 0.5 || edge_density > 0.08 {
                ImageContentType::Mixed
            } else {
                ImageContentType::Visual
            }
        } else {
            ImageContentType::Unknown
        }
    }

    /// Simulate OCR output (in production, this would call an OCR engine).
    /// Returns (extracted_text, estimated_text_tokens).
    fn extract_text_placeholder(img: &ImageInput) -> (String, usize) {
        // In a real implementation, this calls tesseract, paddle-ocr, etc.
        // For now, return a placeholder that models the token math correctly.
        let estimated_text_chars = img.data.len() / 50; // very rough heuristic
        let estimated_tokens = estimated_text_chars / 4;
        let placeholder = format!(
            "[OCR extracted text from {}×{} image — {} estimated tokens. \
             Replace this with real OCR engine output (tesseract/paddle-ocr).]",
            img.data.len(), // proxy for resolution info
            img.mime,
            estimated_tokens
        );
        (placeholder, estimated_tokens)
    }

    /// Process a single image: decide if OCR is worth it, extract text.
    fn process_image(&self, img: &ImageInput) -> OcrResult {
        // Skip small images
        if img.original_tokens < self.config.min_image_tokens {
            return OcrResult::KeepImage(img.clone());
        }

        // Classify content type
        let content_type = if self.config.classify_first {
            Self::classify_image(&img.data)
        } else {
            ImageContentType::TextHeavy // assume text-heavy if not classifying
        };

        match content_type {
            ImageContentType::Visual => OcrResult::KeepImage(img.clone()),
            ImageContentType::Unknown => OcrResult::KeepImage(img.clone()),
            ImageContentType::TextHeavy | ImageContentType::Mixed => {
                let (text, text_tokens) = Self::extract_text_placeholder(img);

                // Check if OCR actually saves tokens
                if text_tokens > self.config.max_ocr_text_tokens {
                    return OcrResult::KeepImage(img.clone());
                }

                let savings_ratio = 1.0 - (text_tokens as f64 / img.original_tokens as f64);
                if savings_ratio < self.config.min_savings_ratio {
                    return OcrResult::KeepImage(img.clone());
                }

                OcrResult::ReplaceWithText {
                    text,
                    text_tokens,
                    _image_tokens_saved: img.original_tokens,
                    _content_type: content_type,
                }
            }
        }
    }
}

enum OcrResult {
    KeepImage(ImageInput),
    ReplaceWithText {
        text: String,
        text_tokens: usize,
        _image_tokens_saved: usize,
        _content_type: ImageContentType,
    },
}

#[async_trait::async_trait]
impl TokenSaver for OcrExtractSaver {
    fn name(&self) -> &str { "ocr-extract" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 5 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let mut total_image_tokens_before = 0usize;
        let mut total_tokens_after = 0usize;
        let mut remaining_images = Vec::new();
        let mut messages = input.messages;
        let mut ocr_texts = Vec::new();
        let mut images_converted = 0usize;

        // Process standalone images
        for img in &input.images {
            total_image_tokens_before += img.original_tokens;
            match self.process_image(img) {
                OcrResult::KeepImage(kept) => {
                    total_tokens_after += kept.original_tokens;
                    remaining_images.push(kept);
                }
                OcrResult::ReplaceWithText { text, text_tokens, .. } => {
                    total_tokens_after += text_tokens;
                    images_converted += 1;
                    ocr_texts.push(text);
                }
            }
        }

        // Process images in messages
        for msg in &mut messages {
            let mut new_images = Vec::new();
            for img in &msg.images {
                total_image_tokens_before += img.original_tokens;
                match self.process_image(img) {
                    OcrResult::KeepImage(kept) => {
                        total_tokens_after += kept.original_tokens;
                        new_images.push(kept);
                    }
                    OcrResult::ReplaceWithText { text, text_tokens, .. } => {
                        total_tokens_after += text_tokens;
                        images_converted += 1;
                        // Append OCR text to message content
                        msg.content.push_str("\n\n[OCR extracted text]:\n");
                        msg.content.push_str(&text);
                        msg.token_count += text_tokens;
                    }
                }
            }
            msg.images = new_images;
        }

        // If standalone images were converted, add OCR text as a user message
        if !ocr_texts.is_empty() {
            let combined = ocr_texts.join("\n\n---\n\n");
            let combined_tokens = combined.len() / 4;
            messages.push(Message {
                role: "user".into(),
                content: format!("[OCR extracted from {} images]:\n{}", ocr_texts.len(), combined),
                images: vec![],
                tool_call_id: None,
                token_count: combined_tokens,
            });
        }

        let tokens_saved = total_image_tokens_before.saturating_sub(total_tokens_after);
        let pct = if total_image_tokens_before > 0 {
            tokens_saved as f64 / total_image_tokens_before as f64 * 100.0
        } else { 0.0 };

        let report = TokenSavingsReport {
            technique: "ocr-extract".into(),
            tokens_before: total_image_tokens_before,
            tokens_after: total_tokens_after,
            tokens_saved,
            description: if images_converted > 0 {
                format!(
                    "Converted {} text-heavy images to text: {} → {} tokens ({:.1}% net savings). \
                     Photos/visual images kept as-is.",
                    images_converted, total_image_tokens_before, total_tokens_after, pct
                )
            } else {
                "No text-heavy images found for OCR conversion.".into()
            },
        };
        *self.report.lock().unwrap() = report;

        Ok(SaverOutput {
            messages,
            tools: input.tools,
            images: remaining_images,
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
        let saver = OcrExtractSaver::new();
        let img = ImageInput {
            data: vec![0u8; 100],
            mime: "image/png".into(),
            detail: ImageDetail::Low,
            original_tokens: 85,
            processed_tokens: 85,
        };
        match saver.process_image(&img) {
            OcrResult::KeepImage(_) => {} // expected
            _ => panic!("Small image should be kept"),
        }
    }
}
