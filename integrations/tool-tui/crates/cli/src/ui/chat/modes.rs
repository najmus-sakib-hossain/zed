use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMode {
    Agent,
    Plan,
    Ask,
}

impl ChatMode {
    pub fn next(&self) -> Self {
        match self {
            Self::Agent => Self::Plan,
            Self::Plan => Self::Ask,
            Self::Ask => Self::Agent,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Agent => Self::Ask,
            Self::Plan => Self::Agent,
            Self::Ask => Self::Plan,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Agent => "▸",
            Self::Plan => "◆",
            Self::Ask => "◉",
        }
    }

    pub fn icon_alt(&self) -> &'static str {
        match self {
            Self::Agent => "›",
            Self::Plan => "■",
            Self::Ask => "○",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Agent => "Execute tasks autonomously",
            Self::Plan => "Create execution plan first",
            Self::Ask => "Ask questions and get answers",
        }
    }
}

impl fmt::Display for ChatMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Agent => write!(f, "Agent"),
            Self::Plan => write!(f, "Plan"),
            Self::Ask => write!(f, "Ask"),
        }
    }
}
