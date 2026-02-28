pub mod colors;

pub use colors::{Radius, Spacing, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum ThemeMode {
    Light,
    Dark,
}

impl Theme {
    pub fn new(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }
}
