//! Parser utilities for extracting FINAL() answers from LLM responses.
//!
//! This module provides functions to detect and extract final answers from
//! LLM responses in the RLM protocol.

use regex::Regex;

/// Checks if a response contains a FINAL() statement.
///
/// # Arguments
///
/// * `response` - The LLM response text to check
///
/// # Returns
///
/// `true` if the response contains "FINAL(", `false` otherwise
///
/// # Examples
///
/// ```
/// use rlm::parser::is_final;
///
/// assert!(is_final("FINAL(\"answer\")"));
/// assert!(!is_final("Still searching..."));
/// ```
pub fn is_final(response: &str) -> bool {
    response.contains("FINAL(")
}

/// Extracts the answer from a FINAL() statement.
///
/// Supports multiple quote styles:
/// - Triple double quotes: `FINAL("""answer""")`
/// - Triple single quotes: `FINAL('''answer''')`
/// - Double quotes: `FINAL("answer")`
/// - Single quotes: `FINAL('answer')`
///
/// # Arguments
///
/// * `response` - The LLM response containing a FINAL() statement
///
/// # Returns
///
/// `Some(String)` with the extracted answer, or `None` if no valid FINAL() found
///
/// # Examples
///
/// ```
/// use rlm::parser::extract_final;
///
/// let response = r#"FINAL("The answer is 42")"#;
/// assert_eq!(extract_final(response), Some("The answer is 42".to_string()));
/// ```
pub fn extract_final(response: &str) -> Option<String> {
    // Try different FINAL patterns in order of preference
    let patterns = vec![
        r#"FINAL\s*\(\s*"""(.*)"""\s*\)"#,  // Triple double quotes
        r#"FINAL\s*\(\s*'''(.*)'''\s*\)"#,  // Triple single quotes
        r#"FINAL\s*\(\s*"([^"]*)"\s*\)"#,   // Double quotes
        r#"FINAL\s*\(\s*'([^']*)'\s*\)"#,   // Single quotes
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(response) {
                if let Some(answer) = caps.get(1) {
                    return Some(answer.as_str().trim().to_string());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_final() {
        assert_eq!(
            extract_final(r#"FINAL("test answer")"#),
            Some("test answer".to_string())
        );

        assert_eq!(
            extract_final(r#"FINAL('test answer')"#),
            Some("test answer".to_string())
        );

        assert_eq!(
            extract_final(r#"Some code\nFINAL("the answer is 42")"#),
            Some("the answer is 42".to_string())
        );

        assert_eq!(
            extract_final("No final here"),
            None
        );
    }
}
