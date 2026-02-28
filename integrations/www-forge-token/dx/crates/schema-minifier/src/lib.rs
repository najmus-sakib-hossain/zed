//! # schema-minifier
//!
//! Minifies tool/function JSON schemas to reduce token consumption
//! while preserving enough information for accurate tool selection.
//!
//! ## Evidence (TOKEN.md ⚠️ Partly Real)
//! - Stripping descriptions saves tokens BUT models use them to decide tool selection
//! - Safe minification (defaults, whitespace, examples): **20-35% savings**
//! - Aggressive description stripping: 40-70% but degrades tool selection quality
//! - **Honest savings: 20-35% (safe mode), up to 50% (aggressive)**
//!
//! STAGE: PromptAssembly (priority 20)

use dx_core::*;
use std::sync::Mutex;

/// Minification aggressiveness level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinifyLevel {
    /// Safe: remove defaults, examples, whitespace. Keep descriptions.
    Safe,
    /// Moderate: shorten descriptions to first sentence.
    Moderate,
    /// Aggressive: strip descriptions entirely. Risk: degraded tool selection.
    Aggressive,
}

/// Configuration for schema minification.
#[derive(Debug, Clone)]
pub struct SchemaMinifierConfig {
    /// Minification level
    pub level: MinifyLevel,
    /// Keys to always remove from JSON schemas
    pub strip_keys: Vec<String>,
    /// Maximum description length (0 = no limit, only for Moderate+)
    pub max_description_len: usize,
    /// Tools whose descriptions should NEVER be stripped (critical tools)
    pub protected_tools: Vec<String>,
}

impl Default for SchemaMinifierConfig {
    fn default() -> Self {
        Self {
            level: MinifyLevel::Safe,
            strip_keys: vec![
                "default".into(),
                "examples".into(),
                "example".into(),
                "$comment".into(),
                "deprecated".into(),
                "x-internal".into(),
            ],
            max_description_len: 200,
            protected_tools: vec![],
        }
    }
}

pub struct SchemaMinifierSaver {
    config: SchemaMinifierConfig,
    report: Mutex<TokenSavingsReport>,
}

impl SchemaMinifierSaver {
    pub fn new() -> Self {
        Self::with_config(SchemaMinifierConfig::default())
    }

    pub fn with_config(config: SchemaMinifierConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Minify a JSON schema value recursively.
    fn minify_value(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (key, val) in map {
                    // Strip blacklisted keys
                    if self.config.strip_keys.iter().any(|k| k == key) {
                        continue;
                    }
                    // Handle description based on level
                    if key == "description" {
                        match self.config.level {
                            MinifyLevel::Aggressive => continue, // strip entirely
                            MinifyLevel::Moderate => {
                                if let serde_json::Value::String(s) = val {
                                    let shortened = self.shorten_description(s);
                                    new_map.insert(key.clone(), serde_json::Value::String(shortened));
                                    continue;
                                }
                            }
                            MinifyLevel::Safe => {} // keep as-is
                        }
                    }
                    new_map.insert(key.clone(), self.minify_value(val));
                }
                serde_json::Value::Object(new_map)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.minify_value(v)).collect())
            }
            other => other.clone(),
        }
    }

    /// Shorten a description to its first sentence or max length.
    fn shorten_description(&self, desc: &str) -> String {
        // First sentence
        let first_sentence = desc.split_once(". ")
            .map(|(s, _)| format!("{}.", s))
            .unwrap_or_else(|| desc.to_string());

        if first_sentence.len() <= self.config.max_description_len {
            first_sentence
        } else {
            format!("{}…", &first_sentence[..self.config.max_description_len.saturating_sub(1)])
        }
    }

    /// Estimate token count for a JSON value.
    fn estimate_tokens(value: &serde_json::Value) -> usize {
        let json_str = serde_json::to_string(value).unwrap_or_default();
        json_str.len() / 4 // rough: ~4 chars per token
    }

    /// Minify a single tool schema.
    fn minify_tool(&self, tool: &ToolSchema) -> ToolSchema {
        let is_protected = self.config.protected_tools.contains(&tool.name);

        let new_description = if is_protected {
            tool.description.clone()
        } else {
            match self.config.level {
                MinifyLevel::Aggressive => String::new(),
                MinifyLevel::Moderate => self.shorten_description(&tool.description),
                MinifyLevel::Safe => tool.description.clone(),
            }
        };

        let new_params = if is_protected {
            tool.parameters.clone()
        } else {
            self.minify_value(&tool.parameters)
        };

        let new_token_count = Self::estimate_tokens(&new_params)
            + new_description.len() / 4
            + tool.name.len() / 4
            + 10; // overhead

        ToolSchema {
            name: tool.name.clone(),
            description: new_description,
            parameters: new_params,
            token_count: new_token_count,
        }
    }
}

#[async_trait::async_trait]
impl TokenSaver for SchemaMinifierSaver {
    fn name(&self) -> &str { "schema-minifier" }
    fn stage(&self) -> SaverStage { SaverStage::PromptAssembly }
    fn priority(&self) -> u32 { 20 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.tools.iter().map(|t| t.token_count).sum();

        let minified_tools: Vec<ToolSchema> = input.tools.iter()
            .map(|t| self.minify_tool(t))
            .collect();

        let tokens_after: usize = minified_tools.iter().map(|t| t.token_count).sum();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);
        let pct = if tokens_before > 0 {
            tokens_saved as f64 / tokens_before as f64 * 100.0
        } else { 0.0 };

        let report = TokenSavingsReport {
            technique: "schema-minifier".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Minified {} tool schemas ({:?} level): {} → {} tokens ({:.1}% saved). \
                 Stripped keys: {:?}. Protected: {:?}.",
                minified_tools.len(), self.config.level,
                tokens_before, tokens_after, pct,
                self.config.strip_keys, self.config.protected_tools
            ),
        };
        *self.report.lock().unwrap() = report;

        Ok(SaverOutput {
            messages: input.messages,
            tools: minified_tools,
            images: input.images,
            skipped: false,
            cached_response: None,
        })
    }

    fn last_savings(&self) -> TokenSavingsReport {
        self.report.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big_tool() -> ToolSchema {
        ToolSchema {
            name: "read_file".into(),
            description: "Read the contents of a file from the filesystem. This is a very detailed description that goes on for many tokens.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The absolute path to the file to read from the local filesystem.",
                        "default": "/tmp/file.txt",
                        "examples": ["/home/user/code.rs", "/etc/config.toml"]
                    },
                    "encoding": {
                        "type": "string",
                        "description": "The encoding to use when reading the file contents.",
                        "default": "utf-8",
                        "$comment": "This is an internal comment"
                    }
                },
                "required": ["path"]
            }),
            token_count: 200,
        }
    }

    #[tokio::test]
    async fn test_safe_minification() {
        let saver = SchemaMinifierSaver::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![],
            tools: vec![big_tool()],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        let report = saver.last_savings();
        // Safe mode should save something (removed defaults, examples, comments)
        assert!(report.tokens_saved > 0);
        // Description should still be present
        assert!(!out.tools[0].description.is_empty());
    }

    #[tokio::test]
    async fn test_aggressive_minification() {
        let config = SchemaMinifierConfig {
            level: MinifyLevel::Aggressive,
            ..Default::default()
        };
        let saver = SchemaMinifierSaver::with_config(config);
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![],
            tools: vec![big_tool()],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        // Description should be stripped
        assert!(out.tools[0].description.is_empty());
    }
}
