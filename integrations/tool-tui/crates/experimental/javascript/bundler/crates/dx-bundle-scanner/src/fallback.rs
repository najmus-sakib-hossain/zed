//! Scalar fallback implementations for non-SIMD platforms

use dx_bundle_core::{ScanResult, SourceSpan, TypeScriptPattern};

/// Scan source using scalar operations (fallback)
pub fn scan_scalar(source: &[u8]) -> ScanResult {
    let mut result = ScanResult::default();

    // Find imports
    for i in 0..source.len().saturating_sub(6) {
        if &source[i..i + 7] == b"import " {
            result.imports.push(i as u32);
        }
    }

    // Find exports
    for i in 0..source.len().saturating_sub(6) {
        if &source[i..i + 7] == b"export " {
            result.exports.push(i as u32);
        }
    }

    // Find JSX
    find_jsx_scalar(source, &mut result.jsx_elements);

    // Find TypeScript
    find_typescript_scalar(source, &mut result.typescript_patterns);

    // Find strings
    find_strings_scalar(source, &mut result.strings);

    // Find comments
    find_comments_scalar(source, &mut result.comments);

    result
}

/// Check if source has JSX (scalar)
pub fn has_jsx_scalar(source: &[u8]) -> bool {
    for i in 0..source.len().saturating_sub(1) {
        if source[i] == b'<' {
            let next = source[i + 1];
            // JSX: < followed by uppercase letter (component) or lowercase (element)
            if next.is_ascii_alphabetic() {
                // Make sure it's not a comparison
                if i > 0 {
                    let prev = source[i - 1];
                    if prev == b' '
                        || prev == b'('
                        || prev == b'['
                        || prev == b'{'
                        || prev == b'='
                        || prev == b','
                        || prev == b':'
                        || prev == b'\n'
                    {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }
    }
    false
}

/// Find JSX positions (scalar)
pub fn find_jsx_scalar(source: &[u8], positions: &mut Vec<u32>) {
    for i in 0..source.len().saturating_sub(1) {
        if source[i] == b'<' {
            let next = source[i + 1];
            if next.is_ascii_alphabetic() || next == b'/' || next == b'>' {
                // Verify context (not in string)
                if is_valid_jsx_position(source, i) {
                    positions.push(i as u32);
                }
            }
        }
    }
}

/// Check if source has TypeScript syntax (scalar)
pub fn has_typescript_scalar(source: &[u8]) -> bool {
    // Look for interface, type, : Type patterns
    for i in 0..source.len().saturating_sub(9) {
        if &source[i..i + 10] == b"interface " {
            return true;
        }
    }

    for i in 0..source.len().saturating_sub(4) {
        if &source[i..i + 5] == b"type " {
            // Make sure it's not "typeof"
            if i == 0 || !source[i - 1].is_ascii_alphanumeric() {
                return true;
            }
        }
    }

    // Check for type annotations (:)
    for i in 1..source.len().saturating_sub(1) {
        if source[i] == b':' {
            let prev = source[i - 1];
            let next = source[i + 1];
            // Type annotation: identifier: Type
            if prev.is_ascii_alphanumeric() && (next == b' ' || next.is_ascii_alphabetic()) {
                return true;
            }
        }
    }

    false
}

/// Find TypeScript patterns (scalar)
pub fn find_typescript_scalar(source: &[u8], patterns: &mut Vec<(u32, TypeScriptPattern)>) {
    let len = source.len();

    // Find interfaces
    for i in 0..len.saturating_sub(9) {
        if &source[i..i + 10] == b"interface " {
            patterns.push((i as u32, TypeScriptPattern::Interface));
        }
    }

    // Find type declarations
    for i in 0..len.saturating_sub(4) {
        if &source[i..i + 5] == b"type " && (i == 0 || !source[i - 1].is_ascii_alphanumeric()) {
            patterns.push((i as u32, TypeScriptPattern::TypeAlias));
        }
    }

    // Find type annotations
    for i in 1..len.saturating_sub(1) {
        if source[i] == b':' && is_type_annotation(source, i) {
            patterns.push((i as u32, TypeScriptPattern::TypeAnnotation));
        }
    }

    // Find generic parameters
    for i in 0..len.saturating_sub(1) {
        if source[i] == b'<' && is_generic_params(source, i) {
            patterns.push((i as u32, TypeScriptPattern::GenericParams));
        }
    }

    // Find as expressions
    for i in 0..len.saturating_sub(3) {
        if &source[i..i + 4] == b" as " {
            patterns.push((i as u32, TypeScriptPattern::AsExpression));
        }
    }
}

/// Find string literals (scalar)
pub fn find_strings_scalar(source: &[u8], spans: &mut Vec<SourceSpan>) {
    let mut i = 0;
    let len = source.len();

    while i < len {
        let byte = source[i];

        if byte == b'"' || byte == b'\'' || byte == b'`' {
            let start = i;
            let quote = byte;
            i += 1;

            while i < len {
                if source[i] == b'\\' && i + 1 < len {
                    i += 2;
                    continue;
                }
                if source[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }

            spans.push(SourceSpan::new(start as u32, i as u32));
        } else {
            i += 1;
        }
    }
}

/// Find comments (scalar)
pub fn find_comments_scalar(source: &[u8], spans: &mut Vec<SourceSpan>) {
    let mut i = 0;
    let len = source.len();

    while i + 1 < len {
        if source[i] == b'/' {
            if source[i + 1] == b'/' {
                let start = i;
                i += 2;
                while i < len && source[i] != b'\n' {
                    i += 1;
                }
                spans.push(SourceSpan::new(start as u32, i as u32));
            } else if source[i + 1] == b'*' {
                let start = i;
                i += 2;
                while i + 1 < len {
                    if source[i] == b'*' && source[i + 1] == b'/' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                spans.push(SourceSpan::new(start as u32, i as u32));
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
}

// ========== Helper Functions ==========

fn is_valid_jsx_position(source: &[u8], pos: usize) -> bool {
    // Quick check: not in string or comment
    if pos > 0 {
        let prev = source[pos - 1];
        // Must be after certain characters to be JSX
        if prev == b'"' || prev == b'\'' || prev == b'`' {
            return false;
        }
    }
    true
}

fn is_type_annotation(source: &[u8], pos: usize) -> bool {
    if pos == 0 || pos + 1 >= source.len() {
        return false;
    }

    let prev = source[pos - 1];
    let next = source[pos + 1];

    // Type annotation: after identifier, before type
    if !prev.is_ascii_alphanumeric() && prev != b')' && prev != b']' {
        return false;
    }

    if next == b':' {
        return false; // This is ::
    }

    // Skip whitespace
    let mut i = pos + 1;
    while i < source.len() && (source[i] == b' ' || source[i] == b'\t') {
        i += 1;
    }

    if i >= source.len() {
        return false;
    }

    let after = source[i];
    after.is_ascii_alphabetic() || after == b'{' || after == b'[' || after == b'('
}

fn is_generic_params(source: &[u8], pos: usize) -> bool {
    // Generic params: < after identifier, before type param
    if pos == 0 || pos + 1 >= source.len() {
        return false;
    }

    let prev = source[pos - 1];
    let next = source[pos + 1];

    // Must be after identifier
    if !prev.is_ascii_alphanumeric() {
        return false;
    }

    // Next must be type parameter (uppercase letter typically)
    next.is_ascii_uppercase() || next.is_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_scalar() {
        let source = b"import { foo } from 'bar';\nexport const x = 1;";
        let result = scan_scalar(source);
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.exports.len(), 1);
    }

    #[test]
    fn test_has_jsx() {
        assert!(has_jsx_scalar(b"return <div>Hello</div>"));
        assert!(has_jsx_scalar(b"return <Component />"));
        assert!(!has_jsx_scalar(b"const x = 1 < 2"));
    }

    #[test]
    fn test_has_typescript() {
        assert!(has_typescript_scalar(b"interface Foo {}"));
        assert!(has_typescript_scalar(b"type Bar = string"));
        assert!(has_typescript_scalar(b"const x: number = 1"));
        assert!(!has_typescript_scalar(b"const x = 1"));
    }
}
