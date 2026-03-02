//! DX UI — All user-facing DX components rendered by GPUI.
//!
//! This crate provides the visual layer for the Universal AI Platform:
//! - Profile switcher (Chat, Code, Plan, Study, Deep Research, Search)
//! - Plan/Study/ComingSoon views
//! - Notion-style left sidebar
//! - Mood/media action bar toggle
//! - Session history rail
//! - Floating AI panel (compact, medium, full modes)
//! - Flow Bar (persistent bottom-center voice widget)
//! - AI Face Widget (procedural GPU-rendered avatar)
//! - Hardware tier display and manual override
//! - Visual polish tokens and animation helpers
//! - Social sharing UI popover
//! - Media preview panels (image, video, audio, 3D, PDF)
//! - Model download progress UI

mod ai_face_widget;
mod coming_soon_view;
mod dx_sidebar;
mod floating_ai_panel;
mod flow_bar_ui;
mod media_preview;
mod model_download_ui;
mod mood_action_bar;
mod plan_view;
mod profile_switcher;
mod session_history_rail;
mod social_share_ui;
mod study_view;
mod tier_display;
mod visual_polish;

pub use ai_face_widget::AiFaceWidget;
pub use coming_soon_view::ComingSoonView;
pub use dx_sidebar::DxSidebar;
pub use floating_ai_panel::{FloatingAiPanel, FloatingPanelMode};
pub use flow_bar_ui::FlowBarUi;
pub use media_preview::{AudioPreview, ImagePreview, PdfPreview, ThreeDPreview, VideoPreview};
pub use model_download_ui::ModelDownloadUi;
pub use mood_action_bar::MoodActionBar;
pub use plan_view::PlanView;
pub use profile_switcher::ProfileSwitcher;
pub use session_history_rail::SessionHistoryRail;
pub use social_share_ui::SocialShareUi;
pub use study_view::StudyView;
pub use tier_display::TierDisplay;
pub use visual_polish::{DxAnimation, DxThemeTokens};

use gpui::App;

/// Initialize the DX UI subsystem.
pub fn init(_cx: &mut App) {
    log::info!("DX UI initialized");
}
