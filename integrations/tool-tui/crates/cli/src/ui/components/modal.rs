//! Modal component

pub struct Modal {
    pub title: String,
    pub content: String,
    pub width: usize,
    pub height: usize,
}

impl Modal {
    pub fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
            width: 60,
            height: 10,
        }
    }

    pub fn render(&self) -> String {
        let border = "═".repeat(self.width - 2);
        let mut output = String::new();

        output.push_str(&format!("╔{}╗\n", border));
        output.push_str(&format!("║ {:^width$} ║\n", self.title, width = self.width - 4));
        output.push_str(&format!("╠{}╣\n", border));

        for line in self.content.lines() {
            output.push_str(&format!("║ {:width$} ║\n", line, width = self.width - 4));
        }

        output.push_str(&format!("╚{}╝", border));
        output
    }
}
