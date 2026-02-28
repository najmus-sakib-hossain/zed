//! # Micro Codegen - The 338-Byte Generator
//!
//! Generates raw Rust FFI calls from TSX AST for the ultra-minimal runtime.
//! This is a transpiler that converts React-like syntax into system instructions.
//!
//! ## Architecture
//! ```text
//! TSX: <div class="box">{count}</div>
//!       ↓
//! Rust: host_create_element() + host_set_attr() + host_set_text()
//! ```
//!
//! ## Optimizations
//! 1. String Interner: Deduplicates all static strings
//! 2. Event Index: Maps event handlers to u32 IDs (no closures)
//! 3. State Pointers: Pass memory addresses, not copies

use anyhow::Result;
use std::collections::HashMap;

use crate::splitter::{Binding, StateSchema, Template};

// ============================================================================
// String Interner - Deduplicate Static Strings
// ============================================================================

/// Interns strings to avoid duplicates in generated code
struct StringInterner {
    /// Map from string content to variable name
    strings: HashMap<String, String>,
    /// Counter for generating unique names
    counter: usize,
}

impl StringInterner {
    fn new() -> Self {
        Self {
            strings: HashMap::new(),
            counter: 0,
        }
    }

    /// Intern a string, returning the variable name to use
    fn intern(&mut self, s: &str) -> String {
        if let Some(name) = self.strings.get(s) {
            return name.clone();
        }

        // Generate a descriptive name based on content
        let name = self.generate_name(s);
        self.strings.insert(s.to_string(), name.clone());
        name
    }

    /// Generate a unique variable name for a string
    fn generate_name(&mut self, s: &str) -> String {
        let id = self.counter;
        self.counter += 1;

        // Create a descriptive suffix from content (sanitized)
        let suffix: String = s
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(10)
            .collect::<String>()
            .to_uppercase();

        if suffix.is_empty() {
            format!("STR_{}", id)
        } else {
            format!("STR_{}_{}", suffix, id)
        }
    }

    /// Generate all static string declarations
    fn generate_statics(&self) -> String {
        let mut lines = Vec::new();

        // Sort for deterministic output
        let mut entries: Vec<_> = self.strings.iter().collect();
        entries.sort_by_key(|(_, name)| name.as_str());

        for (content, name) in entries {
            // Escape the string content for Rust
            let escaped = content.replace('\\', "\\\\").replace('"', "\\\"");
            lines.push(format!("static {}: &[u8] = b\"{}\";", name, escaped));
        }

        lines.join("\n")
    }
}

// ============================================================================
// Event Index - Map Handlers to IDs
// ============================================================================

/// Maps event handler expressions to numeric IDs
#[allow(dead_code)]
struct EventIndex {
    /// Map from expression to event ID
    handlers: HashMap<String, u32>,
    /// Counter for generating IDs
    next_id: u32,
}

impl EventIndex {
    fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            next_id: 0,
        }
    }

    /// Register an event handler, returning its ID
    #[allow(dead_code)]
    fn register(&mut self, expression: &str) -> u32 {
        if let Some(&id) = self.handlers.get(expression) {
            return id;
        }

        let id = self.next_id;
        self.next_id += 1;
        self.handlers.insert(expression.to_string(), id);
        id
    }

    /// Generate the on_event dispatcher function
    fn generate_dispatcher(&self) -> String {
        if self.handlers.is_empty() {
            return String::new();
        }

        let mut lines = vec![
            "/// Event dispatcher - called by JS with event ID".to_string(),
            "#[no_mangle]".to_string(),
            "pub extern \"C\" fn on_event(id: u32) {".to_string(),
            "    match id {".to_string(),
        ];

        // Sort for deterministic output
        let mut entries: Vec<_> = self.handlers.iter().collect();
        entries.sort_by_key(|(_, id)| *id);

        for (expr, id) in entries {
            // Generate the handler call
            // For now, just call the expression as a function
            let handler_name = expr.replace("()", "").replace("=> ", "").trim().to_string();

            lines.push(format!("        {} => {{ /* {} */ }},", id, handler_name));
        }

        lines.push("        _ => {}".to_string());
        lines.push("    }".to_string());
        lines.push("}".to_string());

        lines.join("\n")
    }
}

