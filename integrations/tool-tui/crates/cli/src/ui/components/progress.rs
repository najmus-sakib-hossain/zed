//! Progress bar component

pub struct ProgressBar {
    pub current: usize,
    pub total: usize,
    pub width: usize,
}

impl ProgressBar {
    pub fn new(total: usize) -> Self {
        Self {
            current: 0,
            total,
            width: 40,
        }
    }

    pub fn render(&self) -> String {
        let percent = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as usize
        } else {
            0
        };

        let filled = (self.current as f64 / self.total as f64 * self.width as f64) as usize;
        let empty = self.width - filled;

        format!("[{}{}] {}%", "█".repeat(filled), "░".repeat(empty), percent)
    }
}
