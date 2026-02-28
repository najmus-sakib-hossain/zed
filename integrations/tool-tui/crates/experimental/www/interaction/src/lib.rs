//! # dx-interaction — User Action Preservation
//!
//! Preserve user actions (selection, focus, scroll) during DOM updates.
//!
//! ## What It Preserves
//! - Text selection
//! - Input focus
//! - Scroll positions
//! - Cursor position in inputs

#![forbid(unsafe_code)]

/// Binary protocol opcodes
pub mod opcodes {
    pub const INTERACTION_SAVE: u8 = 0xC0;
    pub const INTERACTION_RESTORE: u8 = 0xC1;
}

/// Interaction state snapshot
#[derive(Debug, Clone, Default)]
pub struct InteractionState {
    /// Input element ID and cursor position
    pub focus_element_id: Option<String>,
    pub cursor_position: Option<u32>,

    /// Text selection
    pub selection_start: Option<u32>,
    pub selection_end: Option<u32>,
    pub selection_element_id: Option<String>,

    /// Scroll positions (element ID -> position)
    pub scroll_positions: Vec<(String, i32, i32)>, // (id, x, y)
}

impl InteractionState {
    /// Create new empty state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if state has focus data
    pub fn has_focus(&self) -> bool {
        self.focus_element_id.is_some()
    }

    /// Check if state has selection data
    pub fn has_selection(&self) -> bool {
        self.selection_element_id.is_some()
    }

    /// Check if state has scroll data
    pub fn has_scroll(&self) -> bool {
        !self.scroll_positions.is_empty()
    }
}

/// Selection saver (captures text selection before update)
#[cfg(target_arch = "wasm32")]
pub mod selection {
    use super::*;
    use web_sys::{Selection, window};

    /// Save current selection
    pub fn save() -> Option<InteractionState> {
        let window = window()?;
        let selection = window.get_selection().ok()??;

        if selection.range_count() == 0 {
            return None;
        }

        let range = selection.get_range_at(0).ok()?;
        let start_container = range.start_container()?;
        let end_container = range.end_container()?;

        // Get element ID if available
        let element_id = if let Some(element) = start_container.parent_element() {
            element.id().into()
        } else {
            return None;
        };

        Some(InteractionState {
            selection_start: Some(range.start_offset() as u32),
            selection_end: Some(range.end_offset() as u32),
            selection_element_id: Some(element_id),
            ..Default::default()
        })
    }

    /// Restore selection
    pub fn restore(state: &InteractionState) -> bool {
        if !state.has_selection() {
            return false;
        }

        let window = window().expect("window not available");
        let document = window.document().expect("document not available");

        let element_id = state.selection_element_id.as_ref().unwrap();
        let element = match document.get_element_by_id(element_id) {
            Some(el) => el,
            None => return false,
        };

        let selection = match window.get_selection() {
            Ok(Some(sel)) => sel,
            _ => return false,
        };

        selection.remove_all_ranges().ok()?;

        let range = document.create_range().ok()?;
        if let Some(first_child) = element.first_child() {
            range.set_start(&first_child, state.selection_start.unwrap_or(0)).ok()?;
            range.set_end(&first_child, state.selection_end.unwrap_or(0)).ok()?;
            selection.add_range(&range).ok()?;
        }

        Some(true).is_some()
    }
}

/// Focus tracker (tracks focused element)
#[cfg(target_arch = "wasm32")]
pub mod focus {
    use super::*;
    use web_sys::{HtmlInputElement, HtmlTextAreaElement, window};

    /// Save current focus and cursor position
    pub fn save() -> Option<InteractionState> {
        let window = window()?;
        let document = window.document()?;
        let active_element = document.active_element()?;

        let element_id = active_element.id();
        if element_id.is_empty() {
            return None;
        }

        // Get cursor position for input/textarea
        let cursor_position = if let Some(input) = active_element.dyn_ref::<HtmlInputElement>() {
            input.selection_start().ok()?
        } else if let Some(textarea) = active_element.dyn_ref::<HtmlTextAreaElement>() {
            textarea.selection_start().ok()?
        } else {
            None
        };

        Some(InteractionState {
            focus_element_id: Some(element_id),
            cursor_position,
            ..Default::default()
        })
    }

