//! # Splitter Module - The Holographic Engine
//!
//! The secret sauce. Separates the "Bone" from the "Muscle."
//!
//! ## Algorithm
//! Scan JSX: `<div class="box">Count: {state.count}</div>`
//! - **Extraction 1 (Template):** `<div class="box">Count: <!--SLOT_0--></div>` -> Saved to `template_map`
//! - **Extraction 2 (Binding):** `SLOT_0` maps to `state.count`
//!
//! ## Output
//! - `templates`: A list of unique DOM structures
//! - `bindings`: A mapping of Slot IDs to Rust expressions

use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::parser::{Component, ParsedModule, StateDef};

// Re-export shared types from dx-www-packet
pub use dx_www_packet::{SlotDef, SlotType, Template};

// Pre-compiled regex patterns for JSX parsing
// SAFETY: These patterns are compile-time constants that have been validated during development.
// Compilation failure would indicate a bug in the source code.
static MAP_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{([\w.]+)\.map\s*\(\s*(?:\(([^)]*)\)|(\w+))\s*=>\s*")
        .unwrap_or_else(|e| panic!("BUG: Invalid map regex pattern: {}", e))
});

static AND_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{([^}]+)\s*&&\s*([^}]+)\}")
        .unwrap_or_else(|e| panic!("BUG: Invalid and regex pattern: {}", e))
});

static TERNARY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{([^}?]+)\s*\?\s*([^}:]+)\s*:\s*([^}]+)\}")
        .unwrap_or_else(|e| panic!("BUG: Invalid ternary regex pattern: {}", e))
});

static EXPRESSION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{([^}]+)\}")
        .unwrap_or_else(|e| panic!("BUG: Invalid expression regex pattern: {}", e))
});

static STATE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"state\.(\w+)")
        .unwrap_or_else(|e| panic!("BUG: Invalid state regex pattern: {}", e))
});

static TAG_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<(/?)(\w+)[^>]*>")
        .unwrap_or_else(|e| panic!("BUG: Invalid tag regex pattern: {}", e))
});

static ATTR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\w+)\s*=\s*["'][^"']*$"#)
        .unwrap_or_else(|e| panic!("BUG: Invalid attr regex pattern: {}", e))
});

static KEY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"key\s*=\s*\{([^}]+)\}"#)
        .unwrap_or_else(|e| panic!("BUG: Invalid key regex pattern: {}", e))
});

/// Binding flags for conditional and iteration bindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingFlag {
    /// Normal binding - always evaluated
    Normal,
    /// Conditional binding - from && or ternary expressions
    Conditional,
    /// Iteration binding - from .map() calls
    Iteration,
}

/// Binding from Slot to Rust expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub slot_id: u32,
    pub component: String,
    pub expression: String,             // Rust expression (e.g., "self.count")
    pub dirty_bit: u8,                  // Which bit in dirty_mask
    pub flag: BindingFlag,              // Conditional, iteration, or normal
    pub key_expression: Option<String>, // Key for iteration bindings
}

/// State schema for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSchema {
    pub component: String,
    pub fields: Vec<StateField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateField {
    pub name: String,
    pub type_name: String,
    pub initial_value: String,
    pub dirty_bit: u8,
}

/// Split components into templates and bindings
pub fn split_components(
    modules: Vec<ParsedModule>,
    verbose: bool,
) -> Result<(Vec<Template>, Vec<Binding>, Vec<StateSchema>)> {
    if verbose {
        println!("  Splitting components...");
    }

    let mut templates = Vec::new();
    let mut bindings = Vec::new();
    let mut schemas = Vec::new();
    let mut template_dedup: HashMap<String, u32> = HashMap::new();
    let mut next_template_id = 0u32;
    let mut next_slot_id = 0u32;

    for module in &modules {
        for component in &module.components {
            if verbose {
                println!("    Processing component: {}", component.name);
            }

            // Extract state schema
            let schema = extract_state_schema(component)?;
            schemas.push(schema);

            // Parse JSX body and split
            let (template, component_bindings) = split_jsx(
                &component.jsx_body,
                &component.name,
                &component.state,
                &mut next_template_id,
                &mut next_slot_id,
                &mut template_dedup,
            )?;

            if let Some(template) = template {
                templates.push(template);
            }
            bindings.extend(component_bindings);
        }
    }

    if verbose {
        println!("  Extracted {} templates, {} bindings", templates.len(), bindings.len());
    }

    Ok((templates, bindings, schemas))
}

