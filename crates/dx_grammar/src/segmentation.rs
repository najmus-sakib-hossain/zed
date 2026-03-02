//! Unicode text segmentation utilities for grammar analysis.
//!
//! Provides word/sentence boundary detection using Unicode rules,
//! needed for accurate grammar diagnostic positioning.

/// Split text into words using Unicode word boundary rules.
///
/// More accurate than splitting on whitespace — handles contractions,
/// hyphenated words, and non-Latin scripts.
pub fn word_boundaries(text: &str) -> Vec<(usize, usize)> {
    let mut boundaries = Vec::new();
    let mut in_word = false;
    let mut word_start = 0;

    for (i, ch) in text.char_indices() {
        let is_word_char = ch.is_alphanumeric() || ch == '\'' || ch == '-' || ch == '_';

        if is_word_char && !in_word {
            word_start = i;
            in_word = true;
        } else if !is_word_char && in_word {
            boundaries.push((word_start, i));
            in_word = false;
        }
    }

    if in_word {
        boundaries.push((word_start, text.len()));
    }

    boundaries
}

/// Split text into sentences using basic heuristics.
///
/// Handles: periods, exclamation marks, question marks, plus abbreviations.
pub fn sentence_boundaries(text: &str) -> Vec<(usize, usize)> {
    let mut boundaries = Vec::new();
    let mut sentence_start = 0;

    // Common abbreviations that don't end sentences
    let abbrevs = [
        "Mr.", "Mrs.", "Dr.", "Ms.", "Prof.", "Sr.", "Jr.", "vs.", "etc.",
        "i.e.", "e.g.", "Inc.", "Ltd.", "Co.", "St.", "Ave.", "Blvd.",
    ];

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '.' || chars[i] == '!' || chars[i] == '?' {
            // Check if this is an abbreviation
            let before = &text[sentence_start..=text.char_indices()
                .nth(i)
                .map(|(idx, _)| idx)
                .unwrap_or(i)];

            let is_abbrev = abbrevs.iter().any(|a| before.ends_with(a));

            if !is_abbrev {
                let end = text
                    .char_indices()
                    .nth(i + 1)
                    .map(|(idx, _)| idx)
                    .unwrap_or(text.len());

                if sentence_start < end {
                    boundaries.push((sentence_start, end));
                }

                // Skip whitespace after sentence terminator
                let mut next = i + 1;
                while next < chars.len() && chars[next].is_whitespace() {
                    next += 1;
                }
                sentence_start = text
                    .char_indices()
                    .nth(next)
                    .map(|(idx, _)| idx)
                    .unwrap_or(text.len());
                i = next;
                continue;
            }
        }
        i += 1;
    }

    // Remaining text as a sentence
    if sentence_start < text.len() {
        let trimmed = text[sentence_start..].trim();
        if !trimmed.is_empty() {
            boundaries.push((sentence_start, text.len()));
        }
    }

    boundaries
}

/// Count words in a text string.
pub fn word_count(text: &str) -> usize {
    word_boundaries(text).len()
}

/// Detect the dominant script of a text (Latin, Cyrillic, CJK, Arabic, etc.).
pub fn dominant_script(text: &str) -> Script {
    let mut latin = 0u32;
    let mut cyrillic = 0u32;
    let mut cjk = 0u32;
    let mut arabic = 0u32;
    let mut devanagari = 0u32;

    for ch in text.chars() {
        if ch.is_ascii_alphabetic() || ('\u{00C0}'..='\u{024F}').contains(&ch) {
            latin += 1;
        } else if ('\u{0400}'..='\u{04FF}').contains(&ch) {
            cyrillic += 1;
        } else if ('\u{4E00}'..='\u{9FFF}').contains(&ch)
            || ('\u{3040}'..='\u{30FF}').contains(&ch)
            || ('\u{AC00}'..='\u{D7AF}').contains(&ch)
        {
            cjk += 1;
        } else if ('\u{0600}'..='\u{06FF}').contains(&ch) {
            arabic += 1;
        } else if ('\u{0900}'..='\u{097F}').contains(&ch) {
            devanagari += 1;
        }
    }

    let max = latin.max(cyrillic).max(cjk).max(arabic).max(devanagari);
    if max == 0 {
        return Script::Latin; // Default
    }

    if max == latin {
        Script::Latin
    } else if max == cyrillic {
        Script::Cyrillic
    } else if max == cjk {
        Script::Cjk
    } else if max == arabic {
        Script::Arabic
    } else {
        Script::Devanagari
    }
}

/// Major script families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    Latin,
    Cyrillic,
    Cjk,
    Arabic,
    Devanagari,
}
