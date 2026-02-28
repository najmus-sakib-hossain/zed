//! Key actions and operators

use super::motions::VimMotion;
use super::modes::EditorMode;

/// Action resulting from a key press
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// No action / key not bound
    None,
    /// Move cursor
    Move(VimMotion),
    /// Move cursor with count
    MoveCount(VimMotion, u32),
    /// Change mode
    ChangeMode(EditorMode),
    /// Delete with motion
    Delete(VimMotion),
    /// Change with motion (delete + enter insert)
    Change(VimMotion),
    /// Yank with motion
    Yank(VimMotion),
    /// Put (paste)
    Put,
    /// Put before cursor
    PutBefore,
    /// Undo
    Undo,
    /// Redo
    Redo,
    /// Insert character
    InsertChar(char),
    /// Delete character under cursor
    DeleteChar,
    /// Delete character before cursor
    Backspace,
    /// Insert newline
    NewLine,
    /// Insert newline below
    NewLineBelow,
    /// Insert newline above
    NewLineAbove,
    /// Join lines
    JoinLines,
    /// Indent
    Indent,
    /// Outdent
    Outdent,
    /// Start search forward
    SearchForward,
    /// Start search backward
    SearchBackward,
    /// Execute command
    ExecuteCommand(String),
    /// Save file
    Save,
    /// Quit
    Quit,
    /// Save and quit
    SaveQuit,
    /// Force quit
    ForceQuit,
    /// Open file
    OpenFile,
    /// Split horizontal
    SplitHorizontal,
    /// Split vertical
    SplitVertical,
    /// Close split
    CloseSplit,
    /// Focus next split
    FocusNextSplit,
    /// Focus previous split
    FocusPrevSplit,
    /// Scroll up half page
    ScrollUpHalf,
    /// Scroll down half page
    ScrollDownHalf,
    /// Scroll up full page
    ScrollUpFull,
    /// Scroll down full page
    ScrollDownFull,
    /// Center cursor line
    CenterCursor,
    /// Toggle fold
    ToggleFold,
    /// Open all folds
    OpenAllFolds,
    /// Close all folds
    CloseAllFolds,
    /// Go to definition
    GoToDefinition,
    /// Show references
    ShowReferences,
    /// Show hover info
    ShowHover,
    /// Format file
    Format,
    /// Comment/uncomment
    ToggleComment,
    /// Repeat last action (.)
    RepeatLast,
    /// Record macro
    RecordMacro(char),
    /// Play macro
    PlayMacro(char),
    /// Select all
    SelectAll,
    /// Copy selection
    Copy,
    /// Cut selection
    Cut,
    /// Paste
    Paste,
}

/// Pending operator for Vim operator-pending mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOperator {
    /// Delete (d)
    Delete,
    /// Change (c)
    Change,
    /// Yank (y)
    Yank,
    /// Indent (>)
    Indent,
    /// Outdent (<)
    Outdent,
    /// Format (gq)
    Format,
}