/// Extract state schema from component
fn extract_state_schema(component: &Component) -> Result<StateSchema> {
    use crate::errors::DxError;

    let mut fields = Vec::new();
    let mut dirty_bit = 0u8;

    for state_def in &component.state {
        fields.push(StateField {
            name: state_def.name.clone(),
            type_name: state_def.type_annotation.clone(),
            initial_value: state_def.initial_value.clone(),
            dirty_bit,
        });

        dirty_bit += 1;
        if dirty_bit >= 64 {
            let err = DxError::compilation_error_in_component(
                "extracting state schema",
                "Component has more than 64 state fields (dirty_mask overflow). Consider splitting into smaller components.",
                &component.name,
            );
            return Err(anyhow::anyhow!("{}", err.format_detailed()));
        }
    }

    Ok(StateSchema {
        component: component.name.clone(),
        fields,
    })
}

/// Split JSX into template and bindings
fn split_jsx(
    jsx_body: &str,
    component_name: &str,
    state_fields: &[StateDef],
    next_template_id: &mut u32,
    next_slot_id: &mut u32,
    template_dedup: &mut HashMap<String, u32>,
) -> Result<(Option<Template>, Vec<Binding>)> {
    if jsx_body.is_empty() {
        return Ok((None, Vec::new()));
    }

    // Build dirty bit map from state fields
    let dirty_bit_map: HashMap<String, u8> = state_fields
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.clone(), i as u8))
        .collect();

    let mut html = jsx_body.to_string();
    let mut slots = Vec::new();
    let mut bindings = Vec::new();

    // Parse JSX to extract static HTML and dynamic expressions
    // We process expressions in order of specificity:
    // 1. Iteration expressions (.map())
    // 2. Conditional expressions (&& and ternary)
    // 3. Simple expressions

    // Track processed ranges to avoid double-processing
    let mut processed_expressions: Vec<String> = Vec::new();

    // Process .map() iteration expressions
    for capture in MAP_PATTERN.captures_iter(jsx_body) {
        let match_start = capture.get(0).unwrap().start();
        let match_end = capture.get(0).unwrap().end();

        // Find the full expression by counting braces from the opening {
        let remaining = &jsx_body[match_end..];
        let mut brace_count = 1; // We're inside the arrow function body
        let mut paren_count = 1; // We're inside the .map( call
        let mut body_end = 0;

        for (i, c) in remaining.char_indices() {
            match c {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 && paren_count == 0 {
                        body_end = i + 1;
                        break;
                    }
                }
                '(' => paren_count += 1,
                ')' => {
                    paren_count -= 1;
                    if paren_count == 0 && brace_count == 1 {
                        // Found the closing ) of .map(), next } should close the expression
                    }
                }
                _ => {}
            }
        }

        if body_end == 0 {
            continue; // Couldn't find matching braces
        }

        let full_match = &jsx_body[match_start..match_end + body_end];
        if processed_expressions.contains(&full_match.to_string()) {
            continue;
        }
        processed_expressions.push(full_match.to_string());

        let array_expr = capture.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        let item_param =
            capture.get(2).or(capture.get(3)).map(|m| m.as_str().trim()).unwrap_or("item");

        // Extract body expression (everything between => and the final )})
        let body_expr = &remaining[..body_end - 2]; // Remove final )}

        let slot_id = *next_slot_id;
        *next_slot_id += 1;

        let marker = format!("<!--SLOT_{}-->", slot_id);
        html = html.replace(full_match, &marker);

        // Calculate DOM path (simplified - would need proper JSX AST for accurate paths)
        let path = calculate_dom_path(&html, &marker);

        slots.push(SlotDef {
            slot_id,
            slot_type: SlotType::Text,
            path,
        });

        // Determine dirty bit from array expression
        let dirty_bit = find_dirty_bit(array_expr, &dirty_bit_map);

        // Extract key expression if present (e.g., key={item.id})
        let key_expression = extract_key_expression(body_expr);

        let rust_expr = convert_to_rust_expression(array_expr);

        bindings.push(Binding {
            slot_id,
            component: component_name.to_string(),
            expression: format!("{}.iter().map(|{}| {})", rust_expr, item_param, body_expr),
            dirty_bit,
            flag: BindingFlag::Iteration,
            key_expression,
        });
    }

    // Process && conditional expressions
    for capture in AND_PATTERN.captures_iter(jsx_body) {
        let full_match = capture.get(0).unwrap().as_str();
        if processed_expressions.contains(&full_match.to_string()) {
            continue;
        }
        processed_expressions.push(full_match.to_string());

        let condition = capture.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        let true_branch = capture.get(2).map(|m| m.as_str().trim()).unwrap_or("");

        let slot_id = *next_slot_id;
        *next_slot_id += 1;

        let marker = format!("<!--SLOT_{}-->", slot_id);
        html = html.replace(full_match, &marker);

        let path = calculate_dom_path(&html, &marker);

        slots.push(SlotDef {
            slot_id,
            slot_type: SlotType::Text,
            path,
        });

        let dirty_bit = find_dirty_bit(condition, &dirty_bit_map);
        let rust_expr = convert_to_rust_expression(condition);

        bindings.push(Binding {
            slot_id,
            component: component_name.to_string(),
            expression: format!("if {} {{ {} }} else {{ \"\" }}", rust_expr, true_branch),
            dirty_bit,
            flag: BindingFlag::Conditional,
            key_expression: None,
        });
    }

    // Process ternary conditional expressions
    for capture in TERNARY_PATTERN.captures_iter(jsx_body) {
        let full_match = capture.get(0).unwrap().as_str();
        if processed_expressions.contains(&full_match.to_string()) {
            continue;
        }
        processed_expressions.push(full_match.to_string());

        let condition = capture.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        let true_branch = capture.get(2).map(|m| m.as_str().trim()).unwrap_or("");
        let false_branch = capture.get(3).map(|m| m.as_str().trim()).unwrap_or("");

        let slot_id = *next_slot_id;
        *next_slot_id += 1;

        let marker = format!("<!--SLOT_{}-->", slot_id);
        html = html.replace(full_match, &marker);

        let path = calculate_dom_path(&html, &marker);

        slots.push(SlotDef {
            slot_id,
            slot_type: SlotType::Text,
            path,
        });

        let dirty_bit = find_dirty_bit(condition, &dirty_bit_map);
        let rust_expr = convert_to_rust_expression(condition);

        bindings.push(Binding {
            slot_id,
            component: component_name.to_string(),
            expression: format!(
                "if {} {{ {} }} else {{ {} }}",
                rust_expr, true_branch, false_branch
            ),
            dirty_bit,
            flag: BindingFlag::Conditional,
            key_expression: None,
        });
    }

    // Process remaining simple expressions
    for capture in EXPRESSION_PATTERN.captures_iter(jsx_body) {
        let full_match = capture.get(0).unwrap().as_str();
        if processed_expressions.contains(&full_match.to_string()) {
            continue;
        }

        let Some(expr_match) = capture.get(1) else {
            continue;
        };
        let expression = expr_match.as_str().trim();

        // Skip if this looks like a complex expression we should have caught earlier
        if expression.contains(".map(") || expression.contains("&&") || expression.contains("?") {
            continue;
        }

        let slot_id = *next_slot_id;
        *next_slot_id += 1;

        let marker = format!("<!--SLOT_{}-->", slot_id);
        html = html.replace(full_match, &marker);

        // Determine slot type based on context
        let slot_type = determine_slot_type(&html, &marker);
        let path = calculate_dom_path(&html, &marker);

        slots.push(SlotDef {
            slot_id,
            slot_type,
            path,
        });

        let dirty_bit = find_dirty_bit(expression, &dirty_bit_map);
        let rust_expr = convert_to_rust_expression(expression);

        bindings.push(Binding {
            slot_id,
            component: component_name.to_string(),
            expression: rust_expr,
            dirty_bit,
            flag: BindingFlag::Normal,
            key_expression: None,
        });
    }

    // Deduplicate templates
    let hash = blake3::hash(html.as_bytes()).to_hex().to_string();

    let template_id = if let Some(&existing_id) = template_dedup.get(&hash) {
        existing_id
    } else {
        let id = *next_template_id;
        *next_template_id += 1;
        template_dedup.insert(hash.clone(), id);
        id
    };

    let template = Template {
        id: template_id,
        html,
        slots,
        hash,
    };

    Ok((Some(template), bindings))
}

