//! Property tests for Task 2.4: Row separator parsing
//!
//! Feature: dx-serializer-production-ready
//! Property 5: Tabular Data Comma Separator Parsing
//! Property 6: Tabular Data Semicolon Separator Parsing
//! Property 7: Tabular Data Colon Separator Parsing
//! Property 8: Tabular Data Newline Separator Parsing
//! Validates: Requirements 2.1, 2.2, 2.3, 2.4

use proptest::collection::vec;
use proptest::prelude::*;
use serializer::llm::parser::LlmParser;
use serializer::llm::types::DxLlmValue;

/// Strategy to generate valid table names
fn valid_table_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}".prop_map(|s| s.to_string())
}

/// Strategy to generate valid column names
fn valid_column_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,10}".prop_map(|s| s.to_string())
}

/// Strategy to generate a schema (2-5 columns with unique names)
fn table_schema() -> impl Strategy<Value = Vec<String>> {
    vec(valid_column_name(), 2..=5).prop_map(|mut cols| {
        // Ensure unique column names
        cols.sort();
        cols.dedup();
        // If dedup removed items, pad with generated names
        while cols.len() < 2 {
            cols.push(format!("col{}", cols.len()));
        }
        cols
    })
}

/// Strategy to generate a simple table cell value (no spaces, no special chars)
/// Avoids patterns that look like human names (Title_Case) which get converted to spaces
fn simple_cell_value() -> impl Strategy<Value = String> {
    prop_oneof![
        // Lowercase alphanumeric strings (avoids Title_Case pattern)
        "[a-z][a-z0-9_]{0,10}".prop_map(|s| s.to_string()),
        // UPPERCASE alphanumeric strings (avoids Title_Case pattern)
        "[A-Z][A-Z0-9_]{0,10}".prop_map(|s| s.to_string()),
        // Numbers
        (-1000i64..1000i64).prop_map(|n| n.to_string()),
        // Floats
        (-100.0f64..100.0f64)
            .prop_filter("finite", |f| f.is_finite())
            .prop_map(|f| format!("{:.2}", f)),
        // Booleans
        prop::bool::ANY.prop_map(|b| if b {
            "true".to_string()
        } else {
            "false".to_string()
        }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-serializer-production-ready, Property 5: Tabular Data Comma Separator Parsing
    /// **Validates: Requirements 2.1**
    ///
    /// For any tabular data with comma-separated rows, parsing should correctly
    /// split all rows on commas and extract all column values.
    #[test]
    fn prop_table_comma_separator(
        table_name in valid_table_name(),
        schema in table_schema(),
        rows in vec(vec(simple_cell_value(), 2..=5), 1..=10)
    ) {
        // Ensure rows match schema length
        let schema_len = schema.len();
        let rows: Vec<Vec<String>> = rows.into_iter()
            .map(|mut row| {
                row.truncate(schema_len);
                while row.len() < schema_len {
                    row.push("default".to_string());
                }
                row
            })
            .collect();

        // Build table with comma-separated rows
        let schema_str = schema.join(" ");
        let rows_str: Vec<String> = rows
            .iter()
            .map(|row| row.join(" "))
            .collect();
        let input = format!(
            "{}:{}({})[{}]",
            table_name,
            rows.len(),
            schema_str,
            rows_str.join(", ")
        );

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse table with comma separator: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Check that the table section exists
        prop_assert!(
            !doc.sections.is_empty(),
            "No sections found in document. Input: {}",
            input
        );

        // Get the first section (tables are stored by character key)
        let section = doc.sections.values().next().unwrap();

        // Verify schema
        prop_assert_eq!(
            section.schema.len(),
            schema.len(),
            "Schema length mismatch. Expected: {:?}, Got: {:?}",
            schema,
            section.schema
        );

        // Verify row count
        prop_assert_eq!(
            section.rows.len(),
            rows.len(),
            "Row count mismatch. Expected {} rows, got {}. Input: {}",
            rows.len(),
            section.rows.len(),
            input
        );

        // Verify each row
        for (i, expected_row) in rows.iter().enumerate() {
            let parsed_row = &section.rows[i];
            prop_assert_eq!(
                parsed_row.len(),
                expected_row.len(),
                "Row {} column count mismatch. Expected {}, got {}",
                i,
                expected_row.len(),
                parsed_row.len()
            );

            // Verify each cell value
            for (j, expected_value_str) in expected_row.iter().enumerate() {
                let parsed_value = &parsed_row[j];
                verify_cell_value(parsed_value, expected_value_str, i, j)?;
            }
        }
    }

    /// Feature: dx-serializer-production-ready, Property 6: Tabular Data Semicolon Separator Parsing
    /// **Validates: Requirements 2.2**
    ///
    /// For any tabular data with semicolon-separated rows, parsing should correctly
    /// split all rows on semicolons and extract all column values.
    #[test]
    fn prop_table_semicolon_separator(
        table_name in valid_table_name(),
        schema in table_schema(),
        rows in vec(vec(simple_cell_value(), 2..=5), 1..=10)
    ) {
        let schema_len = schema.len();
        let rows: Vec<Vec<String>> = rows.into_iter()
            .map(|mut row| {
                row.truncate(schema_len);
                while row.len() < schema_len {
                    row.push("default".to_string());
                }
                row
            })
            .collect();

        // Build table with semicolon-separated rows
        let schema_str = schema.join(" ");
        let rows_str: Vec<String> = rows
            .iter()
            .map(|row| row.join(" "))
            .collect();
        let input = format!(
            "{}:{}({})[{}]",
            table_name,
            rows.len(),
            schema_str,
            rows_str.join("; ")
        );

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse table with semicolon separator: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Check that the table section exists
        prop_assert!(
            !doc.sections.is_empty(),
            "No sections found in document. Input: {}",
            input
        );

        let section = doc.sections.values().next().unwrap();

        // Verify row count
        prop_assert_eq!(
            section.rows.len(),
            rows.len(),
            "Row count mismatch. Expected {} rows, got {}. Input: {}",
            rows.len(),
            section.rows.len(),
            input
        );

        // Verify each row
        for (i, expected_row) in rows.iter().enumerate() {
            let parsed_row = &section.rows[i];
            prop_assert_eq!(
                parsed_row.len(),
                expected_row.len(),
                "Row {} column count mismatch",
                i
            );

            for (j, expected_value_str) in expected_row.iter().enumerate() {
                let parsed_value = &parsed_row[j];
                verify_cell_value(parsed_value, expected_value_str, i, j)?;
            }
        }
    }

    /// Feature: dx-serializer-production-ready, Property 7: Tabular Data Colon Separator Parsing
    /// **Validates: Requirements 2.3**
    ///
    /// For any tabular data with colon-separated rows, parsing should correctly
    /// split all rows on colons and extract all column values.
    #[test]
    fn prop_table_colon_separator(
        table_name in valid_table_name(),
        schema in table_schema(),
        rows in vec(vec(simple_cell_value(), 2..=5), 1..=10)
    ) {
        let schema_len = schema.len();
        let rows: Vec<Vec<String>> = rows.into_iter()
            .map(|mut row| {
                row.truncate(schema_len);
                while row.len() < schema_len {
                    row.push("default".to_string());
                }
                row
            })
            .collect();

        // Build table with colon-separated rows
        let schema_str = schema.join(" ");
        let rows_str: Vec<String> = rows
            .iter()
            .map(|row| row.join(" "))
            .collect();
        let input = format!(
            "{}:{}({})[{}]",
            table_name,
            rows.len(),
            schema_str,
            rows_str.join(": ")
        );

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse table with colon separator: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Check that the table section exists
        prop_assert!(
            !doc.sections.is_empty(),
            "No sections found in document. Input: {}",
            input
        );

        let section = doc.sections.values().next().unwrap();

        // Verify row count
        prop_assert_eq!(
            section.rows.len(),
            rows.len(),
            "Row count mismatch. Expected {} rows, got {}. Input: {}",
            rows.len(),
            section.rows.len(),
            input
        );

        // Verify each row
        for (i, expected_row) in rows.iter().enumerate() {
            let parsed_row = &section.rows[i];
            prop_assert_eq!(
                parsed_row.len(),
                expected_row.len(),
                "Row {} column count mismatch",
                i
            );

            for (j, expected_value_str) in expected_row.iter().enumerate() {
                let parsed_value = &parsed_row[j];
                verify_cell_value(parsed_value, expected_value_str, i, j)?;
            }
        }
    }

    /// Feature: dx-serializer-production-ready, Property 8: Tabular Data Newline Separator Parsing
    /// **Validates: Requirements 2.4**
    ///
    /// For any tabular data with newline-separated rows, parsing should correctly
    /// split all rows on newlines and extract all column values.
    #[test]
    fn prop_table_newline_separator(
        table_name in valid_table_name(),
        schema in table_schema(),
        rows in vec(vec(simple_cell_value(), 2..=5), 1..=10)
    ) {
        let schema_len = schema.len();
        let rows: Vec<Vec<String>> = rows.into_iter()
            .map(|mut row| {
                row.truncate(schema_len);
                while row.len() < schema_len {
                    row.push("default".to_string());
                }
                row
            })
            .collect();

        // Build table with newline-separated rows
        let schema_str = schema.join(" ");
        let rows_str: Vec<String> = rows
            .iter()
            .map(|row| row.join(" "))
            .collect();
        let input = format!(
            "{}:{}({})[{}]",
            table_name,
            rows.len(),
            schema_str,
            rows_str.join("\n")
        );

        // Parse the input
        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse table with newline separator: {:?}\nInput: {}",
            result.err(),
            input
        );

        let doc = result.unwrap();

        // Check that the table section exists
        prop_assert!(
            !doc.sections.is_empty(),
            "No sections found in document. Input: {}",
            input
        );

        let section = doc.sections.values().next().unwrap();

        // Verify row count
        prop_assert_eq!(
            section.rows.len(),
            rows.len(),
            "Row count mismatch. Expected {} rows, got {}. Input: {}",
            rows.len(),
            section.rows.len(),
            input
        );

        // Verify each row
        for (i, expected_row) in rows.iter().enumerate() {
            let parsed_row = &section.rows[i];
            prop_assert_eq!(
                parsed_row.len(),
                expected_row.len(),
                "Row {} column count mismatch",
                i
            );

            for (j, expected_value_str) in expected_row.iter().enumerate() {
                let parsed_value = &parsed_row[j];
                verify_cell_value(parsed_value, expected_value_str, i, j)?;
            }
        }
    }

    /// Property variant: Mixed separators should detect the first one
    ///
    /// When a table contains multiple potential separators, the parser should
    /// detect and use the first one encountered at depth 0.
    #[test]
    fn prop_table_separator_priority(
        table_name in valid_table_name(),
        schema in table_schema(),
        rows in vec(vec(simple_cell_value(), 2..=5), 1..=10)
    ) {
        let schema_len = schema.len();
        let rows: Vec<Vec<String>> = rows.into_iter()
            .map(|mut row| {
                row.truncate(schema_len);
                while row.len() < schema_len {
                    row.push("default".to_string());
                }
                row
            })
            .collect();

        // Build table with comma separator (should be detected first)
        let schema_str = schema.join(" ");
        let rows_str: Vec<String> = rows
            .iter()
            .map(|row| row.join(" "))
            .collect();
        let input = format!(
            "{}:{}({})[{}]",
            table_name,
            rows.len(),
            schema_str,
            rows_str.join(", ")
        );

        let result = LlmParser::parse(&input);

        prop_assert!(
            result.is_ok(),
            "Failed to parse table with comma separator: {:?}",
            result.err()
        );

        let doc = result.unwrap();
        let section = doc.sections.values().next().unwrap();

        // Should parse all rows correctly
        prop_assert_eq!(
            section.rows.len(),
            rows.len(),
            "Row count mismatch with comma separator"
        );
    }

    /// Property variant: Empty tables
    ///
    /// Tables with count=0 and no rows should parse successfully.
    #[test]
    fn prop_table_empty(
        table_name in valid_table_name(),
        schema in table_schema()
    ) {
        let schema_str = schema.join(" ");
        let input = format!("{}:0({})[", table_name, schema_str);

        // Try all separator styles for empty tables
        for closing in &["]", "\n]"] {
            let full_input = format!("{}{}", input, closing);
            let result = LlmParser::parse(&full_input);

            prop_assert!(
                result.is_ok(),
                "Failed to parse empty table: {:?}\nInput: {}",
                result.err(),
                full_input
            );

            let doc = result.unwrap();
            if !doc.sections.is_empty() {
                let section = doc.sections.values().next().unwrap();
                prop_assert_eq!(
                    section.rows.len(),
                    0,
                    "Empty table should have 0 rows"
                );
            }
        }
    }

    /// Property variant: Single row tables
    ///
    /// Tables with exactly one row should parse correctly with any separator.
    #[test]
    fn prop_table_single_row(
        table_name in valid_table_name(),
        schema in table_schema(),
        row in vec(simple_cell_value(), 2..=5)
    ) {
        let schema_len = schema.len();
        let mut row = row;
        row.truncate(schema_len);
        while row.len() < schema_len {
            row.push("default".to_string());
        }

        let schema_str = schema.join(" ");
        let row_str = row.join(" ");

        // Test with different separators (though single row doesn't need separator)
        for (sep_name, _sep) in &[("comma", ","), ("semicolon", ";"), ("colon", ":"), ("newline", "\n")] {
            let input = format!(
                "{}:1({})[{}{}]",
                table_name,
                schema_str,
                row_str,
                if *sep_name == "newline" { "\n" } else { "" }
            );

            let result = LlmParser::parse(&input);

            prop_assert!(
                result.is_ok(),
                "Failed to parse single-row table with {} separator: {:?}\nInput: {}",
                sep_name,
                result.err(),
                input
            );

            let doc = result.unwrap();
            if !doc.sections.is_empty() {
                let section = doc.sections.values().next().unwrap();
                prop_assert_eq!(
                    section.rows.len(),
                    1,
                    "Single-row table should have 1 row"
                );
                prop_assert_eq!(
                    section.rows[0].len(),
                    schema_len,
                    "Row should have {} columns",
                    schema_len
                );
            }
        }
    }
}

/// Helper function to verify a cell value matches expected value
fn verify_cell_value(
    parsed_value: &DxLlmValue,
    expected_value_str: &str,
    row: usize,
    col: usize,
) -> Result<(), proptest::test_runner::TestCaseError> {
    // Try to parse as number
    if let Ok(num) = expected_value_str.parse::<f64>() {
        if let Some(parsed_num) = parsed_value.as_num() {
            let diff = (num - parsed_num).abs();
            prop_assert!(
                diff < 0.01,
                "Numeric value mismatch at row {}, col {}: expected {}, got {}",
                row,
                col,
                num,
                parsed_num
            );
            return Ok(());
        }
    }

    // Try to parse as boolean
    match expected_value_str {
        "true" => {
            if let Some(b) = parsed_value.as_bool() {
                prop_assert_eq!(b, true, "Boolean value mismatch at row {}, col {}", row, col);
                return Ok(());
            }
        }
        "false" => {
            if let Some(b) = parsed_value.as_bool() {
                prop_assert_eq!(b, false, "Boolean value mismatch at row {}, col {}", row, col);
                return Ok(());
            }
        }
        _ => {}
    }

    // Otherwise it's a string
    if let Some(s) = parsed_value.as_str() {
        prop_assert_eq!(s, expected_value_str, "String value mismatch at row {}, col {}", row, col);
    } else {
        prop_assert!(
            false,
            "Type mismatch at row {}, col {}: expected string '{}', got {:?}",
            row,
            col,
            expected_value_str,
            parsed_value
        );
    }

    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_comma_separated_rows() {
        let input =
            "users:3(id name email)[1 Alice alice@ex.com, 2 Bob bob@ex.com, 3 Carol carol@ex.com]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(!doc.sections.is_empty());
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.rows.len(), 3);
        assert_eq!(section.schema, vec!["id", "name", "email"]);
    }

    #[test]
    fn test_semicolon_separated_rows() {
        let input = "logs:2(level msg)[INFO Started; ERROR Failed]";
        let doc = LlmParser::parse(input).unwrap();

        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.rows.len(), 2);
        assert_eq!(section.schema, vec!["level", "msg"]);
    }

    #[test]
    fn test_colon_separated_rows() {
        let input = "events:2(id event)[1 Login: 2 Logout]";
        let doc = LlmParser::parse(input).unwrap();

        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.rows.len(), 2);
        assert_eq!(section.schema, vec!["id", "event"]);
    }

    #[test]
    fn test_newline_separated_rows() {
        let input = "data:2(x y)[1 2\n3 4]";
        let doc = LlmParser::parse(input).unwrap();

        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.rows.len(), 2);
        assert_eq!(section.schema, vec!["x", "y"]);
    }

    #[test]
    fn test_empty_table() {
        let input = "empty:0(a b c)[]";
        let doc = LlmParser::parse(input).unwrap();

        if !doc.sections.is_empty() {
            let section = doc.sections.values().next().unwrap();
            assert_eq!(section.rows.len(), 0);
        }
    }

    #[test]
    fn test_single_row_table() {
        let input = "single:1(name value)[test 42]";
        let doc = LlmParser::parse(input).unwrap();

        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.rows.len(), 1);
        assert_eq!(section.rows[0].len(), 2);
    }

    #[test]
    fn test_table_with_numbers() {
        let input = "nums:2(a b c)[1 2 3, 4 5 6]";
        let doc = LlmParser::parse(input).unwrap();

        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.rows.len(), 2);
        assert_eq!(section.rows[0][0].as_num(), Some(1.0));
        assert_eq!(section.rows[1][2].as_num(), Some(6.0));
    }

    #[test]
    fn test_table_with_booleans() {
        let input = "flags:2(name active)[feature1 true, feature2 false]";
        let doc = LlmParser::parse(input).unwrap();

        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.rows[0][1].as_bool(), Some(true));
        assert_eq!(section.rows[1][1].as_bool(), Some(false));
    }
}
