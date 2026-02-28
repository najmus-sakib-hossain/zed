//! Language detection — simple heuristic to identify text language.

/// Detected language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Language {
    English,
    Spanish,
    French,
    German,
    Portuguese,
    Italian,
    Dutch,
    Russian,
    Chinese,
    Japanese,
    Korean,
    Arabic,
    Unknown,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Spanish => "es",
            Self::French => "fr",
            Self::German => "de",
            Self::Portuguese => "pt",
            Self::Italian => "it",
            Self::Dutch => "nl",
            Self::Russian => "ru",
            Self::Chinese => "zh",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::Arabic => "ar",
            Self::Unknown => "und",
        }
    }
}

/// Simple trigram-based language detection.
/// Returns the most likely language for the given text.
pub fn detect_language(text: &str) -> Language {
    if text.len() < 10 {
        return Language::Unknown;
    }

    let lower = text.to_lowercase();

    // Check for non-Latin scripts first
    if lower.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)) {
        return Language::Chinese;
    }
    if lower.chars().any(|c| ('\u{3040}'..='\u{309f}').contains(&c) || ('\u{30a0}'..='\u{30ff}').contains(&c)) {
        return Language::Japanese;
    }
    if lower.chars().any(|c| ('\u{ac00}'..='\u{d7af}').contains(&c)) {
        return Language::Korean;
    }
    if lower.chars().any(|c| ('\u{0600}'..='\u{06ff}').contains(&c)) {
        return Language::Arabic;
    }
    if lower.chars().any(|c| ('\u{0400}'..='\u{04ff}').contains(&c)) {
        return Language::Russian;
    }

    // Simple word frequency for Latin-script languages
    let words: Vec<&str> = lower.split_whitespace().collect();
    let total = words.len() as f64;
    if total < 3.0 {
        return Language::Unknown;
    }

    let en_words = ["the", "is", "and", "of", "to", "in", "that", "it", "for", "was"];
    let es_words = ["el", "la", "de", "en", "que", "los", "del", "las", "por", "con"];
    let fr_words = ["le", "la", "de", "et", "les", "des", "en", "un", "une", "que"];
    let de_words = ["der", "die", "und", "den", "das", "ist", "ein", "eine", "auf", "dem"];

    let count_matches = |list: &[&str]| -> f64 {
        words.iter().filter(|w| list.contains(w)).count() as f64 / total
    };

    let scores = [
        (Language::English, count_matches(&en_words)),
        (Language::Spanish, count_matches(&es_words)),
        (Language::French, count_matches(&fr_words)),
        (Language::German, count_matches(&de_words)),
    ];

    scores
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .filter(|(_, score)| *score > 0.05)
        .map_or(Language::Unknown, |(lang, _)| *lang)
}
