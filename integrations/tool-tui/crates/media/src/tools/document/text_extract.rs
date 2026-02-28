//! Text extraction from documents.
//!
//! Extract text content from PDF, DOC, DOCX, and other formats.

use crate::error::{DxError, Result};
use crate::tools::ToolOutput;
use std::path::Path;
use std::process::Command;

/// Text extraction options.
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    /// Preserve layout formatting.
    pub preserve_layout: bool,
    /// Extract specific pages only (1-indexed).
    pub pages: Option<Vec<u32>>,
    /// Include page separators.
    pub page_separators: bool,
    /// Encoding for output.
    pub encoding: String,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            preserve_layout: false,
            pages: None,
            page_separators: false,
            encoding: "utf-8".to_string(),
        }
    }
}

/// Extract text from a document.
///
/// # Arguments
/// * `input` - Path to document
///
/// # Example
/// ```no_run
/// use dx_media::tools::document::text_extract;
///
/// let text = text_extract::extract("document.pdf").unwrap();
/// println!("{}", text.message);
/// ```
pub fn extract<P: AsRef<Path>>(input: P) -> Result<ToolOutput> {
    extract_with_options(input, ExtractOptions::default())
}

/// Extract text with options.
pub fn extract_with_options<P: AsRef<Path>>(
    input: P,
    options: ExtractOptions,
) -> Result<ToolOutput> {
    let input_path = input.as_ref();

    if !input_path.exists() {
        return Err(DxError::FileIo {
            path: input_path.to_path_buf(),
            message: "Input file not found".to_string(),
            source: None,
        });
    }

    let extension = input_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    match extension.as_str() {
        "pdf" => extract_from_pdf(input_path, &options),
        "doc" | "docx" | "odt" | "rtf" => extract_from_office(input_path),
        "txt" | "text" | "md" | "markdown" => extract_from_text(input_path),
        "html" | "htm" => extract_from_html(input_path),
        _ => Err(DxError::Config {
            message: format!("Unsupported document format: {}", extension),
            source: None,
        }),
    }
}

/// Extract text from PDF.
fn extract_from_pdf(input: &Path, options: &ExtractOptions) -> Result<ToolOutput> {
    // Try pdftotext (Poppler)
    if let Ok(result) = extract_pdf_with_pdftotext(input, options) {
        return Ok(result);
    }

    // Try Apache Tika
    if let Ok(result) = extract_with_tika(input) {
        return Ok(result);
    }

    // Try xpdf
    if let Ok(result) = extract_pdf_with_xpdf(input) {
        return Ok(result);
    }

    Err(DxError::Config {
        message: "PDF text extraction failed. Install pdftotext (Poppler) or Apache Tika."
            .to_string(),
        source: None,
    })
}

/// Extract using pdftotext.
fn extract_pdf_with_pdftotext(input: &Path, options: &ExtractOptions) -> Result<ToolOutput> {
    let mut cmd = Command::new("pdftotext");

    if options.preserve_layout {
        cmd.arg("-layout");
    }

    if let Some(ref pages) = options.pages {
        if let (Some(first), Some(last)) = (pages.first(), pages.last()) {
            cmd.arg("-f").arg(first.to_string()).arg("-l").arg(last.to_string());
        }
    }

    // Output to stdout
    cmd.arg(input).arg("-");

    let result = cmd.output().map_err(|e| DxError::Config {
        message: format!("Failed to run pdftotext: {}", e),
        source: None,
    })?;

    if !result.status.success() {
        return Err(DxError::Config {
            message: "pdftotext failed".to_string(),
            source: None,
        });
    }

    let text = String::from_utf8_lossy(&result.stdout).to_string();
    let line_count = text.lines().count();
    let char_count = text.chars().count();

    Ok(ToolOutput::success(text)
        .with_metadata("line_count", line_count.to_string())
        .with_metadata("char_count", char_count.to_string())
        .with_metadata("method", "pdftotext".to_string()))
}

