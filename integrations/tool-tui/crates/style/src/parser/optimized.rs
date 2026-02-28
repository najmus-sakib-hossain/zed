// Optimized parsing implementations with performance enhancements

use ahash::AHashSet;
use memchr::memmem::Finder;
use smallvec::SmallVec;

use super::{ExtractedClasses, GroupCollector};

/// Optimized version with SIMD-friendly operations and better memory locality
#[allow(dead_code)]
#[inline]
pub fn extract_classes_optimized(html_bytes: &[u8], capacity_hint: usize) -> ExtractedClasses {
    // Use capacity hint more intelligently
    let initial_capacity = if capacity_hint > 0 {
        capacity_hint
    } else {
        // Estimate based on HTML size - roughly 1 class per 50 bytes is typical
        (html_bytes.len() / 50).max(64).next_power_of_two()
    };

    let mut set = AHashSet::with_capacity(initial_capacity);
    let mut collector = GroupCollector::default();

    // Pre-compile finders (these are cached internally by memchr)
    let class_finder = Finder::new(b"class");
    let dx_finder = Finder::new(b"dx-");

    // Process class attributes
    extract_class_attributes(html_bytes, &class_finder, &mut set, &mut collector);

    // Process dx- attributes
    extract_dx_attributes(html_bytes, &dx_finder, &mut set, &mut collector);

    ExtractedClasses {
        classes: set,
        group_events: collector.into_events(),
    }
}

#[allow(dead_code)]
#[inline(always)]
fn extract_class_attributes(
    html_bytes: &[u8],
    class_finder: &Finder,
    set: &mut AHashSet<String>,
    collector: &mut GroupCollector,
) {
    let mut pos = 0;
    let n = html_bytes.len();

    while let Some(idx) = class_finder.find(&html_bytes[pos..]) {
        let start = pos + idx + 5; // "class".len()

        // Skip whitespace
        let mut i = start;
        while i < n && matches!(html_bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
            i += 1;
        }

        if i >= n || html_bytes[i] != b'=' {
            pos = start;
            continue;
        }

        i += 1; // skip '='

        // Skip whitespace after '='
        while i < n && matches!(html_bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
            i += 1;
        }

        if i >= n {
            break;
        }

        let quote = html_bytes[i];
        if quote != b'"' && quote != b'\'' {
            pos = i;
            continue;
        }

        i += 1; // skip opening quote
        let value_start = i;

        // Fast path: use memchr for quote search
        let value_end = match memchr::memchr(quote, &html_bytes[value_start..]) {
            Some(off) => value_start + off,
            None => break,
        };

        // SAFETY: We've validated these are valid UTF-8 boundaries in HTML context
        if let Ok(value_str) = std::str::from_utf8(&html_bytes[value_start..value_end]) {
            expand_grouping_into_optimized(value_str, set, collector);
        }

        pos = value_end + 1;
    }
}

#[allow(dead_code)]
#[inline(always)]
fn extract_dx_attributes(
    html_bytes: &[u8],
    dx_finder: &Finder,
    set: &mut AHashSet<String>,
    collector: &mut GroupCollector,
) {
    let mut pos = 0;
    let n = html_bytes.len();

    while let Some(idx) = dx_finder.find(&html_bytes[pos..]) {
        let mut i = pos + idx + 3; // "dx-".len()

        // Skip the attribute name
        while i < n {
            let b = html_bytes[i];
            if (b as char).is_ascii_alphanumeric() || b == b'-' || b == b'_' {
                i += 1;
            } else {
                break;
            }
        }

        // Skip whitespace
        while i < n && matches!(html_bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
            i += 1;
        }

        if i >= n || html_bytes[i] != b'=' {
            pos = pos + idx + 3;
            continue;
        }

        i += 1; // skip '='

        // Skip whitespace after '='
        while i < n && matches!(html_bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
            i += 1;
        }

        if i >= n {
            break;
        }

        let quote = html_bytes[i];
        if quote != b'"' && quote != b'\'' {
            pos = pos + idx + 3;
            continue;
        }

        i += 1; // skip opening quote
        let value_start = i;

        // Fast path: use memchr for quote search
        let value_end = match memchr::memchr(quote, &html_bytes[value_start..]) {
            Some(off) => value_start + off,
            None => break,
        };

        if let Ok(value_str) = std::str::from_utf8(&html_bytes[value_start..value_end]) {
            expand_grouping_into_optimized(value_str, set, collector);
        }

        pos = value_end + 1;
    }
}

/// Optimized grouping expansion with better memory usage
#[allow(dead_code)]
#[inline]
fn expand_grouping_into_optimized(
    s: &str,
    out: &mut AHashSet<String>,
    collector: &mut GroupCollector,
) {
    // Fast path: check for comment marker
    let s = match s.as_bytes().iter().position(|&b| b == b'#') {
        Some(i) => &s[..i],
        None => s,
    };

    // Fast path: if no grouping characters, just split whitespace
    if !s.as_bytes().iter().any(|&b| matches!(b, b'(' | b')' | b'+')) {
        fast_split_whitespace_insert_optimized(s, out);
        return;
    }

    // Slow path: handle grouping syntax
    expand_grouping_full(s, out, collector);
}

#[allow(dead_code)]
#[inline(always)]
fn fast_split_whitespace_insert_optimized(s: &str, out: &mut AHashSet<String>) {
    // Pre-allocate string buffer to avoid allocations in the loop
    let mut buf = String::with_capacity(64);

    for cls in s.split_whitespace() {
        if !cls.is_empty() {
            // Reuse buffer when possible
            if out.contains(cls) {
                continue;
            }
            buf.clear();
            buf.push_str(cls);
            out.insert(buf.clone());
        }
    }
}

