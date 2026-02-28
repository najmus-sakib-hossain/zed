//! Rule Compiler
//!
//! Compiles extracted rules into binary format using dx-serializer.
//! Generates both:
//! - `rules.dxm` (machine format) - 0.70ns field access
//! - `rules.dx` (LLM format) - human-readable for contributors
//!
//! Can load rules from either:
//! - Direct extraction from submodules (`extract_all_rules`)
//! - .sr files (`load_from_sr_files`)

use super::dxs_parser::load_sr_directory;
use super::extractor::extract_all_rules;
use super::schema::{DxRuleDatabase, Language};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Compiled binary rule format metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct CompiledRules {
    /// Binary format version
    pub version: u32,
    /// Total rule count
    pub count: u32,
    /// Compilation timestamp
    pub compiled_at: String,
    /// Total binary size
    pub binary_size: u64,
    /// Rule database
    pub database: DxRuleDatabase,
}

/// Compile all rules to binary format (from .sr files)
pub fn compile_rules<P: AsRef<Path>>(output_dir: P) -> Result<CompiledRules> {
    // Default to .sr files - no submodules needed
    let sr_dir = Path::new("rules");
    if sr_dir.exists() {
        compile_from_sr(sr_dir, output_dir)
    } else {
        anyhow::bail!(
            "No .sr rules found. Expected rules directory at: {:?}\n\
             Run: dx-check rule generate --output rules",
            sr_dir.canonicalize().unwrap_or_else(|_| sr_dir.to_path_buf())
        )
    }
}

/// Compile rules from .sr files
pub fn compile_from_sr<P: AsRef<Path>, Q: AsRef<Path>>(
    sr_dir: P,
    output_dir: Q,
) -> Result<CompiledRules> {
    compile_rules_with_source_path(sr_dir, output_dir, RuleSource::SrFiles)
}

/// Rule loading source
enum RuleSource {
    Extraction,
    SrFiles,
}

fn compile_rules_with_source<P: AsRef<Path>>(
    output_dir: P,
    source: RuleSource,
) -> Result<CompiledRules> {
    compile_rules_with_source_path(&output_dir, &output_dir, source)
}

fn compile_rules_with_source_path<P: AsRef<Path>, Q: AsRef<Path>>(
    source_dir: P,
    output_dir: Q,
    source: RuleSource,
) -> Result<CompiledRules> {
    let source_dir = source_dir.as_ref();
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    println!("🔨 Compiling dx-check rules...\n");

    // Step 1: Load rules based on source
    let mut database = match source {
        RuleSource::Extraction => {
            println!("📦 Extracting rules from submodules...\n");
            extract_all_rules()
        }
        RuleSource::SrFiles => {
            println!("📂 Loading rules from .sr files...\n");
            load_sr_directory(source_dir)?
        }
    };

    // Step 2: Update metadata
    database.stats.compiled_at = Some(chrono::Utc::now().to_rfc3339());

    println!("\n📊 Compilation Summary:");
    println!("  Total rules:        {}", database.rule_count);
    println!("  Fixable:            {}", database.stats.fixable_count);
    println!("  Recommended:        {}", database.stats.recommended_count);
    println!("  Format rules:       {}", database.stats.format_rule_count);
    println!("  Lint rules:         {}", database.stats.lint_rule_count);
    println!("\n  Rules by language:");
    for (lang, count) in &database.stats.rules_per_language {
        println!("    {lang:<15} {count}");
    }
    println!("\n  Rules by source:");
    for (source, count) in &database.stats.rules_per_source {
        println!("    {source:<15} {count}");
    }

    // Step 3: Serialize to LLM format (human-readable .dx file)
    let llm_path = output_dir.join("rules.dx");
    let llm_content = serialize_to_llm(&database)?;
    fs::write(&llm_path, llm_content).context("Failed to write rules.dx")?;
    println!("\n✅ Wrote LLM format: {}", llm_path.display());

    // Step 4: Serialize to Machine format (binary .dxm file)
    let machine_path = output_dir.join("rules.dxm");
    let binary_data = serialize_to_binary(&database)?;
    fs::write(&machine_path, &binary_data).context("Failed to write rules.dxm")?;

    let binary_size = binary_data.len() as u64;
    database.stats.binary_size_bytes = binary_size;

    println!("✅ Wrote Machine format: {} ({} bytes)", machine_path.display(), binary_size);

    // Step 5: Write metadata JSON for debugging
    let metadata_path = output_dir.join("rules-metadata.json");
    let metadata = serde_json::to_string_pretty(&database.stats)?;
    fs::write(&metadata_path, metadata)?;
    println!("✅ Wrote metadata: {}", metadata_path.display());

    println!("\n🎉 Compilation complete!");
    println!("   Binary size: {} KB", binary_size / 1024);
    println!(
        "   Rules per KB: {}",
        f64::from(database.rule_count) / (binary_size as f64 / 1024.0)
    );

    Ok(CompiledRules {
        version: DxRuleDatabase::VERSION,
        count: database.rule_count,
        compiled_at: database.stats.compiled_at.clone().unwrap(),
        binary_size,
        database,
    })
}