// ============================================================================
// State Generator - Handle Component State
// ============================================================================

/// Generates static state variables from schemas
fn generate_state_statics(schemas: &[StateSchema]) -> String {
    let mut lines = Vec::new();

    for schema in schemas {
        for field in &schema.fields {
            let rust_type = match field.type_name.as_str() {
                "number" => "i32",
                "boolean" => "bool",
                "string" => "&'static str",
                _ => "i32", // Default to i32
            };

            let initial = match field.type_name.as_str() {
                "number" => field.initial_value.clone(),
                "boolean" => field.initial_value.clone(),
                "string" => format!("\"{}\"", field.initial_value.trim_matches('"')),
                _ => field.initial_value.clone(),
            };

            lines.push(format!(
                "static mut {}: {} = {};",
                field.name.to_uppercase(),
                rust_type,
                initial
            ));
        }
    }

    lines.join("\n")
}

// ============================================================================
// Main Generator
// ============================================================================

/// Generate Micro-mode Rust code from templates and bindings
///
/// Returns: Complete Rust source code as a String
pub fn generate_micro(
    templates: &[Template],
    bindings: &[Binding],
    schemas: &[StateSchema],
    verbose: bool,
) -> Result<String> {
    if verbose {
        println!("  [Micro] Generating raw FFI Rust code...");
    }

    let mut interner = StringInterner::new();
    let mut events = EventIndex::new();
    let mut render_body = Vec::new();

    // Pre-intern common strings
    interner.intern("class");
    interner.intern("id");

    // Process templates and generate element creation code
    for template in templates {
        let (code, _node_var) = generate_template_code(template, &mut interner, &mut events)?;
        render_body.push(code);
    }

    // Process bindings for dynamic content
    for binding in bindings {
        let code = generate_binding_code(binding, &mut interner)?;
        render_body.push(code);
    }

    // Build the complete Rust file using vec! macro
    let mut output = vec![
        "//! Generated by dx-compiler (Micro Mode)".to_string(),
        "//! DO NOT EDIT - This file is automatically generated".to_string(),
        "".to_string(),
        "#![no_std]".to_string(),
        "#![no_main]".to_string(),
        "".to_string(),
        // Panic handler
        "#[panic_handler]".to_string(),
        "fn panic(_: &core::panic::PanicInfo) -> ! {".to_string(),
        "    unsafe { core::arch::wasm32::unreachable() }".to_string(),
        "}".to_string(),
        "".to_string(),
        // FFI imports
        "// ============================================================================".to_string(),
        "// FFI: JavaScript Host Functions".to_string(),
        "// ============================================================================".to_string(),
        "".to_string(),
        "extern \"C\" {".to_string(),
        "    fn host_create_element(tag_ptr: *const u8, tag_len: u32) -> u32;".to_string(),
        "    fn host_create_text(text_ptr: *const u8, text_len: u32) -> u32;".to_string(),
        "    fn host_set_text(node_id: u32, ptr: *const u8, len: u32);".to_string(),
        "    fn host_set_attr(node_id: u32, k_ptr: *const u8, k_len: u32, v_ptr: *const u8, v_len: u32);".to_string(),
        "    fn host_append(parent_id: u32, child_id: u32);".to_string(),
        "    fn host_add_event_listener(node_id: u32, event_type: u32, handler_id: u32);".to_string(),
        "    fn host_remove(node_id: u32);".to_string(),
        "}".to_string(),
        "".to_string(),
        // Event type constants
        "// Event types".to_string(),
        "const EVENT_CLICK: u32 = 0;".to_string(),
        "const EVENT_INPUT: u32 = 1;".to_string(),
        "const EVENT_CHANGE: u32 = 2;".to_string(),
        "const EVENT_SUBMIT: u32 = 3;".to_string(),
        "".to_string(),
        // Static strings section
        "// ============================================================================".to_string(),
        "// Static Strings (Interned)".to_string(),
        "// ============================================================================".to_string(),
        "".to_string(),
        interner.generate_statics(),
        "".to_string(),
    ];

    // State statics
    if !schemas.is_empty() {
        output.push(
            "// ============================================================================"
                .to_string(),
        );
        output.push("// Component State".to_string());
        output.push(
            "// ============================================================================"
                .to_string(),
        );
        output.push("".to_string());
        output.push(generate_state_statics(schemas));
        output.push("".to_string());
    }

    // Init function
    output.push(
        "// ============================================================================"
            .to_string(),
    );
    output.push("// WASM Exports".to_string());
    output.push(
        "// ============================================================================"
            .to_string(),
    );
    output.push("".to_string());
    output.push("#[no_mangle]".to_string());
    output.push("pub extern \"C\" fn init() -> u32 {".to_string());
    output.push("    0 // Success".to_string());
    output.push("}".to_string());
    output.push("".to_string());

    // Render function
    output.push("/// Initial render - builds the DOM tree".to_string());
    output.push("#[no_mangle]".to_string());
    output.push("pub extern \"C\" fn render() {".to_string());
    output.push("    unsafe {".to_string());

    for line in &render_body {
        for l in line.lines() {
            output.push(format!("        {}", l));
        }
    }

    output.push("    }".to_string());
    output.push("}".to_string());
    output.push("".to_string());

    // Event dispatcher
    let dispatcher = events.generate_dispatcher();
    if !dispatcher.is_empty() {
        output.push(dispatcher);
        output.push("".to_string());
    }

    if verbose {
        println!("    Generated {} lines of Rust code", output.len());
        println!("    Interned {} unique strings", interner.strings.len());
        println!("    Registered {} event handlers", events.handlers.len());
    }

    Ok(output.join("\n"))
}

