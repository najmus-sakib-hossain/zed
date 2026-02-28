//! Multi-Tokenizer Benchmark Suite for DX Markdown.
//!
//! This module provides comprehensive benchmarking across multiple LLM tokenizers
//! to measure token savings and optimization effectiveness.
//!
//! # Supported Tokenizers
//!
//! - OpenAI: cl100k_base (GPT-4), o200k_base (GPT-4o/GPT-5), p50k_base (GPT-3.5)
//! - Anthropic: Claude tokenizer (approximated via cl100k)
//! - Google: Gemini tokenizer (approximated via cl100k)
//! - Meta: LLaMA tokenizer (approximated via cl100k)
//! - Mistral: Mistral tokenizer (approximated via cl100k)
//! - Cohere: Command tokenizer (approximated via cl100k)
//!
//! Note: Non-OpenAI tokenizers are approximated using cl100k_base as a baseline.
//! For accurate measurements, integrate native tokenizer libraries.
//!
//! # Stability
//!
//! This module is **experimental** and not part of the stable API.
//! It may use `unwrap()` and `expect()` for convenience as it's not production code.

// Allow unwrap/expect in experimental benchmark code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::compiler::DxMarkdown;
use crate::error::CompileError;
use crate::tokenizer::Tokenizer;
use crate::types::{CompilerConfig, TokenizerType};
use std::collections::HashMap;
use std::fmt;

/// Supported tokenizer providers for benchmarking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BenchmarkTokenizer {
    /// OpenAI cl100k_base (GPT-4, Claude-compatible)
    OpenAiCl100k,
    /// OpenAI o200k_base (GPT-4o, GPT-5)
    OpenAiO200k,
    /// OpenAI p50k_base (GPT-3.5)
    OpenAiP50k,
    /// Anthropic Claude (approximated via cl100k)
    Claude,
    /// Google Gemini (approximated via cl100k)
    Gemini,
    /// Meta LLaMA (approximated via cl100k)
    Llama,
    /// Mistral (approximated via cl100k)
    Mistral,
    /// Cohere Command (approximated via cl100k)
    Cohere,
}

impl BenchmarkTokenizer {
    /// Get all available tokenizers.
    pub fn all() -> Vec<Self> {
        vec![
            Self::OpenAiCl100k,
            Self::OpenAiO200k,
            Self::OpenAiP50k,
            Self::Claude,
            Self::Gemini,
            Self::Llama,
            Self::Mistral,
            Self::Cohere,
        ]
    }

    /// Get OpenAI tokenizers only.
    pub fn openai_only() -> Vec<Self> {
        vec![Self::OpenAiCl100k, Self::OpenAiO200k, Self::OpenAiP50k]
    }

    /// Get the display name for this tokenizer.
    pub fn name(&self) -> &'static str {
        match self {
            Self::OpenAiCl100k => "OpenAI cl100k (GPT-4)",
            Self::OpenAiO200k => "OpenAI o200k (GPT-4o)",
            Self::OpenAiP50k => "OpenAI p50k (GPT-3.5)",
            Self::Claude => "Anthropic Claude",
            Self::Gemini => "Google Gemini",
            Self::Llama => "Meta LLaMA",
            Self::Mistral => "Mistral",
            Self::Cohere => "Cohere Command",
        }
    }

    /// Get the short name for this tokenizer.
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::OpenAiCl100k => "cl100k",
            Self::OpenAiO200k => "o200k",
            Self::OpenAiP50k => "p50k",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
            Self::Llama => "llama",
            Self::Mistral => "mistral",
            Self::Cohere => "cohere",
        }
    }

    /// Check if this tokenizer is natively supported.
    pub fn is_native(&self) -> bool {
        matches!(self, Self::OpenAiCl100k | Self::OpenAiO200k | Self::OpenAiP50k)
    }

    /// Get the underlying TokenizerType for counting.
    fn to_tokenizer_type(self) -> TokenizerType {
        match self {
            Self::OpenAiCl100k
            | Self::Claude
            | Self::Gemini
            | Self::Llama
            | Self::Mistral
            | Self::Cohere => TokenizerType::Cl100k,
            Self::OpenAiO200k => TokenizerType::O200k,
            Self::OpenAiP50k => TokenizerType::P50k,
        }
    }

    /// Get the cost per 1M input tokens (USD) for this provider.
    pub fn cost_per_million_input(&self) -> f64 {
        match self {
            Self::OpenAiCl100k => 10.0, // GPT-4 Turbo
            Self::OpenAiO200k => 2.50,  // GPT-4o
            Self::OpenAiP50k => 0.50,   // GPT-3.5 Turbo
            Self::Claude => 3.00,       // Claude 3 Sonnet
            Self::Gemini => 0.075,      // Gemini 1.5 Flash
            Self::Llama => 0.20,        // LLaMA via API
            Self::Mistral => 0.25,      // Mistral Medium
            Self::Cohere => 0.50,       // Command R
        }
    }

    /// Parse tokenizer from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cl100k" | "openai" | "gpt4" => Some(Self::OpenAiCl100k),
            "o200k" | "gpt4o" | "gpt5" => Some(Self::OpenAiO200k),
            "p50k" | "gpt35" | "gpt3.5" => Some(Self::OpenAiP50k),
            "claude" | "anthropic" => Some(Self::Claude),
            "gemini" | "google" => Some(Self::Gemini),
            "llama" | "meta" => Some(Self::Llama),
            "mistral" => Some(Self::Mistral),
            "cohere" | "command" => Some(Self::Cohere),
            _ => None,
        }
    }
}

