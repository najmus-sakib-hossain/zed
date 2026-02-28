use crate::core::engine::StyleEngine;
use crate::core::engine::build_block;

#[derive(Default)]
pub struct PendingAnimation {
    pub duration: String,
    pub delay: String,
    pub fill_mode: String,
    pub from: Vec<String>,
    pub via: Vec<String>,
    pub to_: Vec<String>,
    pub has_main: bool,
}

pub fn generate_animation_css(full_class: &str) -> Option<String> {
    if !full_class.starts_with("animate:") {
        return None;
    }
    let rest = &full_class[8..];
    let mut parts = rest.split(':');
    let duration = parts.next().unwrap_or("1s");
    let delay = parts.next().unwrap_or("0s");
    Some(format!("ANIM|animate|{}|{}", duration, delay))
}

pub fn resolve_animation_tokens(engine: &StyleEngine, tokens: &[String]) -> String {
    let mut decls: Vec<String> = Vec::new();
    for t in tokens {
        for piece in t.split('+') {
            let piece = piece.trim();
            if piece.is_empty() {
                continue;
            }
            if let Some(css) = engine.precompiled.get(piece) {
                decls.push(css.clone());
                continue;
            }
            if let Some(rest) = piece.strip_prefix("opacity-") {
                if let Ok(num) = rest.parse::<u32>() {
                    let val = if num >= 100 {
                        "1".to_string()
                    } else {
                        format!("{}", (num as f32) / 100.0)
                    };
                    decls.push(format!("opacity: {}", val));
                    continue;
                }
            }
            if let Some(c) = crate::core::color::generate_color_css(engine, piece) {
                decls.push(c);
                continue;
            }
            if let Some(d) = crate::core::engine::generate_dynamic_css(engine, piece) {
                decls.push(d);
                continue;
            }
        }
    }
    use ahash::AHashMap;
    let mut last_for: AHashMap<&str, usize> = AHashMap::new();
    for (i, d) in decls.iter().enumerate() {
        if let Some(idx) = d.find(':') {
            last_for.insert(d[..idx].trim(), i);
        }
    }
    let mut out = String::new();
    for (i, d) in decls.iter().enumerate() {
        if let Some(idx) = d.find(':') {
            let name = d[..idx].trim();
            if last_for.get(name) == Some(&i) {
                if !out.is_empty() {
                    out.push_str("; ");
                }
                out.push_str(d.trim().trim_end_matches(';'));
            }
        }
    }
    out
}

pub fn decode_animation_if_pending(
    engine: &StyleEngine,
    selector: &str,
    pending: &mut Option<PendingAnimation>,
    out: &mut String,
) {
    if let Some(pa) = pending.take() {
        if !pa.has_main {
            return;
        }
        let base_selector = if let Some(space_idx) = selector.find("\\ ") {
            &selector[..space_idx]
        } else {
            selector
        };
        let hash = format!("{:x}", seahash::hash(base_selector.as_bytes()));
        let mut frames: Vec<(u32, String)> = Vec::new();
        if !pa.from.is_empty() {
            frames.push((0, resolve_animation_tokens(engine, &pa.from)));
        }
        if !pa.to_.is_empty() {
            frames.push((100, resolve_animation_tokens(engine, &pa.to_)));
        }
        if !pa.via.is_empty() {
            let count = pa.via.len();
            for (i, v) in pa.via.iter().enumerate() {
                let pct = ((i + 1) as f32) / ((count + 1) as f32) * 100.0;
                frames
                    .push((pct as u32, resolve_animation_tokens(engine, std::slice::from_ref(v))));
            }
        }
        frames.sort_by_key(|(p, _)| *p);
        let mut kf_body = String::new();
        for (pct, decls) in &frames {
            let dtrim = decls.trim();
            if dtrim.is_empty() {
                continue;
            }
            let line = if dtrim.ends_with(';') {
                dtrim.to_string()
            } else {
                format!("{};", dtrim)
            };
            kf_body.push_str(&format!("  {}% {{ {} }}\n", pct, line));
        }
        if !kf_body.is_empty() {
            out.push_str("@keyframes dx-animation-");
            out.push_str(&hash);
            out.push_str(" {\n");
            out.push_str(&kf_body);
            out.push_str("}\n\n");
            let mut parts: Vec<String> = Vec::new();
            parts.push(pa.duration.clone());
            if pa.delay != "0s" {
                parts.push(pa.delay.clone());
            }
            if !pa.fill_mode.is_empty() {
                parts.push(pa.fill_mode.clone());
            }
            parts.push(format!("dx-animation-{}", hash));
            let mut filtered: Vec<String> = Vec::new();
            let mut seen_fill = false;
            for p in parts.into_iter() {
                if p.starts_with("from(") || p.starts_with("to(") || p.starts_with("via(") {
                    continue;
                }
                if p == "forwards" {
                    if seen_fill {
                        continue;
                    }
                    seen_fill = true;
                }
                filtered.push(p);
            }
            let value = filtered.join(" ");
            out.push_str(&build_block(base_selector, &format!("animation: {}", value)));
        }
    }
}

pub fn process_anim_line(line: &str, pending_anim: &mut Option<PendingAnimation>) {
    if let Some(rest) = line.strip_prefix("ANIM|") {
        let parts: Vec<&str> = rest.split('|').collect();
        if parts.is_empty() {
            return;
        }
        match parts[0] {
            "animate" => {
                let duration_val = parts.get(1).copied().unwrap_or("1s").to_string();
                let delay_val = parts.get(2).copied().unwrap_or("0s").to_string();
                let pa = pending_anim.get_or_insert(PendingAnimation {
                    duration: duration_val.clone(),
                    delay: delay_val.clone(),
                    fill_mode: String::new(),
                    from: Vec::new(),
                    via: Vec::new(),
                    to_: Vec::new(),
                    has_main: true,
                });
                pa.duration = duration_val;
                pa.delay = delay_val;
                pa.has_main = true;
            }
            "fill" => {
                if let Some(mode) = parts.get(1) {
                    let pa = pending_anim.get_or_insert(PendingAnimation {
                        duration: "1s".into(),
                        delay: "0s".into(),
                        fill_mode: String::new(),
                        from: Vec::new(),
                        via: Vec::new(),
                        to_: Vec::new(),
                        has_main: false,
                    });
                    pa.fill_mode = (*mode).to_string();
                }
            }
            "from" | "to" | "via" => {
                if let Some(tokens) = parts.get(1) {
                    let pa = pending_anim.get_or_insert(PendingAnimation {
                        duration: "1s".into(),
                        delay: "0s".into(),
                        fill_mode: String::new(),
                        from: Vec::new(),
                        via: Vec::new(),
                        to_: Vec::new(),
                        has_main: false,
                    });
                    match parts[0] {
                        "from" => pa.from.push((*tokens).to_string()),
                        "to" => pa.to_.push((*tokens).to_string()),
                        "via" => pa.via.push((*tokens).to_string()),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
