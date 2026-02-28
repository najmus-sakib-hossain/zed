//! Human-readable formatter for DX data
//!
//! Transforms machine-optimized DX into beautiful, aligned, easy-to-read format.
//! This is what you see in the IDE extension (DX View).

// Writing to a String via std::fmt::Write never fails (infallible),
// so unwrap() on write!/writeln! to String is safe and idiomatic.
#![allow(clippy::unwrap_used)]

use crate::error::Result;
use crate::types::{DxArray, DxObject, DxTable, DxValue};
use std::fmt::Write as FmtWrite;

/// Formatter configuration
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Column padding for alignment
    pub column_padding: usize,
    /// Use Unicode symbols (✓/✗ instead of +/-)
    pub use_unicode: bool,
    /// Add section dividers
    pub add_dividers: bool,
    /// Color output (ANSI codes)
    pub use_colors: bool,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            column_padding: 4,
            use_unicode: true,
            add_dividers: true,
            use_colors: false,
        }
    }
}

/// Human-readable formatter
pub struct HumanFormatter {
    config: FormatterConfig,
    output: String,
    indent: usize,
}

impl HumanFormatter {
    pub fn new(config: FormatterConfig) -> Self {
        Self {
            config,
            output: String::new(),
            indent: 0,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(FormatterConfig::default())
    }

    /// Format a value to human-readable string
    pub fn format(&mut self, value: &DxValue) -> Result<String> {
        self.output.clear();
        self.indent = 0;

        if self.config.add_dividers {
            self.write_header();
        }

        self.format_value(value)?;

        Ok(self.output.clone())
    }

    /// Write header
    fn write_header(&mut self) {
        let line = "─".repeat(70);
        writeln!(self.output, "┌{}┐", line).unwrap();
        writeln!(self.output, "│  DX HUMAN VIEW  •  Enhanced Readability Mode{}│", " ".repeat(31))
            .unwrap();
        writeln!(self.output, "└{}┘", line).unwrap();
        writeln!(self.output).unwrap();
    }

    /// Format a value
    fn format_value(&mut self, value: &DxValue) -> Result<()> {
        match value {
            DxValue::Object(obj) => self.format_object(obj),
            DxValue::Table(table) => self.format_table(table),
            DxValue::Array(arr) => self.format_array(arr),
            _ => {
                self.write_indent();
                self.write_simple_value(value)?;
                writeln!(self.output).unwrap();
                Ok(())
            }
        }
    }

    /// Format an object
    fn format_object(&mut self, obj: &DxObject) -> Result<()> {
        // Calculate max key length for alignment
        let max_key_len = obj.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

        for (key, value) in obj.iter() {
            self.write_indent();

            // Write key with padding
            write!(self.output, "{}", key).unwrap();
            let padding = max_key_len - key.len() + self.config.column_padding;
            write!(self.output, "{}", " ".repeat(padding)).unwrap();

            match value {
                DxValue::Table(table) => {
                    writeln!(self.output).unwrap();
                    if self.config.add_dividers {
                        self.write_indent();
                        writeln!(self.output, "┌─ {} TABLE ─┐", key.to_uppercase()).unwrap();
                    }
                    self.indent += 1;
                    self.format_table(table)?;
                    self.indent -= 1;
                    if self.config.add_dividers {
                        self.write_indent();
                        writeln!(self.output, "└{}┘", "─".repeat(15)).unwrap();
                    }
                }
                DxValue::Array(arr) if arr.is_stream => {
                    write!(self.output, "> ").unwrap();
                    for (i, val) in arr.values.iter().enumerate() {
                        if i > 0 {
                            write!(self.output, " | ").unwrap();
                        }
                        self.write_simple_value(val)?;
                    }
                    writeln!(self.output).unwrap();
                }
                DxValue::Object(nested) => {
                    writeln!(self.output).unwrap();
                    self.indent += 1;
                    self.format_object(nested)?;
                    self.indent -= 1;
                }
                _ => {
                    write!(self.output, ": ").unwrap();
                    self.write_simple_value(value)?;
                    writeln!(self.output).unwrap();
                }
            }
        }

        Ok(())
    }

    /// Format a table with aligned columns
    fn format_table(&mut self, table: &DxTable) -> Result<()> {
        if table.rows.is_empty() {
            self.write_indent();
            writeln!(self.output, "(empty table)").unwrap();
            return Ok(());
        }

        // Calculate column widths
        let mut col_widths: Vec<usize> =
            table.schema.columns.iter().map(|c| c.name.len()).collect();

        for row in &table.rows {
            for (i, value) in row.iter().enumerate() {
                let val_str = self.value_to_string(value);
                col_widths[i] = col_widths[i].max(val_str.len());
            }
        }

        // Write column headers
        self.write_indent();
        for (i, col) in table.schema.columns.iter().enumerate() {
            if i > 0 {
                write!(self.output, "  ").unwrap();
            }
            write!(self.output, "{:width$}", col.name, width = col_widths[i]).unwrap();
        }
        writeln!(self.output).unwrap();

        // Write separator
        self.write_indent();
        for (i, width) in col_widths.iter().enumerate() {
            if i > 0 {
                write!(self.output, "  ").unwrap();
            }
            write!(self.output, "{}", "─".repeat(*width)).unwrap();
        }
        writeln!(self.output).unwrap();

        // Write rows
        for row in &table.rows {
            self.write_indent();
            for (i, value) in row.iter().enumerate() {
                if i > 0 {
                    write!(self.output, "  ").unwrap();
                }
                let val_str = self.value_to_string(value);
                write!(self.output, "{:width$}", val_str, width = col_widths[i]).unwrap();
            }
            writeln!(self.output).unwrap();
        }

        Ok(())
    }

    /// Format an array
    fn format_array(&mut self, arr: &DxArray) -> Result<()> {
        if arr.is_stream {
            write!(self.output, "> ").unwrap();
            for (i, val) in arr.values.iter().enumerate() {
                if i > 0 {
                    write!(self.output, " | ").unwrap();
                }
                self.write_simple_value(val)?;
            }
            writeln!(self.output).unwrap();
        } else {
            for (i, val) in arr.values.iter().enumerate() {
                self.write_indent();
                write!(self.output, "{}. ", i + 1).unwrap();
                self.write_simple_value(val)?;
                writeln!(self.output).unwrap();
            }
        }
        Ok(())
    }

    /// Write a simple value
    fn write_simple_value(&mut self, value: &DxValue) -> Result<()> {
        let s = match value {
            DxValue::Null => "null".to_string(),
            DxValue::Bool(true) => {
                if self.config.use_unicode {
                    "✓".to_string()
                } else {
                    "+".to_string()
                }
            }
            DxValue::Bool(false) => {
                if self.config.use_unicode {
                    "✗".to_string()
                } else {
                    "-".to_string()
                }
            }
            DxValue::Int(i) => i.to_string(),
            DxValue::Float(f) => format!("{:.2}", f),
            DxValue::String(s) => s.clone(),
            DxValue::Ref(id) => format!("@{}", id),
            _ => format!("{:?}", value),
        };

        write!(self.output, "{}", s).unwrap();
        Ok(())
    }

    /// Convert value to string for width calculation
    fn value_to_string(&self, value: &DxValue) -> String {
        match value {
            DxValue::Null => "null".to_string(),
            DxValue::Bool(true) => {
                if self.config.use_unicode {
                    "✓".to_string()
                } else {
                    "+".to_string()
                }
            }
            DxValue::Bool(false) => {
                if self.config.use_unicode {
                    "✗".to_string()
                } else {
                    "-".to_string()
                }
            }
            DxValue::Int(i) => i.to_string(),
            DxValue::Float(f) => format!("{:.2}", f),
            DxValue::String(s) => s.clone(),
            DxValue::Ref(id) => format!("@{}", id),
            _ => format!("{:?}", value),
        }
    }

    /// Write indentation
    fn write_indent(&mut self) {
        write!(self.output, "{}", "  ".repeat(self.indent)).unwrap();
    }
}

/// Format a value with default config
#[must_use = "formatting result should be used"]
pub fn format_human(value: &DxValue) -> Result<String> {
    let mut formatter = HumanFormatter::with_defaults();
    formatter.format(value)
}

/// Format with custom config
#[must_use = "formatting result should be used"]
pub fn format_human_with_config(value: &DxValue, config: FormatterConfig) -> Result<String> {
    let mut formatter = HumanFormatter::new(config);
    formatter.format(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_format_simple() {
        let input = b"name:Alice
age:30
active:+";

        let value = parse(input).unwrap();
        let formatted = format_human(&value).unwrap();

        assert!(formatted.contains("Alice"));
        assert!(formatted.contains("30"));
        assert!(formatted.contains("✓"));
    }

    #[test]
    fn test_format_table() {
        let input = b"users=id%i name%s active%b
1 Alice +
2 Bob -";

        let value = parse(input).unwrap();
        let formatted = format_human(&value).unwrap();

        assert!(formatted.contains("USERS TABLE"));
        assert!(formatted.contains("Alice"));
        assert!(formatted.contains("Bob"));
        assert!(formatted.contains("✓"));
        assert!(formatted.contains("✗"));
    }
}