/// Extract using xpdf pdftotext.
fn extract_pdf_with_xpdf(input: &Path) -> Result<ToolOutput> {
    let mut cmd = Command::new("pdftotext");
    cmd.arg("-enc").arg("UTF-8").arg(input).arg("-");

    let result = cmd.output().map_err(|e| DxError::Config {
        message: format!("Failed to run xpdf: {}", e),
        source: None,
    })?;

    if !result.status.success() {
        return Err(DxError::Config {
            message: "xpdf extraction failed".to_string(),
            source: None,
        });
    }

    let text = String::from_utf8_lossy(&result.stdout).to_string();
    Ok(ToolOutput::success(text).with_metadata("method", "xpdf".to_string()))
}

/// Extract using Apache Tika.
fn extract_with_tika(input: &Path) -> Result<ToolOutput> {
    let mut cmd = Command::new("tika");
    cmd.arg("--text").arg(input);

    let result = cmd.output().map_err(|e| DxError::Config {
        message: format!("Failed to run tika: {}", e),
        source: None,
    })?;

    if !result.status.success() {
        return Err(DxError::Config {
            message: "Tika extraction failed".to_string(),
            source: None,
        });
    }

    let text = String::from_utf8_lossy(&result.stdout).to_string();
    Ok(ToolOutput::success(text).with_metadata("method", "tika".to_string()))
}

/// Extract text from Office documents.
fn extract_from_office(input: &Path) -> Result<ToolOutput> {
    // Try antiword for .doc
    if input.extension().is_some_and(|e| e.eq_ignore_ascii_case("doc")) {
        if let Ok(result) = extract_doc_with_antiword(input) {
            return Ok(result);
        }
    }

    // Try docx2txt for .docx
    if let Ok(result) = extract_with_docx2txt(input) {
        return Ok(result);
    }

    // Try LibreOffice
    if let Ok(result) = extract_with_libreoffice(input) {
        return Ok(result);
    }

    // Try Apache Tika
    if let Ok(result) = extract_with_tika(input) {
        return Ok(result);
    }

    Err(DxError::Config {
        message: "Office document extraction failed".to_string(),
        source: None,
    })
}

/// Extract using antiword.
fn extract_doc_with_antiword(input: &Path) -> Result<ToolOutput> {
    let mut cmd = Command::new("antiword");
    cmd.arg(input);

    let result = cmd.output().map_err(|e| DxError::Config {
        message: format!("Failed to run antiword: {}", e),
        source: None,
    })?;

    if !result.status.success() {
        return Err(DxError::Config {
            message: "antiword failed".to_string(),
            source: None,
        });
    }

    let text = String::from_utf8_lossy(&result.stdout).to_string();
    Ok(ToolOutput::success(text).with_metadata("method", "antiword".to_string()))
}

/// Extract using docx2txt.
fn extract_with_docx2txt(input: &Path) -> Result<ToolOutput> {
    let mut cmd = Command::new("docx2txt");
    cmd.arg(input).arg("-");

    let result = cmd.output().map_err(|e| DxError::Config {
        message: format!("Failed to run docx2txt: {}", e),
        source: None,
    })?;

    if !result.status.success() {
        return Err(DxError::Config {
            message: "docx2txt failed".to_string(),
            source: None,
        });
    }

    let text = String::from_utf8_lossy(&result.stdout).to_string();
    Ok(ToolOutput::success(text).with_metadata("method", "docx2txt".to_string()))
}

/// Extract using LibreOffice.
fn extract_with_libreoffice(input: &Path) -> Result<ToolOutput> {
    let temp_dir = std::env::temp_dir();

    let lo_names = if cfg!(windows) {
        vec!["soffice", "libreoffice"]
    } else {
        vec!["libreoffice", "soffice"]
    };

    for lo in lo_names {
        let mut cmd = Command::new(lo);
        cmd.arg("--headless")
            .arg("--convert-to")
            .arg("txt:Text")
            .arg("--outdir")
            .arg(&temp_dir)
            .arg(input);

        if let Ok(result) = cmd.output() {
            if result.status.success() {
                let output_name =
                    format!("{}.txt", input.file_stem().unwrap_or_default().to_string_lossy());
                let output_path = temp_dir.join(&output_name);

                if let Ok(text) = std::fs::read_to_string(&output_path) {
                    let _ = std::fs::remove_file(&output_path);
                    return Ok(ToolOutput::success(text)
                        .with_metadata("method", "libreoffice".to_string()));
                }
            }
        }
    }

    Err(DxError::Config {
        message: "LibreOffice extraction failed".to_string(),
        source: None,
    })
}

