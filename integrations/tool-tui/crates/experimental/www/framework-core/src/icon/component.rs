//! Icon component data structures

/// Icon component representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IconComponent {
    /// Icon name in "set:name" format (e.g., "heroicons:home")
    pub name: String,
    /// Icon size in pixels (default: 24)
    pub size: u32,
    /// Icon color (optional, defaults to currentColor)
    pub color: Option<String>,
    /// Additional CSS classes
    pub class: Option<String>,
}

impl IconComponent {
    /// Create a new icon component
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            size: 24,
            color: None,
            class: None,
        }
    }

    /// Set the icon size
    pub fn with_size(mut self, size: u32) -> Self {
        self.size = size;
        self
    }

    /// Set the icon color
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set additional CSS classes
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Parse icon name into set and name components
    pub fn parse_name(&self) -> (&str, &str) {
        if let Some(colon_pos) = self.name.find(':') {
            let (set, name) = self.name.split_at(colon_pos);
            (set, &name[1..])
        } else {
            ("lucide", self.name.as_str())
        }
    }

    /// Get the icon set name
    pub fn set(&self) -> &str {
        self.parse_name().0
    }

    /// Get the icon name (without set prefix)
    pub fn icon_name(&self) -> &str {
        self.parse_name().1
    }
}