impl fmt::Display for BenchmarkTokenizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Result of benchmarking a single file with a single tokenizer.
#[derive(Debug, Clone)]
pub struct TokenizerResult {
    /// The tokenizer used.
    pub tokenizer: BenchmarkTokenizer,
    /// Tokens before optimization.
    pub tokens_before: usize,
    /// Tokens after optimization.
    pub tokens_after: usize,
    /// Tokens saved.
    pub tokens_saved: usize,
    /// Savings percentage.
    pub savings_percent: f64,
    /// Estimated cost savings (USD per 1M tokens).
    pub cost_savings_per_million: f64,
}

impl TokenizerResult {
    /// Create a new tokenizer result.
    pub fn new(tokenizer: BenchmarkTokenizer, tokens_before: usize, tokens_after: usize) -> Self {
        let tokens_saved = tokens_before.saturating_sub(tokens_after);
        let savings_percent = if tokens_before > 0 {
            (tokens_saved as f64 / tokens_before as f64) * 100.0
        } else {
            0.0
        };
        let cost_per_million = tokenizer.cost_per_million_input();
        let cost_savings_per_million = (tokens_saved as f64 / 1_000_000.0) * cost_per_million;

        Self {
            tokenizer,
            tokens_before,
            tokens_after,
            tokens_saved,
            savings_percent,
            cost_savings_per_million,
        }
    }
}

/// Result of benchmarking a single file across all tokenizers.
#[derive(Debug, Clone)]
pub struct FileBenchmarkResult {
    /// File name or identifier.
    pub file_name: String,
    /// File category (badge-heavy, table-heavy, etc.).
    pub category: FileCategory,
    /// Original content size in bytes.
    pub original_size: usize,
    /// Optimized content size in bytes.
    pub optimized_size: usize,
    /// Results per tokenizer.
    pub tokenizer_results: Vec<TokenizerResult>,
}

impl FileBenchmarkResult {
    /// Get the average savings percentage across all tokenizers.
    pub fn average_savings(&self) -> f64 {
        if self.tokenizer_results.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.tokenizer_results.iter().map(|r| r.savings_percent).sum();
        sum / self.tokenizer_results.len() as f64
    }

    /// Get the best tokenizer for this file (highest savings).
    pub fn best_tokenizer(&self) -> Option<&TokenizerResult> {
        self.tokenizer_results
            .iter()
            .max_by(|a, b| a.savings_percent.partial_cmp(&b.savings_percent).unwrap())
    }
}

/// File category for benchmark datasets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
    /// Badge-heavy READMEs with many shields.io badges.
    BadgeHeavy,
    /// Table-heavy documentation with many markdown tables.
    TableHeavy,
    /// Code-heavy documentation with many code blocks.
    CodeHeavy,
    /// Prose-heavy documentation with mostly text.
    ProseHeavy,
    /// Mixed real-world files.
    Mixed,
}

