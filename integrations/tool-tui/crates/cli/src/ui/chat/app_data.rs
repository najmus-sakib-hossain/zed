//! Data structures for chat app

#[derive(Debug, Clone)]
pub struct Task {
    pub title: String,
    pub description: String,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPriority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone)]
pub struct GitChange {
    pub file_path: String,
    pub change_type: ChangeType,
    pub diff: String,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Modified,
    Added,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub status: AgentStatus,
    pub model: String,
    pub task: String,
    pub progress: f32,
    pub tokens_used: usize,
    pub duration: std::time::Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    Paused,
    Completed,
    Failed,
}

impl AgentStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Running => "[RUN]",
            Self::Paused => "[PAUSE]",
            Self::Completed => "[DONE]",
            Self::Failed => "[FAIL]",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        match self {
            Self::Running => ratatui::style::Color::Cyan,
            Self::Paused => ratatui::style::Color::Yellow,
            Self::Completed => ratatui::style::Color::Green,
            Self::Failed => ratatui::style::Color::Red,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    ModeSelector,
    Input,
}

/// Format large numbers with k/M/B suffixes
pub fn format_count(count: usize) -> String {
    if count >= 1_000_000_000 {
        format!("{}B", count / 1_000_000_000)
    } else if count >= 1_000_000 {
        format!("{}M", count / 1_000_000)
    } else if count >= 1_000 {
        format!("{}k", count / 1_000)
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_count() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1k");
        assert_eq!(format_count(1_500), "1k");
        assert_eq!(format_count(999_999), "999k");
        assert_eq!(format_count(1_000_000), "1M");
        assert_eq!(format_count(1_500_000), "1M");
        assert_eq!(format_count(1_000_000_000), "1B");
    }
}
