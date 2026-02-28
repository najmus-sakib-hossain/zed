//! HTML Parsing Utilities
//!
//! Fast byte-level HTML parsing for extracting class attributes and grouped calls.

/// Extract all class attributes from HTML.
///
/// Returns a vector of tuples containing:
/// - The full class attribute string (e.g., `class="flex p-4"`)
/// - The class values (e.g., `flex p-4`)
///
/// Handles both single and double quotes, escaped quotes, and UTF-8 content.
pub fn iter_class_attributes(html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        // Look for "class" keyword (ASCII, so byte indexing is safe here)
        if i + 5 <= bytes.len() && bytes[i..i + 5].eq_ignore_ascii_case(b"class") {
            let mut j = i + 5;
            // Skip whitespace (ASCII)
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'=' {
                j += 1;
                // Skip whitespace (ASCII)
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                // Handle both double and single quotes
                if j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'\'') {
                    let quote = bytes[j];
                    let val_start = j + 1;
                    let mut val_end = val_start;

                    // Find closing quote, handling escaped quotes
                    while val_end < bytes.len() {
                        if bytes[val_end] == quote {
                            // Check if it's escaped
                            if val_end > val_start && bytes[val_end - 1] == b'\\' {
                                val_end += 1;
                                continue;
                            }
                            break;
                        }
                        val_end += 1;
                    }

                    if val_end < bytes.len() {
                        // Use from_utf8_lossy to safely handle UTF-8
                        let full = String::from_utf8_lossy(&bytes[i..=val_end]).to_string();
                        let classes =
                            String::from_utf8_lossy(&bytes[val_start..val_end]).to_string();
                        out.push((full, classes));
                        i = val_end + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    out
}

/// Find grouped calls in HTML (e.g., `@Button(...)`, `@Card(...)`).
///
/// Returns a vector of tuples containing:
/// - The group name (e.g., `Button`)
/// - The inner content (e.g., `flex p-4`)
pub fn find_grouped_calls_in_text(html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let mut j = i + 1;
            while j < bytes.len() {
                let c = bytes[j];
                if c.is_ascii_uppercase()
                    || c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || c == b'_'
                    || c == b'-'
                {
                    j += 1;
                    continue;
                }
                break;
            }
            if j > i + 1 {
                let mut k = j;
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b'(' {
                    let mut depth = 0usize;
                    let mut m = k;
                    while m < bytes.len() {
                        if bytes[m] == b'(' {
                            depth += 1;
                        } else if bytes[m] == b')' {
                            depth = depth.saturating_sub(1);
                            if depth == 0 {
                                break;
                            }
                        }
                        m += 1;
                    }
                    if m < bytes.len() && bytes[m] == b')' {
                        let name = String::from_utf8_lossy(&bytes[i + 1..j]).to_string();
                        let inner = String::from_utf8_lossy(&bytes[k + 1..m]).to_string();
                        out.push((name, inner));
                        i = m + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    out
}

/// Replace grouped tokens in class strings with their aliases.
///
/// For example, `@Button(flex p-4)` becomes `Button` if `alias` is `"Button"`.
pub fn replace_grouped_tokens_in_classes(classes_str: &str, alias: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    let s = classes_str.as_bytes();
    while i < s.len() {
        if s[i] == b'@' {
            let mut j = i + 1;
            while j < s.len()
                && ((s[j] >= b'A' && s[j] <= b'Z')
                    || (s[j] >= b'a' && s[j] <= b'z')
                    || (s[j] >= b'0' && s[j] <= b'9')
                    || s[j] == b'_'
                    || s[j] == b'-')
            {
                j += 1;
            }
            let name = String::from_utf8_lossy(&s[i + 1..j]).to_string();
            if name == alias {
                let mut k = j;
                while k < s.len() && s[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k < s.len() && s[k] == b'(' {
                    let mut depth = 0usize;
                    let mut m = k;
                    while m < s.len() {
                        if s[m] == b'(' {
                            depth += 1;
                        } else if s[m] == b')' {
                            depth = depth.saturating_sub(1);
                            if depth == 0 {
                                break;
                            }
                        }
                        m += 1;
                    }
                    if m < s.len() && s[m] == b')' {
                        if !out.is_empty() && !out.ends_with(' ') {
                            out.push(' ');
                        }
                        out.push_str(alias);
                        i = m + 1;
                        while i < s.len() && s[i].is_ascii_whitespace() {
                            i += 1;
                        }
                        continue;
                    }
                }
            }
        }
        out.push(s[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_class_attributes_basic() {
        let html = r#"<div class="flex p-4">Hello</div>"#;
        let attrs = iter_class_attributes(html);
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].1, "flex p-4");
    }

    #[test]
    fn test_iter_class_attributes_single_quotes() {
        let html = r#"<div class='bg-white text-black'>Hello</div>"#;
        let attrs = iter_class_attributes(html);
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].1, "bg-white text-black");
    }

    #[test]
    fn test_iter_class_attributes_multiline() {
        let html = r#"<div class="flex
            p-4
            bg-white">Hello</div>"#;
        let attrs = iter_class_attributes(html);
        assert_eq!(attrs.len(), 1);
        assert!(attrs[0].1.contains("flex"));
        assert!(attrs[0].1.contains("p-4"));
        assert!(attrs[0].1.contains("bg-white"));
    }

    #[test]
    fn test_iter_class_attributes_utf8() {
        let html = r#"<div class="日本語 中文 한국어">Hello</div>"#;
        let attrs = iter_class_attributes(html);
        assert_eq!(attrs.len(), 1);
        assert!(attrs[0].1.contains("日本語"));
        assert!(attrs[0].1.contains("中文"));
        assert!(attrs[0].1.contains("한국어"));
    }

    #[test]
    fn test_iter_class_attributes_escaped_quotes() {
        let html = r#"<div class="test \"escaped\" class">Hello</div>"#;
        let attrs = iter_class_attributes(html);
        assert_eq!(attrs.len(), 1);
        assert!(attrs[0].1.contains("escaped"));
    }
}