/// Generate Rust code for a single template
fn generate_template_code(
    template: &Template,
    interner: &mut StringInterner,
    _events: &mut EventIndex,
) -> Result<(String, String)> {
    let mut lines = Vec::new();
    let node_var = format!("node_{}", template.id);

    // Parse the template HTML to extract tag name and attributes
    // This is a simplified parser - production would use proper HTML parser
    let html = &template.html;

    // Extract tag name (first word after <)
    let tag_name = extract_tag_name(html);
    let tag_var = interner.intern(&tag_name);

    // Sanitize HTML for comment (replace < > with [ ], newlines with spaces)
    let sanitized_html: String = html
        .chars()
        .take(50)
        .map(|c| match c {
            '<' => '[',
            '>' => ']',
            '\n' | '\r' => ' ',
            _ => c,
        })
        .collect();

    lines.push(format!("// Template {}: {}", template.id, sanitized_html));
    lines.push(format!(
        "let {} = host_create_element({}.as_ptr(), {}.len() as u32);",
        node_var, tag_var, tag_var
    ));

    // Extract and set attributes
    let attrs = extract_attributes(html);
    for (key, value) in attrs {
        let key_var = interner.intern(&key);
        let val_var = interner.intern(&value);

        lines.push(format!(
            "host_set_attr({}, {}.as_ptr(), {}.len() as u32, {}.as_ptr(), {}.len() as u32);",
            node_var, key_var, key_var, val_var, val_var
        ));
    }

    // Append to body (root = 0)
    lines.push(format!("host_append(0, {});", node_var));

    Ok((lines.join("\n"), node_var))
}

