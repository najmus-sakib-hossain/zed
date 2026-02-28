use crossterm::event::{KeyCode, KeyModifiers};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn simple(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::empty(),
        }
    }

    pub fn ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    pub fn alt(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::ALT,
        }
    }

    pub fn shift(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    pub fn matches(&self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        self.code == code && self.modifiers == modifiers
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::new();

        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift");
        }

        let key = match self.code {
            KeyCode::Char(c) => c.to_uppercase().to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Delete => "Delete".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Up => "↑".to_string(),
            KeyCode::Down => "↓".to_string(),
            KeyCode::Left => "←".to_string(),
            KeyCode::Right => "→".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PgUp".to_string(),
            KeyCode::PageDown => "PgDn".to_string(),
            KeyCode::F(n) => format!("F{}", n),
            _ => "?".to_string(),
        };

        parts.push(&key);
        parts.join("+")
    }
}

#[derive(Debug, Clone)]
pub struct ShortcutAction {
    pub name: String,
    pub description: String,
    pub category: String,
}

pub struct KeyboardShortcutManager {
    bindings: HashMap<KeyBinding, ShortcutAction>,
    enabled: bool,
}

impl KeyboardShortcutManager {
    pub fn new() -> Self {
        let mut manager = Self {
            bindings: HashMap::new(),
            enabled: true,
        };
        manager.register_default_shortcuts();
        manager
    }

    fn register_default_shortcuts(&mut self) {
        // Navigation
        self.register(
            KeyBinding::simple(KeyCode::Up),
            "move_up",
            "Move selection up",
            "Navigation",
        );
        self.register(
            KeyBinding::simple(KeyCode::Down),
            "move_down",
            "Move selection down",
            "Navigation",
        );
        self.register(KeyBinding::simple(KeyCode::Left), "move_left", "Move left", "Navigation");
        self.register(KeyBinding::simple(KeyCode::Right), "move_right", "Move right", "Navigation");
        self.register(
            KeyBinding::simple(KeyCode::Home),
            "move_start",
            "Move to start",
            "Navigation",
        );
        self.register(KeyBinding::simple(KeyCode::End), "move_end", "Move to end", "Navigation");
        self.register(
            KeyBinding::simple(KeyCode::PageUp),
            "page_up",
            "Scroll page up",
            "Navigation",
        );
        self.register(
            KeyBinding::simple(KeyCode::PageDown),
            "page_down",
            "Scroll page down",
            "Navigation",
        );

        // Selection
        self.register(KeyBinding::simple(KeyCode::Enter), "select", "Select/Confirm", "Selection");
        self.register(
            KeyBinding::simple(KeyCode::Char(' ')),
            "toggle",
            "Toggle selection",
            "Selection",
        );
        self.register(
            KeyBinding::ctrl(KeyCode::Char('a')),
            "select_all",
            "Select all",
            "Selection",
        );
        self.register(
            KeyBinding::simple(KeyCode::Esc),
            "clear_selection",
            "Clear selection",
            "Selection",
        );

        // Editing
        self.register(KeyBinding::ctrl(KeyCode::Char('c')), "copy", "Copy", "Editing");
        self.register(KeyBinding::ctrl(KeyCode::Char('x')), "cut", "Cut", "Editing");
        self.register(KeyBinding::ctrl(KeyCode::Char('v')), "paste", "Paste", "Editing");
        self.register(KeyBinding::ctrl(KeyCode::Char('z')), "undo", "Undo", "Editing");
        self.register(KeyBinding::ctrl(KeyCode::Char('y')), "redo", "Redo", "Editing");
        self.register(KeyBinding::simple(KeyCode::Delete), "delete", "Delete", "Editing");
        self.register(KeyBinding::simple(KeyCode::Backspace), "backspace", "Backspace", "Editing");

        // Drag & Drop
        self.register(
            KeyBinding::ctrl(KeyCode::Char('d')),
            "start_drag",
            "Start drag",
            "Drag & Drop",
        );
        self.register(
            KeyBinding::simple(KeyCode::Char('t')),
            "drop_target",
            "Drop to target",
            "Drag & Drop",
        );
        self.register(
            KeyBinding::simple(KeyCode::Char('s')),
            "drop_source",
            "Drop to source",
            "Drag & Drop",
        );

        // Search & Filter
        self.register(KeyBinding::ctrl(KeyCode::Char('f')), "search", "Search/Filter", "Search");
        self.register(KeyBinding::ctrl(KeyCode::Char('g')), "find_next", "Find next", "Search");
        self.register(KeyBinding::ctrl(KeyCode::Char('h')), "find_prev", "Find previous", "Search");

        // View
        self.register(KeyBinding::simple(KeyCode::Tab), "switch_pane", "Switch pane", "View");
        self.register(KeyBinding::ctrl(KeyCode::Char('l')), "refresh", "Refresh", "View");
        self.register(KeyBinding::simple(KeyCode::F(1)), "help", "Show help", "View");
        self.register(KeyBinding::simple(KeyCode::F(5)), "reload", "Reload", "View");

        // Application
        self.register(KeyBinding::simple(KeyCode::Char('q')), "quit", "Quit", "Application");
        self.register(
            KeyBinding::ctrl(KeyCode::Char('q')),
            "force_quit",
            "Force quit",
            "Application",
        );
        self.register(KeyBinding::ctrl(KeyCode::Char('s')), "save", "Save", "Application");
    }

    pub fn register(&mut self, binding: KeyBinding, name: &str, description: &str, category: &str) {
        self.bindings.insert(
            binding,
            ShortcutAction {
                name: name.to_string(),
                description: description.to_string(),
                category: category.to_string(),
            },
        );
    }

    pub fn get_action(&self, code: KeyCode, modifiers: KeyModifiers) -> Option<&ShortcutAction> {
        if !self.enabled {
            return None;
        }

        let binding = KeyBinding::new(code, modifiers);
        self.bindings.get(&binding)
    }

    pub fn get_all_shortcuts(&self) -> Vec<(KeyBinding, &ShortcutAction)> {
        self.bindings.iter().map(|(k, v)| (k.clone(), v)).collect()
    }

    pub fn get_shortcuts_by_category(&self, category: &str) -> Vec<(KeyBinding, &ShortcutAction)> {
        self.bindings
            .iter()
            .filter(|(_, action)| action.category == category)
            .map(|(k, v)| (k.clone(), v))
            .collect()
    }

    pub fn get_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> =
            self.bindings.values().map(|action| action.category.clone()).collect();
        categories.sort();
        categories.dedup();
        categories
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn format_help(&self) -> String {
        let mut output = String::new();
        output.push_str("Keyboard Shortcuts\n");
        output.push_str("==================\n\n");

        for category in self.get_categories() {
            output.push_str(&format!("{}:\n", category));
            let mut shortcuts = self.get_shortcuts_by_category(&category);
            shortcuts.sort_by(|a, b| a.0.display().cmp(&b.0.display()));

            for (binding, action) in shortcuts {
                output.push_str(&format!("  {:20} - {}\n", binding.display(), action.description));
            }
            output.push('\n');
        }

        output
    }
}

impl Default for KeyboardShortcutManager {
    fn default() -> Self {
        Self::new()
    }
}
