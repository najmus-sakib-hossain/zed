//! # dx-print — Print Stylesheet Generator
//!
//! Automatically generate print-optimized CSS.
//!
//! ## Features
//! - Print-specific CSS generation
//! - Page break optimization
//! - Link expansion
//! - Color to grayscale conversion

#![forbid(unsafe_code)]

/// Print stylesheet configuration
#[derive(Debug, Clone)]
pub struct PrintConfig {
    /// Remove backgrounds
    pub remove_backgrounds: bool,
    /// Expand links (show URLs)
    pub expand_links: bool,
    /// Force grayscale
    pub force_grayscale: bool,
    /// Optimize page breaks
    pub optimize_page_breaks: bool,
}

impl Default for PrintConfig {
    fn default() -> Self {
        Self {
            remove_backgrounds: true,
            expand_links: true,
            force_grayscale: false,
            optimize_page_breaks: true,
        }
    }
}

/// Print stylesheet analyzer
pub struct PrintAnalyzer {
    config: PrintConfig,
}

impl PrintAnalyzer {
    /// Create new analyzer
    pub fn new(config: PrintConfig) -> Self {
        Self { config }
    }

    /// Generate print CSS
    pub fn generate_print_css(&self) -> String {
        let mut css = String::new();

        // Media query wrapper
        css.push_str("@media print {\n");

        // Hide non-printable elements
        css.push_str("  /* Hide UI elements */\n");
        css.push_str("  nav, aside, .no-print, button, input[type=\"button\"] {\n");
        css.push_str("    display: none !important;\n");
        css.push_str("  }\n\n");

        // Optimize backgrounds
        if self.config.remove_backgrounds {
            css.push_str("  /* Remove backgrounds to save ink */\n");
            css.push_str("  * {\n");
            css.push_str("    background: white !important;\n");
            css.push_str("    color: black !important;\n");
            css.push_str("  }\n\n");
        }

        // Expand links
        if self.config.expand_links {
            css.push_str("  /* Show link URLs */\n");
            css.push_str("  a[href]:after {\n");
            css.push_str("    content: \" (\" attr(href) \")\";\n");
            css.push_str("    font-size: 0.9em;\n");
            css.push_str("    color: #666;\n");
            css.push_str("  }\n\n");
        }

        // Force grayscale
        if self.config.force_grayscale {
            css.push_str("  /* Convert to grayscale */\n");
            css.push_str("  * {\n");
            css.push_str("    -webkit-print-color-adjust: exact;\n");
            css.push_str("    print-color-adjust: exact;\n");
            css.push_str("    filter: grayscale(100%);\n");
            css.push_str("  }\n\n");
        }

        // Page break optimization
        if self.config.optimize_page_breaks {
            css.push_str("  /* Optimize page breaks */\n");
            css.push_str("  h1, h2, h3, h4, h5, h6 {\n");
            css.push_str("    page-break-after: avoid;\n");
            css.push_str("    page-break-inside: avoid;\n");
            css.push_str("  }\n\n");
            css.push_str("  img, table, figure {\n");
            css.push_str("    page-break-inside: avoid;\n");
            css.push_str("  }\n\n");
            css.push_str("  p, blockquote, li {\n");
            css.push_str("    orphans: 3;\n");
            css.push_str("    widows: 3;\n");
            css.push_str("  }\n\n");
        }

        // Page settings
        css.push_str("  /* Page settings */\n");
        css.push_str("  @page {\n");
        css.push_str("    margin: 2cm;\n");
        css.push_str("    size: A4;\n");
        css.push_str("  }\n\n");

        // Typography optimization
        css.push_str("  /* Typography */\n");
        css.push_str("  body {\n");
        css.push_str("    font-size: 12pt;\n");
        css.push_str("    line-height: 1.5;\n");
        css.push_str("  }\n\n");

        css.push_str("}\n");

        css
    }

    /// Analyze existing CSS for print issues
    pub fn analyze_css(&self, css: &str) -> Vec<PrintIssue> {
        let mut issues = Vec::new();

        // Check for fixed positioning (doesn't work in print)
        if css.contains("position: fixed") || css.contains("position:fixed") {
            issues.push(PrintIssue::new(
                "fixed-position",
                "Fixed positioning doesn't work in print",
                "Use static or relative positioning",
            ));
        }

        // Check for viewport units (not reliable in print)
        if css.contains("vw") || css.contains("vh") {
            issues.push(PrintIssue::new(
                "viewport-units",
                "Viewport units may not work correctly in print",
                "Use fixed units (cm, pt, px) for print",
            ));
        }

        // Check for missing print media query
        if !css.contains("@media print") {
            issues.push(PrintIssue::new(
                "missing-print-query",
                "No @media print rules found",
                "Add print-specific styles for better output",
            ));
        }

        issues
    }
}

impl Default for PrintAnalyzer {
    fn default() -> Self {
        Self::new(PrintConfig::default())
    }
}

/// Print-related issue
#[derive(Debug, Clone)]
pub struct PrintIssue {
    pub rule: String,
    pub message: String,
    pub suggestion: String,
}

impl PrintIssue {
    /// Create new issue
    pub fn new(
        rule: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            rule: rule.into(),
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PrintConfig::default();
        assert!(config.remove_backgrounds);
        assert!(config.expand_links);
        assert!(config.optimize_page_breaks);
    }

    #[test]
    fn test_generate_print_css() {
        let analyzer = PrintAnalyzer::default();
        let css = analyzer.generate_print_css();

        assert!(css.contains("@media print"));
        assert!(css.contains("display: none"));
        assert!(css.contains("attr(href)"));
        assert!(css.contains("page-break"));
    }

    #[test]
    fn test_analyze_css() {
        let analyzer = PrintAnalyzer::default();
        let issues = analyzer.analyze_css("body { position: fixed; width: 100vw; }");

        assert!(issues.iter().any(|i| i.rule == "fixed-position"));
        assert!(issues.iter().any(|i| i.rule == "viewport-units"));
    }

    #[test]
    fn test_missing_print_query() {
        let analyzer = PrintAnalyzer::default();
        let issues = analyzer.analyze_css("body { color: black; }");

        assert!(issues.iter().any(|i| i.rule == "missing-print-query"));
    }
}
