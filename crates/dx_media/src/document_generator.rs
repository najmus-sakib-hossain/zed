//! Document generation engine — PDF, Excel, CSV, HTML, SVG, charts.
//!
//! Entirely local. Zero cloud dependency. The LLM generates structured content,
//! Rust renders it into pixel-perfect documents.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported document output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocumentFormat {
    Pdf,
    Html,
    Markdown,
    Xlsx,
    Csv,
    Svg,
}

impl DocumentFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            DocumentFormat::Pdf => "pdf",
            DocumentFormat::Html => "html",
            DocumentFormat::Markdown => "md",
            DocumentFormat::Xlsx => "xlsx",
            DocumentFormat::Csv => "csv",
            DocumentFormat::Svg => "svg",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            DocumentFormat::Pdf => "application/pdf",
            DocumentFormat::Html => "text/html",
            DocumentFormat::Markdown => "text/markdown",
            DocumentFormat::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            DocumentFormat::Csv => "text/csv",
            DocumentFormat::Svg => "image/svg+xml",
        }
    }
}

/// A structured section of a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentSection {
    /// A heading with level (1-6) and text.
    Heading { level: u8, text: String },
    /// A paragraph of text (supports Markdown).
    Paragraph { text: String },
    /// A code block with optional language.
    CodeBlock { language: Option<String>, code: String },
    /// A table with headers and rows.
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    /// An image (base64 or file path).
    Image { data: Vec<u8>, caption: Option<String>, width: Option<u32> },
    /// A chart specification (rendered by plotters or similar).
    Chart { chart_type: ChartType, data: ChartData, title: Option<String> },
    /// A page break.
    PageBreak,
    /// A horizontal rule.
    HorizontalRule,
    /// A list (ordered or unordered).
    List { ordered: bool, items: Vec<String> },
}

/// Types of charts supported.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChartType {
    Bar,
    Line,
    Pie,
    Scatter,
    Area,
}

/// Data for chart rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub labels: Vec<String>,
    pub series: Vec<ChartSeries>,
}

/// A single data series in a chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSeries {
    pub name: String,
    pub values: Vec<f64>,
}

/// Request to generate a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRequest {
    pub title: String,
    pub format: DocumentFormat,
    pub sections: Vec<DocumentSection>,
    pub author: Option<String>,
    pub page_size: Option<PageSize>,
}

/// Standard page sizes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PageSize {
    A4,
    Letter,
    Legal,
    Custom { width_mm: f64, height_mm: f64 },
}

/// Generated document output.
#[derive(Debug, Clone)]
pub struct DocumentOutput {
    pub format: DocumentFormat,
    pub data: Vec<u8>,
    pub saved_path: Option<PathBuf>,
    pub page_count: Option<u32>,
}

/// Generate a document from structured sections.
///
/// This is the local document generation engine. The LLM generates
/// `DocumentSection` structs, and this function renders them into the
/// requested format using pure Rust (genpdf, printpdf, typst, etc.).
pub fn generate_document(request: &DocumentRequest) -> Result<DocumentOutput> {
    match request.format {
        DocumentFormat::Markdown => generate_markdown(request),
        DocumentFormat::Html => generate_html(request),
        DocumentFormat::Csv => generate_csv(request),
        _ => {
            // PDF, XLSX, SVG generation requires additional crate dependencies.
            // Placeholder that generates Markdown fallback.
            log::warn!(
                "Document format {:?} not yet fully implemented, falling back to Markdown",
                request.format
            );
            generate_markdown(request)
        }
    }
}

