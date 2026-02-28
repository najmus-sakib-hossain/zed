use crate::core::engine::StyleEngine;

#[allow(dead_code)]
pub fn generate_container_group(
    engine: &StyleEngine,
    raw: &str,
    escaped_selector: &str,
) -> Option<String> {
    if !raw.starts_with("?@container>") {
        return None;
    }
    const PREFIX: &str = "?@container>";
    let after_prefix = &raw[PREFIX.len()..];
    let paren_idx = after_prefix.find('(')?;
    let size_part = after_prefix[..paren_idx].trim();
    if size_part.is_empty() {
        return None;
    }
    let inner_raw = after_prefix[paren_idx + 1..].strip_suffix(')')?;
    let size_expr = if size_part.chars().all(|c| c.is_ascii_digit()) {
        format!("{}px", size_part)
    } else {
        size_part.to_string()
    };
    let inner_utils: Vec<&str> =
        inner_raw.split(|c: char| c.is_whitespace()).filter(|s| !s.is_empty()).collect();
    if inner_utils.is_empty() {
        return None;
    }
    use ahash::AHashMap;
    let mut decls: AHashMap<String, (usize, String)> = AHashMap::new();
    let mut order: usize = 0;
    for util in &inner_utils {
        if let Some(raw_css) = engine.compute_css(util) {
            if let Some(open) = raw_css.find('{') {
                if let Some(close) = raw_css.find('}') {
                    if close > open {
                        let body = &raw_css[open + 1..close];
                        for seg in body.split(';') {
                            let seg = seg.trim();
                            if seg.is_empty() {
                                continue;
                            }
                            if let Some(colon) = seg.find(':') {
                                let prop = seg[..colon].trim().to_string();
                                let value =
                                    seg[colon + 1..].trim().trim_end_matches(';').to_string();
                                decls.insert(prop, (order, value));
                                order += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    if decls.is_empty() {
        return None;
    }
    let mut ordered: Vec<(String, (usize, String))> = decls.into_iter().collect();
    ordered.sort_by_key(|(_, (ord, _))| *ord);
    let mut body = String::new();
    for (prop, (_, val)) in ordered {
        body.push_str("    ");
        body.push_str(&prop);
        body.push_str(": ");
        body.push_str(&val);
        body.push_str(";\n");
    }
    let mut out = String::with_capacity(body.len() + escaped_selector.len() + 64);
    out.push_str("@container (min-width: ");
    out.push_str(&size_expr);
    out.push_str(") {\n  .");
    out.push_str(escaped_selector);
    out.push_str(" {\n");
    out.push_str(&body);
    out.push_str("  }\n}\n");
    Some(out)
}
