//! .sr File Generator
//!
//! Generates human-readable .sr files from extracted rules.
//! Uses dx-serializer human format for maximum readability.

use crate::rules::extractor::extract_all_rules;
use crate::rules::schema::{DxCategory, DxRule, DxSeverity, Language, RuleSource};
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Generate all .sr files in the specified directory
pub fn generate_all_sr_files(output_dir: &Path) -> Result<()> {
    println!("🔨 Generating .sr files...");

    // Extract all rules
    let db = extract_all_rules();

    // Create output directory
    fs::create_dir_all(output_dir)?;

    // Group rules by language
    let mut language_groups: std::collections::HashMap<Language, Vec<DxRule>> =
        std::collections::HashMap::new();

    for rule in &db.rules {
        language_groups.entry(rule.language).or_default().push(rule.clone());
    }

    // Generate one .sr file per language
    let mut total_rules = 0;
    for (language, rules) in &language_groups {
        let filename = format!("{}-rules.sr", language_to_prefix(*language));
        let filepath = output_dir.join(&filename);

        generate_sr_file(&filepath, *language, rules)?;

        total_rules += rules.len();
        println!("  ✅ {} ({} rules)", filename, rules.len());
    }

    println!("✨ Generated {} rules across {} languages", total_rules, language_groups.len());
    Ok(())
}

/// Generate a single .sr file for one language
fn generate_sr_file(filepath: &Path, language: Language, rules: &[DxRule]) -> Result<()> {
    let mut content = String::new();

    // Add meta section
    content.push_str(&format!("# {} Rules\n", language_to_name(language)));
    content.push_str(&format!("# Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d")));

    content.push_str("@meta\n");
    content.push_str(&format!("language: \"{}\"\n", language_to_name(language)));
    content.push_str(&format!(
        "source: \"{}\"\n",
        RuleSource::as_str(&rules.first().map_or(RuleSource::DxCheck, |r| r.source))
    ));
    content.push_str("version: \"0.1.0\"\n");
    content.push_str(&format!("total_rules: {}\n", rules.len()));
    content.push('\n');

    // Add each rule
    for rule in rules {
        content.push_str(&rule_to_sr(rule));
        content.push('\n');
    }

    fs::write(filepath, content)?;
    Ok(())
}

/// Convert a single rule to .sr format
fn rule_to_sr(rule: &DxRule) -> String {
    let mut s = String::new();

    s.push_str("@rule\n");
    s.push_str(&format!("name: \"{}\"\n", rule.name));
    s.push_str(&format!("prefixed_name: \"{}\"\n", rule.prefixed_name));
    s.push_str(&format!("category: \"{}\"\n", category_to_str(rule.category)));
    s.push_str(&format!("severity: \"{}\"\n", severity_to_str(rule.default_severity)));
    s.push_str(&format!("fixable: {}\n", rule.fixable));
    s.push_str(&format!("recommended: {}\n", rule.recommended));
    s.push_str(&format!("is_formatter: {}\n", rule.is_formatter));

    // Add description (multi-line support)
    if rule.description.contains('\n') {
        s.push_str("description: |\n");
        for line in rule.description.lines() {
            s.push_str(&format!("  {line}\n"));
        }
    } else {
        s.push_str(&format!("description: \"{}\"\n", escape_string(&rule.description)));
    }

    // Add docs_url if present
    if let Some(docs_url) = &rule.docs_url {
        s.push_str(&format!("docs_url: \"{docs_url}\"\n"));
    }

    // Add additional fields if present
    if let Some(options) = &rule.options_schema {
        s.push_str(&format!("options_schema: \"{}\"\n", escape_string(options)));
    }

    s
}

/// Convert Language enum to human-readable name
fn language_to_name(language: Language) -> &'static str {
    match language {
        Language::JavaScript => "JavaScript",
        Language::TypeScript => "TypeScript",
        Language::Python => "Python",
        Language::Go => "Go",
        Language::Rust => "Rust",
        Language::Php => "PHP",
        Language::Markdown => "Markdown",
        Language::Toml => "TOML",
        Language::Kotlin => "Kotlin",
        Language::C => "C",
        Language::Cpp => "C++",
        Language::Json => "JSON",
        Language::Css => "CSS",
        Language::Html => "HTML",
        Language::Yaml => "YAML",
        Language::Universal => "Universal",
    }
}

/// Convert Language enum to file prefix
fn language_to_prefix(language: Language) -> &'static str {
    language.prefix()
}

/// Convert `DxCategory` to string
fn category_to_str(category: DxCategory) -> &'static str {
    category.as_str()
}

/// Convert `DxSeverity` to string
fn severity_to_str(severity: DxSeverity) -> &'static str {
    match severity {
        DxSeverity::Off => "off",
        DxSeverity::Warn => "warn",
        DxSeverity::Error => "error",
    }
}

/// Escape strings for .sr format
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("hello \"world\""), "hello \\\"world\\\"");
        assert_eq!(escape_string("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_language_conversions() {
        assert_eq!(language_to_name(Language::JavaScript), "JavaScript");
        assert_eq!(language_to_prefix(Language::JavaScript), "js");
        assert_eq!(language_to_name(Language::Rust), "Rust");
        assert_eq!(language_to_prefix(Language::Rust), "rs");
    }

    #[test]
    fn test_rule_to_sr() {
        let rule = DxRule {
            rule_id: 1,
            language: Language::JavaScript,
            category: DxCategory::Correctness,
            source: RuleSource::DxCheck,
            default_severity: DxSeverity::Error,
            name: "test-rule".to_string(),
            prefixed_name: "js/test-rule".to_string(),
            description: "Test description".to_string(),
            fixable: true,
            recommended: true,
            is_formatter: false,
            docs_url: Some("https://example.com".to_string()),
            options_schema: None,
            related_rules: vec![],
            deprecated_by: None,
        };

        let sr = rule_to_sr(&rule);

        assert!(sr.contains("@rule"));
        assert!(sr.contains("name: \"test-rule\""));
        assert!(sr.contains("category: \"correctness\""));
        assert!(sr.contains("fixable: true"));
    }
}
