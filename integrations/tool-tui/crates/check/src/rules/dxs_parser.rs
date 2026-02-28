//! .sr File Parser
//!
//! Parses .sr (DX Serializer source) rule files into `DxRule` structures.
//! Uses a simple line-based parser for the human format.

use super::schema::{DxCategory, DxRule, DxRuleDatabase, DxSeverity, Language, RuleSource};
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;

/// Parse a single .sr file
pub fn parse_sr_file(path: &Path) -> Result<Vec<DxRule>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read .sr file: {}", path.display()))?;

    parse_sr_content(&content, path)
}

/// Parse .sr content from a string
pub fn parse_sr_content(content: &str, _path: &Path) -> Result<Vec<DxRule>> {
    let mut rules = Vec::new();
    let mut current_section: Option<Section> = None;
    let mut meta = DxsMeta::default();
    let mut current_rule = DxsRule::default();
    let mut in_multiline = false;
    let mut multiline_field = String::new();
    let mut multiline_value = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Section headers
        if trimmed == "@meta" {
            current_section = Some(Section::Meta);
            continue;
        } else if trimmed == "@rule" {
            // Save previous rule if exists
            if !current_rule.name.is_empty() {
                rules.push(build_dx_rule(&current_rule, &meta, rules.len())?);
                current_rule = DxsRule::default();
            }
            current_section = Some(Section::Rule);
            continue;
        }

        // Parse fields
        if let Some(ref section) = current_section {
            if in_multiline {
                if line.starts_with("  ") {
                    // Continue multiline value
                    multiline_value.push(line.trim_start_matches("  ").to_string());
                } else {
                    // End multiline
                    let value = multiline_value.join("\n");
                    set_field(section, &multiline_field, &value, &mut meta, &mut current_rule)?;
                    multiline_field.clear();
                    multiline_value.clear();
                    in_multiline = false;

                    // Process current line
                    parse_field_line(
                        trimmed,
                        section,
                        &mut meta,
                        &mut current_rule,
                        &mut in_multiline,
                        &mut multiline_field,
                    )?;
                }
            } else {
                parse_field_line(
                    trimmed,
                    section,
                    &mut meta,
                    &mut current_rule,
                    &mut in_multiline,
                    &mut multiline_field,
                )?;
            }
        }
    }

    // Handle last multiline if any
    if in_multiline {
        let value = multiline_value.join("\n");
        set_field(
            &current_section.unwrap(),
            &multiline_field,
            &value,
            &mut meta,
            &mut current_rule,
        )?;
    }

    // Save last rule
    if !current_rule.name.is_empty() {
        rules.push(build_dx_rule(&current_rule, &meta, rules.len())?);
    }

    Ok(rules)
}

fn parse_field_line(
    line: &str,
    section: &Section,
    meta: &mut DxsMeta,
    rule: &mut DxsRule,
    in_multiline: &mut bool,
    multiline_field: &mut String,
) -> Result<()> {
    if let Some((key, value)) = line.split_once(':') {
        let key = key.trim();
        let value = value.trim();

        if value == "|" {
            // Start multiline
            *in_multiline = true;
            *multiline_field = key.to_string();
        } else {
            set_field(section, key, value, meta, rule)?;
        }
    }

    Ok(())
}

fn set_field(
    section: &Section,
    key: &str,
    value: &str,
    meta: &mut DxsMeta,
    rule: &mut DxsRule,
) -> Result<()> {
    let unquoted = value.trim_matches('"');

    match section {
        Section::Meta => match key {
            "language" => meta.language = unquoted.to_string(),
            "source" => meta.source = unquoted.to_string(),
            "version" => meta.version = unquoted.to_string(),
            "total_rules" => meta.total_rules = value.parse().unwrap_or(0),
            _ => {}
        },
        Section::Rule => match key {
            "name" => rule.name = unquoted.to_string(),
            "prefixed_name" => rule.prefixed_name = unquoted.to_string(),
            "category" => rule.category = unquoted.to_string(),
            "severity" => rule.severity = unquoted.to_string(),
            "fixable" => rule.fixable = value == "true",
            "recommended" => rule.recommended = value == "true",
            "is_formatter" => rule.is_formatter = value == "true",
            "description" => rule.description = unquoted.to_string(),
            "docs_url" => rule.docs_url = Some(unquoted.to_string()),
            "options_schema" => rule.options_schema = Some(unquoted.to_string()),
            _ => {}
        },
    }

    Ok(())
}

fn build_dx_rule(dxs_rule: &DxsRule, meta: &DxsMeta, index: usize) -> Result<DxRule> {
    let language = parse_language(&meta.language)?;
    let category = parse_category(&dxs_rule.category)?;
    let severity = parse_severity(&dxs_rule.severity)?;
    let source = parse_source(&meta.source)?;

    let rule_id = ((language as u16) << 12) | (index as u16);

    Ok(DxRule {
        rule_id,
        language,
        category,
        source,
        default_severity: severity,
        name: dxs_rule.name.clone(),
        prefixed_name: if dxs_rule.prefixed_name.is_empty() {
            format!("{}/{}", language.prefix(), dxs_rule.name)
        } else {
            dxs_rule.prefixed_name.clone()
        },
        description: dxs_rule.description.clone(),
        fixable: dxs_rule.fixable,
        recommended: dxs_rule.recommended,
        is_formatter: dxs_rule.is_formatter,
        docs_url: dxs_rule.docs_url.clone(),
        options_schema: dxs_rule.options_schema.clone(),
        deprecated_by: None,
        related_rules: vec![],
    })
}

