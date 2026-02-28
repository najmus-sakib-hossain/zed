//! Grammar pipeline — orchestrates all three tiers and deduplicates results.

use anyhow::Result;

use crate::diagnostic::GrammarDiagnostic;
use crate::harper_tier::HarperTier;
use crate::llm_tier::LlmTier;
use crate::nlprule_tier::NlpruleTier;
use crate::writing_profile::WritingProfile;

/// Configuration for the grammar pipeline.
#[derive(Debug, Clone)]
pub struct GrammarPipelineConfig {
    pub profile: WritingProfile,
    pub enable_harper: bool,
    pub enable_nlprule: bool,
    pub enable_llm: bool,
    /// Maximum text length to process (chars). Longer text is truncated.
    pub max_text_length: usize,
}

impl Default for GrammarPipelineConfig {
    fn default() -> Self {
        Self {
            profile: WritingProfile::default(),
            enable_harper: true,
            enable_nlprule: true,
            enable_llm: false, // Requires API key
            max_text_length: 50_000,
        }
    }
}

/// The three-tier grammar pipeline.
pub struct GrammarPipeline {
    pub config: GrammarPipelineConfig,
    harper: HarperTier,
    nlprule: NlpruleTier,
    llm: LlmTier,
}

impl GrammarPipeline {
    pub fn new(config: GrammarPipelineConfig) -> Self {
        let mut harper = HarperTier::new();
        harper.set_enabled(config.enable_harper);

        let mut nlprule = NlpruleTier::new();
        nlprule.set_enabled(config.enable_nlprule);

        let mut llm = LlmTier::new();
        llm.set_enabled(config.enable_llm);

        Self {
            config,
            harper,
            nlprule,
            llm,
        }
    }

    /// Access the LLM tier for provider configuration.
    pub fn llm_tier_mut(&mut self) -> &mut LlmTier {
        &mut self.llm
    }

    /// Run all enabled tiers and return deduplicated diagnostics.
    pub async fn check(&self, text: &str) -> Result<Vec<GrammarDiagnostic>> {
        let text = if text.len() > self.config.max_text_length {
            &text[..self.config.max_text_length]
        } else {
            text
        };

        let mut all_diagnostics = Vec::new();

        // Tier 1: Harper (< 10ms)
        if let Ok(mut diags) = self.harper.check(text) {
            self.filter_by_profile(&mut diags);
            all_diagnostics.extend(diags);
        }

        // Tier 2: nlprule (< 50ms)
        if let Ok(mut diags) = self.nlprule.check(text) {
            self.filter_by_profile(&mut diags);
            all_diagnostics.extend(diags);
        }

        // Tier 3: LLM (< 500ms)
        if let Ok(mut diags) = self.llm.check(text).await {
            self.filter_by_profile(&mut diags);
            all_diagnostics.extend(diags);
        }

        // Deduplicate across tiers
        Ok(GrammarDiagnostic::deduplicate(all_diagnostics))
    }

    /// Filter diagnostics by the writing profile.
    fn filter_by_profile(&self, diagnostics: &mut Vec<GrammarDiagnostic>) {
        diagnostics.retain(|d| {
            if let Some(ref rule_id) = d.rule_id {
                self.config.profile.is_rule_enabled(rule_id)
            } else {
                true
            }
        });
    }
}