impl FileCategory {
    /// Get the display name for this category.
    pub fn name(&self) -> &'static str {
        match self {
            Self::BadgeHeavy => "Badge-heavy",
            Self::TableHeavy => "Table-heavy",
            Self::CodeHeavy => "Code-heavy",
            Self::ProseHeavy => "Prose-heavy",
            Self::Mixed => "Mixed",
        }
    }

    /// Get expected savings range for this category.
    pub fn expected_savings(&self) -> (f64, f64) {
        match self {
            Self::BadgeHeavy => (40.0, 60.0),
            Self::TableHeavy => (30.0, 50.0),
            Self::CodeHeavy => (20.0, 35.0),
            Self::ProseHeavy => (15.0, 25.0),
            Self::Mixed => (25.0, 40.0),
        }
    }
}

impl fmt::Display for FileCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Complete benchmark suite result.
#[derive(Debug, Clone)]
pub struct BenchmarkSuiteResult {
    /// Results per file.
    pub file_results: Vec<FileBenchmarkResult>,
    /// Summary statistics per tokenizer.
    pub tokenizer_summaries: HashMap<BenchmarkTokenizer, TokenizerSummary>,
    /// Summary statistics per category.
    pub category_summaries: HashMap<FileCategory, CategorySummary>,
}

/// Summary statistics for a single tokenizer across all files.
#[derive(Debug, Clone, Default)]
pub struct TokenizerSummary {
    /// Total tokens before optimization.
    pub total_tokens_before: usize,
    /// Total tokens after optimization.
    pub total_tokens_after: usize,
    /// Total tokens saved.
    pub total_tokens_saved: usize,
    /// Average savings percentage.
    pub average_savings_percent: f64,
    /// Total estimated cost savings.
    pub total_cost_savings: f64,
    /// Number of files processed.
    pub file_count: usize,
}

/// Summary statistics for a file category.
#[derive(Debug, Clone, Default)]
pub struct CategorySummary {
    /// Average savings percentage across all tokenizers.
    pub average_savings_percent: f64,
    /// Number of files in this category.
    pub file_count: usize,
    /// Whether savings met expected targets.
    pub met_target: bool,
}

/// Benchmark runner for multi-tokenizer analysis.
pub struct BenchmarkRunner {
    /// Tokenizers to benchmark.
    tokenizers: Vec<BenchmarkTokenizer>,
    /// Compiler configuration.
    config: CompilerConfig,
    /// Cached tokenizer instances.
    tokenizer_cache: HashMap<TokenizerType, Tokenizer>,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner with all tokenizers.
    pub fn new() -> Result<Self, CompileError> {
        Self::with_tokenizers(BenchmarkTokenizer::all())
    }

    /// Create a benchmark runner with specific tokenizers.
    pub fn with_tokenizers(tokenizers: Vec<BenchmarkTokenizer>) -> Result<Self, CompileError> {
        let mut tokenizer_cache = HashMap::new();

        // Pre-initialize unique tokenizer types
        for tok in &tokenizers {
            let tok_type = tok.to_tokenizer_type();
            if let std::collections::hash_map::Entry::Vacant(e) = tokenizer_cache.entry(tok_type) {
                e.insert(Tokenizer::new(tok_type)?);
            }
        }

        Ok(Self {
            tokenizers,
            config: CompilerConfig::default(),
            tokenizer_cache,
        })
    }

    /// Create a benchmark runner with OpenAI tokenizers only.
    pub fn openai_only() -> Result<Self, CompileError> {
        Self::with_tokenizers(BenchmarkTokenizer::openai_only())
    }

    /// Set the compiler configuration.
    pub fn with_config(mut self, config: CompilerConfig) -> Self {
        self.config = config;
        self
    }

