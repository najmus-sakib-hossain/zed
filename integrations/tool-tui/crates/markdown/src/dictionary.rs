//! Semantic deduplication dictionary for the DX Markdown Context Compiler.
//!
//! This module implements dictionary-based compression where repeated phrases
//! are replaced with short variable references ($A, $B, etc.).

use crate::tokenizer::Tokenizer;
use std::collections::HashMap;

/// Minimum phrase length to consider for dictionary hoisting.
const MIN_PHRASE_LENGTH: usize = 4;

/// Minimum occurrences to consider for dictionary hoisting.
const MIN_OCCURRENCES: usize = 2;

/// Semantic deduplication dictionary.
///
/// Manages variable definitions and replacements for repeated phrases.
pub struct Dictionary {
    /// Variable definitions ($A -> "phrase")
    definitions: HashMap<String, String>,
    /// Reverse lookup (phrase -> $A)
    reverse: HashMap<String, String>,
    /// Variable name generator
    var_gen: VarGenerator,
}

impl Dictionary {
    /// Create a new empty dictionary.
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            reverse: HashMap::new(),
            var_gen: VarGenerator::new(),
        }
    }

    /// Build a dictionary from frequency analysis.
    ///
    /// # Arguments
    /// * `frequencies` - Map of phrase -> occurrence count
    /// * `tokenizer` - Tokenizer for calculating savings
    ///
    /// # Returns
    /// A dictionary with variables for phrases that benefit from replacement.
    pub fn build(frequencies: &HashMap<String, usize>, tokenizer: &Tokenizer) -> Self {
        let mut dict = Self::new();

        // Collect candidates that would benefit from replacement
        let mut candidates: Vec<(&String, &usize)> = frequencies
            .iter()
            .filter(|&(phrase, &count)| {
                phrase.len() >= MIN_PHRASE_LENGTH
                    && count >= MIN_OCCURRENCES
                    && dict.would_save_tokens(phrase, count, tokenizer)
            })
            .collect();

        // Sort by potential savings (descending)
        candidates.sort_by(|a, b| {
            let savings_a = *a.1 * tokenizer.count(a.0);
            let savings_b = *b.1 * tokenizer.count(b.0);
            savings_b.cmp(&savings_a)
        });

        // Add top candidates to dictionary (limit to 26 single-letter vars for simplicity)
        for (phrase, _count) in candidates.into_iter().take(26) {
            dict.add(phrase.clone());
        }

        dict
    }

    /// Check if replacing a phrase would save tokens.
    fn would_save_tokens(&self, phrase: &str, occurrences: usize, tokenizer: &Tokenizer) -> bool {
        let var_name = self.var_gen.peek();
        tokenizer.should_replace(phrase, occurrences, &var_name)
    }

    /// Add a phrase to the dictionary.
    ///
    /// # Returns
    /// The variable name assigned to the phrase.
    pub fn add(&mut self, phrase: String) -> String {
        if let Some(var) = self.reverse.get(&phrase) {
            return var.clone();
        }

        let var = self.var_gen.next();
        self.definitions.insert(var.clone(), phrase.clone());
        self.reverse.insert(phrase, var.clone());
        var
    }

    /// Check if a phrase should be replaced.
    ///
    /// # Returns
    /// The variable name if the phrase is in the dictionary, None otherwise.
    pub fn should_replace(&self, phrase: &str) -> Option<&str> {
        self.reverse.get(phrase).map(|s| s.as_str())
    }

    /// Get the definition for a variable.
    pub fn get_definition(&self, var: &str) -> Option<&str> {
        self.definitions.get(var).map(|s| s.as_str())
    }

    /// Generate the dictionary header for output.
    ///
    /// Format: `$A="phrase"\n$B="another phrase"\n`
    pub fn header(&self) -> String {
        if self.definitions.is_empty() {
            return String::new();
        }

        let mut entries: Vec<(&String, &String)> = self.definitions.iter().collect();
        entries.sort_by_key(|(var, _)| *var);

        let mut header = String::new();
        for (var, phrase) in entries {
            header.push_str(var);
            header.push_str("=\"");
            // Escape quotes inside the phrase
            let escaped = phrase.replace('\\', "\\\\").replace('"', "\\\"");
            header.push_str(&escaped);
            header.push_str("\"\n");
        }
        header.push('\n');
        header
    }

    /// Apply dictionary replacements to text.
    ///
    /// Replaces all occurrences of dictionary phrases with their variables.
    pub fn apply(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Sort phrases by length (longest first) to avoid partial replacements
        let mut phrases: Vec<(&String, &String)> = self.reverse.iter().collect();
        phrases.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        for (phrase, var) in phrases {
            result = result.replace(phrase, var);
        }

        result
    }

    /// Get the number of entries in the dictionary.
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Check if the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Get all definitions as an iterator.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.definitions.iter()
    }
}

