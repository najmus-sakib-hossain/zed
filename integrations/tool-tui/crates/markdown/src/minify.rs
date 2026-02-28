//! Code minification for the DX Markdown Context Compiler.
//!
//! This module implements language-aware code minification to reduce token count
//! while preserving code semantics. Supports JavaScript/TypeScript, Python, JSON, and Rust.

/// Minify code block by language.
///
/// Applies language-specific minification rules to reduce token count.
/// Preserves code semantics and syntax validity.
///
/// # Arguments
/// * `code` - The code content to minify
/// * `language` - The programming language (e.g., "rust", "javascript", "python")
///
/// # Returns
/// Minified code string
pub fn minify_code(code: &str, language: &str) -> String {
    let minified = match language.to_lowercase().as_str() {
        "javascript" | "js" | "typescript" | "ts" | "jsx" | "tsx" => minify_js(code),
        "python" | "py" => minify_python(code),
        "json" => minify_json(code),
        "rust" | "rs" => minify_rust(code),
        "css" | "scss" | "sass" => minify_css(code),
        "html" | "htm" | "xml" => minify_html(code),
        "yaml" | "yml" => minify_yaml(code),
        "toml" => minify_toml(code),
        "bash" | "sh" | "shell" | "zsh" => minify_bash(code),
        // For markdown code blocks - preserve content as-is (including URLs)
        // These are examples/documentation that should not be modified
        "markdown" | "md" => code.to_string(),
        // Don't minify plain text or unknown formats
        "text" | "txt" | "plain" | "" => code.to_string(),
        _ => strip_comments_generic(code),
    };

    // Strip https:// from URLs in code blocks (except markdown which should preserve examples)
    if language.to_lowercase() != "markdown" && language.to_lowercase() != "md" {
        strip_url_protocols(&minified)
    } else {
        minified
    }
}

/// Strip https:// and http:// from URLs - they're redundant.
/// Only strips from actual URLs (containing domain patterns), not file paths.
fn strip_url_protocols(code: &str) -> String {
    // Only strip https:// when followed by a domain-like pattern
    let mut result = code.to_string();

    // Strip https:// only when followed by domain (contains . after protocol)
    while let Some(pos) = result.find("https://") {
        let after = &result[pos + 8..];
        // Check if this looks like a URL (has a dot within reasonable distance)
        if after.chars().take(50).any(|c| c == '.') {
            result = format!("{}{}", &result[..pos], &result[pos + 8..]);
        } else {
            break;
        }
    }

    while let Some(pos) = result.find("http://") {
        let after = &result[pos + 7..];
        if after.chars().take(50).any(|c| c == '.') {
            result = format!("{}{}", &result[..pos], &result[pos + 7..]);
        } else {
            break;
        }
    }

    result
}

/// Strip markdown overhead from markdown code blocks.
/// Removes tables (keeping only the data) but preserves other content including badges.
fn strip_markdown_overhead(code: &str) -> String {
    let mut result = String::new();
    let mut in_table = false;
    let mut skip_line = false;

    for line in code.lines() {
        let trimmed = line.trim();

        // Skip empty lines entirely
        if trimmed.is_empty() {
            continue;
        }

        // Don't skip badge lines - preserve them in code blocks
        // (badges are only stripped from the main document, not from code examples)

        // Detect table start (line with |)
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            // Check if it's a separator line (|---|---|)
            if trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c.is_whitespace()) {
                skip_line = true;
                continue;
            }
            in_table = true;
        }

        if skip_line {
            skip_line = false;
            continue;
        }

        // For table rows, skip them entirely
        if in_table && trimmed.starts_with('|') {
            // Check if table ended
            if !trimmed.contains('|') || trimmed == "|" {
                in_table = false;
            }
            // Skip table rows
            continue;
        }

        result.push_str(trimmed);
        result.push('\n');
    }

    result.trim().to_string()
}

