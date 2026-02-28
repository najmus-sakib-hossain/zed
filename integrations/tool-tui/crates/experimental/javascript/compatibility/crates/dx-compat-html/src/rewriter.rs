//! HTML Rewriter implementation using lol_html.
//!
//! Provides streaming HTML transformation with element selection and manipulation.

use crate::error::HtmlResult;
use lol_html::html_content::ContentType as LolContentType;
use lol_html::{element, rewrite_str, RewriteStrSettings};

/// Content type for HTML insertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// HTML content (will be parsed)
    Html,
    /// Plain text content (will be escaped)
    Text,
}

impl From<ContentType> for LolContentType {
    fn from(ct: ContentType) -> Self {
        match ct {
            ContentType::Html => LolContentType::Html,
            ContentType::Text => LolContentType::Text,
        }
    }
}

/// Element handler function type.
pub type ElementHandler = Box<dyn Fn(&mut ElementProxy) + 'static>;

/// Document handler function type.
pub type DocumentHandler = Box<dyn Fn(&mut DocumentProxy) + 'static>;

/// Element handler registration.
struct ElementHandlerEntry {
    selector: String,
    handler: ElementHandler,
}

/// HTML Rewriter for streaming HTML transformation.
///
/// Similar to Cloudflare's HTMLRewriter API.
pub struct HTMLRewriter {
    element_handlers: Vec<ElementHandlerEntry>,
    document_handlers: Vec<DocumentHandler>,
}

impl HTMLRewriter {
    /// Create a new HTML rewriter.
    pub fn new() -> Self {
        Self {
            element_handlers: Vec::new(),
            document_handlers: Vec::new(),
        }
    }

    /// Register an element handler for a CSS selector.
    ///
    /// # Example
    /// ```ignore
    /// let mut rewriter = HTMLRewriter::new();
    /// rewriter.on("a[href]", |el| {
    ///     if let Some(href) = el.get_attribute("href") {
    ///         el.set_attribute("href", &format!("https://proxy.example.com/{}", href));
    ///     }
    /// });
    /// ```
    pub fn on<F>(&mut self, selector: &str, handler: F) -> &mut Self
    where
        F: Fn(&mut ElementProxy) + 'static,
    {
        self.element_handlers.push(ElementHandlerEntry {
            selector: selector.to_string(),
            handler: Box::new(handler),
        });
        self
    }

    /// Register a document-level handler.
    pub fn on_document<F>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(&mut DocumentProxy) + 'static,
    {
        self.document_handlers.push(Box::new(handler));
        self
    }

    /// Transform HTML content.
    ///
    /// Applies all registered handlers to the HTML and returns the transformed result.
    pub fn transform(&self, html: &str) -> HtmlResult<String> {
        // Build element content handlers
        let mut element_content_handlers = Vec::new();

        for entry in &self.element_handlers {
            let handler = &entry.handler;

            // Create a closure that wraps the element in our proxy
            let element_handler = element!(entry.selector.as_str(), |el| {
                let mut proxy = ElementProxy::new(el);
                handler(&mut proxy);
                proxy.apply(el);
                Ok(())
            });

            element_content_handlers.push(element_handler);
        }

        let result = rewrite_str(
            html,
            RewriteStrSettings {
                element_content_handlers,
                ..RewriteStrSettings::default()
            },
        )?;

        Ok(result)
    }
}

impl Default for HTMLRewriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Proxy for element manipulation.
///
/// Collects modifications to be applied to the element.
pub struct ElementProxy {
    tag_name: String,
    attributes: Vec<(String, String)>,
    removed_attributes: Vec<String>,
    before_content: Option<(String, ContentType)>,
    after_content: Option<(String, ContentType)>,
    prepend_content: Option<(String, ContentType)>,
    append_content: Option<(String, ContentType)>,
    replace_content: Option<(String, ContentType)>,
    inner_content: Option<(String, ContentType)>,
    removed: bool,
}

impl ElementProxy {
    fn new(el: &lol_html::html_content::Element) -> Self {
        let attributes: Vec<(String, String)> =
            el.attributes().iter().map(|a| (a.name(), a.value())).collect();

        Self {
            tag_name: el.tag_name(),
            attributes,
            removed_attributes: Vec::new(),
            before_content: None,
            after_content: None,
            prepend_content: None,
            append_content: None,
            replace_content: None,
            inner_content: None,
            removed: false,
        }
    }

