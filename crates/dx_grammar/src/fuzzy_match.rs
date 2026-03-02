//! Fuzzy string matching for spelling correction.
//!
//! Uses edit distance (Levenshtein) and phonetic matching to suggest
//! corrections for misspelled words.

/// Calculate Levenshtein edit distance between two strings.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row = vec![0; b_len + 1];

    for (i, a_ch) in a.chars().enumerate() {
        curr_row[0] = i + 1;
        for (j, b_ch) in b.chars().enumerate() {
            let cost = if a_ch == b_ch { 0 } else { 1 };
            curr_row[j + 1] = (curr_row[j] + 1)
                .min(prev_row[j + 1] + 1)
                .min(prev_row[j] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

/// Find the best spelling correction candidates for a word.
///
/// Returns candidates sorted by edit distance (closest first).
pub fn suggest_corrections(word: &str, dictionary: &[&str], max_distance: usize) -> Vec<String> {
    let lower_word = word.to_lowercase();
    let mut candidates: Vec<(String, usize)> = dictionary
        .iter()
        .filter_map(|&dict_word| {
            let dist = edit_distance(&lower_word, dict_word);
            if dist <= max_distance && dist > 0 {
                Some((dict_word.to_string(), dist))
            } else {
                None
            }
        })
        .collect();

    candidates.sort_by_key(|(_, dist)| *dist);
    candidates.into_iter().take(5).map(|(w, _)| w).collect()
}

/// Simple phonetic code (Soundex-like) for English words.
///
/// Groups similar-sounding words together for phonetic matching.
pub fn phonetic_code(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }

    let word = word.to_uppercase();
    let chars: Vec<char> = word.chars().collect();
    let mut code = String::with_capacity(4);

    // Keep first letter
    code.push(chars[0]);

    let soundex_map = |c: char| -> Option<char> {
        match c {
            'B' | 'F' | 'P' | 'V' => Some('1'),
            'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => Some('2'),
            'D' | 'T' => Some('3'),
            'L' => Some('4'),
            'M' | 'N' => Some('5'),
            'R' => Some('6'),
            _ => None,
        }
    };

    let mut last_code = soundex_map(chars[0]);

    for &ch in &chars[1..] {
        if code.len() >= 4 {
            break;
        }
        let current_code = soundex_map(ch);
        if current_code.is_some() && current_code != last_code {
            code.push(current_code.unwrap());
        }
        last_code = current_code;
    }

    // Pad with zeros to length 4
    while code.len() < 4 {
        code.push('0');
    }

    code
}

/// Check if two words are phonetically similar.
pub fn sounds_similar(a: &str, b: &str) -> bool {
    phonetic_code(a) == phonetic_code(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_distance() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", "abc"), 0);
    }

    #[test]
    fn test_phonetic_code() {
        // Robert and Rupert should have the same Soundex
        assert_eq!(phonetic_code("Robert"), phonetic_code("Rupert"));
    }

    #[test]
    fn test_sounds_similar() {
        assert!(sounds_similar("Smith", "Smyth"));
    }
}