fn parse_language(s: &str) -> Result<Language> {
    match s {
        "JavaScript" => Ok(Language::JavaScript),
        "TypeScript" => Ok(Language::TypeScript),
        "Python" => Ok(Language::Python),
        "Go" => Ok(Language::Go),
        "Rust" => Ok(Language::Rust),
        "PHP" => Ok(Language::Php),
        "Markdown" => Ok(Language::Markdown),
        "TOML" => Ok(Language::Toml),
        "Kotlin" => Ok(Language::Kotlin),
        "C" => Ok(Language::C),
        "C++" => Ok(Language::Cpp),
        "JSON" => Ok(Language::Json),
        "CSS" => Ok(Language::Css),
        "HTML" => Ok(Language::Html),
        "YAML" => Ok(Language::Yaml),
        _ => Err(anyhow!("Unknown language: {s}")),
    }
}

fn parse_category(s: &str) -> Result<DxCategory> {
    match s {
        "correctness" => Ok(DxCategory::Correctness),
        "suspicious" => Ok(DxCategory::Suspicious),
        "style" => Ok(DxCategory::Style),
        "performance" => Ok(DxCategory::Performance),
        "security" => Ok(DxCategory::Security),
        "complexity" => Ok(DxCategory::Complexity),
        "a11y" | "accessibility" => Ok(DxCategory::Accessibility),
        "imports" => Ok(DxCategory::Imports),
        "types" => Ok(DxCategory::Types),
        "docs" | "documentation" => Ok(DxCategory::Documentation),
        "deprecated" => Ok(DxCategory::Deprecated),
        "format" => Ok(DxCategory::Format),
        _ => Err(anyhow!("Unknown category: {s}")),
    }
}

fn parse_severity(s: &str) -> Result<DxSeverity> {
    match s {
        "off" => Ok(DxSeverity::Off),
        "warn" | "warning" => Ok(DxSeverity::Warn),
        "error" => Ok(DxSeverity::Error),
        _ => Err(anyhow!("Unknown severity: {s}")),
    }
}

fn parse_source(s: &str) -> Result<RuleSource> {
    match s {
        "dx-check" => Ok(RuleSource::DxCheck),
        "biome" => Ok(RuleSource::Biome),
        "oxc" => Ok(RuleSource::Oxc),
        "ruff" => Ok(RuleSource::Ruff),
        "mago" => Ok(RuleSource::Mago),
        "gofmt.rs" => Ok(RuleSource::GofmtRs),
        "gold" => Ok(RuleSource::Gold),
        "rustfmt" => Ok(RuleSource::Rustfmt),
        "clippy" => Ok(RuleSource::Clippy),
        "taplo" => Ok(RuleSource::Taplo),
        "rumdl" => Ok(RuleSource::Rumdl),
        "cpp-linter-rs" => Ok(RuleSource::CppLinter),
        "ktlint" => Ok(RuleSource::Ktlint),
        _ => Err(anyhow!("Unknown source: {s}")),
    }
}

#[derive(Debug, Default)]
struct DxsMeta {
    language: String,
    source: String,
    version: String,
    total_rules: u32,
}

#[derive(Debug, Default)]
struct DxsRule {
    name: String,
    prefixed_name: String,
    category: String,
    severity: String,
    fixable: bool,
    recommended: bool,
    is_formatter: bool,
    description: String,
    docs_url: Option<String>,
    options_schema: Option<String>,
}

#[derive(Debug)]
enum Section {
    Meta,
    Rule,
}

/// Load all .sr files from a directory
pub fn load_sr_directory(dir: &Path) -> Result<DxRuleDatabase> {
    let mut db = DxRuleDatabase::new();

    println!("📂 Loading .sr files from: {}", dir.display());

    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("sr") {
            println!("  📄 Parsing: {}", path.file_name().unwrap().to_string_lossy());
            let rules = parse_sr_file(&path)?;

            for rule in rules {
                db.add_rule(rule);
            }
        }
    }

    println!("✅ Loaded {} rules from .sr files\n", db.rule_count);
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let content = r#"
# JavaScript Rules

@meta
language: "JavaScript"
source: "biome"
version: "0.1.0"
total_rules: 1

@rule
name: "noConsole"
prefixed_name: "js/noConsole"
category: "suspicious"
severity: "warn"
fixable: false
recommended: true
is_formatter: false
description: "Disallow the use of console"
docs_url: "https://biomejs.dev/linter/rules/no-console"
"#;

        let path = Path::new("test.sr");
        let rules = parse_sr_content(content, path).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "noConsole");
        assert_eq!(rules[0].category, DxCategory::Suspicious);
        assert!(!rules[0].fixable);
        assert!(rules[0].recommended);
    }
}
