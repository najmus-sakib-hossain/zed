//! # dx-fallback — HTML Fallback Mode
//!
//! Generate static HTML fallback for no-JS clients and crawlers.
//!
//! ## Features
//! - Server-side HTML generation
//! - SEO-friendly output
//! - Progressive enhancement
//! - Zero client-side JavaScript required

#![forbid(unsafe_code)]

use maud::{DOCTYPE, Markup, PreEscaped, html};

/// Page metadata
#[derive(Debug, Clone)]
pub struct PageMeta {
    pub title: String,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub author: Option<String>,
    pub canonical: Option<String>,
}

impl PageMeta {
    /// Create new page metadata
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            keywords: None,
            author: None,
            canonical: None,
        }
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set keywords
    pub fn keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = Some(keywords);
        self
    }

    /// Set canonical URL
    pub fn canonical(mut self, url: impl Into<String>) -> Self {
        self.canonical = Some(url.into());
        self
    }
}

/// HTML generator
pub struct HTMLGenerator;

impl HTMLGenerator {
    /// Generate complete HTML document
    pub fn generate(meta: &PageMeta, body_content: Markup) -> String {
        let page = html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    title { (meta.title) }

                    @if let Some(ref desc) = meta.description {
                        meta name="description" content=(desc);
                    }

                    @if let Some(ref keywords) = meta.keywords {
                        meta name="keywords" content=(keywords.join(", "));
                    }

                    @if let Some(ref author) = meta.author {
                        meta name="author" content=(author);
                    }

                    @if let Some(ref canonical) = meta.canonical {
                        link rel="canonical" href=(canonical);
                    }

                    // Basic styles for fallback
                    style {
                        (PreEscaped(r#"
                            body {
                                font-family: system-ui, -apple-system, sans-serif;
                                line-height: 1.6;
                                max-width: 800px;
                                margin: 0 auto;
                                padding: 20px;
                                color: #333;
                            }
                            h1, h2, h3 { color: #222; }
                            a { color: #0066cc; }
                            code { 
                                background: #f4f4f4; 
                                padding: 2px 6px; 
                                border-radius: 3px;
                            }
                            .noscript-warning {
                                background: #fff3cd;
                                border: 1px solid #ffc107;
                                padding: 15px;
                                margin-bottom: 20px;
                                border-radius: 4px;
                            }
                        "#))
                    }
                }
                body {
                    noscript {
                        div class="noscript-warning" {
                            p {
                                strong { "JavaScript is disabled." }
                                " This site works best with JavaScript enabled. "
                                "You are viewing a fallback version with limited functionality."
                            }
                        }
                    }

                    (body_content)
                }
            }
        };

        page.into_string()
    }

    /// Generate simple page
    pub fn simple_page(title: &str, content: &str) -> String {
        let meta = PageMeta::new(title);
        let body = html! {
            main {
                h1 { (title) }
                div { (PreEscaped(content)) }
            }
        };
        Self::generate(&meta, body)
    }

    /// Generate error page
    pub fn error_page(code: u16, message: &str) -> String {
        let meta = PageMeta::new(format!("Error {}", code));
        let body = html! {
            main {
                h1 { "Error " (code) }
                p { (message) }
                a href="/" { "← Back to home" }
            }
        };
        Self::generate(&meta, body)
    }

    /// Generate loading page
    pub fn loading_page(title: &str) -> String {
        let meta = PageMeta::new(title);
        let body = html! {
            main {
                h1 { (title) }
                p { "Loading..." }
                noscript {
                    p { "Please enable JavaScript to use this application." }
                }
            }
        };
        Self::generate(&meta, body)
    }
}

/// Component renderer (for common UI elements)
pub struct ComponentRenderer;

impl ComponentRenderer {
    /// Render navigation
    pub fn nav(links: &[(&str, &str)]) -> Markup {
        html! {
            nav {
                ul style="list-style: none; padding: 0; display: flex; gap: 20px;" {
                    @for (label, href) in links {
                        li {
                            a href=(href) { (label) }
                        }
                    }
                }
            }
        }
    }

    /// Render footer
    pub fn footer(text: &str) -> Markup {
        html! {
            footer style="margin-top: 40px; padding-top: 20px; border-top: 1px solid #ddd; color: #666;" {
                p { (text) }
            }
        }
    }

    /// Render card
    pub fn card(title: &str, content: &str) -> Markup {
        html! {
            div style="border: 1px solid #ddd; border-radius: 8px; padding: 20px; margin-bottom: 20px;" {
                h3 { (title) }
                p { (content) }
            }
        }
    }

    /// Render button
    pub fn button(text: &str, href: &str) -> Markup {
        html! {
            a href=(href)
              style="display: inline-block; padding: 10px 20px; background: #0066cc; color: white; text-decoration: none; border-radius: 4px;" {
                (text)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_meta() {
        let meta = PageMeta::new("Test Page")
            .description("A test page")
            .keywords(vec!["test".to_string(), "demo".to_string()])
            .canonical("https://example.com/test");

        assert_eq!(meta.title, "Test Page");
        assert_eq!(meta.description, Some("A test page".to_string()));
        assert_eq!(meta.keywords.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_simple_page() {
        let html = HTMLGenerator::simple_page("Hello", "<p>World</p>");

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>Hello</title>"));
        assert!(html.contains("<p>World</p>"));
    }

    #[test]
    fn test_error_page() {
        let html = HTMLGenerator::error_page(404, "Page not found");

        assert!(html.contains("Error 404"));
        assert!(html.contains("Page not found"));
        assert!(html.contains("Back to home"));
    }

    #[test]
    fn test_loading_page() {
        let html = HTMLGenerator::loading_page("Dashboard");

        assert!(html.contains("<title>Dashboard</title>"));
        assert!(html.contains("Loading..."));
    }

    #[test]
    fn test_nav_component() {
        let links = vec![("Home", "/"), ("About", "/about")];
        let nav = ComponentRenderer::nav(&links);
        let html = nav.into_string();

        assert!(html.contains("Home"));
        assert!(html.contains("/about"));
    }

    #[test]
    fn test_card_component() {
        let card = ComponentRenderer::card("Title", "Content");
        let html = card.into_string();

        assert!(html.contains("<h3>Title</h3>"));
        assert!(html.contains("<p>Content</p>"));
    }
}
