//! Template Cache: Pre-parsed HtmlTemplateElement storage
//!
//! Templates are parsed ONCE and cloned via native `cloneNode()`.

use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlTemplateElement, Node};

/// Maximum number of templates (matches dx_packet::MAX_TEMPLATES)
const MAX_TEMPLATES: usize = 4096;

/// Template cache using fixed-size array (no Vec allocation)
pub struct TemplateCache {
    /// Pre-parsed templates (None = not registered)
    templates: [Option<HtmlTemplateElement>; MAX_TEMPLATES],
    /// Count of registered templates
    count: u16,
    /// Cached document reference
    document: Document,
}

impl TemplateCache {
    /// Create new cache
    pub fn new() -> Result<Self, u8> {
        let window = web_sys::window().ok_or(1u8)?;
        let document = window.document().ok_or(2u8)?;

        Ok(Self {
            templates: std::array::from_fn(|_| None),
            count: 0,
            document,
        })
    }

    /// Register a template from HTML string
    ///
    /// # Arguments
    /// * `id` - Template ID (must be < MAX_TEMPLATES)
    /// * `html` - HTML string to parse
    pub fn register(&mut self, id: u16, html: &str) -> Result<(), u8> {
        if id as usize >= MAX_TEMPLATES {
            return Err(4u8); // TemplateNotFound (out of range)
        }

        // Create template element
        let template = self
            .document
            .create_element("template")
            .map_err(|_| 3u8)?
            .dyn_into::<HtmlTemplateElement>()
            .map_err(|_| 3u8)?;

        // Parse HTML into template content
        template.set_inner_html(html);

        self.templates[id as usize] = Some(template);
        self.count += 1;

        Ok(())
    }

    /// Clone a template's content
    ///
    /// Uses native `cloneNode(true)` for C++ speed
    pub fn clone_template(&self, id: u16) -> Result<Node, u8> {
        self.templates
            .get(id as usize)
            .and_then(|t| t.as_ref())
            .and_then(|t| t.content().clone_node_with_deep(true).ok())
            .ok_or(4u8) // TemplateNotFound
    }

    /// Get template count
    pub fn count(&self) -> u16 {
        self.count
    }
}
