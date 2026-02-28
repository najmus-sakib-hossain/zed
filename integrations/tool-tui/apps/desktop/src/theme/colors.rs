use gpui::{px, rgb, Hsla, Pixels};

// ─── Design Token Constants ─────────────────────────────────────────────────

/// Spacing scale following shadcn-ui / Tailwind conventions.
#[derive(Debug, Clone, Copy)]
pub struct Spacing;

#[allow(dead_code)]
impl Spacing {
    pub const NONE: Pixels = px(0.0);
    pub const PX: Pixels = px(1.0);
    pub const HALF: Pixels = px(2.0);
    pub const ONE: Pixels = px(4.0);
    pub const ONE_HALF: Pixels = px(6.0);
    pub const TWO: Pixels = px(8.0);
    pub const TWO_HALF: Pixels = px(10.0);
    pub const THREE: Pixels = px(12.0);
    pub const THREE_HALF: Pixels = px(14.0);
    pub const FOUR: Pixels = px(16.0);
    pub const FIVE: Pixels = px(20.0);
    pub const SIX: Pixels = px(24.0);
    pub const EIGHT: Pixels = px(32.0);
    pub const TEN: Pixels = px(40.0);
    pub const TWELVE: Pixels = px(48.0);
    pub const SIXTEEN: Pixels = px(64.0);
    pub const TWENTY: Pixels = px(80.0);
    pub const TWENTY_FOUR: Pixels = px(96.0);
}

/// Radius scale matching shadcn-ui
#[derive(Debug, Clone, Copy)]
pub struct Radius;

#[allow(dead_code)]
impl Radius {
    pub const NONE: Pixels = px(0.0);
    pub const SM: Pixels = px(4.0);
    pub const DEFAULT: Pixels = px(6.0);
    pub const MD: Pixels = px(8.0);
    pub const LG: Pixels = px(12.0);
    pub const XL: Pixels = px(16.0);
    pub const XXL: Pixels = px(24.0);
    pub const FULL: Pixels = px(9999.0);
}

// ─── Theme Struct ───────────────────────────────────────────────────────────

/// Complete shadcn-ui compatible theme with extended design tokens.
/// All colors use HSLA for proper alpha compositing on the GPU.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Theme {
    // ── Core semantic colors ──
    pub background: Hsla,
    pub foreground: Hsla,
    pub card: Hsla,
    pub card_foreground: Hsla,
    pub popover: Hsla,
    pub popover_foreground: Hsla,
    pub primary: Hsla,
    pub primary_foreground: Hsla,
    pub secondary: Hsla,
    pub secondary_foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub destructive: Hsla,
    pub destructive_foreground: Hsla,
    pub border: Hsla,
    pub input: Hsla,
    pub ring: Hsla,

    // ── Extended semantic colors ──
    pub success: Hsla,
    pub success_foreground: Hsla,
    pub warning: Hsla,
    pub warning_foreground: Hsla,
    pub info: Hsla,
    pub info_foreground: Hsla,

    // ── Chart colors ──
    pub chart_1: Hsla,
    pub chart_2: Hsla,
    pub chart_3: Hsla,
    pub chart_4: Hsla,
    pub chart_5: Hsla,

    // ── Sidebar ──
    pub sidebar: Hsla,
    pub sidebar_foreground: Hsla,
    pub sidebar_primary: Hsla,
    pub sidebar_primary_foreground: Hsla,
    pub sidebar_accent: Hsla,
    pub sidebar_accent_foreground: Hsla,
    pub sidebar_border: Hsla,
    pub sidebar_ring: Hsla,

    // ── Overlay / transparent ──
    pub overlay: Hsla,
    pub ghost_hover: Hsla,
}

