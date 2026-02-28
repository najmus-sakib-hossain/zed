//! AI Profile system — six distinct operational modes for DX.
//!
//! Each profile transforms the entire AI panel content and behavior.

use serde::{Deserialize, Serialize};

/// The six AI profiles available in DX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiProfile {
    /// Standard AI chat — the default conversational assistant.
    Chat,
    /// Code generation and editing mode.
    Code,
    /// Planning mode — generates plans, todos, roadmaps.
    Plan,
    /// Study mode — 3-column layout: sources/chat/studio.
    Study,
    /// Deep research mode — extended multi-step research.
    DeepResearch,
    /// Search mode — web and knowledge base search.
    Search,
}

impl AiProfile {
    /// All profiles in display order.
    pub fn all() -> &'static [AiProfile] {
        &[
            AiProfile::Chat,
            AiProfile::Code,
            AiProfile::Plan,
            AiProfile::Study,
            AiProfile::DeepResearch,
            AiProfile::Search,
        ]
    }

    /// Human-readable name.
    pub fn display_name(&self) -> &'static str {
        match self {
            AiProfile::Chat => "Chat",
            AiProfile::Code => "Code",
            AiProfile::Plan => "Plan",
            AiProfile::Study => "Study",
            AiProfile::DeepResearch => "Deep Research",
            AiProfile::Search => "Search",
        }
    }

    /// Icon name for the profile.
    pub fn icon_name(&self) -> &'static str {
        match self {
            AiProfile::Chat => "message-circle",
            AiProfile::Code => "code",
            AiProfile::Plan => "list-checks",
            AiProfile::Study => "book-open",
            AiProfile::DeepResearch => "microscope",
            AiProfile::Search => "search",
        }
    }

    /// Description of what this profile does.
    pub fn description(&self) -> &'static str {
        match self {
            AiProfile::Chat => "General-purpose AI assistant for questions and conversation",
            AiProfile::Code => "Focused code generation, editing, and debugging",
            AiProfile::Plan => "Create plans, roadmaps, and task breakdowns",
            AiProfile::Study => "Research with sources, notes, and a workspace studio",
            AiProfile::DeepResearch => "Extended multi-step research with reasoning chains",
            AiProfile::Search => "Search the web and knowledge bases for information",
        }
    }

    /// Whether the profile is currently implemented (vs. coming soon).
    pub fn is_implemented(&self) -> bool {
        matches!(self, AiProfile::Chat | AiProfile::Code | AiProfile::Plan)
    }

    /// Keyboard shortcut hint.
    pub fn shortcut_hint(&self) -> Option<&'static str> {
        match self {
            AiProfile::Chat => Some("Alt+1"),
            AiProfile::Code => Some("Alt+2"),
            AiProfile::Plan => Some("Alt+3"),
            AiProfile::Study => Some("Alt+4"),
            AiProfile::DeepResearch => Some("Alt+5"),
            AiProfile::Search => Some("Alt+6"),
            _ => None,
        }
    }
}
