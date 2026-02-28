//! Filler phrase detection and removal for the DX Markdown Context Compiler.
//!
//! This module identifies and removes common filler phrases that add no semantic
//! value to documentation, improving signal-to-noise ratio for LLMs.

/// Common filler phrases that can be safely removed.
/// These phrases add no semantic value and waste tokens.
const FILLER_PATTERNS: &[&str] = &[
    "in this section",
    "as mentioned",
    "as mentioned above",
    "as mentioned below",
    "as mentioned earlier",
    "as mentioned previously",
    "please note",
    "please note that",
    "it is important to",
    "it is important to note",
    "it is worth noting",
    "it's worth noting",
    "it's important to",
    "let's take a look",
    "let's look at",
    "let's see",
    "let's explore",
    "we will discuss",
    "we'll discuss",
    "we will explore",
    "we'll explore",
    "as you can see",
    "as we can see",
    "for more information",
    "for more details",
    "refer to the documentation",
    "see the documentation",
    "in order to",
    "at the end of the day",
    "that being said",
    "having said that",
    "first and foremost",
    "last but not least",
    "needless to say",
    "it goes without saying",
    "as a matter of fact",
    "in fact",
    "basically",
    "essentially",
    "actually",
    "obviously",
    "clearly",
    "simply put",
    "to put it simply",
    "in other words",
    "that is to say",
    "in summary",
    "to summarize",
    "in conclusion",
    "to conclude",
    "moving on",
    "moving forward",
    "going forward",
    "with that said",
    "with that being said",
    "as such",
    "therefore",
    "thus",
    "hence",
    "consequently",
    "as a result",
    "for this reason",
    "for these reasons",
    "due to the fact that",
    "owing to the fact that",
    "in light of",
    "in view of",
    "with respect to",
    "with regard to",
    "in regard to",
    "regarding",
    "concerning",
    "pertaining to",
    "in terms of",
    "when it comes to",
    "as far as",
    "as for",
    "as to",
];

/// Sentence-starting filler phrases that can be removed entirely.
/// These typically start sentences and can be stripped without losing meaning.
const SENTENCE_STARTERS: &[&str] = &[
    "in this section, we",
    "in this section we",
    "in this chapter, we",
    "in this chapter we",
    "in this guide, we",
    "in this guide we",
    "in this tutorial, we",
    "in this tutorial we",
    "in this document, we",
    "in this document we",
    "as mentioned above,",
    "as mentioned earlier,",
    "as mentioned previously,",
    "as noted above,",
    "as noted earlier,",
    "as stated above,",
    "as stated earlier,",
    "please note that",
    "note that",
    "keep in mind that",
    "bear in mind that",
    "it is important to note that",
    "it is worth noting that",
    "it should be noted that",
];

/// Check if a sentence starts with a filler phrase.
///
/// # Arguments
/// * `sentence` - The sentence to check
///
/// # Returns
/// `true` if the sentence starts with a known filler phrase
pub fn starts_with_filler(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    let trimmed = lower.trim();

    for pattern in SENTENCE_STARTERS {
        if trimmed.starts_with(pattern) {
            return true;
        }
    }

    false
}

/// Check if text contains a filler phrase.
///
/// # Arguments
/// * `text` - The text to check
///
/// # Returns
/// `true` if the text contains a known filler phrase
pub fn contains_filler(text: &str) -> bool {
    let lower = text.to_lowercase();

    for pattern in FILLER_PATTERNS {
        if lower.contains(pattern) {
            return true;
        }
    }

    false
}

/// Strip filler phrases from text.
///
/// Removes common filler phrases while preserving the semantic content.
/// Does not modify content inside code blocks.
///
/// # Arguments
/// * `text` - The text to process
///
/// # Returns
/// Text with filler phrases removed
pub fn strip_filler(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_code_block = false;

    for line in text.lines() {
        // Track code blocks
        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Don't modify code blocks
        if in_code_block {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Process the line
        let processed = strip_filler_from_line(line);
        if !processed.trim().is_empty() {
            result.push_str(&processed);
            result.push('\n');
        }
    }

    // Remove trailing newline if original didn't have one
    if !text.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Strip filler phrases from a single line.
fn strip_filler_from_line(line: &str) -> String {
    let trimmed = line.trim();

    // Skip empty lines
    if trimmed.is_empty() {
        return line.to_string();
    }

    // Get indentation
    let indent = line.len() - line.trim_start().len();
    let indent_str: String = " ".repeat(indent);

    // Check for sentence-starting fillers
    let lower = trimmed.to_lowercase();
    for pattern in SENTENCE_STARTERS {
        if lower.starts_with(pattern) {
            let remaining = &trimmed[pattern.len()..].trim_start();
            if !remaining.is_empty() {
                // Capitalize the first letter of remaining text
                let capitalized = capitalize_first(remaining);
                return format!("{}{}", indent_str, capitalized);
            }
        }
    }

    // Remove inline filler phrases
    let mut result = trimmed.to_string();
    for pattern in FILLER_PATTERNS {
        // Case-insensitive replacement
        result = remove_phrase_case_insensitive(&result, pattern);
    }

    // Clean up double spaces
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }

    // Clean up space before punctuation
    result = result.replace(" ,", ",");
    result = result.replace(" .", ".");
    result = result.replace(" ;", ";");
    result = result.replace(" :", ":");

    format!("{}{}", indent_str, result.trim())
}

/// Remove a phrase from text case-insensitively.
fn remove_phrase_case_insensitive(text: &str, pattern: &str) -> String {
    let lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    if let Some(pos) = lower.find(&pattern_lower) {
        let before = &text[..pos];
        let after = &text[pos + pattern.len()..];
        format!("{}{}", before, after)
    } else {
        text.to_string()
    }
}

/// Capitalize the first letter of a string.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Get a list of all filler patterns.
pub fn get_filler_patterns() -> &'static [&'static str] {
    FILLER_PATTERNS
}

