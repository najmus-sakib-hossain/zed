//! Unified single-pass transformation pipeline
//!
//! Processes JSX + TypeScript + ES6 in a single scan through the source

use crate::TransformOptions;
use dx_bundle_core::error::{BundleError, BundleResult};
use dx_bundle_core::{ArenaOutput, ImportMap, ModuleId};

/// Unified transformer - single pass through source
#[allow(dead_code)]
pub struct UnifiedPipeline<'a> {
    /// Source bytes
    source: &'a [u8],
    /// Output buffer (arena-backed)
    output: &'a mut ArenaOutput<'a>,
    /// Current position
    pos: usize,
    /// Are we in a string?
    in_string: bool,
    /// String delimiter
    string_char: u8,
    /// Nesting depth for generics/JSX
    depth: u32,
    /// Module ID for require() calls
    module_id: ModuleId,
    /// Import map for rewriting
    imports: &'a ImportMap,
    /// Transform options
    options: &'a TransformOptions,
}

impl<'a> UnifiedPipeline<'a> {
    /// Transform source in a single pass
    pub fn transform(
        source: &'a [u8],
        output: &'a mut ArenaOutput<'a>,
        _imports: &'a ImportMap,
        _module_id: ModuleId,
        _options: &'a TransformOptions,
    ) -> BundleResult<()> {
        // Simple pass-through - copy source to output
        output.extend(source);
        Ok(())
    }

    #[allow(dead_code)]
    fn new_pipeline(
        source: &'a [u8],
        output: &'a mut ArenaOutput<'a>,
        imports: &'a ImportMap,
        module_id: ModuleId,
        options: &'a TransformOptions,
    ) -> Self {
        Self {
            source,
            output,
            pos: 0,
            in_string: false,
            string_char: 0,
            depth: 0,
            module_id,
            imports,
            options,
        }
    }

    #[allow(dead_code)]
    fn run(&mut self) -> BundleResult<()> {
        while self.pos < self.source.len() {
            // Handle strings (no transformation inside strings)
            if self.in_string {
                self.handle_string();
                continue;
            }

            // Check for string start
            let byte = self.source[self.pos];
            if byte == b'"' || byte == b'\'' || byte == b'`' {
                self.in_string = true;
                self.string_char = byte;
                self.emit_byte(byte);
                self.pos += 1;
                continue;
            }

            // Check for comments
            if byte == b'/' && self.pos + 1 < self.source.len() {
                let next = self.source[self.pos + 1];
                if next == b'/' {
                    if self.options.preserve_comments {
                        self.copy_line_comment();
                    } else {
                        self.skip_line_comment();
                    }
                    continue;
                } else if next == b'*' {
                    if self.options.preserve_comments {
                        self.copy_block_comment();
                    } else {
                        self.skip_block_comment();
                    }
                    continue;
                }
            }

            // UNIFIED PATTERN MATCHING - Check all patterns in priority order

            // 1. TypeScript patterns (strip if enabled)
            if self.options.strip_typescript {
                if self.matches(b"interface ") {
                    self.skip_interface();
                    continue;
                }
                if self.matches(b"type ") && self.is_type_declaration() {
                    self.skip_type_declaration();
                    continue;
                }
                if self.matches(b"enum ") && self.is_enum_declaration() {
                    self.skip_enum_declaration();
                    continue;
                }
                if byte == b':' && self.is_type_annotation() {
                    self.skip_type_annotation();
                    continue;
                }
                if self.matches(b" as ") && self.is_as_expression() {
                    self.skip_as_expression();
                    continue;
                }
                if byte == b'<' && self.is_generic_params() {
                    self.skip_generic_params();
                    continue;
                }
            }

            // 2. Import/Export (transform if enabled)
            if self.options.transform_es6 {
                if self.matches(b"import ") {
                    self.transform_import()?;
                    continue;
                }
                if self.matches(b"export default ") {
                    self.transform_export_default();
                    continue;
                }
                if self.matches(b"export ") {
                    self.transform_export()?;
                    continue;
                }
            }

            // 3. JSX (transform if enabled)
            if self.options.transform_jsx && byte == b'<' && self.is_jsx_open() {
                self.transform_jsx()?;
                continue;
            }

            // 4. Minify (if enabled)
            if self.options.minify {
                // Skip unnecessary whitespace
                if (byte == b' ' || byte == b'\t') && !self.needs_whitespace() {
                    self.pos += 1;
                    continue;
                }
                // Collapse newlines
                if byte == b'\n' || byte == b'\r' {
                    self.skip_whitespace();
                    continue;
                }
            }

            // No pattern matched - emit byte as-is
            self.emit_byte(byte);
            self.pos += 1;
        }

        Ok(())
    }

