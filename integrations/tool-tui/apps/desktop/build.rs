#[cfg(windows)]
fn main() {
    use std::{env, fs, path::PathBuf};

    println!("cargo:rerun-if-changed=assets/logo.png");
    generate_zed_model_catalog();

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    let ico_path = out_dir.join("dx-logo.ico");

    let resized = image::open("assets/logo.png")
        .expect("failed to read assets/logo.png")
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3);

    resized
        .save_with_format(&ico_path, image::ImageFormat::Ico)
        .expect("failed to generate .ico from logo.png");

    let rc_path = out_dir.join("dx-app-icon.rc");
    let ico_for_rc = ico_path.to_string_lossy().replace('\\', "/");
    fs::write(&rc_path, format!("1 ICON \"{}\"\n", ico_for_rc)).expect("failed to write rc file");

    let _ = embed_resource::compile(rc_path, embed_resource::NONE);
}

#[cfg(windows)]
fn generate_zed_model_catalog() {
    use std::{collections::BTreeMap, env, fs, path::PathBuf};

    // Keep the generated output stable between builds.
    #[derive(Clone, Debug, Default)]
    struct ModelInfo {
        id: String,
        name: Option<String>,
        max_tokens: Option<u64>,
        max_output_tokens: Option<u64>,
    }

    fn find_fn_block(src: &str, fn_sig_fragment: &str) -> Option<String> {
        let start = src.find(fn_sig_fragment)?;
        let src_after = &src[start..];
        let brace_start_rel = src_after.find('{')?;
        let mut i = start + brace_start_rel;
        let bytes = src.as_bytes();
        let mut depth = 0i32;
        let mut end = None;
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i + 1);
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        let end = end?;
        Some(src[start..end].to_string())
    }

    fn parse_match_string_map(block: &str) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        let mut pending_variant: Option<String> = None;

        for raw_line in block.lines() {
            let line = raw_line.trim();
            if line.starts_with("//") || line.is_empty() {
                continue;
            }

            if let Some(variant) = pending_variant.take() {
                if let Some((_, rhs)) = line.split_once('"') {
                    if let Some((value, _)) = rhs.split_once('"') {
                        out.insert(variant, value.to_string());
                        continue;
                    }
                }
                // Keep looking on the next line.
                pending_variant = Some(variant);
                continue;
            }

            if !line.contains("=>") {
                continue;
            }

            let Some((lhs, rhs)) = line.split_once("=>") else {
                continue;
            };

            let lhs = lhs.trim().trim_end_matches(',');
            let rhs = rhs.trim();

            // Extract variant name from `Self::Foo` / `Model::Foo`.
            let variant = lhs
                .split("::")
                .last()
                .unwrap_or("")
                .trim()
                .trim_end_matches(',')
                .trim_end_matches('{')
                .trim();
            if variant.is_empty() {
                continue;
            }

            if let Some((_, rhs_after_quote)) = rhs.split_once('"') {
                if let Some((value, _)) = rhs_after_quote.split_once('"') {
                    out.insert(variant.to_string(), value.to_string());
                    continue;
                }
            }

            // Handle multi-line arm like `Foo => {` then string on next line.
            if rhs.starts_with('{') || rhs.ends_with('{') {
                pending_variant = Some(variant.to_string());
            }
        }

        out
    }

    fn parse_match_u64_map(block: &str) -> BTreeMap<String, u64> {
        let mut out = BTreeMap::new();
        let mut pending_variant: Option<String> = None;

        fn parse_u64_literal(s: &str) -> Option<u64> {
            let cleaned = s
                .trim()
                .trim_end_matches(',')
                .trim_end_matches(';')
                .split_whitespace()
                .next()?
                .replace('_', "");
            cleaned.parse::<u64>().ok()
        }

        for raw_line in block.lines() {
            let line = raw_line.trim();
            if line.starts_with("//") || line.is_empty() {
                continue;
            }

            if let Some(variant) = pending_variant.take() {
                if let Some(val) = parse_u64_literal(line) {
                    out.insert(variant, val);
                    continue;
                }
                pending_variant = Some(variant);
                continue;
            }

            if !line.contains("=>") {
                continue;
            }

            let Some((lhs, rhs)) = line.split_once("=>") else {
                continue;
            };

            let lhs = lhs.trim().trim_end_matches(',');
            let rhs = rhs.trim();

            let variant = lhs
                .split("::")
                .last()
                .unwrap_or("")
                .trim()
                .trim_end_matches(',')
                .trim_end_matches('{')
                .trim();
            if variant.is_empty() {
                continue;
            }

            if let Some(val) = parse_u64_literal(rhs) {
                out.insert(variant.to_string(), val);
                continue;
            }

            if rhs.starts_with('{') || rhs.ends_with('{') {
                pending_variant = Some(variant.to_string());
            }
        }

        out
    }

    fn collect_models(
        zed_file: &PathBuf,
        id_sig: &str,
        display_sig: &str,
        max_tokens_sig: &str,
        max_out_sig: &str,
    ) -> Vec<ModelInfo> {
        let Ok(src) = fs::read_to_string(zed_file) else {
            return Vec::new();
        };

        let ids = find_fn_block(&src, id_sig)
            .map(|b| parse_match_string_map(&b))
            .unwrap_or_default();
        let names = find_fn_block(&src, display_sig)
            .map(|b| parse_match_string_map(&b))
            .unwrap_or_default();
        let max_tokens = find_fn_block(&src, max_tokens_sig)
            .map(|b| parse_match_u64_map(&b))
            .unwrap_or_default();

        // Some providers return Option<u64> or u64; we only parse numeric literals.
        let max_out = find_fn_block(&src, max_out_sig)
            .map(|b| parse_match_u64_map(&b))
            .unwrap_or_default();

        let mut out = Vec::new();
        for (variant, id) in ids {
            if variant == "Custom" {
                continue;
            }
            out.push(ModelInfo {
                id,
                name: names.get(&variant).cloned(),
                max_tokens: max_tokens.get(&variant).copied(),
                max_output_tokens: max_out.get(&variant).copied(),
            });
        }

        out
    }

    // Locate repository root (workspace root) from build script's CWD.
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(&manifest_dir)
        .to_path_buf();

    let zed_root = repo_root.join("integrations").join("zed").join("crates");

    let providers: Vec<(&str, PathBuf)> = vec![
        ("Anthropic", zed_root.join("anthropic").join("src").join("anthropic.rs")),
        ("OpenAi", zed_root.join("open_ai").join("src").join("open_ai.rs")),
        ("GoogleAi", zed_root.join("google_ai").join("src").join("google_ai.rs")),
        ("DeepSeek", zed_root.join("deepseek").join("src").join("deepseek.rs")),
        ("Mistral", zed_root.join("mistral").join("src").join("mistral.rs")),
        ("XAi", zed_root.join("x_ai").join("src").join("x_ai.rs")),
        ("Bedrock", zed_root.join("bedrock").join("src").join("models.rs")),
        ("Vercel", zed_root.join("vercel").join("src").join("vercel.rs")),
    ];

    // Make Cargo rerun build script when Zed model sources change.
    for (_, path) in &providers {
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.to_string_lossy());
        }
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    let out_file = out_dir.join("zed_model_catalog.rs");

    let mut generated = String::new();
    generated.push_str("// @generated by build.rs (from integrations/zed model sources)\n");
    generated
        .push_str("pub fn models_for_provider(kind: AiProviderKind) -> Option<Vec<AiModel>> {\n");
    generated.push_str("    match kind {\n");

    for (variant, path) in &providers {
        let models = collect_models(
            path,
            "pub fn id(&self)",
            "pub fn display_name(&self)",
            "pub fn max_token_count(&self)",
            "pub fn max_output_tokens(&self)",
        );

        if models.is_empty() {
            continue;
        }

        generated.push_str(&format!("        AiProviderKind::{} => Some(vec![\n", variant));
        for m in models {
            let name = m.name.unwrap_or_else(|| m.id.clone());
            let ctx = m.max_tokens.unwrap_or(0);
            let out_tok = m.max_output_tokens;
            generated.push_str("            AiModel {\n");
            generated.push_str(&format!("                id: \"{}\".to_string(),\n", m.id));
            generated.push_str(&format!(
                "                name: \"{}\".to_string(),\n",
                name.replace('"', "\\\"")
            ));
            generated
                .push_str(&format!("                provider: AiProviderKind::{},\n", variant));
            generated.push_str(&format!("                context_window: {},\n", ctx));
            match out_tok {
                Some(v) => generated
                    .push_str(&format!("                max_output_tokens: Some({}),\n", v)),
                None => generated.push_str("                max_output_tokens: None,\n"),
            }
            generated.push_str("            },\n");
        }
        generated.push_str("        ]),\n");
    }

    generated.push_str("        _ => None,\n");
    generated.push_str("    }\n");
    generated.push_str("}\n");

    fs::write(&out_file, generated).expect("failed to write zed_model_catalog.rs");
}

#[cfg(not(windows))]
fn main() {}