    /// Restore focus and cursor position
    pub fn restore(state: &InteractionState) -> bool {
        if !state.has_focus() {
            return false;
        }

        let window = window().expect("window not available");
        let document = window.document().expect("document not available");

        let element_id = state.focus_element_id.as_ref().unwrap();
        let element = match document.get_element_by_id(element_id) {
            Some(el) => el,
            None => return false,
        };

        // Focus element
        if let Ok(_) = element.dyn_ref::<web_sys::HtmlElement>().unwrap().focus() {
            // Restore cursor position if applicable
            if let Some(pos) = state.cursor_position {
                if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
                    let _ = input.set_selection_range(pos, pos);
                } else if let Some(textarea) = element.dyn_ref::<HtmlTextAreaElement>() {
                    let _ = textarea.set_selection_range(pos, pos);
                }
            }
            true
        } else {
            false
        }
    }
}

/// Scroll recorder (records scroll positions)
#[cfg(target_arch = "wasm32")]
pub mod scroll {
    use super::*;
    use web_sys::{Element, window};

    /// Save scroll positions of all scrollable elements
    pub fn save() -> InteractionState {
        let window = window().expect("window not available");
        let document = window.document().expect("document not available");

        let mut scroll_positions = Vec::new();

        // Save window scroll
        if let (Some(x), Some(y)) = (window.scroll_x().ok(), window.scroll_y().ok()) {
            scroll_positions.push(("__window__".to_string(), x as i32, y as i32));
        }

        // Find all scrollable elements (simplified - in production, would scan DOM)
        // For now, we'll just illustrate the concept

        InteractionState {
            scroll_positions,
            ..Default::default()
        }
    }

    /// Restore scroll positions
    pub fn restore(state: &InteractionState) -> bool {
        if !state.has_scroll() {
            return false;
        }

        let window = window().expect("window not available");
        let document = window.document().expect("document not available");

        for (id, x, y) in &state.scroll_positions {
            if id == "__window__" {
                let _ = window.scroll_to_with_x_and_y(*x as f64, *y as f64);
            } else {
                if let Some(element) = document.get_element_by_id(id) {
                    element.set_scroll_left(*x);
                    element.set_scroll_top(*y);
                }
            }
        }

        true
    }
}

/// Combined state manager (saves and restores all interactions)
pub struct InteractionManager {
    current_state: Option<InteractionState>,
}

impl InteractionManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            current_state: None,
        }
    }

    /// Save all interaction state
    #[cfg(target_arch = "wasm32")]
    pub fn save(&mut self) {
        let mut state = InteractionState::new();

        // Save focus
        if let Some(focus_state) = focus::save() {
            state.focus_element_id = focus_state.focus_element_id;
            state.cursor_position = focus_state.cursor_position;
        }

        // Save selection
        if let Some(selection_state) = selection::save() {
            state.selection_start = selection_state.selection_start;
            state.selection_end = selection_state.selection_end;
            state.selection_element_id = selection_state.selection_element_id;
        }

        // Save scroll
        let scroll_state = scroll::save();
        state.scroll_positions = scroll_state.scroll_positions;

        self.current_state = Some(state);
    }

    /// Restore all interaction state
    #[cfg(target_arch = "wasm32")]
    pub fn restore(&self) -> bool {
        if let Some(ref state) = self.current_state {
            let mut success = true;

            // Restore in order: scroll -> focus -> selection
            if state.has_scroll() {
                success &= scroll::restore(state);
            }

            if state.has_focus() {
                success &= focus::restore(state);
            }

            if state.has_selection() {
                success &= selection::restore(state);
            }

            success
        } else {
            false
        }
    }

    /// Clear saved state
    pub fn clear(&mut self) {
        self.current_state = None;
    }

    /// Get current saved state
    pub fn get_state(&self) -> Option<&InteractionState> {
        self.current_state.as_ref()
    }
}

impl Default for InteractionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_state() {
        let mut state = InteractionState::new();

        assert!(!state.has_focus());
        assert!(!state.has_selection());
        assert!(!state.has_scroll());

        state.focus_element_id = Some("input-1".to_string());
        state.cursor_position = Some(5);

        assert!(state.has_focus());
        assert_eq!(state.cursor_position, Some(5));
    }

    #[test]
    fn test_manager() {
        let mut manager = InteractionManager::new();

        assert!(manager.get_state().is_none());

        manager.clear();
        assert!(manager.get_state().is_none());
    }
}