    /// Benchmark a single piece of content.
    pub fn benchmark_content(
        &self,
        content: &str,
        file_name: &str,
        category: FileCategory,
    ) -> Result<FileBenchmarkResult, CompileError> {
        // Compile the content
        let compiler = DxMarkdown::new(self.config.clone())?;
        let result = compiler.compile(content)?;

        // Benchmark each tokenizer
        let mut tokenizer_results = Vec::with_capacity(self.tokenizers.len());

        for bench_tok in &self.tokenizers {
            let tok_type = bench_tok.to_tokenizer_type();
            let tokenizer = self.tokenizer_cache.get(&tok_type).unwrap();

            let tokens_before = tokenizer.count(content);
            let tokens_after = tokenizer.count(&result.output);

            tokenizer_results.push(TokenizerResult::new(*bench_tok, tokens_before, tokens_after));
        }

        Ok(FileBenchmarkResult {
            file_name: file_name.to_string(),
            category,
            original_size: content.len(),
            optimized_size: result.output.len(),
            tokenizer_results,
        })
    }

    /// Benchmark multiple files and generate a suite result.
    pub fn benchmark_suite(
        &self,
        files: &[(String, String, FileCategory)], // (name, content, category)
    ) -> Result<BenchmarkSuiteResult, CompileError> {
        let mut file_results = Vec::with_capacity(files.len());

        for (name, content, category) in files {
            let result = self.benchmark_content(content, name, *category)?;
            file_results.push(result);
        }

        // Calculate tokenizer summaries
        let mut tokenizer_summaries: HashMap<BenchmarkTokenizer, TokenizerSummary> = HashMap::new();

        for bench_tok in &self.tokenizers {
            let mut summary = TokenizerSummary::default();

            for file_result in &file_results {
                if let Some(tok_result) =
                    file_result.tokenizer_results.iter().find(|r| r.tokenizer == *bench_tok)
                {
                    summary.total_tokens_before += tok_result.tokens_before;
                    summary.total_tokens_after += tok_result.tokens_after;
                    summary.total_tokens_saved += tok_result.tokens_saved;
                    summary.total_cost_savings += tok_result.cost_savings_per_million;
                    summary.file_count += 1;
                }
            }

            if summary.total_tokens_before > 0 {
                summary.average_savings_percent = (summary.total_tokens_saved as f64
                    / summary.total_tokens_before as f64)
                    * 100.0;
            }

            tokenizer_summaries.insert(*bench_tok, summary);
        }

        // Calculate category summaries
        let mut category_summaries: HashMap<FileCategory, CategorySummary> = HashMap::new();

        for category in [
            FileCategory::BadgeHeavy,
            FileCategory::TableHeavy,
            FileCategory::CodeHeavy,
            FileCategory::ProseHeavy,
            FileCategory::Mixed,
        ] {
            let category_files: Vec<_> =
                file_results.iter().filter(|f| f.category == category).collect();

            if !category_files.is_empty() {
                let avg_savings: f64 =
                    category_files.iter().map(|f| f.average_savings()).sum::<f64>()
                        / category_files.len() as f64;
                let (min_expected, max_expected) = category.expected_savings();

                category_summaries.insert(
                    category,
                    CategorySummary {
                        average_savings_percent: avg_savings,
                        file_count: category_files.len(),
                        met_target: avg_savings >= min_expected
                            && avg_savings <= max_expected + 20.0,
                    },
                );
            }
        }

        Ok(BenchmarkSuiteResult {
            file_results,
            tokenizer_summaries,
            category_summaries,
        })
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new().expect("failed to create benchmark runner")
    }
}

/// Report generator for benchmark results.
pub struct BenchmarkReporter;