/// Extract from plain text files.
fn extract_from_text(input: &Path) -> Result<ToolOutput> {
    let text = std::fs::read_to_string(input).map_err(|e| DxError::FileIo {
        path: input.to_path_buf(),
        message: format!("Failed to read file: {}", e),
        source: None,
    })?;

    let line_count = text.lines().count();
    let char_count = text.chars().count();

    Ok(ToolOutput::success(text)
        .with_metadata("line_count", line_count.to_string())
        .with_metadata("char_count", char_count.to_string())
        .with_metadata("method", "direct_read".to_string()))
}

/// Extract from HTML files.
fn extract_from_html(input: &Path) -> Result<ToolOutput> {
    let html = std::fs::read_to_string(input).map_err(|e| DxError::FileIo {
        path: input.to_path_buf(),
        message: format!("Failed to read file: {}", e),
        source: None,
    })?;

    // Basic HTML tag stripping
    let text = strip_html_tags(&html);

    Ok(ToolOutput::success(text).with_metadata("method", "html_strip".to_string()))
}

/// Basic HTML tag stripper.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    let lower = html.to_lowercase();
    let chars: Vec<char> = html.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        if !in_tag {
            if chars[i] == '<' {
                // Check for script/style tags
                let remaining: String = lower_chars[i..].iter().take(10).collect();
                if remaining.starts_with("<script") {
                    in_script = true;
                } else if remaining.starts_with("<style") {
                    in_style = true;
                } else if remaining.starts_with("</script") {
                    in_script = false;
                } else if remaining.starts_with("</style") {
                    in_style = false;
                }
                in_tag = true;
            } else if !in_script && !in_style {
                result.push(chars[i]);
            }
        } else if chars[i] == '>' {
            in_tag = false;
        }
        i += 1;
    }

    // Decode common HTML entities
    let result = result
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    // Clean up whitespace
    let lines: Vec<&str> = result.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    lines.join("\n")
}

/// Extract and save to file.
pub fn extract_to_file<P: AsRef<Path>>(input: P, output: P) -> Result<ToolOutput> {
    let result = extract(&input)?;

    std::fs::write(output.as_ref(), &result.message).map_err(|e| DxError::FileIo {
        path: output.as_ref().to_path_buf(),
        message: format!("Failed to write output: {}", e),
        source: None,
    })?;

    Ok(ToolOutput::success_with_path("Text extracted and saved", output.as_ref()))
}

/// Batch extract from multiple files.
pub fn batch_extract<P: AsRef<Path>>(inputs: &[P], output_dir: P) -> Result<ToolOutput> {
    let output_dir = output_dir.as_ref();
    std::fs::create_dir_all(output_dir).map_err(|e| DxError::FileIo {
        path: output_dir.to_path_buf(),
        message: format!("Failed to create directory: {}", e),
        source: None,
    })?;

    let mut extracted = Vec::new();

    for input in inputs {
        let input_path = input.as_ref();
        let file_stem = input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
        let output_path = output_dir.join(format!("{}.txt", file_stem));

        if extract_to_file(input_path, &output_path).is_ok() {
            extracted.push(output_path);
        }
    }

    Ok(ToolOutput::success(format!("Extracted text from {} files", extracted.len()))
        .with_paths(extracted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html() {
        let html = "<p>Hello <b>World</b>!</p>";
        let text = strip_html_tags(html);
        assert_eq!(text, "Hello World!");
    }

    #[test]
    fn test_strip_html_entities() {
        let html = "<p>&amp; &lt; &gt;</p>";
        let text = strip_html_tags(html);
        assert_eq!(text, "& < >");
    }
}
