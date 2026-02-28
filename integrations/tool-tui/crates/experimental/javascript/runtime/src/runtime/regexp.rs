//! RegExp Engine

use crate::error::{DxError, DxResult};

pub struct RegExp {
    pattern: String,
    flags: RegExpFlags,
}

#[derive(Default, Clone)]
pub struct RegExpFlags {
    pub global: bool,
    pub ignorecase: bool,
    pub multiline: bool,
    pub sticky: bool,
    pub unicode: bool,
}

impl RegExp {
    pub fn new(pattern: &str, flags: &str) -> DxResult<Self> {
        let mut f = RegExpFlags::default();
        for c in flags.chars() {
            match c {
                'g' => f.global = true,
                'i' => f.ignorecase = true,
                'm' => f.multiline = true,
                'y' => f.sticky = true,
                'u' => f.unicode = true,
                _ => return Err(DxError::RuntimeError(format!("Invalid flag: {}", c))),
            }
        }
        Ok(Self {
            pattern: pattern.to_string(),
            flags: f,
        })
    }

    pub fn test(&self, input: &str) -> bool {
        self.exec(input).is_some()
    }

    pub fn exec(&self, input: &str) -> Option<RegExpMatch> {
        let pattern = if self.flags.ignorecase {
            self.pattern.to_lowercase()
        } else {
            self.pattern.clone()
        };
        let text = if self.flags.ignorecase {
            input.to_lowercase()
        } else {
            input.to_string()
        };

        text.find(&pattern).map(|pos| RegExpMatch {
            matched: input[pos..pos + pattern.len()].to_string(),
            index: pos,
            input: input.to_string(),
            groups: vec![],
        })
    }

    pub fn replace(&self, input: &str, replacement: &str) -> String {
        if self.flags.global {
            input.replace(&self.pattern, replacement)
        } else {
            input.replacen(&self.pattern, replacement, 1)
        }
    }

    pub fn split(&self, input: &str) -> Vec<String> {
        input.split(&self.pattern).map(String::from).collect()
    }
}

pub struct RegExpMatch {
    pub matched: String,
    pub index: usize,
    pub input: String,
    pub groups: Vec<String>,
}
