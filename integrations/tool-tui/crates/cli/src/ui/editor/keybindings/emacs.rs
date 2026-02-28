//! Emacs keybinding handlers

use super::actions::KeyAction;
use super::motions::VimMotion;
use super::EditorKeybindings;
use crate::ui::components::traits::KeyEvent;

/// Handle Emacs keybinding
pub(super) fn handle_emacs_key(_bindings: &mut EditorKeybindings, key: KeyEvent) -> KeyAction {
    match key {
        // Navigation
        KeyEvent::Ctrl('f') | KeyEvent::Right => KeyAction::Move(VimMotion::Right),
        KeyEvent::Ctrl('b') | KeyEvent::Left => KeyAction::Move(VimMotion::Left),
        KeyEvent::Ctrl('n') | KeyEvent::Down => KeyAction::Move(VimMotion::Down),
        KeyEvent::Ctrl('p') | KeyEvent::Up => KeyAction::Move(VimMotion::Up),
        KeyEvent::Ctrl('a') | KeyEvent::Home => KeyAction::Move(VimMotion::LineStart),
        KeyEvent::Ctrl('e') | KeyEvent::End => KeyAction::Move(VimMotion::LineEnd),
        KeyEvent::Alt('f') => KeyAction::Move(VimMotion::WordNext),
        KeyEvent::Alt('b') => KeyAction::Move(VimMotion::WordPrev),
        KeyEvent::Alt('<') => KeyAction::Move(VimMotion::GoToLine(Some(1))),
        KeyEvent::Alt('>') => KeyAction::Move(VimMotion::GoToLine(None)),
        // Editing
        KeyEvent::Ctrl('d') | KeyEvent::Delete => KeyAction::DeleteChar,
        KeyEvent::Backspace => KeyAction::Backspace,
        KeyEvent::Ctrl('k') => KeyAction::Delete(VimMotion::LineEnd),
        KeyEvent::Alt('d') => KeyAction::Delete(VimMotion::WordNext),
        KeyEvent::Ctrl('w') => KeyAction::Delete(VimMotion::WordPrev),
        KeyEvent::Ctrl('y') => KeyAction::Paste,
        KeyEvent::Ctrl('/') | KeyEvent::Ctrl('_') => KeyAction::Undo,
        // Search
        KeyEvent::Ctrl('s') => KeyAction::SearchForward,
        KeyEvent::Ctrl('r') => KeyAction::SearchBackward,
        // File operations
        KeyEvent::Ctrl('x') => {
            // C-x prefix - need to handle next key
            KeyAction::None
        }
        // Text input
        KeyEvent::Char(c) => KeyAction::InsertChar(c),
        KeyEvent::Enter => KeyAction::NewLine,
        KeyEvent::Tab => KeyAction::InsertChar('\t'),
        _ => KeyAction::None,
    }
}
