//! Tokens Command - Token analysis and efficiency metrics
//!
//! Analyzes token counts across multiple LLM models and calculates efficiency metrics.
//!
//! ## Usage
//!
//! ```bash
//! # Analyze a single file
//! dx tokens file data.dx
//!
//! # Output as JSON
//! dx tokens file data.dx --json
//!
//! # Compare DX vs other formats
//! dx tokens file data.dx --compare
//!
//! # Analyze multiple files
//! dx tokens glob "data/*.dx"
//! ```

use anyhow::{Context, Result};
use console::style;
use glob::glob;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serializer::{ModelType, TokenCounter, llm_to_document};

/// Tokens command arguments
#[derive(clap::Args, Debug)]
pub struct TokensArgs {
    /// Path or glob pattern to analyze
    pub input: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Compare DX vs JSON/YAML/TOML formats (file mode only)
    #[arg(long)]
    pub compare: bool,

    /// Show token IDs (file mode only)
    #[arg(long)]
    pub show_ids: bool,

    /// Show individual file results (glob mode only)
    #[arg(short, long)]
    pub verbose: bool,
}

impl TokensArgs {
    pub async fn execute(self) -> Result<()> {
        // Check if input is a file or a glob pattern
        let path = PathBuf::from(&self.input);

        if path.exists() && path.is_file() {
            // It's a file - analyze single file
            analyze_file(&path, self.json, self.compare, self.show_ids).await
        } else {
            // It's a glob pattern - analyze multiple files
            analyze_glob(&self.input, self.json, self.verbose).await
        }
    }
}

/// Token count result for JSON output
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenCountResult {
    pub model: String,
    pub count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<u32>,
}

/// Savings comparison for JSON output
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavingsResult {
    pub vs_json: f64,
    pub vs_yaml: f64,
    pub vs_toml: f64,
}

/// Complete analysis result for JSON output
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    pub file: String,
    pub token_counts: Vec<TokenCountResult>,
    pub savings: Option<SavingsResult>,
}

/// Aggregated result for multiple files
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregatedResult {
    pub total_files: usize,
    pub total_tokens: HashMap<String, usize>,
    pub average_savings: Option<SavingsResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<AnalysisResult>,
}

/// Calculate savings percentage
fn calculate_savings(dx_tokens: usize, other_tokens: usize) -> f64 {
    if other_tokens > 0 {
        ((other_tokens as f64 - dx_tokens as f64) / other_tokens as f64) * 100.0
    } else {
        0.0
    }
}

/// Analyze a single file
async fn analyze_file(
    path: &PathBuf,
    json_output: bool,
    compare: bool,
    show_ids: bool,
) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    let counter = TokenCounter::new();
    let counts = counter.count_primary_models(&content);

    if json_output {
        let mut token_counts: Vec<TokenCountResult> = counts
            .iter()
            .map(|(model, info)| TokenCountResult {
                model: model.to_string(),
                count: info.count,
                ids: if show_ids { info.ids.clone() } else { vec![] },
            })
            .collect();
        token_counts.sort_by(|a, b| a.model.cmp(&b.model));

        let savings = if compare {
            // Try to parse as DX and convert to other formats for comparison
            calculate_format_savings(&content, &counter)
        } else {
            None
        };

        let result = AnalysisResult {
            file: path.display().to_string(),
            token_counts,
            savings,
        };

        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!();
        println!(
            "  {} Token Analysis: {}",
            style("[*]").cyan().bold(),
            style(path.display()).cyan()
        );
        println!();
        println!("  {:<20} {:>10}", style("Model").white().bold(), style("Tokens").white().bold());
        println!("  {}", style("─".repeat(32)).dim());

        let mut sorted_counts: Vec<_> = counts.iter().collect();
        sorted_counts.sort_by_key(|(model, _)| model.to_string());

        for (model, info) in sorted_counts {
            println!("  {:<20} {:>10}", style(model.to_string()).cyan(), style(info.count).green());

            if show_ids && !info.ids.is_empty() {
                let ids_str: String = info
                    .ids
                    .iter()
                    .take(10)
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let suffix = if info.ids.len() > 10 { "..." } else { "" };
                println!("    {} IDs: [{}{}]", style("[>]").dim(), ids_str, suffix);
            }
        }

        if compare {
            println!();
            println!("  {} Format Comparison", style("[>]").cyan().bold());
            println!("  {}", style("─".repeat(32)).dim());

            if let Some(savings) = calculate_format_savings(&content, &counter) {
                print_savings("vs JSON", savings.vs_json);
                print_savings("vs YAML", savings.vs_yaml);
                print_savings("vs TOML", savings.vs_toml);
            } else {
                println!("  {} Unable to calculate format comparison", style("[i]").blue());
                println!("    (file may not be valid DX format)");
            }
        }
        println!();
    }

    Ok(())
}

