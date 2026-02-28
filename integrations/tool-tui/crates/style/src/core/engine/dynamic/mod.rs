use crate::core::engine::StyleEngine;

pub fn generate_dynamic_css(engine: &StyleEngine, class_name: &str) -> Option<String> {
    if let Some(arg) = class_name.strip_prefix("transition(") {
        if let Some(end) = arg.find(')') {
            let dur = &arg[..end];
            let duration = if dur.is_empty() { "150ms" } else { dur };
            return Some(format!(
                "transition-property: all; transition-duration: {}; transition-timing-function: cubic-bezier(0.4,0,0.2,1)",
                duration
            ));
        }
    }
    if let Some(generators) = engine.generators.as_ref() {
        if let Some(dash) = class_name.find('-') {
            let prefix = &class_name[..dash];
            if let Some(map) = &engine.generator_map {
                if let Some(&idx) = map.get(prefix) {
                    if let Some(g) = generators.get(idx) {
                        let value_str = &class_name[dash + 1..];
                        if let Some(result) = handle_generator_value(g, value_str) {
                            return Some(result);
                        }
                    }
                }
            }
        }
        for g in generators {
            if class_name.starts_with(&format!("{}-", g.prefix)) {
                let value_str = &class_name[g.prefix.len() + 1..];
                let (value_str, is_negative) = if let Some(stripped) = value_str.strip_prefix('-') {
                    (stripped, true)
                } else {
                    (value_str, false)
                };
                let num_val_opt: Option<f32> = if value_str.is_empty() {
                    Some(1.0)
                } else if let Ok(num) = value_str.parse::<f32>() {
                    Some(num)
                } else {
                    match value_str {
                        "none" => Some(0.0),
                        "sm" => Some(0.5),
                        "" => Some(1.0),
                        "md" => Some(1.5),
                        "lg" => Some(2.0),
                        "xl" => Some(3.0),
                        "2xl" => Some(4.0),
                        "3xl" => Some(6.0),
                        "full" => None,
                        _ => None,
                    }
                };
                if g.prefix.starts_with("rounded") && value_str == "full" {
                    return Some(match g.prefix.as_str() {
                        "rounded" => "border-radius: 9999px".to_string(),
                        "rounded-t" => {
                            "border-top-left-radius: 9999px; border-top-right-radius: 9999px"
                                .to_string()
                        }
                        "rounded-r" => {
                            "border-top-right-radius: 9999px; border-bottom-right-radius: 9999px"
                                .to_string()
                        }
                        "rounded-b" => {
                            "border-bottom-right-radius: 9999px; border-bottom-left-radius: 9999px"
                                .to_string()
                        }
                        "rounded-l" => {
                            "border-top-left-radius: 9999px; border-bottom-left-radius: 9999px"
                                .to_string()
                        }
                        _ => return None,
                    });
                }
                let num_val = match num_val_opt {
                    Some(v) => v,
                    None => continue,
                };
                let final_value = num_val * g.multiplier * if is_negative { -1.0 } else { 1.0 };
                let css_value = if g.unit.is_empty() {
                    format!("{}", final_value)
                } else {
                    format!("{}{}", final_value, g.unit)
                };
                if g.property.contains(',') {
                    let parts: Vec<&str> = g.property.split(',').map(|s| s.trim()).collect();
                    let mut out = String::new();
                    for (i, p) in parts.iter().enumerate() {
                        if !p.is_empty() {
                            if i > 0 {
                                out.push(' ');
                            }
                            out.push_str(p);
                            out.push_str(": ");
                            out.push_str(&css_value);
                            out.push(';');
                        }
                    }
                    return Some(out);
                } else {
                    return Some(format!("{}: {}", g.property, css_value));
                }
            }
        }
    }
    None
}

fn handle_generator_value(
    g: &crate::core::engine::GeneratorMeta,
    raw_value: &str,
) -> Option<String> {
    let (raw_value, is_negative) = if let Some(stripped) = raw_value.strip_prefix('-') {
        (stripped, true)
    } else {
        (raw_value, false)
    };
    let num_val_opt: Option<f32> = if raw_value.is_empty() {
        Some(1.0)
    } else if let Some((a, b)) = raw_value.split_once('/') {
        if let (Ok(na), Ok(nb)) = (a.parse::<f32>(), b.parse::<f32>()) {
            Some(na / nb)
        } else {
            None
        }
    } else if let Ok(num) = raw_value.parse::<f32>() {
        Some(num)
    } else {
        match raw_value {
            "none" if g.prefix.starts_with("rounded") => Some(0.0),
            "sm" if g.prefix.starts_with("rounded") => Some(0.5),
            "md" if g.prefix.starts_with("rounded") => Some(1.5),
            "lg" if g.prefix.starts_with("rounded") => Some(2.0),
            "xl" if g.prefix.starts_with("rounded") => Some(3.0),
            "2xl" if g.prefix.starts_with("rounded") => Some(4.0),
            "3xl" if g.prefix.starts_with("rounded") => Some(6.0),
            "full" if g.prefix.starts_with("rounded") => {
                return Some(match g.prefix.as_str() {
                    "rounded" => "border-radius: 9999px".to_string(),
                    "rounded-t" => {
                        "border-top-left-radius: 9999px; border-top-right-radius: 9999px"
                            .to_string()
                    }
                    "rounded-r" => {
                        "border-top-right-radius: 9999px; border-bottom-right-radius: 9999px"
                            .to_string()
                    }
                    "rounded-b" => {
                        "border-bottom-right-radius: 9999px; border-bottom-left-radius: 9999px"
                            .to_string()
                    }
                    "rounded-l" => {
                        "border-top-left-radius: 9999px; border-bottom-left-radius: 9999px"
                            .to_string()
                    }
                    _ => return None,
                });
            }
            _ => None,
        }
    };
    let num_val = num_val_opt?;
    let final_value = num_val * g.multiplier * if is_negative { -1.0 } else { 1.0 };
    let css_value = if g.unit.is_empty() {
        format!("{}", final_value)
    } else {
        format!("{}{}", final_value, g.unit)
    };
    if g.property.contains(',') {
        let parts: Vec<&str> = g.property.split(',').map(|s| s.trim()).collect();
        let mut out = String::new();
        for (i, p) in parts.iter().enumerate() {
            if !p.is_empty() {
                if i > 0 {
                    out.push(' ');
                }
                out.push_str(p);
                out.push_str(": ");
                out.push_str(&css_value);
                out.push(';');
            }
        }
        Some(out)
    } else {
        Some(format!("{}: {}", g.property, css_value))
    }
}
