//! dx_grammar — Three-tier grammar pipeline.
//!
//! Tier 1: Harper (< 10ms, local, basic grammar)
//! Tier 2: nlprule (< 50ms, local, LanguageTool-quality)
//! Tier 3: LLM rewrite (< 500ms, cloud, deep style analysis)
//!
//! Each tier can be enabled/disabled independently. Results combine
//! with deduplication so the user sees a unified list of suggestions.

pub mod detection;
pub mod diagnostic;
pub mod harper_tier;
pub mod llm_tier;
pub mod nlprule_tier;
pub mod pipeline;
pub mod writing_profile;

pub use detection::detect_language;
pub use diagnostic::{GrammarDiagnostic, GrammarSeverity, Span};
pub use pipeline::{GrammarPipeline, GrammarPipelineConfig};
pub use writing_profile::WritingProfile;

/// Initialize the grammar subsystem.
pub fn init(profile: WritingProfile) -> GrammarPipeline {
    GrammarPipeline::new(GrammarPipelineConfig {
        profile,
        ..Default::default()
    })
}
