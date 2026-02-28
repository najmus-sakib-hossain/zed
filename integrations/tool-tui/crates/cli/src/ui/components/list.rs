//! List component

pub struct List {
    pub items: Vec<String>,
    pub selected: Option<usize>,
}

impl List {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            selected: None,
        }
    }

    pub fn render(&self) -> String {
        self.items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if Some(i) == self.selected {
                    format!("▶ {}", item)
                } else {
                    format!("  {}", item)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