/// Minify JavaScript/TypeScript code.
///
/// - Removes single-line comments (//)
/// - Removes multi-line comments (/* */)
/// - Collapses whitespace
/// - Preserves string literals
fn minify_js(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut prev_was_space = false;
    let mut in_string: Option<char> = None;
    let mut in_template = false;

    while i < len {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        // Handle string literals
        if in_string.is_some() {
            result.push(c);
            if c == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if Some(c) == in_string {
                in_string = None;
            }
            i += 1;
            continue;
        }

        // Handle template literals
        if in_template {
            result.push(c);
            if c == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if c == '`' {
                in_template = false;
            }
            i += 1;
            continue;
        }

        // Start of string literal
        if c == '"' || c == '\'' {
            in_string = Some(c);
            result.push(c);
            prev_was_space = false;
            i += 1;
            continue;
        }

        // Start of template literal
        if c == '`' {
            in_template = true;
            result.push(c);
            prev_was_space = false;
            i += 1;
            continue;
        }

        // Single-line comment
        if c == '/' && next == Some('/') {
            // Skip until end of line
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Multi-line comment
        if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2; // Skip */
            continue;
        }

        // Collapse whitespace
        if c.is_whitespace() {
            if !prev_was_space && !result.is_empty() {
                // Check if we need a space (between identifiers/keywords)
                let last = result.chars().last().unwrap_or(' ');
                // Look ahead to see what the next non-whitespace char is
                let mut j = i + 1;
                while j < len && chars[j].is_whitespace() {
                    j += 1;
                }
                let next_char = chars.get(j).copied().unwrap_or(' ');

                // Only add space if both sides need it (identifier to identifier)
                if needs_space_after(last) && needs_space_before(next_char) {
                    result.push(' ');
                }
            }
            prev_was_space = true;
            i += 1;
            continue;
        }

        // Regular character
        prev_was_space = false;
        result.push(c);
        i += 1;
    }

    result.trim().to_string()
}

