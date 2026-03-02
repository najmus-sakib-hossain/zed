//! VisualPolish — DX theme tokens, animation helpers, typography hierarchy.
//!
//! Part 30: Shared design tokens and utility components used across all DX UI.

use gpui::{px, Pixels, SharedString};

// ── Typography Scale ──────────────────────────────────────────────

/// Typography hierarchy for DX UI components.
pub struct DxTypography;

impl DxTypography {
    pub fn display() -> Pixels {
        px(32.0)
    }

    pub fn heading_1() -> Pixels {
        px(24.0)
    }

    pub fn heading_2() -> Pixels {
        px(20.0)
    }

    pub fn heading_3() -> Pixels {
        px(16.0)
    }

    pub fn body() -> Pixels {
        px(14.0)
    }

    pub fn body_small() -> Pixels {
        px(13.0)
    }

    pub fn caption() -> Pixels {
        px(12.0)
    }

    pub fn overline() -> Pixels {
        px(10.0)
    }
}

// ── Spacing Scale ─────────────────────────────────────────────────

/// Spacing tokens following a 4px base grid.
pub struct DxSpacing;

impl DxSpacing {
    pub fn xxs() -> Pixels {
        px(2.0)
    }

    pub fn xs() -> Pixels {
        px(4.0)
    }

    pub fn sm() -> Pixels {
        px(8.0)
    }

    pub fn md() -> Pixels {
        px(12.0)
    }

    pub fn lg() -> Pixels {
        px(16.0)
    }

    pub fn xl() -> Pixels {
        px(24.0)
    }

    pub fn xxl() -> Pixels {
        px(32.0)
    }

    pub fn xxxl() -> Pixels {
        px(48.0)
    }
}

// ── Border Radius ─────────────────────────────────────────────────

pub struct DxRadius;

impl DxRadius {
    pub fn none() -> Pixels {
        px(0.0)
    }

    pub fn sm() -> Pixels {
        px(4.0)
    }

    pub fn md() -> Pixels {
        px(6.0)
    }

    pub fn lg() -> Pixels {
        px(8.0)
    }

    pub fn xl() -> Pixels {
        px(12.0)
    }

    pub fn pill() -> Pixels {
        px(9999.0)
    }

    pub fn circle() -> Pixels {
        px(9999.0)
    }
}

// ── Animation Durations ───────────────────────────────────────────

/// Standard animation durations in milliseconds.
pub struct DxDuration;

impl DxDuration {
    pub fn instant() -> std::time::Duration {
        std::time::Duration::from_millis(50)
    }

    pub fn fast() -> std::time::Duration {
        std::time::Duration::from_millis(100)
    }

    pub fn normal() -> std::time::Duration {
        std::time::Duration::from_millis(200)
    }

    pub fn slow() -> std::time::Duration {
        std::time::Duration::from_millis(350)
    }

    pub fn very_slow() -> std::time::Duration {
        std::time::Duration::from_millis(500)
    }
}

// ── Easing Functions ──────────────────────────────────────────────

/// Standard easing curves for DX animations.
pub struct DxEasing;

impl DxEasing {
    /// Cubic ease-out: fast start, slow end.
    pub fn ease_out(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }

    /// Cubic ease-in: slow start, fast end.
    pub fn ease_in(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        t.powi(3)
    }

    /// Cubic ease-in-out: slow start, fast middle, slow end.
    pub fn ease_in_out(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }

    /// Spring-like overshoot ease-out.
    pub fn spring(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        let c4 = (2.0 * std::f32::consts::PI) / 3.0;
        if t == 0.0 || t == 1.0 {
            t
        } else {
            2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
        }
    }

    /// Linear interpolation (no easing).
    pub fn linear(t: f32) -> f32 {
        t.clamp(0.0, 1.0)
    }
}

// ── Icon Size Tokens ──────────────────────────────────────────────

pub struct DxIconSize;

impl DxIconSize {
    pub fn xs() -> Pixels {
        px(12.0)
    }

    pub fn sm() -> Pixels {
        px(16.0)
    }

    pub fn md() -> Pixels {
        px(20.0)
    }

    pub fn lg() -> Pixels {
        px(24.0)
    }

    pub fn xl() -> Pixels {
        px(32.0)
    }
}

// ── Z-Index Layers ────────────────────────────────────────────────

/// Z-index design tokens for layering DX UI components.
pub struct DxZIndex;

impl DxZIndex {
    pub fn base() -> u32 {
        0
    }

    pub fn sidebar() -> u32 {
        10
    }

    pub fn panel() -> u32 {
        20
    }

    pub fn floating_panel() -> u32 {
        30
    }

    pub fn flow_bar() -> u32 {
        40
    }

    pub fn popover() -> u32 {
        50
    }

    pub fn modal() -> u32 {
        100
    }

    pub fn toast() -> u32 {
        200
    }
}

// ── Shared Component Labels ───────────────────────────────────────

/// Common labels used across DX UI.
pub struct DxLabels;

impl DxLabels {
    pub fn app_name() -> SharedString {
        SharedString::from("DX AI")
    }

    pub fn version() -> SharedString {
        SharedString::from("0.1.0-alpha")
    }

    pub fn tagline() -> SharedString {
        SharedString::from("The Universal AI Platform")
    }

    pub fn coming_soon() -> SharedString {
        SharedString::from("Coming Soon")
    }

    pub fn loading() -> SharedString {
        SharedString::from("Loading...")
    }

    pub fn error_generic() -> SharedString {
        SharedString::from("Something went wrong. Please try again.")
    }

    pub fn no_results() -> SharedString {
        SharedString::from("No results found.")
    }
}
