//! Search categories.

use serde::{Deserialize, Serialize};

/// Categories of search supported by the metasearch engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchCategory {
    General,
    Images,
    Videos,
    News,
    Maps,
    Music,
    Science,
    Files,
    SocialMedia,
    IT,
}

impl std::fmt::Display for SearchCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::General => write!(f, "general"),
            Self::Images => write!(f, "images"),
            Self::Videos => write!(f, "videos"),
            Self::News => write!(f, "news"),
            Self::Maps => write!(f, "maps"),
            Self::Music => write!(f, "music"),
            Self::Science => write!(f, "science"),
            Self::Files => write!(f, "files"),
            Self::SocialMedia => write!(f, "social_media"),
            Self::IT => write!(f, "it"),
        }
    }
}