/// Convert JavaScript expression to Rust expression
fn convert_to_rust_expression(js_expr: &str) -> String {
    js_expr
        .replace("state.", "self.")
        .replace("props.", "self.props.")
        .replace("===", "==")
        .replace("!==", "!=")
        .replace("||", " || ")
        .replace("&&", " && ")
}

/// Find the dirty bit for an expression based on state field references
fn find_dirty_bit(expression: &str, dirty_bit_map: &HashMap<String, u8>) -> u8 {
    // Look for state.fieldName patterns using pre-compiled regex
    for capture in STATE_PATTERN.captures_iter(expression) {
        if let Some(field_name) = capture.get(1) {
            if let Some(&bit) = dirty_bit_map.get(field_name.as_str()) {
                return bit;
            }
        }
    }

    // Default to bit 0 if no state reference found
    0
}

/// Calculate DOM path to a slot marker
fn calculate_dom_path(html: &str, marker: &str) -> Vec<u32> {
    // Simplified path calculation - counts element depth
    // Production would need proper DOM tree traversal
    let marker_pos = html.find(marker).unwrap_or(0);
    let before_marker = &html[..marker_pos];

    let mut depth = 0u32;
    let mut path = vec![0];

    // Count opening and closing tags to determine depth using pre-compiled regex
    for capture in TAG_PATTERN.captures_iter(before_marker) {
        let is_closing = capture.get(1).map(|m| !m.as_str().is_empty()).unwrap_or(false);
        let tag_name = capture.get(2).map(|m| m.as_str()).unwrap_or("");

        // Skip self-closing tags
        if is_self_closing_tag(tag_name) {
            continue;
        }

        if is_closing {
            if depth > 0 {
                depth -= 1;
                path.pop();
            }
        } else {
            depth += 1;
            path.push(depth);
        }
    }

    path
}