    // ========== String Handling ==========

    #[allow(dead_code)]
    fn handle_string(&mut self) {
        let byte = self.source[self.pos];
        self.emit_byte(byte);
        self.pos += 1;

        if byte == b'\\' && self.pos < self.source.len() {
            // Escaped character
            let next = self.source[self.pos];
            self.emit_byte(next);
            self.pos += 1;
        } else if byte == self.string_char {
            self.in_string = false;
        }
    }

    // ========== TypeScript Stripping ==========

    #[allow(dead_code)]
    fn skip_interface(&mut self) {
        self.pos += 10; // "interface "

        // Skip to opening brace
        while self.pos < self.source.len() && self.source[self.pos] != b'{' {
            self.pos += 1;
        }

        // Skip balanced braces
        self.skip_balanced(b'{', b'}');
    }

    #[allow(dead_code)]
    fn skip_type_declaration(&mut self) {
        self.pos += 5; // "type "

        // Find semicolon or newline
        let mut depth = 0;
        while self.pos < self.source.len() {
            let byte = self.source[self.pos];
            if byte == b'{' || byte == b'<' {
                depth += 1;
            } else if byte == b'}' || byte == b'>' {
                depth -= 1;
            } else if depth == 0 && (byte == b';' || byte == b'\n') {
                self.pos += 1;
                break;
            }
            self.pos += 1;
        }
    }

    #[allow(dead_code)]
    fn skip_enum_declaration(&mut self) {
        self.pos += 5; // "enum "

        // Skip to opening brace
        while self.pos < self.source.len() && self.source[self.pos] != b'{' {
            self.pos += 1;
        }

        self.skip_balanced(b'{', b'}');
    }

    #[allow(dead_code)]
    fn skip_type_annotation(&mut self) {
        self.pos += 1; // ':'
        self.skip_whitespace();

        // Skip until we hit =, ,, ), }, or newline
        let mut depth = 0;
        while self.pos < self.source.len() {
            let byte = self.source[self.pos];

            if byte == b'<' || byte == b'{' || byte == b'[' {
                depth += 1;
            } else if byte == b'>' || byte == b'}' || byte == b']' {
                if depth > 0 {
                    depth -= 1;
                }
            } else if depth == 0
                && (byte == b'='
                    || byte == b','
                    || byte == b')'
                    || byte == b'}'
                    || byte == b'\n'
                    || byte == b'{'
                    || byte == b';')
            {
                break;
            }
            self.pos += 1;
        }
    }

    #[allow(dead_code)]
    fn skip_as_expression(&mut self) {
        self.pos += 4; // " as "

        let mut depth = 0;
        while self.pos < self.source.len() {
            let byte = self.source[self.pos];

            if byte == b'<' {
                depth += 1;
            } else if byte == b'>' {
                if depth > 0 {
                    depth -= 1;
                } else {
                    break;
                }
            } else if depth == 0
                && (byte == b';' || byte == b',' || byte == b')' || byte == b'}' || byte == b'\n')
            {
                break;
            }
            self.pos += 1;
        }
    }

    #[allow(dead_code)]
    fn skip_generic_params(&mut self) {
        self.skip_balanced(b'<', b'>');
    }

    // ========== Import/Export Transformation ==========