/// Print savings with color coding
fn print_savings(label: &str, savings: f64) {
    let savings_style = if savings > 0.0 {
        style(format!("{:+.1}%", savings)).green().bold()
    } else if savings < 0.0 {
        style(format!("{:+.1}%", savings)).red().bold()
    } else {
        style(format!("{:+.1}%", savings)).dim()
    };
    println!("  {:<20} {:>10}", style(label).cyan(), savings_style);
}

/// Calculate savings vs other formats
fn calculate_format_savings(dx_content: &str, counter: &TokenCounter) -> Option<SavingsResult> {
    // Try to parse as DX document
    let doc = match llm_to_document(dx_content) {
        Ok(d) => d,
        Err(_) => return None,
    };

    // Convert to other formats for comparison - use simple string representation
    let json_content = format_context_as_json(&doc.context);

    // Simple YAML-like representation
    let yaml_content = format_as_yaml(&doc.context);

    // Simple TOML-like representation
    let toml_content = format_as_toml(&doc.context);

    let dx_tokens = counter.count(dx_content, ModelType::Gpt4o).count;
    let json_tokens = counter.count(&json_content, ModelType::Gpt4o).count;
    let yaml_tokens = counter.count(&yaml_content, ModelType::Gpt4o).count;
    let toml_tokens = counter.count(&toml_content, ModelType::Gpt4o).count;

    Some(SavingsResult {
        vs_json: calculate_savings(dx_tokens, json_tokens),
        vs_yaml: calculate_savings(dx_tokens, yaml_tokens),
        vs_toml: calculate_savings(dx_tokens, toml_tokens),
    })
}

/// Format context as JSON-like string
fn format_context_as_json(
    context: &serializer::IndexMap<String, serializer::DxLlmValue>,
) -> String {
    use serializer::DxLlmValue;

    let mut parts = Vec::new();
    for (key, value) in context {
        let val_str = match value {
            DxLlmValue::Str(s) => format!("\"{}\"", s),
            DxLlmValue::Num(n) => n.to_string(),
            DxLlmValue::Bool(b) => b.to_string(),
            DxLlmValue::Null => "null".to_string(),
            DxLlmValue::Arr(arr) => format!("{:?}", arr),
            DxLlmValue::Ref(r) => format!("\"^{}\"", r),
            DxLlmValue::Obj(obj) => format!("{:?}", obj),
        };
        parts.push(format!("  \"{}\": {}", key, val_str));
    }
    format!("{{\n{}\n}}", parts.join(",\n"))
}

/// Format context as YAML-like string
fn format_as_yaml(context: &serializer::IndexMap<String, serializer::DxLlmValue>) -> String {
    use serializer::DxLlmValue;

    let mut lines = Vec::new();
    for (key, value) in context {
        match value {
            DxLlmValue::Str(s) => lines.push(format!("{}: \"{}\"", key, s)),
            DxLlmValue::Num(n) => lines.push(format!("{}: {}", key, n)),
            DxLlmValue::Bool(b) => lines.push(format!("{}: {}", key, b)),
            DxLlmValue::Null => lines.push(format!("{}: null", key)),
            DxLlmValue::Arr(arr) => {
                lines.push(format!("{}:", key));
                for item in arr {
                    lines.push(format!("  - {:?}", item));
                }
            }
            DxLlmValue::Ref(r) => lines.push(format!("{}: ^{}", key, r)),
            DxLlmValue::Obj(obj) => lines.push(format!("{}: {:?}", key, obj)),
        }
    }
    lines.join("\n")
}

/// Format context as TOML-like string
fn format_as_toml(context: &serializer::IndexMap<String, serializer::DxLlmValue>) -> String {
    use serializer::DxLlmValue;

    let mut lines = Vec::new();
    for (key, value) in context {
        match value {
            DxLlmValue::Str(s) => lines.push(format!("{} = \"{}\"", key, s)),
            DxLlmValue::Num(n) => lines.push(format!("{} = {}", key, n)),
            DxLlmValue::Bool(b) => lines.push(format!("{} = {}", key, b)),
            DxLlmValue::Null => lines.push(format!("# {} = null", key)),
            DxLlmValue::Arr(arr) => {
                let items: Vec<String> = arr.iter().map(|v| format!("{:?}", v)).collect();
                lines.push(format!("{} = [{}]", key, items.join(", ")));
            }
            DxLlmValue::Ref(r) => lines.push(format!("{} = \"^{}\"", key, r)),
            DxLlmValue::Obj(obj) => lines.push(format!("{} = {:?}", key, obj)),
        }
    }
    lines.join("\n")
}

