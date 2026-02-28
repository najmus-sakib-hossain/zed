//! Editor modes and keybinding modes

/// Editor binding mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KeybindingMode {
    /// Vim-style keybindings
    #[default]
    Vim,
    /// Emacs-style keybindings
    Emacs,
    /// Standard arrow-key navigation
    Standard,
}

/// Vim editing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EditorMode {
    /// Normal mode (navigation and commands)
    #[default]
    Normal,
    /// Insert mode (text entry)
    Insert,
    /// Visual mode (selection)
    Visual,
    /// Visual line mode (line selection)
    VisualLine,
    /// Visual block mode (block selection)
    VisualBlock,
    /// Command mode (: commands)
    Command,
    /// Search mode (/ or ?)
    Search,
    /// Replace mode (R)
    Replace,
}

impl EditorMode {
    /// Get mode indicator string
    pub const fn indicator(&self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Visual => "VISUAL",
            Self::VisualLine => "V-LINE",
            Self::VisualBlock => "V-BLOCK",
            Self::Command => "COMMAND",
            Self::Search => "SEARCH",
            Self::Replace => "REPLACE",
        }
    }

    /// Get short indicator (single char)
    pub const fn short_indicator(&self) -> char {
        match self {
            Self::Normal => 'N',
            Self::Insert => 'I',
            Self::Visual => 'V',
            Self::VisualLine => 'L',
            Self::VisualBlock => 'B',
            Self::Command => ':',
            Self::Search => '/',
            Self::Replace => 'R',
        }
    }
}
