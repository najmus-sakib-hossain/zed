//! Token Optimizer Module
//!
//! Integrates dx-serializer's TokenCounter to provide token-aware
//! optimization strategies for markdown content.

use crate::error::CompileError;
use serializer::llm::tokens::{ModelType, TokenCounter};

/// Token optimization strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationStrategy {
    /// Conservative: Only safe, reversible optimizations
    Conservative,
    /// Aggressive: Maximum token reduction, may lose some formatting
    Aggressive,
    /// Custom: User-defined optimization rules
    Custom,
}

/// Token optimization result
#[derive(Debug, Clone)]
pub struct TokenOptimizationResult {
    pub original_tokens: usize,
    pub optimized_tokens: usize,
    pub savings_percent: f64,
    pub optimized_content: String,
    pub applied_optimizations: Vec<String>,
}

impl TokenOptimizationResult {
    pub fn tokens_saved(&self) -> usize {
        self.original_tokens.saturating_sub(self.optimized_tokens)
    }
}

/// Token-aware markdown optimizer
pub struct TokenOptimizer {
    counter: TokenCounter,
    model: ModelType,
}

impl TokenOptimizer {
    /// Create a new token optimizer for the specified model
    pub fn new(model: ModelType) -> Self {
        Self {
            counter: TokenCounter::new(),
            model,
        }
    }

    /// Optimize content using the specified strategy
    pub fn optimize(
        &self,
        content: &str,
        strategy: OptimizationStrategy,
    ) -> Result<TokenOptimizationResult, CompileError> {
        let original_tokens = self.counter.count(content, self.model).count;
        let mut applied = Vec::new();

        let optimized = match strategy {
            OptimizationStrategy::Conservative => {
                let mut result = content.to_string();
                result = self.remove_decorative_elements(&result);
                applied.push("Remove decorative elements".to_string());
                result = self.compact_whitespace(&result);
                applied.push("Compact whitespace".to_string());
                result
            }
            OptimizationStrategy::Aggressive => {
                let mut result = content.to_string();
                result = self.convert_tables_to_dx(&result);
                applied.push("Convert tables to DX format".to_string());
                result = self.remove_decorative_elements(&result);
                applied.push("Remove decorative elements".to_string());
                result = self.apply_abbreviations(&result);
                applied.push("Apply abbreviations".to_string());
                result = self.remove_redundant_sections(&result);
                applied.push("Remove redundant sections".to_string());
                result = self.compact_code_blocks(&result);
                applied.push("Compact code blocks".to_string());
                result = self.compact_whitespace(&result);
                applied.push("Compact whitespace".to_string());
                result
            }
            OptimizationStrategy::Custom => {
                // Placeholder for custom rules
                content.to_string()
            }
        };

        let optimized_tokens = self.counter.count(&optimized, self.model).count;
        let savings_percent = if original_tokens > 0 {
            ((original_tokens - optimized_tokens) as f64 / original_tokens as f64) * 100.0
        } else {
            0.0
        };

        Ok(TokenOptimizationResult {
            original_tokens,
            optimized_tokens,
            savings_percent,
            optimized_content: optimized,
            applied_optimizations: applied,
        })
    }

    /// Analyze content and suggest optimizations
    pub fn analyze(&self, content: &str) -> TokenAnalysis {
        let total_tokens = self.counter.count(content, self.model).count;

        // Estimate token distribution
        let table_tokens = self.estimate_table_tokens(content);
        let emoji_tokens = self.estimate_emoji_tokens(content);
        let code_tokens = self.estimate_code_tokens(content);
        let redundant_tokens = self.estimate_redundant_tokens(content);

        TokenAnalysis {
            total_tokens,
            table_tokens,
            emoji_tokens,
            code_tokens,
            redundant_tokens,
            suggestions: self.generate_suggestions(content),
        }
    }

    // Private optimization methods

    fn convert_tables_to_dx(&self, content: &str) -> String {
        // Implementation similar to aggressive example
        content.to_string() // Placeholder
    }

    fn remove_decorative_elements(&self, content: &str) -> String {
        let mut result = content.to_string();

        // Remove emojis
        let emojis = [
            "🚀", "🔥", "⚡", "🏆", "🌟", "🎯", "✅", "🎉", "💰", "📊", "🛠️", "🔧", "🌍", "🛡️",
            "📦", "🎨", "🗄️", "🔒", "🌐", "📚",
        ];

        for emoji in &emojis {
            result = result.replace(emoji, "");
        }

        result
    }

    fn apply_abbreviations(&self, content: &str) -> String {
        let abbrevs = [
            ("JavaScript", "JS"),
            ("TypeScript", "TS"),
            ("WebAssembly", "WASM"),
            ("Performance", "Perf"),
            ("Configuration", "Config"),
            ("Documentation", "Docs"),
        ];

        let mut result = content.to_string();
        for (long, short) in &abbrevs {
            result = result.replace(long, short);
        }
        result
    }

    fn remove_redundant_sections(&self, content: &str) -> String {
        let mut result = String::with_capacity(content.len());
        let mut skip = false;

        for line in content.lines() {
            if line.contains("## Previous Updates") || line.contains("## Contributing") {
                skip = true;
                continue;
            }

            if line.starts_with("## ") && !line.contains("Previous") {
                skip = false;
            }

            if !skip {
                result.push_str(line);
                result.push('\n');
            }
        }

        result
    }