impl BenchmarkReporter {
    /// Generate a markdown report from benchmark results.
    pub fn to_markdown(result: &BenchmarkSuiteResult) -> String {
        let mut report = String::new();

        report.push_str("# DX Markdown Multi-Tokenizer Benchmark Report\n\n");

        // Summary table
        report.push_str("## Tokenizer Summary\n\n");
        report.push_str("| Tokenizer | Files | Tokens Before | Tokens After | Saved | Savings % | Cost Savings |\n");
        report.push_str("|-----------|-------|---------------|--------------|-------|-----------|-------------|\n");

        for tok in BenchmarkTokenizer::all() {
            if let Some(summary) = result.tokenizer_summaries.get(&tok) {
                report.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {:.1}% | ${:.4} |\n",
                    tok.short_name(),
                    summary.file_count,
                    summary.total_tokens_before,
                    summary.total_tokens_after,
                    summary.total_tokens_saved,
                    summary.average_savings_percent,
                    summary.total_cost_savings
                ));
            }
        }

        report.push('\n');

        // Category summary
        report.push_str("## Category Summary\n\n");
        report.push_str("| Category | Files | Avg Savings | Target | Met Target |\n");
        report.push_str("|----------|-------|-------------|--------|------------|\n");

        for category in [
            FileCategory::BadgeHeavy,
            FileCategory::TableHeavy,
            FileCategory::CodeHeavy,
            FileCategory::ProseHeavy,
            FileCategory::Mixed,
        ] {
            if let Some(summary) = result.category_summaries.get(&category) {
                let (min, max) = category.expected_savings();
                let met = if summary.met_target { "✓" } else { "✗" };
                report.push_str(&format!(
                    "| {} | {} | {:.1}% | {:.0}-{:.0}% | {} |\n",
                    category.name(),
                    summary.file_count,
                    summary.average_savings_percent,
                    min,
                    max,
                    met
                ));
            }
        }

        report.push('\n');

        // Per-file details
        report.push_str("## File Details\n\n");

        for file_result in &result.file_results {
            report.push_str(&format!("### {}\n\n", file_result.file_name));
            report.push_str(&format!("- Category: {}\n", file_result.category));
            report.push_str(&format!("- Original size: {} bytes\n", file_result.original_size));
            report.push_str(&format!("- Optimized size: {} bytes\n", file_result.optimized_size));
            report
                .push_str(&format!("- Average savings: {:.1}%\n\n", file_result.average_savings()));

            report.push_str("| Tokenizer | Before | After | Saved | Savings % |\n");
            report.push_str("|-----------|--------|-------|-------|----------|\n");

            for tok_result in &file_result.tokenizer_results {
                report.push_str(&format!(
                    "| {} | {} | {} | {} | {:.1}% |\n",
                    tok_result.tokenizer.short_name(),
                    tok_result.tokens_before,
                    tok_result.tokens_after,
                    tok_result.tokens_saved,
                    tok_result.savings_percent
                ));
            }

            report.push('\n');
        }

        report
    }

    /// Generate a JSON report from benchmark results.
    pub fn to_json(result: &BenchmarkSuiteResult) -> String {
        let mut json = String::from("{\n");

        // Tokenizer summaries
        json.push_str("  \"tokenizer_summaries\": {\n");
        let tok_entries: Vec<String> = result.tokenizer_summaries.iter().map(|(tok, summary)| {
            format!(
                "    \"{}\": {{\n      \"tokens_before\": {},\n      \"tokens_after\": {},\n      \"tokens_saved\": {},\n      \"savings_percent\": {:.2},\n      \"cost_savings\": {:.6},\n      \"file_count\": {}\n    }}",
                tok.short_name(),
                summary.total_tokens_before,
                summary.total_tokens_after,
                summary.total_tokens_saved,
                summary.average_savings_percent,
                summary.total_cost_savings,
                summary.file_count
            )
        }).collect();
        json.push_str(&tok_entries.join(",\n"));
        json.push_str("\n  },\n");

        // Category summaries
        json.push_str("  \"category_summaries\": {\n");
        let cat_entries: Vec<String> = result.category_summaries.iter().map(|(cat, summary)| {
            format!(
                "    \"{}\": {{\n      \"average_savings_percent\": {:.2},\n      \"file_count\": {},\n      \"met_target\": {}\n    }}",
                cat.name().to_lowercase().replace('-', "_"),
                summary.average_savings_percent,
                summary.file_count,
                summary.met_target
            )
        }).collect();
        json.push_str(&cat_entries.join(",\n"));
        json.push_str("\n  },\n");

        // File count
        json.push_str(&format!("  \"total_files\": {}\n", result.file_results.len()));
        json.push_str("}\n");

        json
    }

    /// Generate an ASCII chart for CLI output.
    pub fn to_ascii_chart(result: &BenchmarkSuiteResult) -> String {
        let mut chart = String::new();

        chart.push('\n');
        chart.push_str(
            "  +---------------------------------------------------------------------------+\n",
        );
        chart.push_str(
            "  |                  DX Markdown Context Compiler Benchmark                  |\n",
        );
        chart.push_str(
            "  +---------------------------------------------------------------------------+\n\n",
        );

        let max_savings = result
            .tokenizer_summaries
            .values()
            .map(|s| s.average_savings_percent)
            .fold(0.0_f64, |a, b| a.max(b));

        let bar_width = 18;

        // Group by file
        for file_result in &result.file_results {
            let size_kb = file_result.original_size as f64 / 1024.0;
            chart.push_str(
                "  +---------------------------------------------------------------------+\n",
            );
            chart.push_str(&format!(
                "  | {:67} |\n",
                format!("{} ({:.1} KB)", file_result.file_name, size_kb)
            ));
            chart.push_str(
                "  +---------------------------------------------------------------------+\n",
            );
            chart.push_str(&format!(
                "  | {:<22} | {:^20} | {:>7} | {:>13} |\n",
                "Tokenizer", "Progress", "Saved", "Tokens"
            ));
            chart.push_str(
                "  +------------------------+----------------------+---------+---------------+\n",
            );

            for tok_result in &file_result.tokenizer_results {
                let bar_len = if max_savings > 0.0 {
                    ((tok_result.savings_percent / max_savings) * bar_width as f64).round() as usize
                } else {
                    0
                };

                let bar_len = bar_len.min(bar_width); // Clamp to prevent overflow
                let bar: String = "#".repeat(bar_len) + &"-".repeat(bar_width - bar_len);

                chart.push_str(&format!(
                    "  | {:<22} | [{}] | {:>5.1}%  | {:>5} -> {:>5} |\n",
                    tok_result.tokenizer.name(),
                    bar,
                    tok_result.savings_percent,
                    tok_result.tokens_before,
                    tok_result.tokens_after
                ));
            }

            chart.push_str(
                "  +------------------------+----------------------+---------+---------------+\n",
            );
            chart.push_str(&format!(
                "  | {:<22} | {:^20} | {:>5.1}%  | {:>13} |\n",
                "Average",
                "",
                file_result.average_savings(),
                ""
            ));
            chart.push_str(
                "  +------------------------+----------------------+---------+---------------+\n\n",
            );
        }

        chart
    }
}