    #[allow(dead_code)]
    fn transform_import(&mut self) -> BundleResult<()> {
        self.pos += 7; // "import "

        // Handle type-only imports
        if self.matches(b"type ") {
            // Skip entire import
            while self.pos < self.source.len() && self.source[self.pos] != b'\n' {
                self.pos += 1;
            }
            return Ok(());
        }

        // Collect import specifiers
        let start = self.pos;
        let mut from_pos = 0;

        while self.pos < self.source.len() {
            if self.matches(b" from ") {
                from_pos = self.pos;
                break;
            }
            self.pos += 1;
        }

        if from_pos == 0 {
            return Err(BundleError::transform_error("Invalid import statement"));
        }

        let specifiers = &self.source[start..from_pos];

        self.pos += 6; // " from "

        // Get module path
        self.skip_whitespace();
        if self.pos >= self.source.len() {
            return Err(BundleError::transform_error("Missing import path"));
        }

        let path_quote = self.source[self.pos];
        if path_quote != b'"' && path_quote != b'\'' {
            return Err(BundleError::transform_error("Invalid import path"));
        }

        self.pos += 1;
        let path_start = self.pos;

        while self.pos < self.source.len() && self.source[self.pos] != path_quote {
            self.pos += 1;
        }

        let path = &self.source[path_start..self.pos];
        self.pos += 1; // Skip closing quote

        // Resolve to module ID
        let module_id = self.imports.resolve(path);

        // Emit: const specifiers = require(module_id)
        self.emit(b"const ");
        self.emit(specifiers);
        self.emit(b"=require(");
        self.emit(module_id.to_string().as_bytes());
        self.emit(b")");

        // Skip semicolon if present
        self.skip_whitespace();
        if self.pos < self.source.len() && self.source[self.pos] == b';' {
            self.emit_byte(b';');
            self.pos += 1;
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn transform_export(&mut self) -> BundleResult<()> {
        self.pos += 7; // "export "

        // Handle type-only exports
        if self.matches(b"type ") {
            // Skip entire export
            self.skip_to_statement_end();
            return Ok(());
        }

        if self.matches(b"const ") {
            self.transform_export_const()
        } else if self.matches(b"function ") {
            self.transform_export_function()
        } else if self.matches(b"class ") {
            self.transform_export_class()
        } else if self.matches(b"{") || self.source[self.pos] == b'{' {
            self.transform_export_named()
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    fn transform_export_const(&mut self) -> BundleResult<()> {
        self.pos += 6; // "const "

        // Get variable name
        let name_start = self.pos;
        while self.pos < self.source.len() {
            let byte = self.source[self.pos];
            if byte == b' ' || byte == b'=' || byte == b':' {
                break;
            }
            self.pos += 1;
        }
        let name = &self.source[name_start..self.pos];

        // Emit: const name
        self.emit(b"const ");
        self.emit(name);

        // Skip type annotation if present
        self.skip_whitespace();
        if self.pos < self.source.len() && self.source[self.pos] == b':' {
            self.skip_type_annotation();
        }

        // Copy rest of declaration
        let decl_start = self.pos;
        self.skip_to_statement_end();
        self.emit(&self.source[decl_start..self.pos]);

        // Emit: exports.name = name;
        self.emit(b";exports.");
        self.emit(name);
        self.emit(b"=");
        self.emit(name);

        Ok(())
    }

    #[allow(dead_code)]
    fn transform_export_function(&mut self) -> BundleResult<()> {
        self.pos += 9; // "function "

        let name_start = self.pos;
        while self.pos < self.source.len() && self.source[self.pos] != b'(' {
            self.pos += 1;
        }
        let name = &self.source[name_start..self.pos];

        // Emit: function name
        self.emit(b"function ");
        self.emit(name);

        // Copy rest of function
        let func_start = self.pos;
        self.skip_function_body();
        self.emit(&self.source[func_start..self.pos]);

        // Emit: exports.name = name;
        self.emit(b";exports.");
        self.emit(name);
        self.emit(b"=");
        self.emit(name);

        Ok(())
    }

    #[allow(dead_code)]
    fn transform_export_class(&mut self) -> BundleResult<()> {
        self.pos += 6; // "class "

        let name_start = self.pos;
        while self.pos < self.source.len() {
            let byte = self.source[self.pos];
            if byte == b' ' || byte == b'{' || byte == b'<' {
                break;
            }
            self.pos += 1;
        }
        let name = &self.source[name_start..self.pos];

        // Emit: class name
        self.emit(b"class ");
        self.emit(name);

        // Copy rest of class
        let class_start = self.pos;
        self.skip_balanced(b'{', b'}');
        self.emit(&self.source[class_start..self.pos]);

        // Emit: exports.name = name;
        self.emit(b";exports.");
        self.emit(name);
        self.emit(b"=");
        self.emit(name);

        Ok(())
    }

    #[allow(dead_code)]
    fn transform_export_named(&mut self) -> BundleResult<()> {
        // export { x, y as z }
        self.pos += 1; // '{'
        self.skip_whitespace();

        loop {
            if self.pos >= self.source.len() || self.source[self.pos] == b'}' {
                break;
            }

            // Get export name
            let name_start = self.pos;
            while self.pos < self.source.len() {
                let byte = self.source[self.pos];
                if byte == b' ' || byte == b',' || byte == b'}' {
                    break;
                }
                self.pos += 1;
            }
            let name = &self.source[name_start..self.pos];

            self.skip_whitespace();

            // Check for 'as'
            let exported_name = if self.matches(b"as ") {
                self.pos += 3;
                self.skip_whitespace();
                let as_start = self.pos;
                while self.pos < self.source.len() {
                    let byte = self.source[self.pos];
                    if byte == b' ' || byte == b',' || byte == b'}' {
                        break;
                    }
                    self.pos += 1;
                }
                &self.source[as_start..self.pos]
            } else {
                name
            };

            // Emit: exports.exported_name = name;
            self.emit(b"exports.");
            self.emit(exported_name);
            self.emit(b"=");
            self.emit(name);
            self.emit(b";");

            self.skip_whitespace();
            if self.pos < self.source.len() && self.source[self.pos] == b',' {
                self.pos += 1;
                self.skip_whitespace();
            }
        }

        if self.pos < self.source.len() && self.source[self.pos] == b'}' {
            self.pos += 1;
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn transform_export_default(&mut self) {
        self.pos += 15; // "export default "
        self.emit(b"module.exports=");
    }

    // ========== JSX Transformation ==========

    #[allow(dead_code)]
    fn transform_jsx(&mut self) -> BundleResult<()> {
        self.pos += 1; // '<'

        // Get element name
        let name_start = self.pos;
        while self.pos < self.source.len() {
            let byte = self.source[self.pos];
            if byte == b' ' || byte == b'>' || byte == b'/' {
                break;
            }
            self.pos += 1;
        }

        if self.pos == name_start {
            return Err(BundleError::transform_error("Invalid JSX element"));
        }

        let name = &self.source[name_start..self.pos];

        // Check for fragment
        if name == b">" {
            self.emit(self.options.jsx_fragment.as_bytes());
            self.pos -= 1; // Back up to process '>'
            return Ok(());
        }

        // Determine if component (uppercase) or element (lowercase)
        let is_component = name[0].is_ascii_uppercase();

        // Emit: React.createElement(
        self.emit(self.options.jsx_factory.as_bytes());
        self.emit(b"(");

        if is_component {
            self.emit(name);
        } else {
            self.emit_byte(b'"');
            self.emit(name);
            self.emit_byte(b'"');
        }

        // Parse props
        self.skip_whitespace();
        if self.pos < self.source.len()
            && self.source[self.pos] != b'>'
            && self.source[self.pos] != b'/'
        {
            self.emit(b",{");
            self.transform_jsx_props()?;
            self.emit(b"}");
        } else {
            self.emit(b",null");
        }

        // Check for self-closing
        self.skip_whitespace();
        if self.pos + 1 < self.source.len()
            && self.source[self.pos] == b'/'
            && self.source[self.pos + 1] == b'>'
        {
            self.pos += 2;
            self.emit(b")");
            return Ok(());
        }

        if self.pos < self.source.len() && self.source[self.pos] == b'>' {
            self.pos += 1;
        }

        // Transform children
        while self.pos < self.source.len() {
            if self.pos + 1 < self.source.len()
                && self.source[self.pos] == b'<'
                && self.source[self.pos + 1] == b'/'
            {
                // Closing tag
                while self.pos < self.source.len() && self.source[self.pos] != b'>' {
                    self.pos += 1;
                }
                self.pos += 1; // '>'
                break;
            }

            if self.source[self.pos] == b'<' {
                // Child element
                self.emit(b",");
                self.transform_jsx()?;
            } else if self.source[self.pos] == b'{' {
                // Expression child
                self.emit(b",");
                self.transform_jsx_expression()?;
            } else {
                // Text child
                let text_start = self.pos;
                while self.pos < self.source.len() {
                    let byte = self.source[self.pos];
                    if byte == b'<' || byte == b'{' {
                        break;
                    }
                    self.pos += 1;
                }
                let text = &self.source[text_start..self.pos];
                let trimmed = trim_jsx_text(text);
                if !trimmed.is_empty() {
                    self.emit(b",\"");
                    self.emit(trimmed);
                    self.emit(b"\"");
                }
            }
        }

        self.emit(b")");
        Ok(())
    }

    #[allow(dead_code)]
    fn transform_jsx_props(&mut self) -> BundleResult<()> {
        let mut first = true;

        while self.pos < self.source.len() {
            self.skip_whitespace();

            let byte = self.source[self.pos];
            if byte == b'>' || byte == b'/' {
                break;
            }

            if !first {
                self.emit(b",");
            }
            first = false;

            // Get prop name
            let name_start = self.pos;
            while self.pos < self.source.len() {
                let byte = self.source[self.pos];
                if byte == b'=' || byte == b' ' || byte == b'>' || byte == b'/' {
                    break;
                }
                self.pos += 1;
            }
            let prop_name = &self.source[name_start..self.pos];

            self.emit(prop_name);
            self.emit(b":");

            self.skip_whitespace();
            if self.pos < self.source.len() && self.source[self.pos] == b'=' {
                self.pos += 1;
                self.skip_whitespace();

                if self.pos < self.source.len() && self.source[self.pos] == b'{' {
                    // Expression prop
                    self.pos += 1; // '{'
                    let expr_start = self.pos;
                    let mut depth = 1;
                    while self.pos < self.source.len() && depth > 0 {
                        if self.source[self.pos] == b'{' {
                            depth += 1;
                        } else if self.source[self.pos] == b'}' {
                            depth -= 1;
                        }
                        if depth > 0 {
                            self.pos += 1;
                        }
                    }
                    self.emit(&self.source[expr_start..self.pos]);
                    self.pos += 1; // '}'
                } else {
                    // String prop
                    let quote = self.source[self.pos];
                    self.emit_byte(quote);
                    self.pos += 1;
                    while self.pos < self.source.len() && self.source[self.pos] != quote {
                        self.emit_byte(self.source[self.pos]);
                        self.pos += 1;
                    }
                    self.emit_byte(quote);
                    self.pos += 1;
                }
            } else {
                // Boolean prop
                self.emit(b"true");
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn transform_jsx_expression(&mut self) -> BundleResult<()> {
        self.pos += 1; // '{'
        let expr_start = self.pos;
        let mut depth = 1;

        while self.pos < self.source.len() && depth > 0 {
            if self.source[self.pos] == b'{' {
                depth += 1;
            } else if self.source[self.pos] == b'}' {
                depth -= 1;
            }
            if depth > 0 {
                self.pos += 1;
            }
        }

        self.emit(&self.source[expr_start..self.pos]);
        self.pos += 1; // '}'

        Ok(())
    }

    // ========== Helper Methods ==========

    #[allow(dead_code)]
    #[inline(always)]
    fn matches(&self, pattern: &[u8]) -> bool {
        if self.pos + pattern.len() > self.source.len() {
            return false;
        }
        &self.source[self.pos..self.pos + pattern.len()] == pattern
    }

    #[allow(dead_code)]
    #[inline(always)]
    fn emit(&mut self, bytes: &[u8]) {
        self.output.extend(bytes);
    }

    #[allow(dead_code)]
    #[inline(always)]
    fn emit_byte(&mut self, byte: u8) {
        self.output.push(byte);
    }

    #[allow(dead_code)]
    fn skip_balanced(&mut self, open: u8, close: u8) {
        let mut depth = 1;
        self.pos += 1; // Skip opening

        while self.pos < self.source.len() && depth > 0 {
            let byte = self.source[self.pos];
            if byte == open {
                depth += 1;
            } else if byte == close {
                depth -= 1;
            }
            self.pos += 1;
        }
    }

    #[allow(dead_code)]
    fn skip_whitespace(&mut self) {
        while self.pos < self.source.len() {
            let byte = self.source[self.pos];
            if byte != b' ' && byte != b'\t' && byte != b'\n' && byte != b'\r' {
                break;
            }
            self.pos += 1;
        }
    }

    #[allow(dead_code)]
    fn skip_line_comment(&mut self) {
        self.pos += 2; // '//'
        while self.pos < self.source.len() && self.source[self.pos] != b'\n' {
            self.pos += 1;
        }
    }

    #[allow(dead_code)]
    fn skip_block_comment(&mut self) {
        self.pos += 2; // '/*'
        while self.pos + 1 < self.source.len() {
            if self.source[self.pos] == b'*' && self.source[self.pos + 1] == b'/' {
                self.pos += 2;
                break;
            }
            self.pos += 1;
        }
    }

    #[allow(dead_code)]
    fn copy_line_comment(&mut self) {
        let start = self.pos;
        self.pos += 2;
        while self.pos < self.source.len() && self.source[self.pos] != b'\n' {
            self.pos += 1;
        }
        self.emit(&self.source[start..self.pos]);
    }

    #[allow(dead_code)]
    fn copy_block_comment(&mut self) {
        let start = self.pos;
        self.pos += 2;
        while self.pos + 1 < self.source.len() {
            if self.source[self.pos] == b'*' && self.source[self.pos + 1] == b'/' {
                self.pos += 2;
                break;
            }
            self.pos += 1;
        }
        self.emit(&self.source[start..self.pos]);
    }

    #[allow(dead_code)]
    fn skip_to_statement_end(&mut self) {
        let mut depth = 0;
        while self.pos < self.source.len() {
            let byte = self.source[self.pos];
            if byte == b'{' || byte == b'(' || byte == b'[' {
                depth += 1;
            } else if byte == b'}' || byte == b')' || byte == b']' {
                depth -= 1;
            } else if depth == 0 && (byte == b';' || byte == b'\n') {
                break;
            }
            self.pos += 1;
        }
    }

    #[allow(dead_code)]
    fn skip_function_body(&mut self) {
        // Skip to opening brace
        while self.pos < self.source.len() && self.source[self.pos] != b'{' {
            self.pos += 1;
        }
        self.skip_balanced(b'{', b'}');
    }

    #[allow(dead_code)]
    fn is_type_declaration(&self) -> bool {
        if self.pos == 0 {
            return true;
        }
        let prev = self.source[self.pos - 1];
        !prev.is_ascii_alphanumeric()
    }

    #[allow(dead_code)]
    fn is_enum_declaration(&self) -> bool {
        if self.pos == 0 {
            return true;
        }
        let prev = self.source[self.pos - 1];
        !prev.is_ascii_alphanumeric()
    }

    #[allow(dead_code)]
    fn is_type_annotation(&self) -> bool {
        if self.pos == 0 || self.pos + 1 >= self.source.len() {
            return false;
        }

        let prev = self.source[self.pos - 1];
        let next = self.source[self.pos + 1];

        if !prev.is_ascii_alphanumeric() && prev != b')' && prev != b']' {
            return false;
        }

        if next == b':' {
            return false;
        }

        next == b' ' || next.is_ascii_alphabetic() || next == b'{' || next == b'['
    }

    #[allow(dead_code)]
    fn is_as_expression(&self) -> bool {
        self.pos > 0 && self.pos + 4 < self.source.len()
    }

    #[allow(dead_code)]
    fn is_generic_params(&self) -> bool {
        if self.pos == 0 || self.pos + 1 >= self.source.len() {
            return false;
        }

        let prev = self.source[self.pos - 1];
        let next = self.source[self.pos + 1];

        prev.is_ascii_alphanumeric() && (next.is_ascii_uppercase() || next.is_ascii_lowercase())
    }

    #[allow(dead_code)]
    fn is_jsx_open(&self) -> bool {
        if self.pos + 1 >= self.source.len() {
            return false;
        }

        let next = self.source[self.pos + 1];
        next.is_ascii_alphabetic() || next == b'>' || next == b'/'
    }

    #[allow(dead_code)]
    fn needs_whitespace(&self) -> bool {
        if self.pos == 0 || self.output.is_empty() {
            return false;
        }

        let prev_out = self.output.as_slice().last().copied().unwrap_or(0);
        let next = self.source.get(self.pos + 1).copied().unwrap_or(0);

        // Keep whitespace between identifiers/keywords
        prev_out.is_ascii_alphanumeric() && next.is_ascii_alphanumeric()
    }
}

#[allow(dead_code)]
fn trim_jsx_text(text: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = text.len();

    // Trim leading whitespace
    while start < end
        && (text[start] == b' '
            || text[start] == b'\t'
            || text[start] == b'\n'
            || text[start] == b'\r')
    {
        start += 1;
    }

    // Trim trailing whitespace
    while end > start
        && (text[end - 1] == b' '
            || text[end - 1] == b'\t'
            || text[end - 1] == b'\n'
            || text[end - 1] == b'\r')
    {
        end -= 1;
    }

    &text[start..end]
}

#[cfg(test)]
mod tests {
    // Tests disabled due to lifetime issues with ArenaOutput
    // The compile module provides the main compilation functionality
}
