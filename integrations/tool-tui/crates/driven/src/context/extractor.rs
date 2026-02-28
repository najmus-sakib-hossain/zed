//! Convention extractor

use super::{NamingConventions, NamingStyle, scanner::ScanResult};
use crate::Result;

/// Extracts coding conventions from a project
#[derive(Debug, Default)]
pub struct ConventionExtractor;

impl ConventionExtractor {
    /// Create a new convention extractor
    pub fn new() -> Self {
        Self
    }

    /// Extract naming conventions from scan results
    pub fn extract(&self, scan_result: &ScanResult) -> Result<NamingConventions> {
        let mut conventions = NamingConventions::default();

        // Detect conventions based on languages
        if scan_result.languages.contains(&"Rust".to_string()) {
            conventions.functions = Some(NamingStyle::SnakeCase);
            conventions.types = Some(NamingStyle::PascalCase);
            conventions.variables = Some(NamingStyle::SnakeCase);
            conventions.files = Some(NamingStyle::SnakeCase);
        } else if scan_result.languages.contains(&"TypeScript".to_string())
            || scan_result.languages.contains(&"JavaScript".to_string())
        {
            conventions.functions = Some(NamingStyle::CamelCase);
            conventions.types = Some(NamingStyle::PascalCase);
            conventions.variables = Some(NamingStyle::CamelCase);
            conventions.files = Some(NamingStyle::KebabCase);
        } else if scan_result.languages.contains(&"Python".to_string()) {
            conventions.functions = Some(NamingStyle::SnakeCase);
            conventions.types = Some(NamingStyle::PascalCase);
            conventions.variables = Some(NamingStyle::SnakeCase);
            conventions.files = Some(NamingStyle::SnakeCase);
        } else if scan_result.languages.contains(&"Go".to_string()) {
            conventions.functions = Some(NamingStyle::CamelCase);
            conventions.types = Some(NamingStyle::PascalCase);
            conventions.variables = Some(NamingStyle::CamelCase);
            conventions.files = Some(NamingStyle::SnakeCase);
        }

        Ok(conventions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_conventions() {
        let extractor = ConventionExtractor::new();
        let scan_result = ScanResult {
            languages: vec!["Rust".to_string()],
            ..Default::default()
        };

        let conventions = extractor.extract(&scan_result).unwrap();
        assert_eq!(conventions.functions, Some(NamingStyle::SnakeCase));
        assert_eq!(conventions.types, Some(NamingStyle::PascalCase));
    }

    #[test]
    fn test_extract_typescript_conventions() {
        let extractor = ConventionExtractor::new();
        let scan_result = ScanResult {
            languages: vec!["TypeScript".to_string()],
            ..Default::default()
        };

        let conventions = extractor.extract(&scan_result).unwrap();
        assert_eq!(conventions.functions, Some(NamingStyle::CamelCase));
        assert_eq!(conventions.types, Some(NamingStyle::PascalCase));
    }
}
