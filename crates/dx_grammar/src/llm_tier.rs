//! LLM tier — deep style analysis using an LLM (< 500ms, cloud).

use anyhow::Result;
use dx_core::{LlmMessage, LlmProvider, LlmRequest, LlmRole};
use std::sync::Arc;

use crate::diagnostic::{DiagnosticSource, GrammarDiagnostic, GrammarSeverity, Span};

/// LLM-powered grammar/style checker.
pub struct LlmTier {
    enabled: bool,
    provider: Option<Arc<dyn LlmProvider>>,
}

impl LlmTier {
    pub fn new() -> Self {
        Self {
            enabled: false, // Disabled by default — requires API key
            provider: None,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled && self.provider.is_some()
    }

    pub fn set_provider(&mut self, provider: Arc<dyn LlmProvider>) {
        self.provider = Some(provider);
    }

    /// Run LLM analysis on the given text.
    pub async fn check(&self, text: &str) -> Result<Vec<GrammarDiagnostic>> {
        if !self.is_enabled() || text.is_empty() {
            return Ok(Vec::new());
        }

        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No LLM provider configured for grammar tier"))?;

        let system_prompt = r#"You are a grammar and style checker. Analyze the following text and return a JSON array of issues. Each issue should have:
- "start": byte offset of the issue start
- "end": byte offset of the issue end
- "severity": one of "spelling", "grammar", "style", "clarity"
- "message": explanation of the issue
- "suggestion": suggested fix (or null)

Return ONLY the JSON array, no other text. If no issues found, return []."#;

        let request = LlmRequest {
            messages: vec![
                LlmMessage {
                    role: LlmRole::System,
                    content: system_prompt.into(),
                    images: Vec::new(),
                },
                LlmMessage {
                    role: LlmRole::User,
                    content: text.into(),
                    images: Vec::new(),
                },
            ],
            max_tokens: Some(1000),
            temperature: Some(0.1),
            model: String::new(),
            top_p: None,
            stop_sequences: Vec::new(),
            stream: false,
        };

        let response = provider.complete(&request).await?;

        parse_llm_diagnostics(&response.content, text.len())
    }
}

impl Default for LlmTier {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse LLM response JSON into diagnostics.
fn parse_llm_diagnostics(json_text: &str, text_len: usize) -> Result<Vec<GrammarDiagnostic>> {
    // Try to extract JSON array from response
    let trimmed = json_text.trim();
    let json_str = if trimmed.starts_with('[') {
        trimmed
    } else if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            &trimmed[start..=end]
        } else {
            return Ok(Vec::new());
        }
    } else {
        return Ok(Vec::new());
    };

    let raw: Vec<RawLlmDiagnostic> = serde_json::from_str(json_str).unwrap_or_default();

    Ok(raw
        .into_iter()
        .filter(|d| d.start < text_len && d.end <= text_len && d.start < d.end)
        .map(|d| GrammarDiagnostic {
            span: Span::new(d.start, d.end),
            severity: match d.severity.as_str() {
                "spelling" => GrammarSeverity::Spelling,
                "grammar" => GrammarSeverity::Grammar,
                "style" => GrammarSeverity::Style,
                "clarity" => GrammarSeverity::Clarity,
                _ => GrammarSeverity::Grammar,
            },
            source: DiagnosticSource::Llm,
            message: d.message,
            suggestion: d.suggestion,
            rule_id: Some("llm_check".into()),
        })
        .collect())
}

#[derive(serde::Deserialize)]
struct RawLlmDiagnostic {
    start: usize,
    end: usize,
    severity: String,
    message: String,
    suggestion: Option<String>,
}
