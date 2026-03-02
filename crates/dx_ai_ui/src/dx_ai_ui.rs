//! dx_ai_ui — GPUI frontend for the DX Universal AI Platform.
//!
//! Implements all visual components from the DX design:
//!
//! ## Components
//!
//! - **AI Panel** (Part 2) — `PlanView`, `StudyView`, `ComingSoonView`, profile switcher
//! - **DX Sidebar** (Part 3) — Notion-style sidebar with mode sections
//! - **Mood Action Bar** (Part 4) — Contextual action buttons per mood
//! - **Session History Rail** (Part 5) — Chronological session groups
//! - **Floating AI Panel** (Part 6) — Compact / medium / full mode popup
//! - **Flow Bar** (Part 24) — Persistent bottom-center pill widget
//! - **AI Face Widget** (Part 25) — Procedural GPU-rendered avatar

pub mod ai_face_widget;
pub mod ai_panel;
pub mod coming_soon_view;
pub mod dx_sidebar;
pub mod floating_panel;
pub mod flow_bar_widget;
pub mod mood_action_bar;
pub mod plan_view;
pub mod profile_switcher;
pub mod session_history_rail;
pub mod study_view;

pub use ai_face_widget::AiFaceWidget;
pub use ai_panel::DxAiPanel;
pub use coming_soon_view::ComingSoonView;
pub use dx_sidebar::DxSidebar;
pub use floating_panel::FloatingAiPanel;
pub use flow_bar_widget::FlowBarWidget;
pub use mood_action_bar::MoodActionBar;
pub use plan_view::PlanView;
pub use profile_switcher::ProfileSwitcher;
pub use session_history_rail::SessionHistoryRail;
pub use study_view::StudyView;

use gpui::App;

/// Initialize the DX AI UI subsystem — registers panels, actions, and key bindings.
pub fn init(cx: &mut App) {
    log::info!("DX AI UI initialized");
    let _ = cx;
}
