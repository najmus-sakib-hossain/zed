// ─── DX Component Library ───────────────────────────────────────────────────
// Shadcn-ui inspired component library built with GPUI (Zed's GPU-accelerated
// UI framework). All components follow shadcn-ui patterns: composable, themed,
// and highly customizable via builder methods.
//
// Architecture:
//   - Top-level components: standalone UI primitives (Button, Card, etc.)
//   - ui/ module: rich interactive components (Dialog, Tabs, Table, etc.)
//   - All components accept &Theme for consistent theming

// Components are a library — suppress dead_code until consumers are wired up.
#![allow(dead_code)]

// ── Core components ──
pub mod avatar;
pub mod badge;
pub mod button;
pub mod card;
pub mod icon;
pub mod icon_grid;
pub mod input;
pub mod label;
pub mod search_bar;
pub mod separator;
pub mod sidebar;
pub mod titlebar;
pub mod workspace_selector;

// ── Extended component library (ui/) ──
pub mod ui;

// ── Re-exports for ergonomic usage ──
//
// Usage:
//   use crate::components::{Button, Card, Badge, ...};
//   use crate::components::ui::{Dialog, Tabs, Table, ...};

// Core primitives
pub use badge::Badge;
pub use button::{Button, ButtonSize};
pub use icon::Icon;
pub use label::Kbd;
// pub use workspace_selector::WorkspaceSelector;