/// Get a list of sentence-starting filler patterns.
pub fn get_sentence_starters() -> &'static [&'static str] {
    SENTENCE_STARTERS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starts_with_filler_true() {
        assert!(starts_with_filler("In this section, we will discuss..."));
        assert!(starts_with_filler("As mentioned above, the API..."));
        assert!(starts_with_filler("Please note that this is important"));
        assert!(starts_with_filler("It is important to note that..."));
    }

    #[test]
    fn test_starts_with_filler_false() {
        assert!(!starts_with_filler("The API provides..."));
        assert!(!starts_with_filler("This function returns..."));
        assert!(!starts_with_filler("Users can configure..."));
    }

    #[test]
    fn test_contains_filler_true() {
        assert!(contains_filler("The API, as mentioned above, provides..."));
        assert!(contains_filler("In order to use this feature..."));
        assert!(contains_filler("Basically, this is how it works"));
    }

    #[test]
    fn test_contains_filler_false() {
        assert!(!contains_filler("The API provides authentication"));
        assert!(!contains_filler("This function returns a value"));
    }

    #[test]
    fn test_strip_filler_sentence_starter() {
        let input = "In this section, we will discuss the API.";
        let result = strip_filler(input);
        assert!(!result.to_lowercase().contains("in this section"));
        assert!(result.contains("API"));
    }

    #[test]
    fn test_strip_filler_inline() {
        let input = "The API, basically, provides authentication.";
        let result = strip_filler(input);
        assert!(!result.to_lowercase().contains("basically"));
        assert!(result.contains("API"));
        assert!(result.contains("authentication"));
    }

    #[test]
    fn test_strip_filler_preserves_code_blocks() {
        let input = r#"Some text
```python
# In this section, we define the function
def foo():
    pass
```
More text"#;
        let result = strip_filler(input);
        // Code block content should be preserved
        assert!(result.contains("In this section"));
        assert!(result.contains("def foo():"));
    }

    #[test]
    fn test_strip_filler_preserves_indentation() {
        let input = "    In this section, we will discuss the API.";
        let result = strip_filler(input);
        // Should preserve leading spaces
        assert!(result.starts_with("    "));
    }

    #[test]
    fn test_strip_filler_empty_line() {
        let input = "";
        let result = strip_filler(input);
        assert_eq!(result, "");
    }

    #[test]
    fn test_strip_filler_multiple_fillers() {
        let input = "Basically, in order to use this, you need to configure it.";
        let result = strip_filler(input);
        // At least one filler should be removed
        let lower = result.to_lowercase();
        assert!(!lower.contains("basically") || !lower.contains("in order to"));
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first("HELLO"), "HELLO");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("a"), "A");
    }

    #[test]
    fn test_remove_phrase_case_insensitive() {
        let result = remove_phrase_case_insensitive("Hello BASICALLY world", "basically");
        assert_eq!(result, "Hello  world");
    }

    #[test]
    fn test_get_filler_patterns() {
        let patterns = get_filler_patterns();
        assert!(!patterns.is_empty());
        assert!(patterns.contains(&"basically"));
        assert!(patterns.contains(&"in order to"));
    }

    #[test]
    fn test_get_sentence_starters() {
        let starters = get_sentence_starters();
        assert!(!starters.is_empty());
        assert!(starters.iter().any(|s| s.contains("in this section")));
    }

    #[test]
    fn test_strip_filler_cleans_punctuation() {
        let input = "The API , basically , provides authentication .";
        let result = strip_filler(input);
        // Should clean up space before punctuation
        assert!(!result.contains(" ,"));
        assert!(!result.contains(" ."));
    }

    #[test]
    fn test_strip_filler_multiline() {
        let input = r#"In this section, we discuss the API.
The API provides authentication.
As mentioned above, it is secure."#;
        let result = strip_filler(input);
        // Should process each line
        assert!(result.contains("API"));
        assert!(result.contains("authentication"));
        assert!(result.contains("secure"));
    }
}
