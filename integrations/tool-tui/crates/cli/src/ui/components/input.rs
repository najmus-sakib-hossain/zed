//! Input component

pub struct Input {
    pub label: String,
    pub placeholder: String,
    pub value: String,
}

impl Input {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            placeholder: String::new(),
            value: String::new(),
        }
    }

    pub fn render(&self) -> String {
        format!("{}: [{}]", self.label, self.value)
    }
}