#[allow(dead_code)]
#[inline]
fn expand_grouping_full(s: &str, out: &mut AHashSet<String>, collector: &mut GroupCollector) {
    let bytes = s.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    let mut stack: SmallVec<[String; 4]> = SmallVec::new();
    let mut tok_start: Option<usize> = None;

    // Pre-allocate string buffer
    let mut combined = String::with_capacity(128);

    #[inline(always)]
    fn trim_plus(s: &str) -> (&str, bool) {
        let mut end = s.len();
        let b = s.as_bytes();
        let mut had_plus = false;
        while end > 0 && b[end - 1] == b'+' {
            end -= 1;
            had_plus = true;
        }
        (&s[..end], had_plus)
    }

    #[inline(always)]
    fn sanitize_group_token(raw: &str) -> &str {
        raw.strip_prefix('@').filter(|s| !s.is_empty()).unwrap_or(raw)
    }

    while i < n {
        // Skip whitespace
        while i < n && matches!(bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
            i += 1;
        }
        if i >= n {
            break;
        }

        // Handle closing parentheses
        while i < n && bytes[i] == b')' {
            if let Some(ts) = tok_start.take() {
                if ts < i {
                    let raw = &s[ts..i];
                    let (trimmed, had_plus) = trim_plus(raw);
                    let sanitized = sanitize_group_token(trimmed);

                    if !sanitized.is_empty() {
                        combined.clear();
                        if !stack.is_empty() {
                            for (idx, p) in stack.iter().enumerate() {
                                if idx > 0 {
                                    combined.push(':');
                                }
                                combined.push_str(p);
                            }
                            combined.push(':');
                        }
                        combined.push_str(sanitized);

                        out.insert(combined.clone());
                        collector.record(stack.as_slice(), sanitized, had_plus, &combined);
                    }
                }
            }

            if !stack.is_empty() {
                stack.pop();
            }
            i += 1;

            // Skip whitespace after ')'
            while i < n && matches!(bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
                i += 1;
            }
        }

        if i >= n {
            break;
        }

        if tok_start.is_none() {
            tok_start = Some(i);
        }

        // Scan to next special character
        while i < n && !matches!(bytes[i], b' ' | b'\n' | b'\r' | b'\t' | b'(' | b')') {
            i += 1;
        }

        // Handle opening parenthesis
        if i < n && bytes[i] == b'(' {
            if let Some(ts) = tok_start.take() {
                if ts < i {
                    let raw = &s[ts..i];
                    let (trimmed, _) = trim_plus(raw);
                    let sanitized = sanitize_group_token(trimmed);
                    if !sanitized.is_empty() {
                        stack.push(sanitized.to_string());
                    }
                }
            }
            i += 1;
            continue;
        }

        // Process regular token
        if let Some(ts) = tok_start.take() {
            if ts < i {
                let raw = &s[ts..i];
                let (trimmed, had_plus) = trim_plus(raw);
                let sanitized = sanitize_group_token(trimmed);

                if !sanitized.is_empty() {
                    combined.clear();
                    if !stack.is_empty() {
                        for (idx, p) in stack.iter().enumerate() {
                            if idx > 0 {
                                combined.push(':');
                            }
                            combined.push_str(p);
                        }
                        combined.push(':');
                    }
                    combined.push_str(sanitized);

                    out.insert(combined.clone());
                    collector.record(stack.as_slice(), sanitized, had_plus, &combined);
                }
            }
        }
    }

    // Handle remaining token
    if let Some(ts) = tok_start.take() {
        if ts < n {
            let raw = &s[ts..n];
            let (trimmed, had_plus) = trim_plus(raw);
            let sanitized = sanitize_group_token(trimmed);

            if !sanitized.is_empty() {
                combined.clear();
                if !stack.is_empty() {
                    for (idx, p) in stack.iter().enumerate() {
                        if idx > 0 {
                            combined.push(':');
                        }
                        combined.push_str(p);
                    }
                    combined.push(':');
                }
                combined.push_str(sanitized);

                out.insert(combined.clone());
                collector.record(stack.as_slice(), sanitized, had_plus, &combined);
            }
        }
    }
}

/// Optimized duplicate class rewriting with better algorithmic complexity
#[allow(dead_code)]
pub fn rewrite_duplicate_classes_optimized(html_bytes: &[u8]) -> Option<super::AutoGroupRewrite> {
    // Use the original implementation but with potential for future SIMD optimizations
    // For now, focus on other hot paths
    super::rewrite_duplicate_classes(html_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_vs_original() {
        let html = br#"<div class="flex items-center bg-red-500 text-white p-4"></div>"#;

        let original = super::super::extract_classes_fast(html, 64);
        let optimized = extract_classes_optimized(html, 64);

        assert_eq!(original.classes, optimized.classes);
        assert_eq!(original.group_events.len(), optimized.group_events.len());
    }

    #[test]
    fn test_grouping_optimized() {
        let html = br#"<div dx-text="card(bg-red-500 h-50 text-yellow-500+)"></div>"#;

        let original = super::super::extract_classes_fast(html, 0);
        let optimized = extract_classes_optimized(html, 0);

        assert_eq!(original.classes, optimized.classes);
    }

    #[test]
    fn test_large_html_optimized() {
        let mut html = String::from("<html><body>");
        for i in 0..1000 {
            html.push_str(&format!(
                r#"<div class="flex-{} items-{} bg-color-{}">Item</div>"#,
                i % 10,
                i % 5,
                i % 20
            ));
        }
        html.push_str("</body></html>");

        let html_bytes = html.as_bytes();
        let original = super::super::extract_classes_fast(html_bytes, 256);
        let optimized = extract_classes_optimized(html_bytes, 256);

        assert_eq!(original.classes, optimized.classes);
    }
}