    fn apply(self, el: &mut lol_html::html_content::Element) {
        // Apply attribute removals
        for name in &self.removed_attributes {
            el.remove_attribute(name);
        }

        // Apply attribute changes
        for (name, value) in &self.attributes {
            let _ = el.set_attribute(name, value);
        }

        // Apply content modifications
        if let Some((content, ct)) = self.before_content {
            el.before(&content, ct.into());
        }

        if let Some((content, ct)) = self.after_content {
            el.after(&content, ct.into());
        }

        if let Some((content, ct)) = self.prepend_content {
            el.prepend(&content, ct.into());
        }

        if let Some((content, ct)) = self.append_content {
            el.append(&content, ct.into());
        }

        if let Some((content, ct)) = self.replace_content {
            el.replace(&content, ct.into());
        }

        if let Some((content, ct)) = self.inner_content {
            el.set_inner_content(&content, ct.into());
        }

        if self.removed {
            el.remove();
        }
    }

    /// Get the tag name.
    pub fn tag_name(&self) -> &str {
        &self.tag_name
    }

    /// Get an attribute value.
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        self.attributes.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone())
    }

    /// Set an attribute value.
    pub fn set_attribute(&mut self, name: &str, value: &str) {
        if let Some(attr) = self.attributes.iter_mut().find(|(n, _)| n == name) {
            attr.1 = value.to_string();
        } else {
            self.attributes.push((name.to_string(), value.to_string()));
        }
    }

    /// Remove an attribute.
    pub fn remove_attribute(&mut self, name: &str) {
        self.attributes.retain(|(n, _)| n != name);
        self.removed_attributes.push(name.to_string());
    }

    /// Check if an attribute exists.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.iter().any(|(n, _)| n == name)
    }

    /// Insert content before the element.
    pub fn before(&mut self, content: &str, content_type: ContentType) {
        self.before_content = Some((content.to_string(), content_type));
    }

    /// Insert content after the element.
    pub fn after(&mut self, content: &str, content_type: ContentType) {
        self.after_content = Some((content.to_string(), content_type));
    }

    /// Prepend content inside the element.
    pub fn prepend(&mut self, content: &str, content_type: ContentType) {
        self.prepend_content = Some((content.to_string(), content_type));
    }

    /// Append content inside the element.
    pub fn append(&mut self, content: &str, content_type: ContentType) {
        self.append_content = Some((content.to_string(), content_type));
    }

    /// Replace the element with content.
    pub fn replace(&mut self, content: &str, content_type: ContentType) {
        self.replace_content = Some((content.to_string(), content_type));
    }

    /// Set the inner content.
    pub fn set_inner_content(&mut self, content: &str, content_type: ContentType) {
        self.inner_content = Some((content.to_string(), content_type));
    }

    /// Remove the element.
    pub fn remove(&mut self) {
        self.removed = true;
    }
}

/// Proxy for document-level manipulation.
pub struct DocumentProxy {
    doctype: Option<String>,
    end_content: Option<String>,
}

impl DocumentProxy {
    /// Create a new document proxy.
    pub fn new() -> Self {
        Self {
            doctype: None,
            end_content: None,
        }
    }

    /// Set the doctype.
    pub fn set_doctype(&mut self, doctype: &str) {
        self.doctype = Some(doctype.to_string());
    }

    /// Append content at the end of the document.
    pub fn append(&mut self, content: &str) {
        self.end_content = Some(content.to_string());
    }
}

impl Default for DocumentProxy {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple HTML transformation helper.
///
/// Transform HTML with a single element handler.
pub fn transform_html<F>(html: &str, selector: &str, handler: F) -> HtmlResult<String>
where
    F: Fn(&mut ElementProxy) + 'static,
{
    let mut rewriter = HTMLRewriter::new();
    rewriter.on(selector, handler);
    rewriter.transform(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_transform() {
        let html = r#"<div class="test">Hello</div>"#;

        let result = transform_html(html, "div", |el| {
            el.set_attribute("class", "modified");
        })
        .unwrap();

        assert!(result.contains(r#"class="modified""#));
    }

    #[test]
    fn test_append_content() {
        let html = r#"<div>Hello</div>"#;

        let result = transform_html(html, "div", |el| {
            el.append(" World", ContentType::Text);
        })
        .unwrap();

        assert!(result.contains("Hello World"));
    }

    #[test]
    fn test_remove_element() {
        let html = r#"<div><span class="remove">Remove me</span><span>Keep me</span></div>"#;

        let result = transform_html(html, "span.remove", |el| {
            el.remove();
        })
        .unwrap();

        assert!(!result.contains("Remove me"));
        assert!(result.contains("Keep me"));
    }

    #[test]
    fn test_multiple_handlers() {
        let html = r#"<div><a href="http://example.com">Link</a></div>"#;

        let mut rewriter = HTMLRewriter::new();
        rewriter.on("a", |el| {
            if let Some(href) = el.get_attribute("href") {
                el.set_attribute("href", &href.replace("http://", "https://"));
            }
        });
        rewriter.on("div", |el| {
            el.set_attribute("class", "container");
        });

        let result = rewriter.transform(html).unwrap();

        assert!(result.contains("https://example.com"));
        assert!(result.contains(r#"class="container""#));
    }
}
