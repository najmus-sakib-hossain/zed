//! Interactive components — keyboards, cards, and buttons.
//!
//! Platform-agnostic builder API that channels translate
//! into their native representation (e.g. Telegram inline
//! keyboards, Slack Block Kit, Discord components).

use serde::{Deserialize, Serialize};

/// A keyboard/button layout composed of rows of buttons.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Keyboard {
    /// Ordered rows; each row is a vec of buttons.
    pub rows: Vec<Vec<Button>>,
}

/// A single interactive button.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Button {
    /// Button label.
    pub text: String,
    /// What happens when clicked.
    pub action: ButtonAction,
}

/// Action triggered by a button.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ButtonAction {
    /// Open a URL in the user's browser.
    Url { url: String },
    /// Send callback data to the application.
    Callback { data: String },
    /// Switch to inline query mode with pre-filled text.
    SwitchInline { query: String },
    /// Copy text to clipboard.
    Copy { text: String },
}

impl Keyboard {
    /// Create a new, empty keyboard.
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Append a row of buttons.
    pub fn add_row(&mut self, buttons: Vec<Button>) {
        self.rows.push(buttons);
    }

    /// Builder variant — returns `self` for chaining.
    pub fn row(mut self, buttons: Vec<Button>) -> Self {
        self.rows.push(buttons);
        self
    }

    /// Quick helper: one callback button in its own row.
    pub fn callback_button(self, text: impl Into<String>, data: impl Into<String>) -> Self {
        self.row(vec![Button {
            text: text.into(),
            action: ButtonAction::Callback { data: data.into() },
        }])
    }

    /// Quick helper: one URL button in its own row.
    pub fn url_button(self, text: impl Into<String>, url: impl Into<String>) -> Self {
        self.row(vec![Button {
            text: text.into(),
            action: ButtonAction::Url { url: url.into() },
        }])
    }

    /// Number of rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Total number of buttons across all rows.
    pub fn button_count(&self) -> usize {
        self.rows.iter().map(|r| r.len()).sum()
    }

    /// Whether the keyboard has any buttons.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

impl Button {
    /// Create a callback button.
    pub fn callback(text: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            action: ButtonAction::Callback { data: data.into() },
        }
    }

    /// Create a URL button.
    pub fn url(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            action: ButtonAction::Url { url: url.into() },
        }
    }

    /// Create a copy-to-clipboard button.
    pub fn copy(text: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            action: ButtonAction::Copy {
                text: content.into(),
            },
        }
    }
}

/// A rich card with optional image, title, body and actions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Card {
    /// Optional card title.
    pub title: Option<String>,
    /// Card body / description.
    pub body: Option<String>,
    /// Image URL.
    pub image_url: Option<String>,
    /// Action buttons beneath the card.
    pub buttons: Vec<Button>,
}

impl Card {
    /// Create a new empty card.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the card title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the card body.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set the card image.
    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.image_url = Some(url.into());
        self
    }

    /// Add a button to the card.
    pub fn button(mut self, btn: Button) -> Self {
        self.buttons.push(btn);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_builder() {
        let kb = Keyboard::new()
            .callback_button("Yes", "yes")
            .callback_button("No", "no")
            .url_button("Docs", "https://dx.dev");

        assert_eq!(kb.row_count(), 3);
        assert_eq!(kb.button_count(), 3);
        assert!(!kb.is_empty());
    }

    #[test]
    fn test_keyboard_add_row() {
        let mut kb = Keyboard::new();
        kb.add_row(vec![Button::callback("A", "a"), Button::callback("B", "b")]);
        assert_eq!(kb.row_count(), 1);
        assert_eq!(kb.button_count(), 2);
    }

    #[test]
    fn test_empty_keyboard() {
        let kb = Keyboard::new();
        assert!(kb.is_empty());
        assert_eq!(kb.button_count(), 0);
    }

    #[test]
    fn test_button_constructors() {
        let cb = Button::callback("OK", "ok_data");
        assert_eq!(cb.text, "OK");
        if let ButtonAction::Callback { data } = &cb.action {
            assert_eq!(data, "ok_data");
        } else {
            panic!("Expected Callback");
        }

        let url = Button::url("Open", "https://example.com");
        if let ButtonAction::Url { url: u } = &url.action {
            assert_eq!(u, "https://example.com");
        } else {
            panic!("Expected Url");
        }
    }

    #[test]
    fn test_card_builder() {
        let card = Card::new()
            .title("PR Review")
            .body("Please review this PR")
            .image("https://img.example.com/pr.png")
            .button(Button::callback("Approve", "approve"))
            .button(Button::callback("Reject", "reject"));

        assert_eq!(card.title.as_deref(), Some("PR Review"));
        assert_eq!(card.buttons.len(), 2);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let kb = Keyboard::new().callback_button("Test", "test_data");
        let json = serde_json::to_string(&kb).expect("serialize");
        let deserialized: Keyboard = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.button_count(), 1);
    }
}
