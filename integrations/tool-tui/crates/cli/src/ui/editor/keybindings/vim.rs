//! Vim keybinding handlers

use super::actions::{KeyAction, PendingOperator};
use super::key_to_string;
use super::modes::EditorMode;
use super::motions::VimMotion;
use super::EditorKeybindings;
use crate::ui::components::traits::KeyEvent;

/// Handle Vim keybinding
pub(super) fn handle_vim_key(bindings: &mut EditorKeybindings, key: KeyEvent) -> KeyAction {
    // Check custom mappings first
    let key_str = key_to_string(&key);
    if let Some(mappings) = bindings.custom_mappings.get(&bindings.editor_mode) {
        if let Some(action) = mappings.get(&key_str) {
            return action.clone();
        }
    }

    match bindings.editor_mode {
        EditorMode::Normal => handle_vim_normal(bindings, key),
        EditorMode::Insert => handle_vim_insert(bindings, key),
        EditorMode::Visual | EditorMode::VisualLine | EditorMode::VisualBlock => {
            handle_vim_visual(bindings, key)
        }
        EditorMode::Command => handle_vim_command(bindings, key),
        EditorMode::Search => handle_vim_search(bindings, key),
        EditorMode::Replace => handle_vim_replace(bindings, key),
    }
}

/// Handle Vim normal mode key
fn handle_vim_normal(bindings: &mut EditorKeybindings, key: KeyEvent) -> KeyAction {
    match key {
        // Count prefix
        KeyEvent::Char(c @ '1'..='9') => {
            let digit = c.to_digit(10).unwrap();
            bindings.count = Some(bindings.count.unwrap_or(0) * 10 + digit);
            KeyAction::None
        }
        KeyEvent::Char('0') if bindings.count.is_some() => {
            bindings.count = Some(bindings.count.unwrap() * 10);
            KeyAction::None
        }

        // Mode changes
        KeyEvent::Char('i') => {
            bindings.set_mode(EditorMode::Insert);
            KeyAction::ChangeMode(EditorMode::Insert)
        }
        KeyEvent::Char('a') => {
            bindings.set_mode(EditorMode::Insert);
            KeyAction::Move(VimMotion::Right)
        }
        KeyEvent::Char('I') => {
            bindings.set_mode(EditorMode::Insert);
            KeyAction::Move(VimMotion::FirstNonBlank)
        }
        KeyEvent::Char('A') => {
            bindings.set_mode(EditorMode::Insert);
            KeyAction::Move(VimMotion::LineEnd)
        }
        KeyEvent::Char('o') => {
            bindings.set_mode(EditorMode::Insert);
            KeyAction::NewLineBelow
        }
        KeyEvent::Char('O') => {
            bindings.set_mode(EditorMode::Insert);
            KeyAction::NewLineAbove
        }
        KeyEvent::Char('R') => {
            bindings.set_mode(EditorMode::Replace);
            KeyAction::ChangeMode(EditorMode::Replace)
        }
        KeyEvent::Char('v') => {
            bindings.set_mode(EditorMode::Visual);
            KeyAction::ChangeMode(EditorMode::Visual)
        }
        KeyEvent::Char('V') => {
            bindings.set_mode(EditorMode::VisualLine);
            KeyAction::ChangeMode(EditorMode::VisualLine)
        }
        KeyEvent::Ctrl('v') => {
            bindings.set_mode(EditorMode::VisualBlock);
            KeyAction::ChangeMode(EditorMode::VisualBlock)
        }
        KeyEvent::Char(':') => {
            bindings.set_mode(EditorMode::Command);
            bindings.command_buffer.clear();
            KeyAction::ChangeMode(EditorMode::Command)
        }
        KeyEvent::Char('/') => {
            bindings.set_mode(EditorMode::Search);
            bindings.search_buffer.clear();
            bindings.search_forward = true;
            KeyAction::SearchForward
        }
        KeyEvent::Char('?') => {
            bindings.set_mode(EditorMode::Search);
            bindings.search_buffer.clear();
            bindings.search_forward = false;
            KeyAction::SearchBackward
        }

        // Basic motions (hjkl)
        KeyEvent::Char('h') | KeyEvent::Left => {
            let count = bindings.count_or_default();
            bindings.count = None;
            KeyAction::MoveCount(VimMotion::Left, count)
        }
        KeyEvent::Char('j') | KeyEvent::Down => {
            let count = bindings.count_or_default();
            bindings.count = None;
            KeyAction::MoveCount(VimMotion::Down, count)
        }
        KeyEvent::Char('k') | KeyEvent::Up => {
            let count = bindings.count_or_default();
            bindings.count = None;
            KeyAction::MoveCount(VimMotion::Up, count)
        }
        KeyEvent::Char('l') | KeyEvent::Right => {
            let count = bindings.count_or_default();
            bindings.count = None;
            KeyAction::MoveCount(VimMotion::Right, count)
        }
        KeyEvent::Char('0') => KeyAction::Move(VimMotion::LineStart),
        KeyEvent::Char('^') => KeyAction::Move(VimMotion::FirstNonBlank),
        KeyEvent::Char('$') => KeyAction::Move(VimMotion::LineEnd),
        KeyEvent::Char('w') => {
            let count = bindings.count_or_default();
            bindings.count = None;
            KeyAction::MoveCount(VimMotion::WordNext, count)
        }
        KeyEvent::Char('e') => {
            let count = bindings.count_or_default();
            bindings.count = None;
            KeyAction::MoveCount(VimMotion::WordEnd, count)
        }
        KeyEvent::Char('b') => {
            let count = bindings.count_or_default();
            bindings.count = None;
            KeyAction::MoveCount(VimMotion::WordPrev, count)
        }
        KeyEvent::Char('W') => {
            let count = bindings.count_or_default();
            bindings.count = None;
            KeyAction::MoveCount(VimMotion::BigWordNext, count)
        }
        KeyEvent::Char('E') => {
            let count = bindings.count_or_default();
            bindings.count = None;
            KeyAction::MoveCount(VimMotion::BigWordEnd, count)
        }
        KeyEvent::Char('B') => {
            let count = bindings.count_or_default();
            bindings.count = None;
            KeyAction::MoveCount(VimMotion::BigWordPrev, count)
        }
        KeyEvent::Char('G') => {
            let line = bindings.count;
            bindings.count = None;
            KeyAction::Move(VimMotion::GoToLine(line))
        }
        KeyEvent::Char('g') => {
            // gg - go to first line
            bindings.count = None;
            KeyAction::Move(VimMotion::GoToLine(Some(1)))
        }
        KeyEvent::Char('}') => KeyAction::Move(VimMotion::ParagraphForward),
        KeyEvent::Char('{') => KeyAction::Move(VimMotion::ParagraphBackward),
        KeyEvent::Char('%') => KeyAction::Move(VimMotion::MatchingBracket),
        KeyEvent::Char('n') => KeyAction::Move(VimMotion::SearchNext),
        KeyEvent::Char('N') => KeyAction::Move(VimMotion::SearchPrev),

        // Operators
        KeyEvent::Char('d') => {
            if bindings.pending_operator == Some(PendingOperator::Delete) {
                // dd - delete line
                bindings.pending_operator = None;
                KeyAction::Delete(VimMotion::Down)
            } else {
                bindings.pending_operator = Some(PendingOperator::Delete);
                KeyAction::None
            }
        }
        KeyEvent::Char('c') => {
            if bindings.pending_operator == Some(PendingOperator::Change) {
                // cc - change line
                bindings.pending_operator = None;
                bindings.set_mode(EditorMode::Insert);
                KeyAction::Change(VimMotion::Down)
            } else {
                bindings.pending_operator = Some(PendingOperator::Change);
                KeyAction::None
            }
        }
        KeyEvent::Char('y') => {
            if bindings.pending_operator == Some(PendingOperator::Yank) {
                // yy - yank line
                bindings.pending_operator = None;
                KeyAction::Yank(VimMotion::Down)
            } else {
                bindings.pending_operator = Some(PendingOperator::Yank);
                KeyAction::None
            }
        }

        // Single-char operations
        KeyEvent::Char('x') => KeyAction::DeleteChar,
        KeyEvent::Char('X') => KeyAction::Backspace,
        KeyEvent::Char('p') => KeyAction::Put,
        KeyEvent::Char('P') => KeyAction::PutBefore,
        KeyEvent::Char('u') => KeyAction::Undo,
        KeyEvent::Ctrl('r') => KeyAction::Redo,
        KeyEvent::Char('.') => bindings.last_action.clone().unwrap_or(KeyAction::None),
        KeyEvent::Char('J') => KeyAction::JoinLines,
        KeyEvent::Char('>') => {
            if bindings.pending_operator == Some(PendingOperator::Indent) {
                bindings.pending_operator = None;
                KeyAction::Indent
            } else {
                bindings.pending_operator = Some(PendingOperator::Indent);
                KeyAction::None
            }
        }
        KeyEvent::Char('<') => {
            if bindings.pending_operator == Some(PendingOperator::Outdent) {
                bindings.pending_operator = None;
                KeyAction::Outdent
            } else {
                bindings.pending_operator = Some(PendingOperator::Outdent);
                KeyAction::None
            }
        }

        // Scroll
        KeyEvent::Ctrl('u') => KeyAction::ScrollUpHalf,
        KeyEvent::Ctrl('d') => KeyAction::ScrollDownHalf,
        KeyEvent::Ctrl('b') | KeyEvent::PageUp => KeyAction::ScrollUpFull,
        KeyEvent::Ctrl('f') | KeyEvent::PageDown => KeyAction::ScrollDownFull,
        KeyEvent::Char('z') => KeyAction::CenterCursor,

        // Macro
        KeyEvent::Char('q') => {
            if let Some(reg) = bindings.recording_macro.take() {
                // Stop recording
                bindings
                    .macros
                    .insert(reg, bindings.macro_buffer.drain(..).collect());
                KeyAction::RecordMacro(reg)
            } else {
                // Start recording (next char is register)
                KeyAction::None
            }
        }
        KeyEvent::Char('@') => {
            // Play macro (next char is register)
            KeyAction::None
        }

        _ => KeyAction::None,
    }
}

