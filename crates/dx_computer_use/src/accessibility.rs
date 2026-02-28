//! Accessibility tree — reads UI element hierarchy for intelligent interaction.

use serde::{Deserialize, Serialize};

/// A node in the accessibility tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityNode {
    /// Platform element ID.
    pub id: String,
    /// Role (button, text, input, etc.).
    pub role: String,
    /// Accessible name/label.
    pub name: Option<String>,
    /// Current value (for inputs).
    pub value: Option<String>,
    /// Bounding rectangle.
    pub bounds: Option<Rect>,
    /// Whether the element is focusable.
    pub focusable: bool,
    /// Whether the element is enabled.
    pub enabled: bool,
    /// Child nodes.
    pub children: Vec<AccessibilityNode>,
}

/// Rectangle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn center(&self) -> (i32, i32) {
        (
            self.x + self.width as i32 / 2,
            self.y + self.height as i32 / 2,
        )
    }
}

/// Reads the system accessibility tree.
pub struct AccessibilityTree;

impl AccessibilityTree {
    /// Read the accessibility tree for the focused window.
    pub fn read_focused_window() -> anyhow::Result<AccessibilityNode> {
        // Placeholder — real implementation uses platform accessibility APIs
        // Windows: UI Automation (IUIAutomation)
        // macOS: AXUIElement
        // Linux: AT-SPI2
        log::info!("Reading accessibility tree for focused window");
        Ok(AccessibilityNode {
            id: "root".into(),
            role: "window".into(),
            name: Some("Unknown Window".into()),
            value: None,
            bounds: Some(Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }),
            focusable: true,
            enabled: true,
            children: Vec::new(),
        })
    }

    /// Find a node by name.
    pub fn find_by_name<'a>(root: &'a AccessibilityNode, name: &str) -> Option<&'a AccessibilityNode> {
        if root.name.as_deref() == Some(name) {
            return Some(root);
        }
        for child in &root.children {
            if let Some(found) = Self::find_by_name(child, name) {
                return Some(found);
            }
        }
        None
    }

    /// Find all nodes matching a role.
    pub fn find_by_role<'a>(root: &'a AccessibilityNode, role: &str) -> Vec<&'a AccessibilityNode> {
        let mut results = Vec::new();
        if root.role == role {
            results.push(root);
        }
        for child in &root.children {
            results.extend(Self::find_by_role(child, role));
        }
        results
    }
}