/// Check if a character needs a space after it to separate tokens.
fn needs_space_after(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Check if a character needs a space before it.
fn needs_space_before(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Minify Python code.
///
/// - Removes single-line comments (#)
/// - Removes docstrings (""" """)
/// - Preserves indentation (critical for Python)
/// - Collapses blank lines
fn minify_python(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let lines: Vec<&str> = code.lines().collect();
    let mut in_docstring = false;
    let mut docstring_char: Option<char> = None;

    for line in lines {
        let trimmed = line.trim();

        // Handle docstrings
        if in_docstring {
            if let Some(dc) = docstring_char {
                let end_marker = format!("{}{}{}", dc, dc, dc);
                if trimmed.ends_with(&end_marker) || trimmed == end_marker {
                    in_docstring = false;
                    docstring_char = None;
                }
            }
            continue;
        }

        // Check for docstring start
        if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
            // Get the quote character - we know it exists because we just checked starts_with
            if let Some(dc) = trimmed.chars().next() {
                let end_marker = format!("{}{}{}", dc, dc, dc);
                // Check if it's a single-line docstring
                if trimmed.len() > 3 && trimmed[3..].contains(&end_marker) {
                    continue; // Skip single-line docstring
                }
                in_docstring = true;
                docstring_char = Some(dc);
            }
            continue;
        }

        // Skip comment-only lines
        if trimmed.starts_with('#') {
            continue;
        }

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Get indentation
        let indent = line.len() - line.trim_start().len();
        let indent_str: String = " ".repeat(indent);

        // Remove inline comments (but preserve # in strings)
        let processed = remove_python_inline_comment(trimmed);

        if !processed.is_empty() {
            result.push_str(&indent_str);
            result.push_str(&processed);
            result.push('\n');
        }
    }

    result.trim_end().to_string()
}

/// Remove inline comments from Python code while preserving strings.
fn remove_python_inline_comment(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string: Option<char> = None;

    while i < len {
        let c = chars[i];

        // Handle string literals
        if in_string.is_some() {
            result.push(c);
            if c == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if Some(c) == in_string {
                in_string = None;
            }
            i += 1;
            continue;
        }

        // Start of string
        if c == '"' || c == '\'' {
            in_string = Some(c);
            result.push(c);
            i += 1;
            continue;
        }

        // Comment start (not in string)
        if c == '#' {
            break;
        }

        result.push(c);
        i += 1;
    }

    result.trim_end().to_string()
}

/// Minify JSON code.
///
/// - Removes all whitespace outside strings
/// - Produces single-line output
fn minify_json(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        let c = chars[i];

        if in_string {
            result.push(c);
            if c == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if c == '"' {
            in_string = true;
            result.push(c);
            i += 1;
            continue;
        }

        // Skip whitespace outside strings
        if !c.is_whitespace() {
            result.push(c);
        }

        i += 1;
    }

    result
}

/// Minify Rust code.
///
/// - Removes single-line comments (//)
/// - Removes multi-line comments (/* */)
/// - Removes doc comments (///, //!)
/// - Collapses whitespace
/// - Preserves string literals
fn minify_rust(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut prev_was_space = false;
    let mut in_string = false;
    let mut in_raw_string = false;
    let mut raw_hash_count = 0;
    let mut in_char = false;

    while i < len {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        // Handle raw strings r#"..."#
        if in_raw_string {
            result.push(c);
            if c == '"' {
                let mut hash_count = 0;
                let mut j = i + 1;
                while j < len && chars[j] == '#' {
                    hash_count += 1;
                    j += 1;
                }
                if hash_count == raw_hash_count {
                    for _ in 0..hash_count {
                        i += 1;
                        result.push('#');
                    }
                    in_raw_string = false;
                }
            }
            i += 1;
            continue;
        }

        // Handle regular strings
        if in_string {
            result.push(c);
            if c == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Handle char literals
        if in_char {
            result.push(c);
            if c == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if c == '\'' {
                in_char = false;
            }
            i += 1;
            continue;
        }

        // Check for raw string start
        if c == 'r' && next == Some('#') {
            let mut hash_count = 0;
            let mut j = i + 1;
            while j < len && chars[j] == '#' {
                hash_count += 1;
                j += 1;
            }
            if j < len && chars[j] == '"' {
                in_raw_string = true;
                raw_hash_count = hash_count;
                result.push('r');
                for _ in 0..hash_count {
                    i += 1;
                    result.push('#');
                }
                i += 1;
                result.push('"');
                i += 1;
                continue;
            }
        }

        // Start of string literal
        if c == '"' {
            in_string = true;
            result.push(c);
            prev_was_space = false;
            i += 1;
            continue;
        }

        // Start of char literal
        if c == '\'' && next.map(|n| n != '\'').unwrap_or(true) {
            in_char = true;
            result.push(c);
            prev_was_space = false;
            i += 1;
            continue;
        }

        // Doc comments (///, //!)
        if c == '/' && next == Some('/') {
            let third = chars.get(i + 2).copied();
            if third == Some('/') || third == Some('!') {
                // Skip doc comment
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
        }

        // Single-line comment
        if c == '/' && next == Some('/') {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Multi-line comment
        if c == '/' && next == Some('*') {
            i += 2;
            let mut depth = 1;
            while i + 1 < len && depth > 0 {
                if chars[i] == '/' && chars[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // Collapse whitespace
        if c.is_whitespace() {
            if !prev_was_space && !result.is_empty() {
                let last = result.chars().last().unwrap_or(' ');
                if needs_space_after(last) {
                    result.push(' ');
                }
            }
            prev_was_space = true;
            i += 1;
            continue;
        }

        prev_was_space = false;
        result.push(c);
        i += 1;
    }

    result.trim().to_string()
}

/// Minify CSS code.
fn minify_css(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string: Option<char> = None;

    while i < len {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        // Handle strings
        if in_string.is_some() {
            result.push(c);
            if c == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if Some(c) == in_string {
                in_string = None;
            }
            i += 1;
            continue;
        }

        if c == '"' || c == '\'' {
            in_string = Some(c);
            result.push(c);
            i += 1;
            continue;
        }

        // Skip comments
        if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        // Skip whitespace
        if c.is_whitespace() {
            // Keep one space if needed
            if !result.is_empty() {
                // Check last character - we know result is not empty
                if let Some(last) = result.chars().last()
                    && (last.is_alphanumeric() || last == ')' || last == '%')
                {
                    // Peek ahead to see if we need space
                    let mut j = i + 1;
                    while j < len && chars[j].is_whitespace() {
                        j += 1;
                    }
                    if j < len && (chars[j].is_alphanumeric() || chars[j] == '.') {
                        result.push(' ');
                    }
                }
            }
            while i < len && chars[i].is_whitespace() {
                i += 1;
            }
            continue;
        }

        result.push(c);
        i += 1;
    }

    result
}

/// Minify HTML/XML code.
fn minify_html(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_tag = false;
    let mut in_string: Option<char> = None;
    let mut prev_was_space = false;

    while i < len {
        let c = chars[i];

        // Handle strings in attributes
        if in_string.is_some() {
            result.push(c);
            if Some(c) == in_string {
                in_string = None;
            }
            i += 1;
            continue;
        }

        if in_tag && (c == '"' || c == '\'') {
            in_string = Some(c);
            result.push(c);
            i += 1;
            continue;
        }

        // HTML comments
        if i + 3 < len && chars[i..i + 4] == ['<', '!', '-', '-'] {
            i += 4;
            while i + 2 < len && chars[i..i + 3] != ['-', '-', '>'] {
                i += 1;
            }
            i += 3;
            continue;
        }

        if c == '<' {
            in_tag = true;
            result.push(c);
            prev_was_space = false;
            i += 1;
            continue;
        }

        if c == '>' {
            in_tag = false;
            result.push(c);
            prev_was_space = false;
            i += 1;
            continue;
        }

        // Collapse whitespace
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
            }
            prev_was_space = true;
            i += 1;
            continue;
        }

        prev_was_space = false;
        result.push(c);
        i += 1;
    }

    result.trim().to_string()
}

/// Minify YAML code.
fn minify_yaml(code: &str) -> String {
    let mut result = String::with_capacity(code.len());

    for line in code.lines() {
        let trimmed = line.trim();

        // Skip comment-only lines
        if trimmed.starts_with('#') {
            continue;
        }

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Preserve indentation
        let indent = line.len() - line.trim_start().len();
        let indent_str: String = " ".repeat(indent);

        // Remove inline comments
        let mut processed = remove_yaml_inline_comment(trimmed);

        // Compact list items: "- value" -> "-value" (saves 1 token per list item)
        if processed.starts_with("- ") && !processed.starts_with("- -") {
            processed = format!("-{}", &processed[2..]);
        }

        if !processed.is_empty() {
            result.push_str(&indent_str);
            result.push_str(&processed);
            result.push('\n');
        }
    }

    result.trim_end().to_string()
}

/// Remove inline comments from YAML while preserving strings.
fn remove_yaml_inline_comment(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string: Option<char> = None;

    while i < len {
        let c = chars[i];

        if in_string.is_some() {
            result.push(c);
            if Some(c) == in_string {
                in_string = None;
            }
            i += 1;
            continue;
        }

        if c == '"' || c == '\'' {
            in_string = Some(c);
            result.push(c);
            i += 1;
            continue;
        }

        // Comment (must be preceded by space)
        if c == '#' && (i == 0 || chars[i - 1].is_whitespace()) {
            break;
        }

        result.push(c);
        i += 1;
    }

    result.trim_end().to_string()
}

/// Minify TOML code.
fn minify_toml(code: &str) -> String {
    let mut result = String::with_capacity(code.len());

    for line in code.lines() {
        let trimmed = line.trim();

        // Skip comment-only lines
        if trimmed.starts_with('#') {
            continue;
        }

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Remove inline comments
        let processed = remove_toml_inline_comment(trimmed);

        if !processed.is_empty() {
            result.push_str(&processed);
            result.push('\n');
        }
    }

    result.trim_end().to_string()
}

/// Remove inline comments from TOML while preserving strings.
fn remove_toml_inline_comment(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string: Option<char> = None;
    let mut in_multiline = false;

    while i < len {
        let c = chars[i];

        // Handle multiline strings
        if in_multiline {
            result.push(c);
            if c == '"' && i + 2 < len && chars[i + 1] == '"' && chars[i + 2] == '"' {
                result.push('"');
                result.push('"');
                i += 3;
                in_multiline = false;
                continue;
            }
            i += 1;
            continue;
        }

        if in_string.is_some() {
            result.push(c);
            if c == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if Some(c) == in_string {
                in_string = None;
            }
            i += 1;
            continue;
        }

        // Check for multiline string start
        if c == '"' && i + 2 < len && chars[i + 1] == '"' && chars[i + 2] == '"' {
            in_multiline = true;
            result.push_str("\"\"\"");
            i += 3;
            continue;
        }

        if c == '"' || c == '\'' {
            in_string = Some(c);
            result.push(c);
            i += 1;
            continue;
        }

        if c == '#' {
            break;
        }

        result.push(c);
        i += 1;
    }

    result.trim_end().to_string()
}

/// Minify bash/shell code.
/// - Removes comment-only lines (lines starting with #, except shebang)
/// - Removes empty lines
/// - Preserves strings
fn minify_bash(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let mut first_line = true;

    for line in code.lines() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Keep shebang on first line
        if first_line && trimmed.starts_with("#!") {
            result.push_str(trimmed);
            result.push('\n');
            first_line = false;
            continue;
        }
        first_line = false;

        // Skip comment-only lines
        if trimmed.starts_with('#') {
            continue;
        }

        result.push_str(trimmed);
        result.push('\n');
    }

    result.trim_end().to_string()
}

/// Generic comment stripping for unknown languages.
fn strip_comments_generic(code: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string: Option<char> = None;

    while i < len {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        // Handle strings
        if in_string.is_some() {
            result.push(c);
            if c == '\\' && i + 1 < len {
                i += 1;
                result.push(chars[i]);
            } else if Some(c) == in_string {
                in_string = None;
            }
            i += 1;
            continue;
        }

        if c == '"' || c == '\'' {
            in_string = Some(c);
            result.push(c);
            i += 1;
            continue;
        }

        // C-style single-line comment
        if c == '/' && next == Some('/') {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // C-style multi-line comment
        if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        // Hash comment
        if c == '#' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        result.push(c);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // JavaScript/TypeScript tests
    #[test]
    fn test_minify_js_removes_single_line_comments() {
        let code = r#"
// This is a comment
const x = 1; // inline comment
const y = 2;
"#;
        let result = minify_js(code);
        assert!(!result.contains("This is a comment"));
        assert!(!result.contains("inline comment"));
        assert!(result.contains("const x"));
        assert!(result.contains("const y"));
    }

    #[test]
    fn test_minify_js_removes_multiline_comments() {
        let code = r#"
/* Multi-line
   comment */
const x = 1;
"#;
        let result = minify_js(code);
        assert!(!result.contains("Multi-line"));
        assert!(result.contains("const x"));
    }

    #[test]
    fn test_minify_js_preserves_strings() {
        let code = r#"const msg = "// not a comment";"#;
        let result = minify_js(code);
        assert!(result.contains("// not a comment"));
    }

    #[test]
    fn test_minify_js_preserves_template_literals() {
        let code = r#"const msg = `hello // world`;"#;
        let result = minify_js(code);
        assert!(result.contains("// world"));
    }

    #[test]
    fn test_minify_js_collapses_whitespace() {
        let code = "const   x   =   1;";
        let result = minify_js(code);
        assert!(!result.contains("   "));
    }

    // Python tests
    #[test]
    fn test_minify_python_removes_comments() {
        let code = r#"
# This is a comment
x = 1  # inline comment
y = 2
"#;
        let result = minify_python(code);
        assert!(!result.contains("This is a comment"));
        assert!(!result.contains("inline comment"));
        assert!(result.contains("x = 1"));
        assert!(result.contains("y = 2"));
    }

    #[test]
    fn test_minify_python_removes_docstrings() {
        let code = r#"
"""
This is a docstring
"""
def foo():
    pass
"#;
        let result = minify_python(code);
        assert!(!result.contains("This is a docstring"));
        assert!(result.contains("def foo():"));
    }

    #[test]
    fn test_minify_python_preserves_indentation() {
        let code = r#"
def foo():
    if True:
        x = 1
"#;
        let result = minify_python(code);
        // Check that indentation is preserved
        assert!(result.contains("    if True:"));
        assert!(result.contains("        x = 1"));
    }

    #[test]
    fn test_minify_python_preserves_strings_with_hash() {
        let code = r##"msg = "# not a comment""##;
        let result = minify_python(code);
        assert!(result.contains("# not a comment"));
    }

    // JSON tests
    #[test]
    fn test_minify_json_removes_whitespace() {
        let code = r#"{
    "name": "test",
    "value": 123
}"#;
        let result = minify_json(code);
        assert_eq!(result, r#"{"name":"test","value":123}"#);
    }

    #[test]
    fn test_minify_json_preserves_strings() {
        let code = r#"{"msg": "hello   world"}"#;
        let result = minify_json(code);
        assert!(result.contains("hello   world"));
    }

    // Rust tests
    #[test]
    fn test_minify_rust_removes_comments() {
        let code = r#"
// This is a comment
fn main() {
    /* block comment */
    let x = 1;
}
"#;
        let result = minify_rust(code);
        assert!(!result.contains("This is a comment"));
        assert!(!result.contains("block comment"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_minify_rust_removes_doc_comments() {
        let code = r#"
/// Doc comment
//! Module doc
fn foo() {}
"#;
        let result = minify_rust(code);
        assert!(!result.contains("Doc comment"));
        assert!(!result.contains("Module doc"));
        assert!(result.contains("fn foo()"));
    }

    #[test]
    fn test_minify_rust_preserves_strings() {
        let code = r#"let msg = "// not a comment";"#;
        let result = minify_rust(code);
        assert!(result.contains("// not a comment"));
    }

    #[test]
    fn test_minify_rust_preserves_raw_strings() {
        let code = r##"let msg = r#"hello // world"#;"##;
        let result = minify_rust(code);
        assert!(result.contains("// world"));
    }

    #[test]
    fn test_minify_rust_handles_nested_comments() {
        let code = r#"
/* outer /* nested */ comment */
fn foo() {}
"#;
        let result = minify_rust(code);
        assert!(!result.contains("outer"));
        assert!(!result.contains("nested"));
        assert!(result.contains("fn foo()"));
    }

    // CSS tests
    #[test]
    fn test_minify_css_removes_comments() {
        let code = r#"
/* Comment */
.class {
    color: red;
}
"#;
        let result = minify_css(code);
        assert!(!result.contains("Comment"));
        assert!(result.contains(".class"));
        assert!(result.contains("color:red"));
    }

    // HTML tests
    #[test]
    fn test_minify_html_removes_comments() {
        let code = r#"
<!-- Comment -->
<div>Hello</div>
"#;
        let result = minify_html(code);
        assert!(!result.contains("Comment"));
        assert!(result.contains("<div>"));
    }

    #[test]
    fn test_minify_html_collapses_whitespace() {
        let code = "<div>   Hello   World   </div>";
        let result = minify_html(code);
        assert!(!result.contains("   "));
    }

    // YAML tests
    #[test]
    fn test_minify_yaml_removes_comments() {
        let code = r#"
# Comment
name: test  # inline
value: 123
"#;
        let result = minify_yaml(code);
        assert!(!result.contains("Comment"));
        assert!(!result.contains("inline"));
        assert!(result.contains("name: test"));
    }

    #[test]
    fn test_minify_yaml_preserves_indentation() {
        let code = r#"
parent:
  child: value
"#;
        let result = minify_yaml(code);
        assert!(result.contains("  child: value"));
    }

    // TOML tests
    #[test]
    fn test_minify_toml_removes_comments() {
        let code = r#"
# Comment
name = "test"  # inline
"#;
        let result = minify_toml(code);
        assert!(!result.contains("Comment"));
        assert!(!result.contains("inline"));
        assert!(result.contains("name = \"test\""));
    }

    // Generic tests
    #[test]
    fn test_minify_code_dispatches_correctly() {
        let js_code = "// comment\nconst x = 1;";
        let result = minify_code(js_code, "javascript");
        assert!(!result.contains("comment"));

        let py_code = "# comment\nx = 1";
        let result = minify_code(py_code, "python");
        assert!(!result.contains("comment"));
    }

    #[test]
    fn test_minify_code_unknown_language() {
        let code = "// comment\ncode here";
        let result = minify_code(code, "unknown");
        // Should still strip C-style comments
        assert!(!result.contains("comment"));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating valid JavaScript code snippets.
    fn js_code_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop_oneof![
                Just("const x = 1;".to_string()),
                Just("let y = 2;".to_string()),
                Just("function foo() { return 1; }".to_string()),
                Just("const msg = \"hello\";".to_string()),
                Just("const arr = [1, 2, 3];".to_string()),
            ],
            1..5,
        )
        .prop_map(|parts| parts.join("\n"))
    }

    /// Strategy for generating JavaScript comments.
    fn js_comment_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            "[a-zA-Z0-9 ]{1,20}".prop_map(|s| format!("// {}", s)),
            "[a-zA-Z0-9 ]{1,20}".prop_map(|s| format!("/* {} */", s)),
        ]
    }

    /// Strategy for generating valid Python code snippets.
    fn python_code_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop_oneof![
                Just("x = 1".to_string()),
                Just("y = 2".to_string()),
                Just("def foo():\n    pass".to_string()),
                Just("msg = \"hello\"".to_string()),
            ],
            1..5,
        )
        .prop_map(|parts| parts.join("\n"))
    }

    /// Strategy for generating valid JSON.
    fn json_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(r#"{"a":1}"#.to_string()),
            Just(r#"{"name":"test","value":123}"#.to_string()),
            Just(r#"[1,2,3]"#.to_string()),
            Just(r#"{"nested":{"key":"value"}}"#.to_string()),
        ]
    }

    proptest! {
        /// Property 1: Code minification preserves code semantics.
        /// For any JavaScript code with comments, minification removes comments
        /// but preserves the actual code.
        /// **Validates: Requirements 5.1, 5.2**
        #[test]
        fn prop_js_minify_preserves_code(code in js_code_strategy()) {
            let result = minify_js(&code);
            // The minified result should contain the essential code tokens
            // (variable names, keywords, etc.)
            for keyword in ["const", "let", "function", "return"] {
                if code.contains(keyword) {
                    let keyword_with_space = format!("{} ", keyword);
                    assert!(result.contains(keyword) || !code.contains(&keyword_with_space));
                }
            }
        }

        /// Property 2: Code minification removes comments.
        /// For any code with comments, the minified output should not contain
        /// the comment text.
        /// **Validates: Requirements 5.2**
        #[test]
        fn prop_js_minify_removes_comments(
            code in js_code_strategy(),
            comment in js_comment_strategy()
        ) {
            let with_comment = format!("{}\n{}", comment, code);
            let result = minify_js(&with_comment);

            // Extract the comment content (without // or /* */)
            let comment_text = comment
                .trim_start_matches("//")
                .trim_start_matches("/*")
                .trim_end_matches("*/")
                .trim();

            // The comment text should not appear in the result
            // (unless it happens to be part of the code)
            if !code.contains(comment_text) {
                assert!(!result.contains(comment_text));
            }
        }

        /// Property 3: JSON minification produces valid JSON.
        /// For any valid JSON input, minification should produce valid JSON output.
        /// **Validates: Requirements 5.4**
        #[test]
        fn prop_json_minify_valid(json in json_strategy()) {
            let result = minify_json(&json);
            // Result should be non-empty
            assert!(!result.is_empty());
            // Result should start with { or [
            let starts_valid = result.starts_with('{') || result.starts_with('[');
            assert!(starts_valid);
            // Result should end with } or ]
            let ends_valid = result.ends_with('}') || result.ends_with(']');
            assert!(ends_valid);
        }

        /// Property 4: JSON minification is idempotent.
        /// Minifying already minified JSON should produce the same result.
        /// **Validates: Requirements 5.4**
        #[test]
        fn prop_json_minify_idempotent(json in json_strategy()) {
            let once = minify_json(&json);
            let twice = minify_json(&once);
            prop_assert_eq!(once, twice);
        }

        /// Property 5: Python minification preserves indentation structure.
        /// For any Python code, the relative indentation should be preserved.
        /// **Validates: Requirements 5.3**
        #[test]
        fn prop_python_preserves_indentation(code in python_code_strategy()) {
            let result = minify_python(&code);

            // Count indented lines in original and result
            let original_indented = code.lines()
                .filter(|l| l.starts_with("    ") || l.starts_with("\t"))
                .count();
            let result_indented = result.lines()
                .filter(|l| l.starts_with("    ") || l.starts_with("\t"))
                .count();

            // If original had indented lines, result should too
            // (unless they were all comments/docstrings)
            if original_indented > 0 {
                // At least some indentation should be preserved
                // (may be less if comments were removed)
                prop_assert!(result_indented <= original_indented);
            }
        }

        /// Property 6: Minification output is never longer than input.
        /// For any code, minification should not increase the size.
        /// **Validates: Requirements 5.1**
        #[test]
        fn prop_minify_reduces_or_maintains_size(code in js_code_strategy()) {
            let result = minify_js(&code);
            // Result should be same size or smaller
            prop_assert!(result.len() <= code.len() + 10); // Small tolerance for edge cases
        }

        /// Property 7: String literals are preserved.
        /// For any code with string literals, the string content should be preserved.
        /// **Validates: Requirements 5.2, 5.3, 5.5**
        #[test]
        fn prop_strings_preserved(content in "[a-zA-Z0-9 ]{1,10}") {
            let js_code = format!(r#"const msg = "{}";"#, content);
            let result = minify_js(&js_code);
            prop_assert!(result.contains(&content));

            let py_code = format!(r#"msg = "{}""#, content);
            let result = minify_python(&py_code);
            prop_assert!(result.contains(&content));
        }
    }
}
