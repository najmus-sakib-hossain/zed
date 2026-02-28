//! Vim motion types for composable commands

/// Vim motion types for composable commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMotion {
    /// Move left (h)
    Left,
    /// Move right (l)
    Right,
    /// Move up (k)
    Up,
    /// Move down (j)
    Down,
    /// Move to start of line (0)
    LineStart,
    /// Move to first non-blank (^)
    FirstNonBlank,
    /// Move to end of line ($)
    LineEnd,
    /// Move to next word (w)
    WordNext,
    /// Move to word end (e)
    WordEnd,
    /// Move to previous word (b)
    WordPrev,
    /// Move to next big word (W)
    BigWordNext,
    /// Move to big word end (E)
    BigWordEnd,
    /// Move to previous big word (B)
    BigWordPrev,
    /// Find character forward (f)
    FindChar(char),
    /// Find character backward (F)
    FindCharBack(char),
    /// Till character forward (t)
    TillChar(char),
    /// Till character backward (T)
    TillCharBack(char),
    /// Go to line (G or gg)
    GoToLine(Option<u32>),
    /// Go to column
    GoToColumn(u32),
    /// Paragraph forward (})
    ParagraphForward,
    /// Paragraph backward ({)
    ParagraphBackward,
    /// Sentence forward ())
    SentenceForward,
    /// Sentence backward (()
    SentenceBackward,
    /// Matching bracket (%)
    MatchingBracket,
    /// Search forward (n)
    SearchNext,
    /// Search backward (N)
    SearchPrev,
}