impl Theme {
    /// Light theme aligned with apps/desktop/theme.css tokens.
    pub fn light() -> Self {
        Self {
            background: rgb(0xFCFCFC).into(),
            foreground: rgb(0x000000).into(),
            card: rgb(0xFFFFFF).into(),
            card_foreground: rgb(0x000000).into(),
            popover: rgb(0xFCFCFC).into(),
            popover_foreground: rgb(0x000000).into(),
            primary: rgb(0x000000).into(),
            primary_foreground: rgb(0xFFFFFF).into(),
            secondary: rgb(0xEBEBEB).into(),
            secondary_foreground: rgb(0x000000).into(),
            muted: rgb(0xF5F5F5).into(),
            muted_foreground: rgb(0x525252).into(),
            accent: rgb(0xEBEBEB).into(),
            accent_foreground: rgb(0x000000).into(),
            destructive: rgb(0xE54B4F).into(),
            destructive_foreground: rgb(0xFFFFFF).into(),
            border: rgb(0xE4E4E4).into(),
            input: rgb(0xEBEBEB).into(),
            ring: rgb(0x000000).into(),

            success: rgb(0x22C55E).into(),
            success_foreground: rgb(0xFFFFFF).into(),
            warning: rgb(0xF59E0B).into(),
            warning_foreground: rgb(0x000000).into(),
            info: rgb(0x3B82F6).into(),
            info_foreground: rgb(0xFFFFFF).into(),

            chart_1: rgb(0xFFAE04).into(),
            chart_2: rgb(0x2D62EF).into(),
            chart_3: rgb(0xA4A4A4).into(),
            chart_4: rgb(0xE4E4E4).into(),
            chart_5: rgb(0x747474).into(),

            sidebar: rgb(0xFCFCFC).into(),
            sidebar_foreground: rgb(0x000000).into(),
            sidebar_primary: rgb(0x000000).into(),
            sidebar_primary_foreground: rgb(0xFFFFFF).into(),
            sidebar_accent: rgb(0xEBEBEB).into(),
            sidebar_accent_foreground: rgb(0x000000).into(),
            sidebar_border: rgb(0xEBEBEB).into(),
            sidebar_ring: rgb(0x000000).into(),

            overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5,
            },
            ghost_hover: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.96,
                a: 1.0,
            },
        }
    }

    /// Dark theme aligned with apps/desktop/theme.css tokens.
    pub fn dark() -> Self {
        Self {
            background: rgb(0x000000).into(),
            foreground: rgb(0xFFFFFF).into(),
            card: rgb(0x090909).into(),
            card_foreground: rgb(0xFFFFFF).into(),
            popover: rgb(0x121212).into(),
            popover_foreground: rgb(0xFFFFFF).into(),
            primary: rgb(0xFFFFFF).into(),
            primary_foreground: rgb(0x000000).into(),
            secondary: rgb(0x222222).into(),
            secondary_foreground: rgb(0xFFFFFF).into(),
            muted: rgb(0x1D1D1D).into(),
            muted_foreground: rgb(0xA4A4A4).into(),
            accent: rgb(0x333333).into(),
            accent_foreground: rgb(0xFFFFFF).into(),
            destructive: rgb(0xFF5B5B).into(),
            destructive_foreground: rgb(0x000000).into(),
            border: rgb(0x3A3A3A).into(), // Increased visibility from 0x242424
            input: rgb(0x333333).into(),
            ring: rgb(0xA4A4A4).into(),

            success: rgb(0x22C55E).into(),
            success_foreground: rgb(0x000000).into(),
            warning: rgb(0xF59E0B).into(),
            warning_foreground: rgb(0x000000).into(),
            info: rgb(0x3B82F6).into(),
            info_foreground: rgb(0xFFFFFF).into(),

            chart_1: rgb(0xFFAE04).into(),
            chart_2: rgb(0x2671F4).into(),
            chart_3: rgb(0x747474).into(),
            chart_4: rgb(0x525252).into(),
            chart_5: rgb(0xE4E4E4).into(),

            sidebar: rgb(0x121212).into(),
            sidebar_foreground: rgb(0xFFFFFF).into(),
            sidebar_primary: rgb(0xFFFFFF).into(),
            sidebar_primary_foreground: rgb(0x000000).into(),
            sidebar_accent: rgb(0x333333).into(),
            sidebar_accent_foreground: rgb(0xFFFFFF).into(),
            sidebar_border: rgb(0x333333).into(),
            sidebar_ring: rgb(0xA4A4A4).into(),

            overlay: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.8,
            },
            ghost_hover: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.15,
                a: 1.0,
            },
        }
    }
}
