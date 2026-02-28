//! Property-Based Tests for Schema Parsing with Space Separators
//!
//! This module tests Requirements 3.1 and 3.2:
//! - Space-separated schema parsing (new format)
//! - Comma-separated schema parsing (backward compatibility)
//!
//! Feature: dx-serializer-production-ready
//! Task: 3.3 Write property tests for schema parsing

use proptest::prelude::*;
use serializer::llm::LlmParser;

// =============================================================================
// Property 11: Space-Separated Schema Parsing
// =============================================================================

/// **Property 11: Space-Separated Schema Parsing**
///
/// *For any* table schema in the format `(col1 col2 col3)` with space-separated
/// columns, parsing should extract all column names correctly.
///
/// **Validates: Requirements 3.1**
#[cfg(test)]
mod property_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_space_separated_schema_parsing(
            columns in prop::collection::vec(
                "[a-zA-Z][a-zA-Z0-9_]*",  // Start with letter (not underscore alone)
                1..10
            )
        ) {
            let schema_str = columns.join(" ");

            // Generate a row with the correct number of columns
            let row_values: Vec<String> = (0..columns.len())
                .map(|i| format!("val{}", i))
                .collect();
            let row_str = row_values.join(" ");

            let input = format!("table:1({})[{}]", schema_str, row_str);

            let result = LlmParser::parse(&input);
            prop_assert!(result.is_ok(), "Failed to parse: {}", input);

            let doc = result.unwrap();
            prop_assert_eq!(doc.sections.len(), 1, "Expected exactly one section");

            let section = doc.sections.values().next().unwrap();
            prop_assert_eq!(
                section.schema.len(),
                columns.len(),
                "Schema column count mismatch for input: {}",
                input
            );

            for (i, col) in columns.iter().enumerate() {
                prop_assert_eq!(
                    &section.schema[i],
                    col,
                    "Column {} mismatch in schema",
                    i
                );
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_space_separated_schema_multiline_rows(
            columns in prop::collection::vec(
                "[a-zA-Z_][a-zA-Z0-9_]*",
                2..6
            ),
            row_count in 1usize..5
        ) {
            let schema_str = columns.join(" ");

            // Generate rows with correct number of columns
            let mut rows = Vec::new();
            for i in 0..row_count {
                let row_values: Vec<String> = (0..columns.len())
                    .map(|j| format!("val{}_{}", i, j))
                    .collect();
                rows.push(row_values.join(" "));
            }
            let rows_str = rows.join("\n");

            let input = format!("table:{}({})[\n{}\n]", row_count, schema_str, rows_str);

            let result = LlmParser::parse(&input);
            prop_assert!(result.is_ok(), "Failed to parse: {}", input);

            let doc = result.unwrap();
            prop_assert_eq!(doc.sections.len(), 1, "Expected exactly one section");

            let section = doc.sections.values().next().unwrap();
            prop_assert_eq!(
                section.schema.len(),
                columns.len(),
                "Schema column count mismatch"
            );
            prop_assert_eq!(
                section.rows.len(),
                row_count,
                "Row count mismatch"
            );

            for (i, col) in columns.iter().enumerate() {
                prop_assert_eq!(
                    &section.schema[i],
                    col,
                    "Column {} mismatch in schema",
                    i
                );
            }
        }
    }
}

// =============================================================================
// Property 12: Comma-Separated Schema Backward Compatibility
// =============================================================================

