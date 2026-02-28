//! Theme type definitions

/// Color mode for terminal output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Always use colors
    Always,
    /// Never use colors
    Never,
    /// Auto-detect based on terminal capabilities
    #[default]
    Auto,
}