/// Analyze multiple files matching a glob pattern
async fn analyze_glob(pattern: &str, json_output: bool, verbose: bool) -> Result<()> {
    let paths: Vec<PathBuf> = glob(pattern)
        .with_context(|| format!("Invalid glob pattern: {}", pattern))?
        .filter_map(|entry| entry.ok())
        .filter(|path| path.is_file())
        .collect();

    if paths.is_empty() {
        if json_output {
            let result = AggregatedResult {
                total_files: 0,
                total_tokens: HashMap::new(),
                average_savings: None,
                files: vec![],
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!();
            println!(
                "  {} No files found matching pattern '{}'",
                style("[i]").yellow().bold(),
                style(pattern).cyan()
            );
            println!();
        }
        return Ok(());
    }

    let counter = TokenCounter::new();
    let mut total_tokens: HashMap<String, usize> = HashMap::new();
    let mut file_results: Vec<AnalysisResult> = Vec::new();
    let mut total_savings = SavingsResult {
        vs_json: 0.0,
        vs_yaml: 0.0,
        vs_toml: 0.0,
    };
    let mut savings_count = 0;

    for path in &paths {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let counts = counter.count_primary_models(&content);

        // Aggregate totals
        for (model, info) in &counts {
            *total_tokens.entry(model.to_string()).or_insert(0) += info.count;
        }

        // Calculate savings if possible
        if let Some(savings) = calculate_format_savings(&content, &counter) {
            total_savings.vs_json += savings.vs_json;
            total_savings.vs_yaml += savings.vs_yaml;
            total_savings.vs_toml += savings.vs_toml;
            savings_count += 1;
        }

        if verbose || json_output {
            let token_counts: Vec<TokenCountResult> = counts
                .iter()
                .map(|(model, info)| TokenCountResult {
                    model: model.to_string(),
                    count: info.count,
                    ids: vec![],
                })
                .collect();

            file_results.push(AnalysisResult {
                file: path.display().to_string(),
                token_counts,
                savings: calculate_format_savings(&content, &counter),
            });
        }
    }

    // Calculate average savings
    let average_savings = if savings_count > 0 {
        Some(SavingsResult {
            vs_json: total_savings.vs_json / savings_count as f64,
            vs_yaml: total_savings.vs_yaml / savings_count as f64,
            vs_toml: total_savings.vs_toml / savings_count as f64,
        })
    } else {
        None
    };

    if json_output {
        let result = AggregatedResult {
            total_files: paths.len(),
            total_tokens,
            average_savings,
            files: file_results,
        };
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!();
        println!(
            "  {} Token Analysis: {} files matching '{}'",
            style("[*]").magenta().bold(),
            style(paths.len()).cyan().bold(),
            style(pattern).cyan()
        );
        println!();

        if verbose {
            for result in &file_results {
                println!("  {} {}", style("[>]").dim(), style(&result.file).cyan());
                for tc in &result.token_counts {
                    println!(
                        "      {}: {} tokens",
                        style(&tc.model).cyan(),
                        style(tc.count).green()
                    );
                }
            }
            println!();
        }

        println!("  {} Aggregated Totals", style("[>]").cyan().bold());
        println!("  {}", style("─".repeat(32)).dim());
        println!("  {:<20} {:>10}", style("Model").white().bold(), style("Tokens").white().bold());
        println!("  {}", style("─".repeat(32)).dim());

        let mut sorted_totals: Vec<_> = total_tokens.iter().collect();
        sorted_totals.sort_by_key(|(model, _)| *model);

        for (model, count) in sorted_totals {
            println!("  {:<20} {:>10}", style(model).cyan(), style(count).green());
        }

        if let Some(savings) = average_savings {
            println!();
            println!("  {} Average Savings", style("[>]").yellow().bold());
            println!("  {}", style("─".repeat(32)).dim());
            print_savings("vs JSON", savings.vs_json);
            print_savings("vs YAML", savings.vs_yaml);
            print_savings("vs TOML", savings.vs_toml);
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_savings() {
        assert!((calculate_savings(73, 100) - 27.0).abs() < 0.1);
        assert!((calculate_savings(100, 100) - 0.0).abs() < 0.1);
        assert!((calculate_savings(110, 100) - (-10.0)).abs() < 0.1);
        assert_eq!(calculate_savings(50, 0), 0.0);
    }

    #[test]
    fn test_token_count_result_serialization() {
        let result = TokenCountResult {
            model: "GPT-4o".to_string(),
            count: 100,
            ids: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("GPT-4o"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_analysis_result_serialization() {
        let result = AnalysisResult {
            file: "test.dx".to_string(),
            token_counts: vec![TokenCountResult {
                model: "GPT-4o".to_string(),
                count: 100,
                ids: vec![],
            }],
            savings: Some(SavingsResult {
                vs_json: 27.0,
                vs_yaml: 15.0,
                vs_toml: 10.0,
            }),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test.dx"));
        assert!(json.contains("tokenCounts"));
        assert!(json.contains("savings"));
    }
}