/// Handle Vim insert mode key
fn handle_vim_insert(bindings: &mut EditorKeybindings, key: KeyEvent) -> KeyAction {
    match key {
        KeyEvent::Escape => {
            bindings.set_mode(EditorMode::Normal);
            KeyAction::ChangeMode(EditorMode::Normal)
        }
        KeyEvent::Char(c) => KeyAction::InsertChar(c),
        KeyEvent::Backspace => KeyAction::Backspace,
        KeyEvent::Delete => KeyAction::DeleteChar,
        KeyEvent::Enter => KeyAction::NewLine,
        KeyEvent::Tab => KeyAction::InsertChar('\t'),
        KeyEvent::Left => KeyAction::Move(VimMotion::Left),
        KeyEvent::Right => KeyAction::Move(VimMotion::Right),
        KeyEvent::Up => KeyAction::Move(VimMotion::Up),
        KeyEvent::Down => KeyAction::Move(VimMotion::Down),
        KeyEvent::Home => KeyAction::Move(VimMotion::LineStart),
        KeyEvent::End => KeyAction::Move(VimMotion::LineEnd),
        KeyEvent::Ctrl('w') => KeyAction::Delete(VimMotion::WordPrev),
        KeyEvent::Ctrl('u') => KeyAction::Delete(VimMotion::LineStart),
        _ => KeyAction::None,
    }
}

