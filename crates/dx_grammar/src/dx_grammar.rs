//! dx_grammar — Three-tier grammar pipeline (Grammarly replacement).
//!
//! ## Pipeline Tiers
//!
//! - **Tier 1: Harper** (< 10ms, local, basic grammar + spelling)
//! - **Tier 2: nlprule** (< 50ms, local, 4000+ LanguageTool patterns)
//! - **Tier 3: LLM rewrite** (< 500ms, cloud/local, deep style analysis)
//!
//! ## Severity Rendering
//!
//! - 🔴 Red squiggly — definitive errors (misspellings, broken grammar)
//! - 🟡 Yellow squiggly — suggestions (wordiness, passive voice)
//! - 🔵 Blue squiggly — style (stronger synonyms, conciseness)
//! - 💜 Purple squiggly — AI insight (restructuring, tone adjustment)
//!
//! ## Context-Aware Profiles (Part 19)
//!
//! Automatically detects the active application and applies the appropriate
//! writing profile (email → strict, Slack → relaxed, terminal → off).

pub mod app_detection;
pub mod detection;
pub mod diagnostic;
pub mod fuzzy_match;
pub mod harper_tier;
pub mod input_interception;
pub mod llm_tier;
pub mod nlprule_tier;
pub mod pipeline;
pub mod segmentation;
pub mod writing_profile;

pub use app_detection::{detect_category, profile_for_category, AppCategory, AppWritingProfile};
pub use detection::detect_language;
pub use diagnostic::{GrammarDiagnostic, GrammarSeverity, Span};
pub use fuzzy_match::{edit_distance, suggest_corrections};
pub use input_interception::{
    CorrectionCategory, CorrectionOffer, InputEvent, InputInterceptionManager, InputInterceptor,
    create_platform_interceptor,
};
pub use pipeline::{GrammarPipeline, GrammarPipelineConfig};
pub use segmentation::{sentence_boundaries, word_boundaries, word_count};
pub use writing_profile::WritingProfile;

/// Initialize the grammar subsystem.
pub fn init(profile: WritingProfile) -> GrammarPipeline {
    GrammarPipeline::new(GrammarPipelineConfig {
        profile,
        ..Default::default()
    })
}

/// Initialize grammar with auto-detected app context.
pub fn init_for_app(process_name: &str, window_title: &str) -> GrammarPipeline {
    let category = detect_category(process_name, window_title);
    let app_profile = profile_for_category(category);

    log::info!(
        "Grammar: detected {} — strictness={:.1}, tone={:?}",
        category.display_name(),
        app_profile.grammar_strictness,
        app_profile.tone
    );

    let profile = WritingProfile::from_app_profile(&app_profile);
    GrammarPipeline::new(GrammarPipelineConfig {
        profile,
        ..Default::default()
    })
}
