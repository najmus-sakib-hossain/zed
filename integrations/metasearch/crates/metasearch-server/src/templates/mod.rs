//! Tera template management.

use std::sync::Arc;
use tera::{Context, Tera};

/// Wrapper around Tera templates.
pub struct Templates {
    tera: Tera,
}

impl Templates {
    /// Load templates from the file system.
    /// Looks for templates in the `templates` directory relative to the working directory.
    pub fn new(template_path: &str) -> anyhow::Result<Self> {
        let pattern = format!("{}/**/*.html", template_path);
        let tera = Tera::new(&pattern)?;
        
        tracing::info!(
            "Loaded {} templates from {}",
            tera.get_template_names().count(),
            template_path
        );
        
        Ok(Self { tera })
    }

    /// Render a template with the given context.
    pub fn render(&self, template_name: &str, context: &Context) -> anyhow::Result<String> {
        Ok(self.tera.render(template_name, context)?)
    }

    /// Get the underlying Tera instance (for extensions).
    pub fn tera_mut(&mut self) -> &mut Tera {
        &mut self.tera
    }
}

/// Thread-safe template handle.
pub type TemplateEngine = Arc<Templates>;
