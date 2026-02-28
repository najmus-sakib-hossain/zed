use ahash::AHashMap;

pub fn build_block(selector: &str, declarations: &str) -> String {
    let decl_raw = declarations.trim().trim_end_matches(';').trim();
    let mut seen: AHashMap<&str, usize> = AHashMap::new();
    let parts: Vec<&str> = if decl_raw.is_empty() {
        Vec::new()
    } else if decl_raw.contains(';') {
        decl_raw.split(';').collect()
    } else {
        vec![decl_raw]
    };
    for (i, p) in parts.iter().enumerate() {
        if let Some(idx) = p.find(':') {
            seen.insert(p[..idx].trim(), i);
        }
    }
    let mut s = String::with_capacity(selector.len() + decl_raw.len() + 16);
    s.push_str(selector);
    s.push_str(" {\n");
    for (i, p) in parts.iter().enumerate() {
        let pt = p.trim();
        if pt.is_empty() {
            continue;
        }
        let name = pt.split(':').next().unwrap_or("").trim();
        if seen.get(name) == Some(&i) {
            s.push_str("  ");
            s.push_str(pt.trim_end_matches(';'));
            s.push_str(";\n");
        }
    }
    s.push_str("}\n");
    s
}

pub fn sanitize_declarations(input: &str) -> String {
    let mut out = input.trim().to_string();
    while out.ends_with(";;") {
        out.pop();
    }
    out
}

pub fn wrap_media_queries(mut css_body: String, media_queries: &[String]) -> String {
    for mq in media_queries.iter().rev() {
        let mut wrapped = String::new();
        wrapped.push_str(mq);
        wrapped.push_str(" {\n");
        for line in css_body.trim_end().lines() {
            if line.is_empty() {
                continue;
            }
            wrapped.push_str("  ");
            wrapped.push_str(line);
            wrapped.push('\n');
        }
        wrapped.push_str("}\n");
        css_body = wrapped;
    }
    if !css_body.ends_with('\n') {
        css_body.push('\n');
    }
    css_body
}
