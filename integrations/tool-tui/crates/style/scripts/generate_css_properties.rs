//! CSS Property Database Generator
//!
//! This script fetches CSS property definitions from MDN/W3C and generates
//! a DX Serializer format file containing all CSS properties with their
//! valid values, units, categories, and browser support information.
//!
//! Usage: cargo run --bin generate_css_properties
//!
//! Output:
//! - `.dx/style/css-properties.sr` - Source file in DX LLM format
//! - `.dx/serializer/css-properties.llm` - LLM-optimized format
//! - `.dx/serializer/css-properties.machine` - Binary format for runtime loading
//!
//! Requirements: 6.1, 6.2, 6.3, 6.4, 6.5

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// CSS Property definition with all metadata
#[derive(Debug, Clone)]
pub struct CssPropertyDef {
    /// Property name (e.g., "display", "flex-direction")
    pub name: String,
    /// Valid keyword values (e.g., ["flex", "block", "grid", "none"])
    pub values: Vec<String>,
    /// Whether property accepts numeric values with units
    pub accepts_numeric: bool,
    /// Valid units if accepts_numeric (e.g., ["px", "rem", "em", "%"])
    pub valid_units: Vec<String>,
    /// Category (layout, typography, color, animation, etc.)
    pub category: String,
    /// Browser support info (simplified)
    pub browser_support: BrowserSupport,
}

/// Browser support information
#[derive(Debug, Clone, Default)]
pub struct BrowserSupport {
    pub chrome: Option<u32>,
    pub firefox: Option<u32>,
    pub safari: Option<u32>,
    pub edge: Option<u32>,
}

/// CSS Property Database containing all properties
pub struct CssPropertyDatabase {
    pub properties: BTreeMap<String, CssPropertyDef>,
    pub categories: BTreeMap<String, Vec<String>>,
    pub shorthands: BTreeMap<String, Vec<String>>,
}

impl CssPropertyDatabase {
    /// Create a new database with all standard CSS properties
    pub fn new() -> Self {
        let mut db = Self {
            properties: BTreeMap::new(),
            categories: BTreeMap::new(),
            shorthands: BTreeMap::new(),
        };
        db.populate_properties();
        db.build_category_index();
        db.build_shorthand_mappings();
        db
    }

