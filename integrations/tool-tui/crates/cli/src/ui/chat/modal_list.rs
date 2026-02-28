use crossterm::event::{KeyCode, KeyModifiers};

#[derive(Debug, Clone)]
pub struct ModalList {
    pub selected: usize,
    pub scroll: usize,
    pub items_count: usize,
    pub wrap_around: bool,
}

impl ModalList {
    pub fn new(items_count: usize) -> Self {
        Self {
            selected: 0,
            scroll: 0,
            items_count,
            wrap_around: true,
        }
    }

    pub fn with_wrap(items_count: usize, wrap_around: bool) -> Self {
        Self {
            selected: 0,
            scroll: 0,
            items_count,
            wrap_around,
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
        visible_items: usize,
    ) -> ModalListAction {
        match (key, modifiers) {
            // Up arrow
            (KeyCode::Up, KeyModifiers::NONE) => {
                if self.selected > 0 {
                    self.selected -= 1;
                    // Ensure selected item is visible by adjusting scroll
                    if self.selected < self.scroll {
                        self.scroll = self.selected;
                    }
                    ModalListAction::SelectionChanged
                } else if self.wrap_around && self.items_count > 0 {
                    // Wrap to bottom
                    self.selected = self.items_count - 1;
                    // Ensure the last item is visible
                    if self.items_count > visible_items {
                        self.scroll = self.items_count - visible_items;
                    } else {
                        self.scroll = 0;
                    }
                    ModalListAction::SelectionChanged
                } else {
                    ModalListAction::None
                }
            }
            // Down arrow
            (KeyCode::Down, KeyModifiers::NONE) => {
                if self.selected < self.items_count.saturating_sub(1) {
                    self.selected += 1;
                    // Scroll when 4 items ahead to account for help text at bottom
                    let scroll_threshold = visible_items.saturating_sub(4);
                    if self.selected >= self.scroll + scroll_threshold {
                        self.scroll = self.selected.saturating_sub(scroll_threshold) + 1;
                    }
                    ModalListAction::SelectionChanged
                } else if self.wrap_around && self.items_count > 0 {
                    // Wrap to top
                    self.selected = 0;
                    self.scroll = 0;
                    ModalListAction::SelectionChanged
                } else {
                    ModalListAction::None
                }
            }
            // Page Up
            (KeyCode::PageUp, KeyModifiers::NONE) => {
                if self.selected > 0 {
                    self.selected = self.selected.saturating_sub(visible_items);
                    // Adjust scroll to keep selection visible
                    if self.selected < self.scroll {
                        self.scroll = self.selected;
                    }
                    ModalListAction::SelectionChanged
                } else {
                    ModalListAction::None
                }
            }
            // Page Down
            (KeyCode::PageDown, KeyModifiers::NONE) => {
                if self.selected < self.items_count.saturating_sub(1) {
                    self.selected = (self.selected + visible_items).min(self.items_count - 1);
                    // Scroll when 4 items ahead to account for help text at bottom
                    let scroll_threshold = visible_items.saturating_sub(4);
                    if self.selected >= self.scroll + scroll_threshold {
                        self.scroll = self.selected.saturating_sub(scroll_threshold) + 1;
                    }
                    ModalListAction::SelectionChanged
                } else {
                    ModalListAction::None
                }
            }
            // Home - Go to first item
            (KeyCode::Home, KeyModifiers::NONE) => {
                if self.selected != 0 {
                    self.selected = 0;
                    self.scroll = 0;
                    ModalListAction::SelectionChanged
                } else {
                    ModalListAction::None
                }
            }
            // End - Go to last item
            (KeyCode::End, KeyModifiers::NONE) => {
                let last = self.items_count.saturating_sub(1);
                if self.selected != last {
                    self.selected = last;
                    // Ensure the last item is visible
                    if self.items_count > visible_items {
                        self.scroll = self.items_count - visible_items;
                    } else {
                        self.scroll = 0;
                    }
                    ModalListAction::SelectionChanged
                } else {
                    ModalListAction::None
                }
            }
            // Enter - Select item
            (KeyCode::Enter, KeyModifiers::NONE) => ModalListAction::ItemSelected(self.selected),
            // Escape - Close
            (KeyCode::Esc, KeyModifiers::NONE) => ModalListAction::Close,
            _ => ModalListAction::None,
        }
    }

    pub fn reset(&mut self) {
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn set_items_count(&mut self, count: usize) {
        self.items_count = count;
        if self.selected >= count {
            self.selected = count.saturating_sub(1);
        }
        if self.scroll > self.selected {
            self.scroll = self.selected;
        }
    }

    pub fn get_visible_range(&self, visible_items: usize) -> (usize, usize) {
        let start = self.scroll;
        let end = (start + visible_items).min(self.items_count);
        (start, end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalListAction {
    None,
    SelectionChanged,
    ItemSelected(usize),
    Close,
}