fn generate_markdown(request: &DocumentRequest) -> Result<DocumentOutput> {
    let mut output = String::new();
    output.push_str(&format!("# {}\n\n", request.title));

    if let Some(author) = &request.author {
        output.push_str(&format!("*By {}*\n\n", author));
    }

    for section in &request.sections {
        match section {
            DocumentSection::Heading { level, text } => {
                let prefix = "#".repeat(*level as usize);
                output.push_str(&format!("{} {}\n\n", prefix, text));
            }
            DocumentSection::Paragraph { text } => {
                output.push_str(&format!("{}\n\n", text));
            }
            DocumentSection::CodeBlock { language, code } => {
                let lang = language.as_deref().unwrap_or("");
                output.push_str(&format!("```{}\n{}\n```\n\n", lang, code));
            }
            DocumentSection::Table { headers, rows } => {
                output.push_str("| ");
                output.push_str(&headers.join(" | "));
                output.push_str(" |\n");
                output.push_str("| ");
                output.push_str(&headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
                output.push_str(" |\n");
                for row in rows {
                    output.push_str("| ");
                    output.push_str(&row.join(" | "));
                    output.push_str(" |\n");
                }
                output.push('\n');
            }
            DocumentSection::Image { caption, .. } => {
                if let Some(cap) = caption {
                    output.push_str(&format!("![{}](image)\n\n", cap));
                } else {
                    output.push_str("![](image)\n\n");
                }
            }
            DocumentSection::Chart { title, .. } => {
                output.push_str(&format!("[Chart: {}]\n\n", title.as_deref().unwrap_or("Untitled")));
            }
            DocumentSection::PageBreak => {
                output.push_str("---\n\n");
            }
            DocumentSection::HorizontalRule => {
                output.push_str("---\n\n");
            }
            DocumentSection::List { ordered, items } => {
                for (i, item) in items.iter().enumerate() {
                    if *ordered {
                        output.push_str(&format!("{}. {}\n", i + 1, item));
                    } else {
                        output.push_str(&format!("- {}\n", item));
                    }
                }
                output.push('\n');
            }
        }
    }

    Ok(DocumentOutput {
        format: DocumentFormat::Markdown,
        data: output.into_bytes(),
        saved_path: None,
        page_count: None,
    })
}

fn generate_html(request: &DocumentRequest) -> Result<DocumentOutput> {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str(&format!("  <title>{}</title>\n", request.title));
    html.push_str("  <meta charset=\"utf-8\">\n");
    html.push_str("  <style>body { font-family: system-ui, sans-serif; max-width: 800px; margin: 0 auto; padding: 2rem; }</style>\n");
    html.push_str("</head>\n<body>\n");
    html.push_str(&format!("  <h1>{}</h1>\n", request.title));

    for section in &request.sections {
        match section {
            DocumentSection::Heading { level, text } => {
                html.push_str(&format!("  <h{}>{}</h{}>\n", level, text, level));
            }
            DocumentSection::Paragraph { text } => {
                html.push_str(&format!("  <p>{}</p>\n", text));
            }
            DocumentSection::CodeBlock { language, code } => {
                let lang_attr = language.as_ref().map_or(String::new(), |l| format!(" class=\"language-{}\"", l));
                html.push_str(&format!("  <pre><code{}>{}</code></pre>\n", lang_attr, code));
            }
            DocumentSection::Table { headers, rows } => {
                html.push_str("  <table border=\"1\">\n    <tr>");
                for h in headers {
                    html.push_str(&format!("<th>{}</th>", h));
                }
                html.push_str("</tr>\n");
                for row in rows {
                    html.push_str("    <tr>");
                    for cell in row {
                        html.push_str(&format!("<td>{}</td>", cell));
                    }
                    html.push_str("</tr>\n");
                }
                html.push_str("  </table>\n");
            }
            DocumentSection::List { ordered, items } => {
                let tag = if *ordered { "ol" } else { "ul" };
                html.push_str(&format!("  <{}>\n", tag));
                for item in items {
                    html.push_str(&format!("    <li>{}</li>\n", item));
                }
                html.push_str(&format!("  </{}>\n", tag));
            }
            DocumentSection::HorizontalRule | DocumentSection::PageBreak => {
                html.push_str("  <hr>\n");
            }
            _ => {}
        }
    }

    html.push_str("</body>\n</html>\n");

    Ok(DocumentOutput {
        format: DocumentFormat::Html,
        data: html.into_bytes(),
        saved_path: None,
        page_count: None,
    })
}

fn generate_csv(request: &DocumentRequest) -> Result<DocumentOutput> {
    let mut csv = String::new();

    for section in &request.sections {
        if let DocumentSection::Table { headers, rows } = section {
            csv.push_str(&headers.join(","));
            csv.push('\n');
            for row in rows {
                csv.push_str(&row.join(","));
                csv.push('\n');
            }
        }
    }

    Ok(DocumentOutput {
        format: DocumentFormat::Csv,
        data: csv.into_bytes(),
        saved_path: None,
        page_count: None,
    })
}