/// Handle Vim visual mode key
fn handle_vim_visual(bindings: &mut EditorKeybindings, key: KeyEvent) -> KeyAction {
    match key {
        KeyEvent::Escape => {
            bindings.set_mode(EditorMode::Normal);
            KeyAction::ChangeMode(EditorMode::Normal)
        }
        // Motions extend selection
        KeyEvent::Char('h') | KeyEvent::Left => KeyAction::Move(VimMotion::Left),
        KeyEvent::Char('j') | KeyEvent::Down => KeyAction::Move(VimMotion::Down),
        KeyEvent::Char('k') | KeyEvent::Up => KeyAction::Move(VimMotion::Up),
        KeyEvent::Char('l') | KeyEvent::Right => KeyAction::Move(VimMotion::Right),
        KeyEvent::Char('w') => KeyAction::Move(VimMotion::WordNext),
        KeyEvent::Char('b') => KeyAction::Move(VimMotion::WordPrev),
        KeyEvent::Char('0') => KeyAction::Move(VimMotion::LineStart),
        KeyEvent::Char('$') => KeyAction::Move(VimMotion::LineEnd),
        KeyEvent::Char('G') => KeyAction::Move(VimMotion::GoToLine(None)),
        KeyEvent::Char('g') => KeyAction::Move(VimMotion::GoToLine(Some(1))),
        // Operations on selection
        KeyEvent::Char('d') | KeyEvent::Char('x') => {
            bindings.set_mode(EditorMode::Normal);
            KeyAction::Cut
        }
        KeyEvent::Char('y') => {
            bindings.set_mode(EditorMode::Normal);
            KeyAction::Copy
        }
        KeyEvent::Char('c') => {
            bindings.set_mode(EditorMode::Insert);
            KeyAction::Cut
        }
        KeyEvent::Char('>') => KeyAction::Indent,
        KeyEvent::Char('<') => KeyAction::Outdent,
        _ => KeyAction::None,
    }
}