/// Serialize database to DX LLM format (human-readable)
fn serialize_to_llm(database: &DxRuleDatabase) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str("# DX Check Rules Database\n");
    output.push_str("# Auto-generated - DO NOT EDIT MANUALLY\n");
    output.push_str("# To modify rules, edit the extractors and run: dx-check rules compile\n");
    output.push_str(&format!("# Version: {}\n", database.version));
    output.push_str(&format!("# Total Rules: {}\n", database.rule_count));
    if let Some(ref timestamp) = database.stats.compiled_at {
        output.push_str(&format!("# Compiled: {timestamp}\n"));
    }
    output.push('\n');

    // Group rules by language
    let languages = [
        Language::JavaScript,
        Language::TypeScript,
        Language::Python,
        Language::Go,
        Language::Rust,
        Language::Php,
        Language::Markdown,
        Language::Toml,
        Language::Kotlin,
        Language::C,
        Language::Cpp,
        Language::Json,
        Language::Css,
    ];

    for language in languages {
        let rules = database.get_by_language(language);
        if rules.is_empty() {
            continue;
        }

        output.push_str(&format!(
            "## {} Rules ({})\n\n",
            language.prefix().to_uppercase(),
            rules.len()
        ));

        for rule in rules {
            output.push_str(&format!("### {}\n\n", rule.prefixed_name));
            output.push_str(&format!("- **ID**: {}\n", rule.rule_id));
            output.push_str(&format!("- **Category**: {}\n", rule.category.as_str()));
            output.push_str(&format!("- **Source**: {}\n", rule.source.as_str()));
            output.push_str(&format!("- **Severity**: {:?}\n", rule.default_severity));
            output.push_str(&format!("- **Fixable**: {}\n", rule.fixable));
            output.push_str(&format!("- **Recommended**: {}\n", rule.recommended));
            if rule.is_formatter {
                output.push_str("- **Type**: Formatter\n");
            }
            if let Some(ref url) = rule.docs_url {
                output.push_str(&format!("- **Docs**: {url}\n"));
            }
            output.push_str(&format!("\n{}\n\n", rule.description));

            if !rule.related_rules.is_empty() {
                output.push_str("**Related**: ");
                output.push_str(&rule.related_rules.join(", "));
                output.push_str("\n\n");
            }

            output.push_str("---\n\n");
        }
    }

    Ok(output)
}

/// Serialize database to binary format using bincode
fn serialize_to_binary(database: &DxRuleDatabase) -> Result<Vec<u8>> {
    // Using bincode for now - can be upgraded to dx-serializer zero-copy format later
    let config = bincode::config::standard();
    let binary = bincode::encode_to_vec(database, config)
        .context("Failed to serialize database to binary")?;

    // Optional: Compress with LZ4
    #[cfg(feature = "compression")]
    {
        use lz4_flex::compress_prepend_size;
        let compressed = compress_prepend_size(&binary);
        println!(
            "  Compression: {} bytes → {} bytes ({:.1}% reduction)",
            binary.len(),
            compressed.len(),
            100.0 * (1.0 - compressed.len() as f64 / binary.len() as f64)
        );
        Ok(compressed)
    }

    #[cfg(not(feature = "compression"))]
    Ok(binary)
}

/// Load compiled rules from binary format
pub fn load_compiled_rules<P: AsRef<Path>>(rules_path: P) -> Result<DxRuleDatabase> {
    let binary = fs::read(rules_path.as_ref()).context("Failed to read compiled rules file")?;

    // Decompress if needed
    #[cfg(feature = "compression")]
    let binary = {
        use lz4_flex::decompress_size_prepended;
        decompress_size_prepended(&binary).context("Failed to decompress rules")?
    };

    let config = bincode::config::standard();
    let (database, _len): (DxRuleDatabase, usize) = bincode::decode_from_slice(&binary, config)
        .context("Failed to deserialize rules database")?;

    database.validate().map_err(|e| anyhow::anyhow!(e))?;

    Ok(database)
}

/// Verify a compiled rules file
pub fn verify_compiled_rules<P: AsRef<Path>>(rules_path: P) -> Result<()> {
    println!("🔍 Verifying compiled rules...");

    let database = load_compiled_rules(rules_path)?;

    println!("✅ Validation passed");
    println!("   Version: {}", database.version);
    println!("   Rules: {}", database.rule_count);
    println!("   Languages: {}", database.language_index.len());

    // Check for any issues
    let mut issues = Vec::new();

    // Verify all rules have unique IDs
    let mut seen_ids = std::collections::HashSet::new();
    for rule in &database.rules {
        if !seen_ids.insert(rule.rule_id) {
            issues.push(format!("Duplicate rule ID: {}", rule.rule_id));
        }
    }

    // Verify all rules have unique prefixed names
    let mut seen_names = std::collections::HashSet::new();
    for rule in &database.rules {
        if !seen_names.insert(&rule.prefixed_name) {
            issues.push(format!("Duplicate rule name: {}", rule.prefixed_name));
        }
    }

    // Verify indexes are consistent
    if database.name_index.len() != database.rules.len() {
        issues.push(format!(
            "Name index size mismatch: {} != {}",
            database.name_index.len(),
            database.rules.len()
        ));
    }

    if !issues.is_empty() {
        println!("\n⚠️  Found {} issues:", issues.len());
        for issue in issues {
            println!("   - {issue}");
        }
        anyhow::bail!("Verification failed");
    }

    println!("✅ No issues found");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_compile_and_load() -> Result<()> {
        let temp = tempdir()?;

        // Compile rules
        let compiled = compile_rules(temp.path())?;
        assert!(compiled.count > 0);

        // Load and verify
        let rules_path = temp.path().join("rules.dxm");
        let loaded = load_compiled_rules(&rules_path)?;

        assert_eq!(loaded.rule_count, compiled.count);
        assert!(loaded.validate().is_ok());

        Ok(())
    }

    #[test]
    fn test_verify() -> Result<()> {
        let temp = tempdir()?;
        compile_rules(temp.path())?;

        let rules_path = temp.path().join("rules.dxm");
        verify_compiled_rules(&rules_path)?;

        Ok(())
    }
}