/// Sample benchmark datasets.
pub mod datasets {
    use super::FileCategory;

    /// Generate a badge-heavy README sample.
    pub fn badge_heavy_readme() -> (String, String, FileCategory) {
        let content = r#"# My Awesome Project

[![Build Status](https://img.shields.io/github/actions/workflow/status/user/repo/ci.yml?branch=main)](https://github.com/user/repo/actions)
[![Coverage](https://img.shields.io/codecov/c/github/user/repo)](https://codecov.io/gh/user/repo)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![npm version](https://img.shields.io/npm/v/my-package.svg)](https://www.npmjs.com/package/my-package)
[![Downloads](https://img.shields.io/npm/dm/my-package.svg)](https://www.npmjs.com/package/my-package)
[![GitHub stars](https://img.shields.io/github/stars/user/repo.svg?style=social)](https://github.com/user/repo)
[![Twitter Follow](https://img.shields.io/twitter/follow/username.svg?style=social)](https://twitter.com/username)
[![Discord](https://img.shields.io/discord/123456789.svg)](https://discord.gg/invite)

A powerful library for doing amazing things.

## Installation

```bash
npm install my-package
```

## Quick Start

```javascript
import { amazing } from 'my-package';
amazing();
```
"#;
        (
            "badge_heavy_readme.md".to_string(),
            content.to_string(),
            FileCategory::BadgeHeavy,
        )
    }

    /// Generate a table-heavy documentation sample.
    pub fn table_heavy_docs() -> (String, String, FileCategory) {
        let content = r#"# API Reference

## Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `create` | `options: CreateOptions` | `Promise<Resource>` | Creates a new resource |
| `read` | `id: string` | `Promise<Resource>` | Reads a resource by ID |
| `update` | `id: string, data: Partial<Resource>` | `Promise<Resource>` | Updates a resource |
| `delete` | `id: string` | `Promise<void>` | Deletes a resource |
| `list` | `query: QueryOptions` | `Promise<Resource[]>` | Lists all resources |

## Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `timeout` | `number` | `30000` | Request timeout in ms |
| `retries` | `number` | `3` | Number of retry attempts |
| `baseUrl` | `string` | `'https://api.example.com'` | API base URL |
| `headers` | `Record<string, string>` | `{}` | Custom headers |
| `debug` | `boolean` | `false` | Enable debug logging |

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| `E001` | `NotFound` | Resource not found |
| `E002` | `Unauthorized` | Authentication required |
| `E003` | `Forbidden` | Access denied |
| `E004` | `ValidationError` | Invalid input data |
| `E005` | `RateLimit` | Too many requests |
"#;
        ("table_heavy_docs.md".to_string(), content.to_string(), FileCategory::TableHeavy)
    }

    /// Generate a code-heavy documentation sample.
    pub fn code_heavy_docs() -> (String, String, FileCategory) {
        let content = r#"# Code Examples

## Basic Usage

```javascript
// Initialize the client
const client = new Client({
  apiKey: process.env.API_KEY,
  timeout: 30000,
});

// Create a resource
const resource = await client.create({
  name: 'My Resource',
  type: 'example',
  metadata: {
    created: new Date().toISOString(),
  },
});

console.log('Created:', resource.id);
```

## Advanced Configuration

```typescript
interface ClientOptions {
  apiKey: string;
  timeout?: number;
  retries?: number;
  onError?: (error: Error) => void;
}

class AdvancedClient {
  private options: ClientOptions;
  
  constructor(options: ClientOptions) {
    this.options = {
      timeout: 30000,
      retries: 3,
      ...options,
    };
  }
  
  async request<T>(endpoint: string): Promise<T> {
    // Implementation
    return {} as T;
  }
}
```

## Error Handling

```python
try:
    result = client.process(data)
except ValidationError as e:
    logger.error(f"Validation failed: {e}")
    raise
except NetworkError as e:
    logger.warning(f"Network error, retrying: {e}")
    result = client.process(data, retry=True)
```
"#;
        ("code_heavy_docs.md".to_string(), content.to_string(), FileCategory::CodeHeavy)
    }

    /// Generate a prose-heavy documentation sample.
    pub fn prose_heavy_docs() -> (String, String, FileCategory) {
        let content = r#"# Architecture Overview

## Introduction

This document describes the high-level architecture of our system. The architecture follows a microservices pattern with event-driven communication between services. Each service is designed to be independently deployable and scalable.

## Design Principles

Our architecture is built on several key principles that guide all technical decisions. First, we prioritize loose coupling between services to enable independent development and deployment. Second, we embrace eventual consistency where appropriate to improve system availability and performance.

The system uses a combination of synchronous and asynchronous communication patterns. Synchronous calls are used for operations that require immediate responses, while asynchronous messaging handles background processing and cross-service notifications.

## Service Boundaries

Services are organized around business capabilities rather than technical layers. Each service owns its data and exposes functionality through well-defined APIs. This approach ensures that changes to one service have minimal impact on others.

## Scalability Considerations

The architecture supports horizontal scaling at multiple levels. Individual services can be scaled independently based on their specific load patterns. The message queue infrastructure provides natural load leveling and helps absorb traffic spikes.

## Security Model

Security is implemented at multiple layers throughout the system. Authentication is handled centrally, while authorization decisions are made by individual services based on their specific requirements.
"#;
        ("prose_heavy_docs.md".to_string(), content.to_string(), FileCategory::ProseHeavy)
    }

    /// Generate a mixed real-world sample.
    pub fn mixed_readme() -> (String, String, FileCategory) {
        let content = r#"# Real-World Project

[![CI](https://github.com/user/repo/workflows/CI/badge.svg)](https://github.com/user/repo/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A comprehensive solution for modern development workflows.

## Features

- Fast and efficient processing
- Easy to configure and extend
- Well-documented API

## Installation

```bash
npm install real-world-project
```

## Quick Start

```javascript
const project = require('real-world-project');
project.init({ debug: true });
```

## API Reference

| Method | Description |
|--------|-------------|
| `init` | Initialize the project |
| `process` | Process input data |
| `cleanup` | Clean up resources |

## Configuration

The project can be configured through environment variables or a config file. See the documentation for more details on available options.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting a pull request.

## License

MIT License - see [LICENSE](LICENSE) for details.
"#;
        ("mixed_readme.md".to_string(), content.to_string(), FileCategory::Mixed)
    }

    /// Get all sample datasets.
    pub fn all_samples() -> Vec<(String, String, FileCategory)> {
        vec![
            badge_heavy_readme(),
            table_heavy_docs(),
            code_heavy_docs(),
            prose_heavy_docs(),
            mixed_readme(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_tokenizer_all() {
        let all = BenchmarkTokenizer::all();
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn test_benchmark_tokenizer_openai_only() {
        let openai = BenchmarkTokenizer::openai_only();
        assert_eq!(openai.len(), 3);
        assert!(openai.iter().all(|t| t.is_native()));
    }

    #[test]
    fn test_benchmark_tokenizer_from_str() {
        assert_eq!(BenchmarkTokenizer::from_str("cl100k"), Some(BenchmarkTokenizer::OpenAiCl100k));
        assert_eq!(BenchmarkTokenizer::from_str("claude"), Some(BenchmarkTokenizer::Claude));
        assert_eq!(BenchmarkTokenizer::from_str("unknown"), None);
    }

    #[test]
    fn test_tokenizer_result() {
        let result = TokenizerResult::new(BenchmarkTokenizer::OpenAiCl100k, 1000, 700);
        assert_eq!(result.tokens_saved, 300);
        assert!((result.savings_percent - 30.0).abs() < 0.1);
    }

    #[test]
    fn test_file_category_expected_savings() {
        let (min, max) = FileCategory::BadgeHeavy.expected_savings();
        assert!(min >= 40.0);
        assert!(max <= 60.0);
    }

    #[test]
    fn test_benchmark_runner_creation() {
        let runner = BenchmarkRunner::new();
        assert!(runner.is_ok());
    }

    #[test]
    fn test_benchmark_runner_openai_only() {
        let runner = BenchmarkRunner::openai_only();
        assert!(runner.is_ok());
    }

    #[test]
    fn test_benchmark_content() {
        let runner = BenchmarkRunner::openai_only().unwrap();
        let (name, content, category) = datasets::badge_heavy_readme();

        let result = runner.benchmark_content(&content, &name, category);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.file_name, name);
        assert_eq!(result.category, FileCategory::BadgeHeavy);
        assert!(!result.tokenizer_results.is_empty());

        // Badge-heavy should have significant savings
        assert!(result.average_savings() > 10.0);
    }

    #[test]
    fn test_benchmark_suite() {
        let runner = BenchmarkRunner::openai_only().unwrap();
        let samples = datasets::all_samples();

        let result = runner.benchmark_suite(&samples);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.file_results.len(), 5);
        assert!(!result.tokenizer_summaries.is_empty());
        assert!(!result.category_summaries.is_empty());
    }

    #[test]
    fn test_markdown_report() {
        let runner = BenchmarkRunner::openai_only().unwrap();
        let samples = datasets::all_samples();
        let result = runner.benchmark_suite(&samples).unwrap();

        let report = BenchmarkReporter::to_markdown(&result);
        assert!(report.contains("# DX Markdown Multi-Tokenizer Benchmark Report"));
        assert!(report.contains("## Tokenizer Summary"));
        assert!(report.contains("cl100k"));
    }

    #[test]
    fn test_json_report() {
        let runner = BenchmarkRunner::openai_only().unwrap();
        let samples = datasets::all_samples();
        let result = runner.benchmark_suite(&samples).unwrap();

        let json = BenchmarkReporter::to_json(&result);
        assert!(json.contains("\"tokenizer_summaries\""));
        assert!(json.contains("\"category_summaries\""));
    }

    #[test]
    fn test_ascii_chart() {
        let runner = BenchmarkRunner::openai_only().unwrap();
        let samples = datasets::all_samples();
        let result = runner.benchmark_suite(&samples).unwrap();

        let chart = BenchmarkReporter::to_ascii_chart(&result);
        assert!(chart.contains("DX Markdown Context Compiler Benchmark"));
        assert!(chart.contains("[#")); // ASCII progress bar
        assert!(chart.contains("+-")); // ASCII table border
    }
}