/// Handle Vim command mode key
fn handle_vim_command(bindings: &mut EditorKeybindings, key: KeyEvent) -> KeyAction {
    match key {
        KeyEvent::Escape => {
            bindings.set_mode(EditorMode::Normal);
            bindings.command_buffer.clear();
            KeyAction::ChangeMode(EditorMode::Normal)
        }
        KeyEvent::Enter => {
            bindings.set_mode(EditorMode::Normal);
            let cmd = bindings.command_buffer.clone();
            bindings.command_buffer.clear();
            execute_command(&cmd)
        }
        KeyEvent::Backspace => {
            bindings.command_buffer.pop();
            if bindings.command_buffer.is_empty() {
                bindings.set_mode(EditorMode::Normal);
                KeyAction::ChangeMode(EditorMode::Normal)
            } else {
                KeyAction::None
            }
        }
        KeyEvent::Char(c) => {
            bindings.command_buffer.push(c);
            KeyAction::None
        }
        _ => KeyAction::None,
    }
}

/// Handle Vim search mode key
fn handle_vim_search(bindings: &mut EditorKeybindings, key: KeyEvent) -> KeyAction {
    match key {
        KeyEvent::Escape => {
            bindings.set_mode(EditorMode::Normal);
            bindings.search_buffer.clear();
            KeyAction::ChangeMode(EditorMode::Normal)
        }
        KeyEvent::Enter => {
            bindings.set_mode(EditorMode::Normal);
            let pattern = bindings.search_buffer.clone();
            KeyAction::ExecuteCommand(format!("search:{}", pattern))
        }
        KeyEvent::Backspace => {
            bindings.search_buffer.pop();
            if bindings.search_buffer.is_empty() {
                bindings.set_mode(EditorMode::Normal);
                KeyAction::ChangeMode(EditorMode::Normal)
            } else {
                KeyAction::None
            }
        }
        KeyEvent::Char(c) => {
            bindings.search_buffer.push(c);
            KeyAction::None
        }
        _ => KeyAction::None,
    }
}

/// Handle Vim replace mode key
fn handle_vim_replace(bindings: &mut EditorKeybindings, key: KeyEvent) -> KeyAction {
    match key {
        KeyEvent::Escape => {
            bindings.set_mode(EditorMode::Normal);
            KeyAction::ChangeMode(EditorMode::Normal)
        }
        KeyEvent::Char(c) => {
            // Replace char and move right
            KeyAction::InsertChar(c)
        }
        _ => KeyAction::None,
    }
}

/// Execute a : command
fn execute_command(cmd: &str) -> KeyAction {
    let cmd = cmd.trim();
    match cmd {
        "w" | "write" => KeyAction::Save,
        "q" | "quit" => KeyAction::Quit,
        "wq" | "x" => KeyAction::SaveQuit,
        "q!" => KeyAction::ForceQuit,
        "e" | "edit" => KeyAction::OpenFile,
        "sp" | "split" => KeyAction::SplitHorizontal,
        "vs" | "vsplit" => KeyAction::SplitVertical,
        "close" => KeyAction::CloseSplit,
        _ => {
            // Check for line number
            if let Ok(line) = cmd.parse::<u32>() {
                KeyAction::Move(VimMotion::GoToLine(Some(line)))
            } else {
                KeyAction::ExecuteCommand(cmd.to_string())
            }
        }
    }
}