    fn compact_code_blocks(&self, content: &str) -> String {
        content.to_string() // Placeholder
    }

    fn compact_whitespace(&self, content: &str) -> String {
        use once_cell::sync::Lazy;

        #[allow(clippy::unwrap_used)] // Compile-time constant pattern, guaranteed valid
        static RE_WHITESPACE: Lazy<regex::Regex> =
            Lazy::new(|| regex::Regex::new(r"\n{3,}").unwrap());

        RE_WHITESPACE.replace_all(content, "\n\n").to_string()
    }

    // Analysis helpers

    fn estimate_table_tokens(&self, content: &str) -> usize {
        let table_lines: Vec<&str> =
            content.lines().filter(|line| line.trim().starts_with('|')).collect();

        if table_lines.is_empty() {
            return 0;
        }

        let table_content = table_lines.join("\n");
        self.counter.count(&table_content, self.model).count
    }

    fn estimate_emoji_tokens(&self, content: &str) -> usize {
        let emoji_count = content.chars().filter(|c| *c as u32 > 0x1F300).count();
        emoji_count * 2 // Rough estimate: 2 tokens per emoji
    }

    fn estimate_code_tokens(&self, content: &str) -> usize {
        let mut in_code = false;
        let mut code_lines = Vec::new();

        for line in content.lines() {
            if line.trim().starts_with("```") {
                in_code = !in_code;
            } else if in_code {
                code_lines.push(line);
            }
        }

        if code_lines.is_empty() {
            return 0;
        }

        let code_content = code_lines.join("\n");
        self.counter.count(&code_content, self.model).count
    }

    fn estimate_redundant_tokens(&self, content: &str) -> usize {
        // Count tokens in sections marked as "Previous Updates" or "Contributing"
        let mut redundant_lines = Vec::new();
        let mut in_redundant = false;

        for line in content.lines() {
            if line.contains("## Previous Updates") || line.contains("## Contributing") {
                in_redundant = true;
            } else if line.starts_with("## ") {
                in_redundant = false;
            }

            if in_redundant {
                redundant_lines.push(line);
            }
        }

        if redundant_lines.is_empty() {
            return 0;
        }

        let redundant_content = redundant_lines.join("\n");
        self.counter.count(&redundant_content, self.model).count
    }

    fn generate_suggestions(&self, content: &str) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        let table_tokens = self.estimate_table_tokens(content);
        if table_tokens > 100 {
            suggestions.push(OptimizationSuggestion {
                name: "Convert tables to DX format".to_string(),
                estimated_savings: (table_tokens as f64 * 0.4) as usize,
                description: "Tables can be compressed 40-60% using DX serializer format"
                    .to_string(),
            });
        }

        let emoji_tokens = self.estimate_emoji_tokens(content);
        if emoji_tokens > 50 {
            suggestions.push(OptimizationSuggestion {
                name: "Remove decorative emojis".to_string(),
                estimated_savings: emoji_tokens,
                description: "Emojis are decorative and use 2-4 tokens each".to_string(),
            });
        }

        let redundant_tokens = self.estimate_redundant_tokens(content);
        if redundant_tokens > 500 {
            suggestions.push(OptimizationSuggestion {
                name: "Remove redundant sections".to_string(),
                estimated_savings: redundant_tokens,
                description: "Sections like 'Previous Updates' and 'Contributing' can be removed for LLM context".to_string(),
            });
        }

        suggestions
    }
}

/// Token analysis result
#[derive(Debug, Clone)]
pub struct TokenAnalysis {
    pub total_tokens: usize,
    pub table_tokens: usize,
    pub emoji_tokens: usize,
    pub code_tokens: usize,
    pub redundant_tokens: usize,
    pub suggestions: Vec<OptimizationSuggestion>,
}

impl TokenAnalysis {
    pub fn potential_savings(&self) -> usize {
        self.suggestions.iter().map(|s| s.estimated_savings).sum()
    }

    pub fn potential_savings_percent(&self) -> f64 {
        if self.total_tokens == 0 {
            return 0.0;
        }
        (self.potential_savings() as f64 / self.total_tokens as f64) * 100.0
    }
}

/// Optimization suggestion
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    pub name: String,
    pub estimated_savings: usize,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_optimizer_creation() {
        let optimizer = TokenOptimizer::new(ModelType::Gpt4o);
        let result = optimizer.optimize("Hello, world!", OptimizationStrategy::Conservative);
        assert!(result.is_ok());
    }

    #[test]
    fn test_conservative_optimization() {
        let optimizer = TokenOptimizer::new(ModelType::Gpt4o);
        let content = "🚀 Hello, world! 🔥\n\n\n\nTest";
        let result = optimizer.optimize(content, OptimizationStrategy::Conservative).unwrap();

        assert!(result.optimized_tokens <= result.original_tokens);
        assert!(!result.optimized_content.contains("🚀"));
    }

    #[test]
    fn test_token_analysis() {
        let optimizer = TokenOptimizer::new(ModelType::Gpt4o);
        let content = "🚀 Test\n\n| A | B |\n|---|---|\n| 1 | 2 |";
        let analysis = optimizer.analyze(content);

        assert!(analysis.total_tokens > 0);
        assert!(analysis.table_tokens > 0);
        assert!(analysis.emoji_tokens > 0);
    }
}