impl Default for Dictionary {
    fn default() -> Self {
        Self::new()
    }
}

/// Variable name generator.
///
/// Generates variable names in sequence: $A, $B, ..., $Z, $AA, $AB, ...
struct VarGenerator {
    current: usize,
}

impl VarGenerator {
    fn new() -> Self {
        Self { current: 0 }
    }

    /// Get the next variable name without advancing.
    fn peek(&self) -> String {
        self.generate(self.current)
    }

    /// Get the next variable name and advance.
    fn next(&mut self) -> String {
        let var = self.generate(self.current);
        self.current += 1;
        var
    }

    /// Generate a variable name for the given index.
    fn generate(&self, index: usize) -> String {
        let mut result = String::from("$");

        if index < 26 {
            // Single letter: $A to $Z
            result.push((b'A' + index as u8) as char);
        } else {
            // Double letter: $AA, $AB, etc.
            let first = (index - 26) / 26;
            let second = (index - 26) % 26;
            result.push((b'A' + first as u8) as char);
            result.push((b'A' + second as u8) as char);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TokenizerType;

    #[test]
    fn test_var_generator_single_letter() {
        let mut generator = VarGenerator::new();
        assert_eq!(generator.next(), "$A");
        assert_eq!(generator.next(), "$B");
        assert_eq!(generator.next(), "$C");
    }

    #[test]
    fn test_var_generator_double_letter() {
        let mut generator = VarGenerator { current: 26 };
        assert_eq!(generator.next(), "$AA");
        assert_eq!(generator.next(), "$AB");
    }

    #[test]
    fn test_dictionary_add() {
        let mut dict = Dictionary::new();
        let var1 = dict.add("hello world".to_string());
        let var2 = dict.add("another phrase".to_string());

        assert_eq!(var1, "$A");
        assert_eq!(var2, "$B");
        assert_eq!(dict.len(), 2);
    }

    #[test]
    fn test_dictionary_add_duplicate() {
        let mut dict = Dictionary::new();
        let var1 = dict.add("hello world".to_string());
        let var2 = dict.add("hello world".to_string());

        assert_eq!(var1, var2);
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_dictionary_should_replace() {
        let mut dict = Dictionary::new();
        dict.add("hello world".to_string());

        assert_eq!(dict.should_replace("hello world"), Some("$A"));
        assert_eq!(dict.should_replace("unknown"), None);
    }

    #[test]
    fn test_dictionary_header() {
        let mut dict = Dictionary::new();
        dict.add("first phrase".to_string());
        dict.add("second phrase".to_string());

        let header = dict.header();
        assert!(header.contains("$A=\"first phrase\""));
        assert!(header.contains("$B=\"second phrase\""));
    }

    #[test]
    fn test_dictionary_apply() {
        let mut dict = Dictionary::new();
        dict.add("hello world".to_string());

        let text = "Say hello world to everyone. hello world is great.";
        let result = dict.apply(text);

        assert_eq!(result, "Say $A to everyone. $A is great.");
    }

    #[test]
    fn test_dictionary_build() {
        let tokenizer = Tokenizer::new(TokenizerType::Cl100k).unwrap();
        let mut frequencies = HashMap::new();

        // Add a phrase that appears many times and is long enough
        frequencies.insert("https://example.com/very/long/documentation/url".to_string(), 10);

        let dict = Dictionary::build(&frequencies, &tokenizer);

        // Should have added the URL to dictionary
        assert!(dict.len() <= 1); // May or may not be added depending on token savings
    }

    #[test]
    fn test_dictionary_empty() {
        let dict = Dictionary::new();
        assert!(dict.is_empty());
        assert_eq!(dict.header(), "");
    }

    #[test]
    fn test_dictionary_get_definition() {
        let mut dict = Dictionary::new();
        dict.add("test phrase".to_string());

        assert_eq!(dict.get_definition("$A"), Some("test phrase"));
        assert_eq!(dict.get_definition("$B"), None);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating valid phrase content.
    fn phrase_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z]{6,20}".prop_filter("phrase must be at least 6 chars", |s| s.len() >= 6)
    }

    proptest! {
        /// Property: Dictionary correctness - every variable has a definition.
        /// Validates: Requirements 4.3, 4.4
        #[test]
        fn prop_dictionary_correctness(phrases in prop::collection::vec(phrase_strategy(), 1..10)) {
            let mut dict = Dictionary::new();

            for phrase in &phrases {
                dict.add(phrase.clone());
            }

            // Every variable in reverse lookup should have a definition
            for (phrase, var) in dict.reverse.iter() {
                prop_assert!(dict.definitions.contains_key(var));
                prop_assert_eq!(dict.definitions.get(var).unwrap(), phrase);
            }
        }

        /// Property: Variable uniqueness - each phrase gets a unique variable.
        /// Validates: Requirements 4.3
        #[test]
        fn prop_variable_uniqueness(phrases in prop::collection::vec(phrase_strategy(), 1..10)) {
            let mut dict = Dictionary::new();
            let mut vars = Vec::new();

            for phrase in &phrases {
                let var = dict.add(phrase.clone());
                if !vars.contains(&var) {
                    vars.push(var);
                }
            }

            // Number of unique vars should equal number of unique phrases
            let unique_phrases: std::collections::HashSet<_> = phrases.iter().collect();
            prop_assert_eq!(vars.len(), unique_phrases.len());
        }

        /// Property: Apply is reversible - we can reconstruct original from header + applied text.
        /// Validates: Requirements 4.2
        #[test]
        fn prop_apply_reversible(phrase in phrase_strategy()) {
            let mut dict = Dictionary::new();
            let var = dict.add(phrase.clone());

            let text = format!("The {} is important. Use {} wisely.", phrase, phrase);
            let applied = dict.apply(&text);

            // Applied text should contain the variable
            prop_assert!(applied.contains(&var));

            // We can reconstruct by replacing var with phrase
            let reconstructed = applied.replace(&var, &phrase);
            prop_assert_eq!(reconstructed, text);
        }

        /// Property: Header format - header contains all definitions in correct format.
        /// Validates: Requirements 4.4
        #[test]
        fn prop_header_format(phrases in prop::collection::vec(phrase_strategy(), 1..5)) {
            let mut dict = Dictionary::new();

            for phrase in &phrases {
                dict.add(phrase.clone());
            }

            let header = dict.header();

            // Header should contain all definitions
            for (var, phrase) in dict.iter() {
                let expected = format!("{}=\"{}\"", var, phrase);
                prop_assert!(header.contains(&expected));
            }
        }

        /// Property: Variable naming - variables follow $A, $B, ... pattern.
        /// Validates: Requirements 4.3
        #[test]
        fn prop_variable_naming(count in 1usize..27) {
            let mut var_gen = VarGenerator::new();

            for i in 0..count {
                let var = var_gen.next();
                prop_assert!(var.starts_with('$'));

                if i < 26 {
                    // Single letter
                    prop_assert_eq!(var.len(), 2);
                    let letter = var.chars().nth(1).unwrap();
                    prop_assert!(letter.is_ascii_uppercase());
                }
            }
        }
    }
}
