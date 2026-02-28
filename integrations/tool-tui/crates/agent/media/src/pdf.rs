//! PDF processing module

use anyhow::Result;
use lopdf::Document;
use std::path::Path;

/// PDF processor
pub struct PdfProcessor;

impl PdfProcessor {
    /// Load a PDF document
    pub fn load(path: &Path) -> Result<Document> {
        let doc = Document::load(path)?;
        Ok(doc)
    }

    /// Get page count
    pub fn page_count(doc: &Document) -> u32 {
        doc.get_pages().len() as u32
    }

    /// Extract text from all pages
    pub fn extract_text(path: &Path) -> Result<String> {
        let doc = Document::load(path)?;
        let mut text = String::new();

        let pages = doc.get_pages();
        for (page_num, _) in pages {
            if let Ok(page_text) = doc.extract_text(&[page_num]) {
                if !text.is_empty() {
                    text.push_str("\n\n--- Page ");
                    text.push_str(&page_num.to_string());
                    text.push_str(" ---\n\n");
                }
                text.push_str(&page_text);
            }
        }

        Ok(text)
    }

    /// Extract text from a specific page
    pub fn extract_page_text(path: &Path, page_num: u32) -> Result<String> {
        let doc = Document::load(path)?;
        let text = doc.extract_text(&[page_num])?;
        Ok(text)
    }

    /// Get PDF metadata
    pub fn metadata(path: &Path) -> Result<super::MediaMetadata> {
        let doc = Document::load(path)?;
        let file_size = std::fs::metadata(path)?.len();
        let page_count = doc.get_pages().len() as u32;

        let mut title = None;

        // Try to extract title from document info dict
        if let Ok(info_dict) = doc.trailer.get(b"Info") {
            if let Ok(info_ref) = info_dict.as_reference() {
                if let Ok(info_obj) = doc.get_object(info_ref) {
                    if let Ok(dict) = info_obj.as_dict() {
                        if let Ok(t) = dict.get(b"Title") {
                            if let Ok(s) = t.as_str() {
                                title = Some(String::from_utf8_lossy(s).to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(super::MediaMetadata {
            media_type: Some("pdf".into()),
            file_size,
            mime_type: Some("application/pdf".into()),
            page_count: Some(page_count),
            title,
            ..Default::default()
        })
    }

    /// Render a PDF page to an image using an external tool (mutool or pdftoppm).
    ///
    /// This uses command-line tools because pure-Rust PDF rendering is limited.
    /// Prefers `mutool` (from MuPDF), falls back to `pdftoppm` (from poppler-utils).
    pub async fn render_page_to_image(
        pdf_path: &Path,
        page_num: u32,
        output_path: &Path,
        dpi: Option<u32>,
    ) -> Result<()> {
        let dpi = dpi.unwrap_or(150);

        // Try mutool first (MuPDF - best quality)
        if Self::is_tool_available("mutool") {
            let status = tokio::process::Command::new("mutool")
                .args([
                    "draw",
                    "-o",
                    output_path.to_str().unwrap_or(""),
                    "-r",
                    &dpi.to_string(),
                    pdf_path.to_str().unwrap_or(""),
                    &page_num.to_string(),
                ])
                .output()
                .await?;

            if status.status.success() {
                return Ok(());
            }
        }

        // Fallback to pdftoppm (poppler-utils)
        if Self::is_tool_available("pdftoppm") {
            let output_prefix = output_path
                .to_str()
                .unwrap_or("")
                .trim_end_matches(".png")
                .trim_end_matches(".jpg");

            let status = tokio::process::Command::new("pdftoppm")
                .args([
                    "-png",
                    "-r",
                    &dpi.to_string(),
                    "-f",
                    &page_num.to_string(),
                    "-l",
                    &page_num.to_string(),
                    "-singlefile",
                    pdf_path.to_str().unwrap_or(""),
                    output_prefix,
                ])
                .output()
                .await?;

            if status.status.success() {
                // pdftoppm appends .png — rename if needed
                let generated = format!("{}.png", output_prefix);
                let gen_path = std::path::Path::new(&generated);
                if gen_path.exists() && gen_path != output_path {
                    tokio::fs::rename(gen_path, output_path).await?;
                }
                return Ok(());
            }
        }

        anyhow::bail!(
            "No PDF renderer available. Install `mutool` (MuPDF) or `pdftoppm` (poppler-utils)."
        )
    }

    /// Check if a CLI tool is available on PATH
    fn is_tool_available(name: &str) -> bool {
        std::process::Command::new(name)
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_module_exists() {
        assert!(true);
    }

    #[test]
    fn test_tool_available_check() {
        // Should not panic regardless of tool presence
        let _ = PdfProcessor::is_tool_available("mutool");
        let _ = PdfProcessor::is_tool_available("pdftoppm");
    }
}