/// **Property 12: Comma-Separated Schema Backward Compatibility**
///
/// *For any* table schema in the legacy format `(col1,col2,col3)` with
/// comma-separated columns, parsing should continue to extract all column
/// names correctly.
///
/// **Validates: Requirements 3.2**
#[cfg(test)]
mod backward_compat_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_comma_separated_schema_parsing(
            columns in prop::collection::vec(
                "[a-zA-Z][a-zA-Z0-9_]*",  // Start with letter (not underscore alone)
                1..10
            )
        ) {
            let schema_str = columns.join(",");

            // Generate a row with the correct number of columns
            let row_values: Vec<String> = (0..columns.len())
                .map(|i| format!("val{}", i))
                .collect();
            let row_str = row_values.join(" ");

            let input = format!("table:1({})[{}]", schema_str, row_str);

            let result = LlmParser::parse(&input);
            prop_assert!(result.is_ok(), "Failed to parse: {}", input);

            let doc = result.unwrap();
            prop_assert_eq!(doc.sections.len(), 1, "Expected exactly one section");

            let section = doc.sections.values().next().unwrap();
            prop_assert_eq!(
                section.schema.len(),
                columns.len(),
                "Schema column count mismatch for input: {}",
                input
            );

            for (i, col) in columns.iter().enumerate() {
                prop_assert_eq!(
                    &section.schema[i],
                    col,
                    "Column {} mismatch in schema",
                    i
                );
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_comma_separated_schema_with_spaces(
            columns in prop::collection::vec(
                "[a-zA-Z][a-zA-Z0-9_]*",  // Start with letter (not underscore alone)
                2..6
            )
        ) {
            // Legacy format with spaces after commas
            let schema_str = columns.join(", ");

            // Generate a row with the correct number of columns
            let row_values: Vec<String> = (0..columns.len())
                .map(|i| format!("val{}", i))
                .collect();
            let row_str = row_values.join(" ");

            let input = format!("table:1({})[{}]", schema_str, row_str);

            let result = LlmParser::parse(&input);
            prop_assert!(result.is_ok(), "Failed to parse: {}", input);

            let doc = result.unwrap();
            prop_assert_eq!(doc.sections.len(), 1, "Expected exactly one section");

            let section = doc.sections.values().next().unwrap();
            prop_assert_eq!(
                section.schema.len(),
                columns.len(),
                "Schema column count mismatch for input: {}",
                input
            );

            for (i, col) in columns.iter().enumerate() {
                prop_assert_eq!(
                    &section.schema[i],
                    col,
                    "Column {} mismatch in schema (spaces should be trimmed)",
                    i
                );
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_comma_separated_schema_multiline_rows(
            columns in prop::collection::vec(
                "[a-zA-Z_][a-zA-Z0-9_]*",
                2..6
            ),
            row_count in 1usize..5
        ) {
            let schema_str = columns.join(",");

            // Generate rows with correct number of columns (comma-separated for legacy)
            let mut rows = Vec::new();
            for i in 0..row_count {
                let row_values: Vec<String> = (0..columns.len())
                    .map(|j| format!("val{}_{}", i, j))
                    .collect();
                rows.push(row_values.join(","));
            }
            let rows_str = rows.join("\n");

            let input = format!("table:{}({})[\n{}\n]", row_count, schema_str, rows_str);

            let result = LlmParser::parse(&input);
            prop_assert!(result.is_ok(), "Failed to parse: {}", input);

            let doc = result.unwrap();
            prop_assert_eq!(doc.sections.len(), 1, "Expected exactly one section");

            let section = doc.sections.values().next().unwrap();
            prop_assert_eq!(
                section.schema.len(),
                columns.len(),
                "Schema column count mismatch"
            );
            prop_assert_eq!(
                section.rows.len(),
                row_count,
                "Row count mismatch"
            );

            for (i, col) in columns.iter().enumerate() {
                prop_assert_eq!(
                    &section.schema[i],
                    col,
                    "Column {} mismatch in schema",
                    i
                );
            }
        }
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_space_separated_schema_simple() {
        let input = "metrics:3(date views clicks)[\n2025-01-01 4836 193\n2025-01-02 6525 196\n2025-01-03 7927 238]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["date", "views", "clicks"]);
        assert_eq!(section.rows.len(), 3);
    }

    #[test]
    fn test_comma_separated_schema_simple() {
        let input = "metrics:3(date,views,clicks)[\n2025-01-01 4836 193\n2025-01-02 6525 196\n2025-01-03 7927 238]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["date", "views", "clicks"]);
        assert_eq!(section.rows.len(), 3);
    }

    #[test]
    fn test_space_separated_schema_many_columns() {
        let input = "data:2(id name email age city country status)[\n1 Alice alice@ex.com 30 NYC USA active\n2 Bob bob@ex.com 25 LA USA inactive]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["id", "name", "email", "age", "city", "country", "status"]);
        assert_eq!(section.rows.len(), 2);
    }

    #[test]
    fn test_comma_separated_schema_with_spaces() {
        let input = "users:2(id, name, email)[1 Alice alice@ex.com\n2 Bob bob@ex.com]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["id", "name", "email"]);
        assert_eq!(section.rows.len(), 2);
    }

    #[test]
    fn test_space_separated_schema_single_column() {
        let input = "names:3(name)[Alice\nBob\nCarol]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["name"]);
        assert_eq!(section.rows.len(), 3);
    }

    #[test]
    fn test_comma_separated_schema_single_column() {
        let input = "ids:3(id)[1\n2\n3]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["id"]);
        assert_eq!(section.rows.len(), 3);
    }

    #[test]
    fn test_space_separated_schema_inline_rows() {
        let input =
            "users:3(id name email)[1 Alice alice@ex.com, 2 Bob bob@ex.com, 3 Carol carol@ex.com]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["id", "name", "email"]);
        assert_eq!(section.rows.len(), 3);
    }

    #[test]
    fn test_comma_separated_schema_inline_rows() {
        let input = "products:2(id,name,price)[101 Widget 9.99, 102 Gadget 19.99]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["id", "name", "price"]);
        assert_eq!(section.rows.len(), 2);
    }

    #[test]
    fn test_space_separated_schema_underscore_columns() {
        let input = "data:1(user_id user_name created_at)[1 Alice 2025-01-01]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["user_id", "user_name", "created_at"]);
        assert_eq!(section.rows.len(), 1);
    }

    #[test]
    fn test_comma_separated_schema_underscore_columns() {
        let input = "data:1(user_id,user_name,created_at)[1 Alice 2025-01-01]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["user_id", "user_name", "created_at"]);
        assert_eq!(section.rows.len(), 1);
    }

    #[test]
    fn test_space_separated_schema_empty_table() {
        let input = "empty:0(id name email)[]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["id", "name", "email"]);
        assert_eq!(section.rows.len(), 0);
    }

    #[test]
    fn test_comma_separated_schema_empty_table() {
        let input = "empty:0(id,name,email)[]";
        let doc = LlmParser::parse(input).expect("Failed to parse");

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["id", "name", "email"]);
        assert_eq!(section.rows.len(), 0);
    }
}
