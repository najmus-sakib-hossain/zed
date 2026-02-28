//! Card component

pub struct Card {
    pub title: Option<String>,
    pub content: String,
    pub width: usize,
}

impl Card {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            title: None,
            content: content.into(),
            width: 60,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        let border = "─".repeat(self.width - 2);

        output.push_str(&format!("┌{}┐\n", border));

        if let Some(title) = &self.title {
            output.push_str(&format!("│ {:width$} │\n", title, width = self.width - 4));
            output.push_str(&format!("├{}┤\n", border));
        }

        for line in self.content.lines() {
            output.push_str(&format!("│ {:width$} │\n", line, width = self.width - 4));
        }

        output.push_str(&format!("└{}┘", border));
        output
    }
}