    /// Populate the database with CSS properties
    /// This is a comprehensive list based on MDN CSS Reference
    fn populate_properties(&mut self) {
        // Layout properties
        self.add_property(CssPropertyDef {
            name: "display".into(),
            values: vec!["block", "inline", "inline-block", "flex", "inline-flex", 
                        "grid", "inline-grid", "none", "contents", "flow-root",
                        "table", "table-row", "table-cell", "list-item"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "layout".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "position".into(),
            values: vec!["static", "relative", "absolute", "fixed", "sticky"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "layout".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "float".into(),
            values: vec!["left", "right", "none", "inline-start", "inline-end"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "layout".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "clear".into(),
            values: vec!["none", "left", "right", "both", "inline-start", "inline-end"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "layout".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "visibility".into(),
            values: vec!["visible", "hidden", "collapse"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "layout".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "overflow".into(),
            values: vec!["visible", "hidden", "scroll", "auto", "clip"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "layout".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "overflow-x".into(),
            values: vec!["visible", "hidden", "scroll", "auto", "clip"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "layout".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(3), safari: Some(3), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "overflow-y".into(),
            values: vec!["visible", "hidden", "scroll", "auto", "clip"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "layout".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(3), safari: Some(3), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "z-index".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec![],  // unitless integer
            category: "layout".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        // Flexbox properties
        self.add_property(CssPropertyDef {
            name: "flex-direction".into(),
            values: vec!["row", "row-reverse", "column", "column-reverse"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(29), firefox: Some(20), safari: Some(9), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "flex-wrap".into(),
            values: vec!["nowrap", "wrap", "wrap-reverse"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(29), firefox: Some(28), safari: Some(9), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "justify-content".into(),
            values: vec!["flex-start", "flex-end", "center", "space-between", "space-around", 
                        "space-evenly", "start", "end", "left", "right", "normal", "stretch"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(29), firefox: Some(20), safari: Some(9), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "align-items".into(),
            values: vec!["stretch", "flex-start", "flex-end", "center", "baseline",
                        "start", "end", "self-start", "self-end", "normal"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(29), firefox: Some(20), safari: Some(9), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "align-content".into(),
            values: vec!["stretch", "flex-start", "flex-end", "center", "space-between",
                        "space-around", "space-evenly", "start", "end", "normal", "baseline"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(29), firefox: Some(28), safari: Some(9), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "align-self".into(),
            values: vec!["auto", "stretch", "flex-start", "flex-end", "center", "baseline",
                        "start", "end", "self-start", "self-end", "normal"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(29), firefox: Some(20), safari: Some(9), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "flex-grow".into(),
            values: vec![].iter().map(|s: &&str| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec![],  // unitless number
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(29), firefox: Some(20), safari: Some(9), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "flex-shrink".into(),
            values: vec![].iter().map(|s: &&str| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec![],  // unitless number
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(29), firefox: Some(20), safari: Some(9), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "flex-basis".into(),
            values: vec!["auto", "content", "max-content", "min-content", "fit-content"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh", "vmin", "vmax", "ch", "ex"].iter().map(|s| s.to_string()).collect(),
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(29), firefox: Some(22), safari: Some(9), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "order".into(),
            values: vec![].iter().map(|s: &&str| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec![],  // unitless integer
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(29), firefox: Some(20), safari: Some(9), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "gap".into(),
            values: vec!["normal"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(84), firefox: Some(63), safari: Some(14), edge: Some(84) },
        });

        self.add_property(CssPropertyDef {
            name: "row-gap".into(),
            values: vec!["normal"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(84), firefox: Some(63), safari: Some(14), edge: Some(84) },
        });

        self.add_property(CssPropertyDef {
            name: "column-gap".into(),
            values: vec!["normal"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "flexbox".into(),
            browser_support: BrowserSupport { chrome: Some(84), firefox: Some(63), safari: Some(14), edge: Some(84) },
        });

        // Grid properties
        self.add_property(CssPropertyDef {
            name: "grid-template-columns".into(),
            values: vec!["none", "auto", "max-content", "min-content", "subgrid"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "fr", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "grid".into(),
            browser_support: BrowserSupport { chrome: Some(57), firefox: Some(52), safari: Some(10), edge: Some(16) },
        });

        self.add_property(CssPropertyDef {
            name: "grid-template-rows".into(),
            values: vec!["none", "auto", "max-content", "min-content", "subgrid"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "fr", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "grid".into(),
            browser_support: BrowserSupport { chrome: Some(57), firefox: Some(52), safari: Some(10), edge: Some(16) },
        });

        self.add_property(CssPropertyDef {
            name: "grid-auto-flow".into(),
            values: vec!["row", "column", "dense", "row dense", "column dense"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "grid".into(),
            browser_support: BrowserSupport { chrome: Some(57), firefox: Some(52), safari: Some(10), edge: Some(16) },
        });

        self.add_property(CssPropertyDef {
            name: "grid-auto-columns".into(),
            values: vec!["auto", "max-content", "min-content"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "fr", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "grid".into(),
            browser_support: BrowserSupport { chrome: Some(57), firefox: Some(52), safari: Some(10), edge: Some(16) },
        });

        self.add_property(CssPropertyDef {
            name: "grid-auto-rows".into(),
            values: vec!["auto", "max-content", "min-content"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "fr", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "grid".into(),
            browser_support: BrowserSupport { chrome: Some(57), firefox: Some(52), safari: Some(10), edge: Some(16) },
        });

        self.add_property(CssPropertyDef {
            name: "grid-column".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec![],  // span values
            category: "grid".into(),
            browser_support: BrowserSupport { chrome: Some(57), firefox: Some(52), safari: Some(10), edge: Some(16) },
        });

        self.add_property(CssPropertyDef {
            name: "grid-row".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec![],  // span values
            category: "grid".into(),
            browser_support: BrowserSupport { chrome: Some(57), firefox: Some(52), safari: Some(10), edge: Some(16) },
        });

        self.add_property(CssPropertyDef {
            name: "place-items".into(),
            values: vec!["start", "end", "center", "stretch", "baseline", "normal"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "grid".into(),
            browser_support: BrowserSupport { chrome: Some(59), firefox: Some(45), safari: Some(11), edge: Some(79) },
        });

        self.add_property(CssPropertyDef {
            name: "place-content".into(),
            values: vec!["start", "end", "center", "stretch", "space-between", "space-around", "space-evenly", "baseline", "normal"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "grid".into(),
            browser_support: BrowserSupport { chrome: Some(59), firefox: Some(45), safari: Some(11), edge: Some(79) },
        });

        // Sizing properties
        self.add_property(CssPropertyDef {
            name: "width".into(),
            values: vec!["auto", "max-content", "min-content", "fit-content", "stretch"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh", "vmin", "vmax", "ch", "ex", "svw", "svh", "lvw", "lvh", "dvw", "dvh"].iter().map(|s| s.to_string()).collect(),
            category: "sizing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "height".into(),
            values: vec!["auto", "max-content", "min-content", "fit-content", "stretch"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh", "vmin", "vmax", "ch", "ex", "svw", "svh", "lvw", "lvh", "dvw", "dvh"].iter().map(|s| s.to_string()).collect(),
            category: "sizing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "min-width".into(),
            values: vec!["auto", "max-content", "min-content", "fit-content"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh", "vmin", "vmax"].iter().map(|s| s.to_string()).collect(),
            category: "sizing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "max-width".into(),
            values: vec!["none", "max-content", "min-content", "fit-content"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh", "vmin", "vmax"].iter().map(|s| s.to_string()).collect(),
            category: "sizing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "min-height".into(),
            values: vec!["auto", "max-content", "min-content", "fit-content"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh", "vmin", "vmax"].iter().map(|s| s.to_string()).collect(),
            category: "sizing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "max-height".into(),
            values: vec!["none", "max-content", "min-content", "fit-content"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh", "vmin", "vmax"].iter().map(|s| s.to_string()).collect(),
            category: "sizing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "aspect-ratio".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec![],  // ratio like 16/9
            category: "sizing".into(),
            browser_support: BrowserSupport { chrome: Some(88), firefox: Some(89), safari: Some(15), edge: Some(88) },
        });

        // Spacing properties
        self.add_property(CssPropertyDef {
            name: "margin".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "spacing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "margin-top".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "spacing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "margin-right".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "spacing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "margin-bottom".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "spacing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "margin-left".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "spacing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "padding".into(),
            values: vec![].iter().map(|s: &&str| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "spacing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "padding-top".into(),
            values: vec![].iter().map(|s: &&str| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "spacing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "padding-right".into(),
            values: vec![].iter().map(|s: &&str| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "spacing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "padding-bottom".into(),
            values: vec![].iter().map(|s: &&str| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "spacing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "padding-left".into(),
            values: vec![].iter().map(|s: &&str| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "spacing".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        // Position properties
        self.add_property(CssPropertyDef {
            name: "top".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "position".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "right".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "position".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "bottom".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "position".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "left".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "position".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "inset".into(),
            values: vec!["auto"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh"].iter().map(|s| s.to_string()).collect(),
            category: "position".into(),
            browser_support: BrowserSupport { chrome: Some(87), firefox: Some(66), safari: Some(14), edge: Some(87) },
        });

        // Typography properties
        self.add_property(CssPropertyDef {
            name: "font-size".into(),
            values: vec!["xx-small", "x-small", "small", "medium", "large", "x-large", "xx-large", "xxx-large", "smaller", "larger"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%", "vw", "vh", "pt", "cm", "mm", "in"].iter().map(|s| s.to_string()).collect(),
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "font-weight".into(),
            values: vec!["normal", "bold", "bolder", "lighter"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec![],  // 100-900
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "font-style".into(),
            values: vec!["normal", "italic", "oblique"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "font-family".into(),
            values: vec!["serif", "sans-serif", "monospace", "cursive", "fantasy", "system-ui", "ui-serif", "ui-sans-serif", "ui-monospace", "ui-rounded"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "line-height".into(),
            values: vec!["normal"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%"].iter().map(|s| s.to_string()).collect(),
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "letter-spacing".into(),
            values: vec!["normal"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em"].iter().map(|s| s.to_string()).collect(),
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "word-spacing".into(),
            values: vec!["normal"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%"].iter().map(|s| s.to_string()).collect(),
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "text-align".into(),
            values: vec!["left", "right", "center", "justify", "start", "end", "match-parent"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "text-decoration".into(),
            values: vec!["none", "underline", "overline", "line-through", "blink"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "text-transform".into(),
            values: vec!["none", "capitalize", "uppercase", "lowercase", "full-width", "full-size-kana"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "text-indent".into(),
            values: vec!["hanging", "each-line"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%"].iter().map(|s| s.to_string()).collect(),
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "white-space".into(),
            values: vec!["normal", "nowrap", "pre", "pre-wrap", "pre-line", "break-spaces"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "word-break".into(),
            values: vec!["normal", "break-all", "keep-all", "break-word"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(15), safari: Some(3), edge: Some(12) },
        });

        self.add_property(CssPropertyDef {
            name: "overflow-wrap".into(),
            values: vec!["normal", "break-word", "anywhere"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: false,
            valid_units: vec![],
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(23), firefox: Some(49), safari: Some(7), edge: Some(18) },
        });

        self.add_property(CssPropertyDef {
            name: "vertical-align".into(),
            values: vec!["baseline", "sub", "super", "text-top", "text-bottom", "middle", "top", "bottom"].iter().map(|s| s.to_string()).collect(),
            accepts_numeric: true,
            valid_units: vec!["px", "rem", "em", "%"].iter().map(|s| s.to_string()).collect(),
            category: "typography".into(),
            browser_support: BrowserSupport { chrome: Some(1), firefox: Some(1), safari: Some(1), edge: Some(12) },
        });
