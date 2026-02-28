//! CSS Property Database
//!
//! Contains the static CSS property definitions used to generate the database.
//! This data is derived from the CSS specification and MDN documentation.

/// Browser support information for a CSS property
#[derive(Debug, Clone, PartialEq)]
pub struct BrowserSupport {
    pub chrome: Option<u32>,
    pub firefox: Option<u32>,
    pub safari: Option<u32>,
    pub edge: Option<u32>,
}

impl Default for BrowserSupport {
    fn default() -> Self {
        Self {
            chrome: Some(1),
            firefox: Some(1),
            safari: Some(1),
            edge: Some(12),
        }
    }
}

/// CSS property definition
#[derive(Debug, Clone, PartialEq)]
pub struct CssPropertyDef {
    pub name: String,
    pub values: Vec<String>,
    pub accepts_numeric: bool,
    pub valid_units: Vec<String>,
    pub browser_support: BrowserSupport,
    pub category: String,
}

impl CssPropertyDef {
    pub fn new(name: &str, category: &str) -> Self {
        Self {
            name: name.to_string(),
            values: Vec::new(),
            accepts_numeric: false,
            valid_units: Vec::new(),
            browser_support: BrowserSupport::default(),
            category: category.to_string(),
        }
    }

    pub fn with_values(mut self, values: &[&str]) -> Self {
        self.values = values.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_numeric(mut self, units: &[&str]) -> Self {
        self.accepts_numeric = true;
        self.valid_units = units.iter().map(|s| s.to_string()).collect();
        self
    }
}

/// Standard length units
pub const LENGTH_UNITS: &[&str] = &["px", "rem", "em", "%", "vw", "vh", "vmin", "vmax", "ch"];
/// Time units
pub const TIME_UNITS: &[&str] = &["ms", "s"];

/// Get all CSS property definitions from the CSS specification.
/// **Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5**
pub fn get_all_css_properties() -> Vec<CssPropertyDef> {
    vec![
        // Layout
        CssPropertyDef::new("display", "layout").with_values(&[
            "none",
            "block",
            "inline",
            "inline-block",
            "flex",
            "inline-flex",
            "grid",
            "inline-grid",
            "contents",
            "flow-root",
        ]),
        CssPropertyDef::new("position", "layout")
            .with_values(&["static", "relative", "absolute", "fixed", "sticky"]),
        CssPropertyDef::new("float", "layout").with_values(&["none", "left", "right"]),
        CssPropertyDef::new("clear", "layout").with_values(&["none", "left", "right", "both"]),
        CssPropertyDef::new("visibility", "layout").with_values(&["visible", "hidden", "collapse"]),
        CssPropertyDef::new("overflow", "layout")
            .with_values(&["visible", "hidden", "scroll", "auto", "clip"]),
        CssPropertyDef::new("overflow-x", "layout")
            .with_values(&["visible", "hidden", "scroll", "auto", "clip"]),
        CssPropertyDef::new("overflow-y", "layout")
            .with_values(&["visible", "hidden", "scroll", "auto", "clip"]),
        CssPropertyDef::new("z-index", "layout")
            .with_values(&["auto"])
            .with_numeric(&[]),
        // Flexbox
        CssPropertyDef::new("flex-direction", "flexbox").with_values(&[
            "row",
            "row-reverse",
            "column",
            "column-reverse",
        ]),
        CssPropertyDef::new("flex-wrap", "flexbox").with_values(&[
            "nowrap",
            "wrap",
            "wrap-reverse",
        ]),
        CssPropertyDef::new("justify-content", "flexbox").with_values(&[
            "flex-start",
            "flex-end",
            "center",
            "space-between",
            "space-around",
            "space-evenly",
            "start",
            "end",
        ]),
        CssPropertyDef::new("align-items", "flexbox").with_values(&[
            "flex-start",
            "flex-end",
            "center",
            "baseline",
            "stretch",
            "start",
            "end",
        ]),
        CssPropertyDef::new("align-content", "flexbox").with_values(&[
            "flex-start",
            "flex-end",
            "center",
            "space-between",
            "space-around",
            "stretch",
        ]),
        CssPropertyDef::new("align-self", "flexbox").with_values(&[
            "auto",
            "flex-start",
            "flex-end",
            "center",
            "baseline",
            "stretch",
        ]),
        CssPropertyDef::new("flex-grow", "flexbox").with_numeric(&[]),
        CssPropertyDef::new("flex-shrink", "flexbox").with_numeric(&[]),
        CssPropertyDef::new("flex-basis", "flexbox")
            .with_values(&["auto", "content"])
            .with_numeric(LENGTH_UNITS),
        CssPropertyDef::new("order", "flexbox").with_numeric(&[]),
        // Grid
        CssPropertyDef::new("grid-auto-flow", "grid").with_values(&["row", "column", "dense"]),
        CssPropertyDef::new("gap", "grid")
            .with_values(&["normal"])
            .with_numeric(LENGTH_UNITS),
        CssPropertyDef::new("row-gap", "grid")
            .with_values(&["normal"])
            .with_numeric(LENGTH_UNITS),
        CssPropertyDef::new("column-gap", "grid")
            .with_values(&["normal"])
            .with_numeric(LENGTH_UNITS),
        CssPropertyDef::new("place-content", "grid").with_values(&[
            "center",
            "start",
            "end",
            "stretch",
            "space-between",
            "space-around",
        ]),
        CssPropertyDef::new("place-items", "grid")
            .with_values(&["center", "start", "end", "stretch", "baseline"]),
        CssPropertyDef::new("place-self", "grid")
            .with_values(&["auto", "center", "start", "end", "stretch"]),
        // Sizing
        CssPropertyDef::new("width", "sizing")
            .with_values(&["auto", "max-content", "min-content", "fit-content"])
            .with_numeric(LENGTH_UNITS),
        CssPropertyDef::new("height", "sizing")
            .with_values(&["auto", "max-content", "min-content", "fit-content"])
            .with_numeric(LENGTH_UNITS),
        CssPropertyDef::new("min-width", "sizing")
            .with_values(&["auto", "max-content", "min-content"])
            .with_numeric(LENGTH_UNITS),
        CssPropertyDef::new("min-height", "sizing")
            .with_values(&["auto", "max-content", "min-content"])
            .with_numeric(LENGTH_UNITS),
        CssPropertyDef::new("max-width", "sizing")
            .with_values(&["none", "max-content", "min-content"])
            .with_numeric(LENGTH_UNITS),
        CssPropertyDef::new("max-height", "sizing")
            .with_values(&["none", "max-content", "min-content"])
            .with_numeric(LENGTH_UNITS),
    ]
}
