//! Standard keybinding handlers

use super::actions::KeyAction;
use super::motions::VimMotion;
use super::EditorKeybindings;
use crate::ui::components::traits::KeyEvent;

/// Handle standard keybinding
pub(super) fn handle_standard_key(_bindings: &mut EditorKeybindings, key: KeyEvent) -> KeyAction {
    match key {
        // Navigation
        KeyEvent::Left => KeyAction::Move(VimMotion::Left),
        KeyEvent::Right => KeyAction::Move(VimMotion::Right),
        KeyEvent::Up => KeyAction::Move(VimMotion::Up),
        KeyEvent::Down => KeyAction::Move(VimMotion::Down),
        KeyEvent::Home => KeyAction::Move(VimMotion::LineStart),
        KeyEvent::End => KeyAction::Move(VimMotion::LineEnd),
        KeyEvent::PageUp => KeyAction::ScrollUpFull,
        KeyEvent::PageDown => KeyAction::ScrollDownFull,
        KeyEvent::Ctrl('a') => KeyAction::SelectAll,
        KeyEvent::Ctrl('c') => KeyAction::Copy,
        KeyEvent::Ctrl('x') => KeyAction::Cut,
        KeyEvent::Ctrl('v') => KeyAction::Paste,
        KeyEvent::Ctrl('z') => KeyAction::Undo,
        KeyEvent::Ctrl('y') => KeyAction::Redo,
        KeyEvent::Ctrl('s') => KeyAction::Save,
        KeyEvent::Ctrl('f') => KeyAction::SearchForward,
        // Text input
        KeyEvent::Char(c) => KeyAction::InsertChar(c),
        KeyEvent::Enter => KeyAction::NewLine,
        KeyEvent::Backspace => KeyAction::Backspace,
        KeyEvent::Delete => KeyAction::DeleteChar,
        KeyEvent::Tab => KeyAction::InsertChar('\t'),
        _ => KeyAction::None,
    }
}