/// Check if a tag is self-closing
fn is_self_closing_tag(tag: &str) -> bool {
    matches!(
        tag.to_lowercase().as_str(),
        "br" | "hr"
            | "img"
            | "input"
            | "meta"
            | "link"
            | "area"
            | "base"
            | "col"
            | "embed"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Determine slot type based on context in HTML
fn determine_slot_type(html: &str, marker: &str) -> SlotType {
    let marker_pos = html.find(marker).unwrap_or(0);
    let before_marker = &html[..marker_pos];

    // Check if we're inside an attribute using pre-compiled regex
    if ATTR_PATTERN.is_match(before_marker) {
        return SlotType::Attribute;
    }

    // Check if we're in a style attribute
    if before_marker.ends_with("style=\"")
        || before_marker.contains("style=\"") && !before_marker.contains("\"")
    {
        return SlotType::Property;
    }

    // Default to text content
    SlotType::Text
}

/// Extract key expression from JSX body (e.g., key={item.id})
fn extract_key_expression(body: &str) -> Option<String> {
    KEY_PATTERN
        .captures(body)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::StateDef;

    fn make_state_fields() -> Vec<StateDef> {
        vec![
            StateDef {
                name: "count".to_string(),
                setter_name: "setCount".to_string(),
                initial_value: "0".to_string(),
                type_annotation: "number".to_string(),
            },
            StateDef {
                name: "items".to_string(),
                setter_name: "setItems".to_string(),
                initial_value: "[]".to_string(),
                type_annotation: "array".to_string(),
            },
            StateDef {
                name: "visible".to_string(),
                setter_name: "setVisible".to_string(),
                initial_value: "true".to_string(),
                type_annotation: "boolean".to_string(),
            },
        ]
    }

    #[test]
    fn test_jsx_splitting() {
        let jsx = r#"<div>Count: {state.count}</div>"#;
        let state_fields = make_state_fields();
        let mut next_template_id = 0;
        let mut next_slot_id = 0;
        let mut dedup = HashMap::new();

        let (template, bindings) = split_jsx(
            jsx,
            "TestComponent",
            &state_fields,
            &mut next_template_id,
            &mut next_slot_id,
            &mut dedup,
        )
        .unwrap();

        assert!(template.is_some());
        let template = template.unwrap();
        assert!(template.html.contains("<!--SLOT_0-->"));
        assert!(!template.html.contains("dummy"));
        assert!(!template.html.contains("TODO"));
        assert!(!template.html.contains("placeholder"));
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].expression, "self.count");
        assert_eq!(bindings[0].dirty_bit, 0); // count is first state field
        assert_eq!(bindings[0].flag, BindingFlag::Normal);
    }

    #[test]
    fn test_conditional_and_expression() {
        let jsx = r#"<div>{state.visible && <span>Visible!</span>}</div>"#;
        let state_fields = make_state_fields();
        let mut next_template_id = 0;
        let mut next_slot_id = 0;
        let mut dedup = HashMap::new();

        let (template, bindings) = split_jsx(
            jsx,
            "TestComponent",
            &state_fields,
            &mut next_template_id,
            &mut next_slot_id,
            &mut dedup,
        )
        .unwrap();

        assert!(template.is_some());
        let template = template.unwrap();
        assert!(template.html.contains("<!--SLOT_"));
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].flag, BindingFlag::Conditional);
        assert_eq!(bindings[0].dirty_bit, 2); // visible is third state field
    }

    #[test]
    fn test_conditional_ternary_expression() {
        let jsx = r#"<div>{state.visible ? "Yes" : "No"}</div>"#;
        let state_fields = make_state_fields();
        let mut next_template_id = 0;
        let mut next_slot_id = 0;
        let mut dedup = HashMap::new();

        let (template, bindings) = split_jsx(
            jsx,
            "TestComponent",
            &state_fields,
            &mut next_template_id,
            &mut next_slot_id,
            &mut dedup,
        )
        .unwrap();

        assert!(template.is_some());
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].flag, BindingFlag::Conditional);
        assert!(bindings[0].expression.contains("if"));
    }

    #[test]
    fn test_iteration_map_expression() {
        let jsx = r#"<ul>{state.items.map((item) => <li key={item.id}>{item.name}</li>)}</ul>"#;
        let state_fields = make_state_fields();
        let mut next_template_id = 0;
        let mut next_slot_id = 0;
        let mut dedup = HashMap::new();

        let (template, bindings) = split_jsx(
            jsx,
            "TestComponent",
            &state_fields,
            &mut next_template_id,
            &mut next_slot_id,
            &mut dedup,
        )
        .unwrap();

        assert!(template.is_some());
        // Should have at least one iteration binding
        let iteration_bindings: Vec<_> =
            bindings.iter().filter(|b| b.flag == BindingFlag::Iteration).collect();
        assert!(!iteration_bindings.is_empty());
        assert_eq!(iteration_bindings[0].dirty_bit, 1); // items is second state field
        assert!(iteration_bindings[0].key_expression.is_some());
    }

    #[test]
    fn test_no_dummy_templates() {
        let jsx = r#"<div class="container"><h1>Title</h1><p>{state.count}</p></div>"#;
        let state_fields = make_state_fields();
        let mut next_template_id = 0;
        let mut next_slot_id = 0;
        let mut dedup = HashMap::new();

        let (template, _bindings) = split_jsx(
            jsx,
            "TestComponent",
            &state_fields,
            &mut next_template_id,
            &mut next_slot_id,
            &mut dedup,
        )
        .unwrap();

        assert!(template.is_some());
        let template = template.unwrap();

        // Verify no dummy/placeholder content
        let html_lower = template.html.to_lowercase();
        assert!(!html_lower.contains("dummy"));
        assert!(!html_lower.contains("todo"));
        assert!(!html_lower.contains("placeholder"));

        // Verify actual JSX structure is preserved
        assert!(template.html.contains("container"));
        assert!(template.html.contains("<h1>Title</h1>"));
    }

    #[test]
    fn test_dirty_bit_mapping() {
        let jsx = r#"<div>{state.count} - {state.visible}</div>"#;
        let state_fields = make_state_fields();
        let mut next_template_id = 0;
        let mut next_slot_id = 0;
        let mut dedup = HashMap::new();

        let (_template, bindings) = split_jsx(
            jsx,
            "TestComponent",
            &state_fields,
            &mut next_template_id,
            &mut next_slot_id,
            &mut dedup,
        )
        .unwrap();

        assert_eq!(bindings.len(), 2);
        // count is at index 0, visible is at index 2
        let count_binding = bindings.iter().find(|b| b.expression.contains("count")).unwrap();
        let visible_binding = bindings.iter().find(|b| b.expression.contains("visible")).unwrap();

        assert_eq!(count_binding.dirty_bit, 0);
        assert_eq!(visible_binding.dirty_bit, 2);
    }
}