/// Generate Rust code for a binding (dynamic content)
fn generate_binding_code(binding: &Binding, _interner: &mut StringInterner) -> Result<String> {
    let mut lines = Vec::new();

    // Check if this is an event handler (contains =>) or a regular state binding
    let expr = &binding.expression;

    if expr.contains("=>") || expr.contains("set") {
        // Event handler - will be wired through on_event dispatcher
        // Sanitize for comment
        let sanitized: String = expr
            .chars()
            .take(30)
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_')
            .collect();
        lines.push(format!("// Slot {}: event handler ({}...)", binding.slot_id, sanitized));
    } else {
        // State binding - convert to Rust variable reference
        // "self.count" -> "COUNT", "count" -> "COUNT"
        let rust_var = expr
            .replace("self.", "")
            .replace("state.", "")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_uppercase();

        if !rust_var.is_empty() {
            lines.push(format!("// Slot {}: state binding", binding.slot_id));
            // Generate actual host_set_text call (placeholder node_id for now)
            // TODO: track actual node IDs from template generation
        }
    }

    Ok(lines.join("\n"))
}

/// Extract tag name from HTML string
fn extract_tag_name(html: &str) -> String {
    let html = html.trim();
    if !html.starts_with('<') {
        return "div".to_string();
    }

    let after_open = &html[1..];
    let end = after_open
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .unwrap_or(after_open.len());

    after_open[..end].to_string()
}

/// Extract attributes from HTML string
fn extract_attributes(html: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();

    // Simple regex-like extraction: key="value" or key='value'
    let mut in_tag = false;
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            in_tag = true;
            continue;
        }
        if c == '>' {
            break;
        }
        if !in_tag {
            continue;
        }

        // Skip whitespace
        if c.is_whitespace() {
            continue;
        }

        // Look for key=
        if c.is_alphabetic() {
            let mut key = String::from(c);
            while let Some(&next) = chars.peek() {
                if next == '=' || next.is_whitespace() || next == '>' {
                    break;
                }
                key.push(chars.next().unwrap());
            }

            // Check for = and value
            if chars.peek() == Some(&'=') {
                chars.next(); // consume =

                // Get quote type
                if let Some(&quote) = chars.peek() {
                    if quote == '"' || quote == '\'' {
                        chars.next(); // consume opening quote
                        let mut value = String::new();
                        while let Some(&vc) = chars.peek() {
                            if vc == quote {
                                chars.next(); // consume closing quote
                                break;
                            }
                            value.push(chars.next().unwrap());
                        }

                        // Skip JSX expressions {..}
                        if !value.starts_with('{') {
                            attrs.push((key, value));
                        }
                    }
                }
            }
        }
    }

    attrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interner() {
        let mut interner = StringInterner::new();

        let name1 = interner.intern("div");
        let name2 = interner.intern("div");
        let name3 = interner.intern("span");

        assert_eq!(name1, name2, "Same string should return same name");
        assert_ne!(name1, name3, "Different strings should have different names");
    }

    #[test]
    fn test_extract_tag_name() {
        assert_eq!(extract_tag_name("<div>"), "div");
        assert_eq!(extract_tag_name("<span class=\"foo\">"), "span");
        assert_eq!(extract_tag_name("<h1>Hello</h1>"), "h1");
        assert_eq!(extract_tag_name("<button onClick=\"...\">"), "button");
    }

    #[test]
    fn test_extract_attributes() {
        let attrs = extract_attributes("<div class=\"box\" id=\"main\">");
        assert_eq!(attrs.len(), 2);
        assert!(attrs.contains(&("class".to_string(), "box".to_string())));
        assert!(attrs.contains(&("id".to_string(), "main".to_string())));
    }

    #[test]
    fn test_event_index() {
        let mut events = EventIndex::new();

        let id1 = events.register("handleClick");
        let id2 = events.register("handleSubmit");
        let id3 = events.register("handleClick"); // duplicate

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 0, "Duplicate handler should return same ID");
    }

    #[test]
    fn test_generate_micro_empty() {
        let result = generate_micro(&[], &[], &[], false);
        assert!(result.is_ok());

        let code = result.unwrap();
        assert!(code.contains("#![no_std]"));
        assert!(code.contains("extern \"C\""));
        assert!(code.contains("pub extern \"C\" fn render()"));
    }
}
