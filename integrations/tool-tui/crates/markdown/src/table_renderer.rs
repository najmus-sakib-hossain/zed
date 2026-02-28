//! Table renderer with Unicode box-drawing support.
//!
//! This module provides advanced table rendering capabilities with
//! Unicode box-drawing characters and type-based alignment.

use crate::types::{CellValue, ColumnDef, TableNode, TypeHint};

/// Unicode box-drawing characters for table rendering.
pub mod box_chars {
    /// Top-left corner (┌)
    pub const TOP_LEFT: char = '┌';
    /// Top-right corner (┐)
    pub const TOP_RIGHT: char = '┐';
    /// Bottom-left corner (└)
    pub const BOTTOM_LEFT: char = '└';
    /// Bottom-right corner (┘)
    pub const BOTTOM_RIGHT: char = '┘';
    /// Horizontal line (─)
    pub const HORIZONTAL: char = '─';
    /// Vertical line (│)
    pub const VERTICAL: char = '│';
    /// T-junction pointing down (┬)
    pub const T_DOWN: char = '┬';
    /// T-junction pointing up (┴)
    pub const T_UP: char = '┴';
    /// T-junction pointing right (├)
    pub const T_RIGHT: char = '├';
    /// T-junction pointing left (┤)
    pub const T_LEFT: char = '┤';
    /// Cross junction (┼)
    pub const CROSS: char = '┼';

    // Double-line box characters for beautiful tables
    /// Double top-left corner (╔)
    pub const DOUBLE_TOP_LEFT: char = '╔';
    /// Double top-right corner (╗)
    pub const DOUBLE_TOP_RIGHT: char = '╗';
    /// Double bottom-left corner (╚)
    pub const DOUBLE_BOTTOM_LEFT: char = '╚';
    /// Double bottom-right corner (╝)
    pub const DOUBLE_BOTTOM_RIGHT: char = '╝';
    /// Double horizontal line (═)
    pub const DOUBLE_HORIZONTAL: char = '═';
    /// Double vertical line (║)
    pub const DOUBLE_VERTICAL: char = '║';
    /// Double T-junction pointing down (╦)
    pub const DOUBLE_T_DOWN: char = '╦';
    /// Double T-junction pointing up (╩)
    pub const DOUBLE_T_UP: char = '╩';
    /// Double T-junction pointing right (╠)
    pub const DOUBLE_T_RIGHT: char = '╠';
    /// Double T-junction pointing left (╣)
    pub const DOUBLE_T_LEFT: char = '╣';
    /// Double cross junction (╬)
    pub const DOUBLE_CROSS: char = '╬';
}

/// Cell alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    /// Left-align content (default for text)
    #[default]
    Left,
    /// Right-align content (default for numbers)
    Right,
    /// Center content
    Center,
}

/// Table renderer configuration.
#[derive(Debug, Clone)]
pub struct TableRendererConfig {
    /// Use Unicode box-drawing characters
    pub unicode: bool,
    /// Column padding (spaces on each side)
    pub padding: usize,
    /// Align numeric columns to the right
    pub align_numbers_right: bool,
}

impl Default for TableRendererConfig {
    fn default() -> Self {
        Self {
            unicode: true,
            padding: 1,
            align_numbers_right: true,
        }
    }
}

/// Table renderer with alignment support.
#[derive(Debug, Clone)]
pub struct TableRenderer {
    /// Renderer configuration
    pub config: TableRendererConfig,
    /// Style counter for rotating table styles (0-3)
    style_counter: usize,
}

impl Default for TableRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TableRenderer {
    /// Create a new table renderer with default configuration.
    pub fn new() -> Self {
        Self {
            config: TableRendererConfig::default(),
            style_counter: 0,
        }
    }

    /// Create a table renderer with custom configuration.
    pub fn with_config(config: TableRendererConfig) -> Self {
        Self {
            config,
            style_counter: 0,
        }
    }

    /// Set the style counter for rotating table styles
    pub fn with_style_counter(mut self, counter: usize) -> Self {
        self.style_counter = counter;
        self
    }

    /// Render a table to string with rotating styles.
    pub fn render(&self, table: &TableNode) -> String {
        if self.config.unicode {
            self.render_unicode_double(table)
        } else {
            self.render_ascii(table)
        }
    }

    /// Calculate optimal column widths based on content.
    pub fn calculate_column_widths(&self, table: &TableNode) -> Vec<usize> {
        let mut widths: Vec<usize> =
            table.schema.iter().map(|col| col.name.chars().count()).collect();

        for row in &table.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let cell_width = self.cell_to_string(cell).chars().count();
                    widths[i] = widths[i].max(cell_width);
                }
            }
        }

        widths
    }

    /// Render a table with Unicode box-drawing characters (double-line style).
    fn render_unicode_double(&self, table: &TableNode) -> String {
        let widths = self.calculate_column_widths(table);
        let mut output = String::new();

        // Beautiful double-line top border
        output.push_str(&self.render_double_top_border(&widths));
        output.push('\n');

        // Header row with double vertical lines
        output.push_str(&self.render_double_header_row(table, &widths));
        output.push('\n');

        // Beautiful double-line header separator
        output.push_str(&self.render_double_header_separator(&widths));
        output.push('\n');

        // Data rows with single vertical lines
        for row in &table.rows {
            output.push_str(&self.render_row(row, &widths, &table.schema));
            output.push('\n');
        }

        // Beautiful double-line bottom border
        output.push_str(&self.render_double_bottom_border(&widths));

        output
    }

    /// Render beautiful double-line top border (╔═╦═╗).
    fn render_double_top_border(&self, widths: &[usize]) -> String {
        let mut border = String::new();
        border.push(box_chars::DOUBLE_TOP_LEFT);

        for (i, width) in widths.iter().enumerate() {
            let segment_width = width + self.config.padding * 2;
            for _ in 0..segment_width {
                border.push(box_chars::DOUBLE_HORIZONTAL);
            }
            if i < widths.len() - 1 {
                border.push(box_chars::DOUBLE_T_DOWN);
            }
        }

        border.push(box_chars::DOUBLE_TOP_RIGHT);
        border
    }

    /// Render header row with double vertical lines.
    fn render_double_header_row(&self, table: &TableNode, widths: &[usize]) -> String {
        let mut row = String::new();
        row.push(box_chars::DOUBLE_VERTICAL);

        for (i, col) in table.schema.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(0);
            let padding = " ".repeat(self.config.padding);

            // Headers are always centered for beauty
            let header_text = &col.name;
            let total_padding = width.saturating_sub(header_text.chars().count());
            let left_pad = total_padding / 2;
            let right_pad = total_padding - left_pad;

            row.push_str(&padding);
            row.push_str(&" ".repeat(left_pad));
            row.push_str(header_text);
            row.push_str(&" ".repeat(right_pad));
            row.push_str(&padding);
            row.push(box_chars::DOUBLE_VERTICAL);
        }

        row
    }

    /// Render beautiful double-line header separator (╠═╬═╣).
    fn render_double_header_separator(&self, widths: &[usize]) -> String {
        let mut separator = String::new();
        separator.push(box_chars::DOUBLE_T_RIGHT);

        for (i, width) in widths.iter().enumerate() {
            let segment_width = width + self.config.padding * 2;
            for _ in 0..segment_width {
                separator.push(box_chars::DOUBLE_HORIZONTAL);
            }
            if i < widths.len() - 1 {
                separator.push(box_chars::DOUBLE_CROSS);
            }
        }

        separator.push(box_chars::DOUBLE_T_LEFT);
        separator
    }

    /// Render beautiful double-line bottom border (╚═╩═╝).
    fn render_double_bottom_border(&self, widths: &[usize]) -> String {
        let mut border = String::new();
        border.push(box_chars::DOUBLE_BOTTOM_LEFT);

        for (i, width) in widths.iter().enumerate() {
            let segment_width = width + self.config.padding * 2;
            for _ in 0..segment_width {
                border.push(box_chars::DOUBLE_HORIZONTAL);
            }
            if i < widths.len() - 1 {
                border.push(box_chars::DOUBLE_T_UP);
            }
        }

        border.push(box_chars::DOUBLE_BOTTOM_RIGHT);
        border
    }

    /// Render top border (┌─┬─┐).
    fn render_top_border(&self, widths: &[usize]) -> String {
        let mut border = String::new();
        border.push(box_chars::TOP_LEFT);

        for (i, width) in widths.iter().enumerate() {
            let segment_width = width + self.config.padding * 2;
            for _ in 0..segment_width {
                border.push(box_chars::HORIZONTAL);
            }
            if i < widths.len() - 1 {
                border.push(box_chars::T_DOWN);
            }
        }

        border.push(box_chars::TOP_RIGHT);
        border
    }

    /// Render header separator (├─┼─┤).
    fn render_header_separator(&self, widths: &[usize]) -> String {
        let mut separator = String::new();
        separator.push(box_chars::T_RIGHT);

        for (i, width) in widths.iter().enumerate() {
            let segment_width = width + self.config.padding * 2;
            for _ in 0..segment_width {
                separator.push(box_chars::HORIZONTAL);
            }
            if i < widths.len() - 1 {
                separator.push(box_chars::CROSS);
            }
        }

        separator.push(box_chars::T_LEFT);
        separator
    }

    /// Render bottom border (└─┴─┘).
    fn render_bottom_border(&self, widths: &[usize]) -> String {
        let mut border = String::new();
        border.push(box_chars::BOTTOM_LEFT);

        for (i, width) in widths.iter().enumerate() {
            let segment_width = width + self.config.padding * 2;
            for _ in 0..segment_width {
                border.push(box_chars::HORIZONTAL);
            }
            if i < widths.len() - 1 {
                border.push(box_chars::T_UP);
            }
        }

        border.push(box_chars::BOTTOM_RIGHT);
        border
    }

    /// Render header row.
    fn render_header_row(&self, table: &TableNode, widths: &[usize]) -> String {
        let mut row = String::new();
        row.push(box_chars::VERTICAL);

        for (i, col) in table.schema.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(0);
            let padding = " ".repeat(self.config.padding);

            // Headers are always left-aligned
            row.push_str(&padding);
            row.push_str(&format!("{:<width$}", col.name, width = width));
            row.push_str(&padding);
            row.push(box_chars::VERTICAL);
        }

        row
    }

    /// Render a data row with │ separators.
    fn render_row(&self, cells: &[CellValue], widths: &[usize], schema: &[ColumnDef]) -> String {
        let mut row = String::new();
        row.push(box_chars::DOUBLE_VERTICAL);

        for (i, cell) in cells.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(0);
            let padding = " ".repeat(self.config.padding);
            let cell_str = self.cell_to_string(cell);
            let alignment = self.get_cell_alignment(cell, schema.get(i));

            // Check if this looks like a percentage or progress value
            let is_progress = self.is_progress_value(&cell_str);

            row.push_str(&padding);

            if is_progress {
                // Render as progress bar
                row.push_str(&self.render_progress_bar(&cell_str, width));
            } else {
                match alignment {
                    Alignment::Left => {
                        row.push_str(&format!("{:<width$}", cell_str, width = width));
                    }
                    Alignment::Right => {
                        row.push_str(&format!("{:>width$}", cell_str, width = width));
                    }
                    Alignment::Center => {
                        let total_padding = width.saturating_sub(cell_str.chars().count());
                        let left_pad = total_padding / 2;
                        let right_pad = total_padding - left_pad;
                        row.push_str(&" ".repeat(left_pad));
                        row.push_str(&cell_str);
                        row.push_str(&" ".repeat(right_pad));
                    }
                }
            }

            row.push_str(&padding);
            row.push(box_chars::DOUBLE_VERTICAL);
        }

        // Handle case where row has fewer cells than schema
        for i in cells.len()..widths.len() {
            let width = widths.get(i).copied().unwrap_or(0);
            let padding = " ".repeat(self.config.padding);
            row.push_str(&padding);
            row.push_str(&" ".repeat(width));
            row.push_str(&padding);
            row.push(box_chars::DOUBLE_VERTICAL);
        }

        row
    }

    /// Check if a value looks like a progress/percentage value
    fn is_progress_value(&self, value: &str) -> bool {
        // Check for patterns like "42%", "42.5%", "0.42", etc.
        if value.ends_with('%') {
            return true;
        }

        // Check for decimal values between 0 and 1 (like 0.42 for 42%)
        if let Ok(f) = value.parse::<f64>()
            && (0.0..=1.0).contains(&f)
            && value.contains('.')
        {
            return true;
        }

        false
    }

    /// Render a progress bar for percentage values
    fn render_progress_bar(&self, value: &str, width: usize) -> String {
        // Parse the percentage
        let percentage = if value.ends_with('%') {
            value.trim_end_matches('%').parse::<f64>().unwrap_or(0.0)
        } else {
            value.parse::<f64>().unwrap_or(0.0) * 100.0
        };

        // Calculate bar width (leave space for percentage text)
        let text = format!("{:.1}%", percentage);
        let bar_width = width.saturating_sub(text.len() + 1);

        if bar_width < 5 {
            // Not enough space for bar, just show text
            return format!("{:<width$}", text, width = width);
        }

        // Calculate filled portion
        let filled = ((percentage / 100.0) * bar_width as f64) as usize;
        let empty = bar_width.saturating_sub(filled);

        // Build progress bar with beautiful characters
        let mut bar = String::new();
        bar.push_str(&"█".repeat(filled));
        bar.push_str(&"░".repeat(empty));
        bar.push(' ');
        bar.push_str(&text);

        // Pad to width
        let current_len = bar.chars().count();
        if current_len < width {
            bar.push_str(&" ".repeat(width - current_len));
        }

        bar
    }

    /// Determine cell alignment based on content type.
    pub fn get_cell_alignment(&self, cell: &CellValue, col_def: Option<&ColumnDef>) -> Alignment {
        // Check type hint first
        if let Some(col) = col_def
            && let Some(hint) = &col.type_hint
        {
            return match hint {
                TypeHint::Integer | TypeHint::Float => Alignment::Right,
                TypeHint::String | TypeHint::Boolean | TypeHint::Date => Alignment::Left,
            };
        }

        // Fall back to cell value type
        if self.config.align_numbers_right {
            match cell {
                CellValue::Integer(_) | CellValue::Float(_) => Alignment::Right,
                CellValue::Text(_) | CellValue::Boolean(_) | CellValue::Null => Alignment::Left,
            }
        } else {
            Alignment::Left
        }
    }

    /// Render a table with ASCII characters (GFM-style with proper spacing).
    pub fn render_ascii(&self, table: &TableNode) -> String {
        let widths = self.calculate_column_widths(table);
        let mut output = String::new();
        let padding = self.config.padding;

        // Top border
        output.push('+');
        for (i, _) in table.schema.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(3);
            output.push_str(&"-".repeat(width + padding * 2));
            output.push('+');
        }
        output.push('\n');

        // Header row
        output.push('|');
        for (i, col) in table.schema.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(0);
            output.push_str(&" ".repeat(padding));
            output.push_str(&format!("{:<width$}", col.name, width = width));
            output.push_str(&" ".repeat(padding));
            output.push('|');
        }
        output.push('\n');

        // Header separator
        output.push('+');
        for (i, _) in table.schema.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(3);
            output.push_str(&"-".repeat(width + padding * 2));
            output.push('+');
        }
        output.push('\n');

        // Data rows
        for row in &table.rows {
            output.push('|');
            for (i, cell) in row.iter().enumerate() {
                let width = widths.get(i).copied().unwrap_or(0);
                let cell_str = self.cell_to_string(cell);
                let alignment = self.get_cell_alignment(cell, table.schema.get(i));

                output.push_str(&" ".repeat(padding));
                match alignment {
                    Alignment::Left => {
                        output.push_str(&format!("{:<width$}", cell_str, width = width));
                    }
                    Alignment::Right => {
                        output.push_str(&format!("{:>width$}", cell_str, width = width));
                    }
                    Alignment::Center => {
                        let total_padding = width.saturating_sub(cell_str.chars().count());
                        let left_pad = total_padding / 2;
                        let right_pad = total_padding - left_pad;
                        output.push_str(&" ".repeat(left_pad));
                        output.push_str(&cell_str);
                        output.push_str(&" ".repeat(right_pad));
                    }
                }
                output.push_str(&" ".repeat(padding));
                output.push('|');
            }
            output.push('\n');
        }

        // Bottom border
        output.push('+');
        for (i, _) in table.schema.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(3);
            output.push_str(&"-".repeat(width + padding * 2));
            output.push('+');
        }

        output
    }

    /// Convert a cell value to string.
    fn cell_to_string(&self, cell: &CellValue) -> String {
        match cell {
            CellValue::Text(t) => t.clone(),
            CellValue::Integer(i) => i.to_string(),
            CellValue::Float(f) => format!("{}", f),
            CellValue::Boolean(b) => b.to_string(),
            CellValue::Null => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_table() -> TableNode {
        TableNode {
            schema: vec![
                ColumnDef {
                    name: "id".to_string(),
                    type_hint: None,
                },
                ColumnDef {
                    name: "name".to_string(),
                    type_hint: None,
                },
                ColumnDef {
                    name: "score".to_string(),
                    type_hint: None,
                },
            ],
            rows: vec![
                vec![
                    CellValue::Integer(1),
                    CellValue::Text("Alice".to_string()),
                    CellValue::Float(95.5),
                ],
                vec![
                    CellValue::Integer(2),
                    CellValue::Text("Bob".to_string()),
                    CellValue::Float(87.0),
                ],
            ],
        }
    }

    #[test]
    fn test_calculate_column_widths() {
        let table = create_test_table();
        let renderer = TableRenderer::new();
        let widths = renderer.calculate_column_widths(&table);

        assert_eq!(widths.len(), 3);
        assert_eq!(widths[0], 2); // "id" is 2 chars, "1" and "2" are 1 char
        assert_eq!(widths[1], 5); // "Alice" is 5 chars
        assert_eq!(widths[2], 5); // "score" is 5 chars, "95.5" is 4 chars
    }

    #[test]
    fn test_unicode_table_structure() {
        let table = create_test_table();
        let renderer = TableRenderer::new();
        let output = renderer.render(&table);

        // Check for required Unicode box-drawing characters
        assert!(output.contains(box_chars::TOP_LEFT));
        assert!(output.contains(box_chars::TOP_RIGHT));
        assert!(output.contains(box_chars::BOTTOM_LEFT));
        assert!(output.contains(box_chars::BOTTOM_RIGHT));
        assert!(output.contains(box_chars::HORIZONTAL));
        assert!(output.contains(box_chars::VERTICAL));
        assert!(output.contains(box_chars::T_DOWN));
        assert!(output.contains(box_chars::T_UP));
        assert!(output.contains(box_chars::T_RIGHT));
        assert!(output.contains(box_chars::T_LEFT));
        assert!(output.contains(box_chars::CROSS));
    }

    #[test]
    fn test_ascii_table_structure() {
        let table = create_test_table();
        let renderer = TableRenderer::with_config(TableRendererConfig {
            unicode: false,
            ..Default::default()
        });
        let output = renderer.render(&table);

        // Check for ASCII box table with + corners
        assert!(output.contains("+"));
        assert!(output.contains("-"));
        assert!(output.contains("|"));
        assert!(output.contains("id"));
        assert!(output.contains("name"));
        assert!(output.contains("Alice"));
        assert!(!output.contains('┌'));
    }

    #[test]
    fn test_numeric_right_alignment() {
        let renderer = TableRenderer::new();

        let int_cell = CellValue::Integer(42);
        let float_cell = CellValue::Float(3.15); // Use 3.15 instead of 3.14 to avoid PI constant warning
        let text_cell = CellValue::Text("hello".to_string());

        assert_eq!(renderer.get_cell_alignment(&int_cell, None), Alignment::Right);
        assert_eq!(renderer.get_cell_alignment(&float_cell, None), Alignment::Right);
        assert_eq!(renderer.get_cell_alignment(&text_cell, None), Alignment::Left);
    }

    #[test]
    fn test_type_hint_alignment() {
        let renderer = TableRenderer::new();

        let col_int = ColumnDef {
            name: "count".to_string(),
            type_hint: Some(TypeHint::Integer),
        };
        let col_str = ColumnDef {
            name: "name".to_string(),
            type_hint: Some(TypeHint::String),
        };

        let cell = CellValue::Text("123".to_string());

        // Type hint should override cell type
        assert_eq!(renderer.get_cell_alignment(&cell, Some(&col_int)), Alignment::Right);
        assert_eq!(renderer.get_cell_alignment(&cell, Some(&col_str)), Alignment::Left);
    }

    #[test]
    fn test_empty_table() {
        let table = TableNode {
            schema: vec![ColumnDef {
                name: "col1".to_string(),
                type_hint: None,
            }],
            rows: vec![],
        };
        let renderer = TableRenderer::new();
        let output = renderer.render(&table);

        // Should still have structure
        assert!(output.contains(box_chars::TOP_LEFT));
        assert!(output.contains(box_chars::BOTTOM_LEFT));
        assert!(output.contains("col1"));
    }

    #[test]
    fn test_render_top_border() {
        let renderer = TableRenderer::new();
        let widths = vec![3, 5, 4];
        let border = renderer.render_top_border(&widths);

        assert!(border.starts_with(box_chars::TOP_LEFT));
        assert!(border.ends_with(box_chars::TOP_RIGHT));
        assert!(border.contains(box_chars::T_DOWN));
    }

    #[test]
    fn test_render_header_separator() {
        let renderer = TableRenderer::new();
        let widths = vec![3, 5, 4];
        let separator = renderer.render_header_separator(&widths);

        assert!(separator.starts_with(box_chars::T_RIGHT));
        assert!(separator.ends_with(box_chars::T_LEFT));
        assert!(separator.contains(box_chars::CROSS));
    }

    #[test]
    fn test_render_bottom_border() {
        let renderer = TableRenderer::new();
        let widths = vec![3, 5, 4];
        let border = renderer.render_bottom_border(&widths);

        assert!(border.starts_with(box_chars::BOTTOM_LEFT));
        assert!(border.ends_with(box_chars::BOTTOM_RIGHT));
        assert!(border.contains(box_chars::T_UP));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate a random cell value
    fn arb_cell_value() -> impl Strategy<Value = CellValue> {
        prop_oneof![
            any::<i64>().prop_map(CellValue::Integer),
            any::<f64>()
                .prop_filter("finite floats only", |f| f.is_finite())
                .prop_map(CellValue::Float),
            "[a-zA-Z0-9 ]{0,20}".prop_map(CellValue::Text),
            any::<bool>().prop_map(CellValue::Boolean),
            Just(CellValue::Null),
        ]
    }

    /// Generate a random column definition
    fn arb_column_def() -> impl Strategy<Value = ColumnDef> {
        (
            "[a-zA-Z][a-zA-Z0-9_]{0,10}",
            prop_oneof![
                Just(None),
                Just(Some(TypeHint::String)),
                Just(Some(TypeHint::Integer)),
                Just(Some(TypeHint::Float)),
                Just(Some(TypeHint::Boolean)),
            ],
        )
            .prop_map(|(name, type_hint)| ColumnDef { name, type_hint })
    }

    /// Generate a random table with 1-5 columns and 0-10 rows
    fn arb_table() -> impl Strategy<Value = TableNode> {
        (1usize..=5, 0usize..=10).prop_flat_map(|(num_cols, num_rows)| {
            let schema = prop::collection::vec(arb_column_def(), num_cols);
            let rows =
                prop::collection::vec(prop::collection::vec(arb_cell_value(), num_cols), num_rows);
            (schema, rows).prop_map(|(schema, rows)| TableNode { schema, rows })
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dxm-human-format, Property 11: Unicode Table Structure**
        /// *For any* table rendered with Unicode mode enabled, the output SHALL contain
        /// the required box-drawing characters (┌, ┐, └, ┘, │, ─, ┬, ┴, ├, ┤, ┼)
        /// forming a valid table structure.
        /// **Validates: Requirements 4.1, 2.4**
        #[test]
        fn prop_unicode_table_structure(table in arb_table()) {
            let renderer = TableRenderer::new();
            let output = renderer.render(&table);

            // Must have corners
            prop_assert!(output.contains(box_chars::TOP_LEFT), "Missing top-left corner");
            prop_assert!(output.contains(box_chars::TOP_RIGHT), "Missing top-right corner");
            prop_assert!(output.contains(box_chars::BOTTOM_LEFT), "Missing bottom-left corner");
            prop_assert!(output.contains(box_chars::BOTTOM_RIGHT), "Missing bottom-right corner");

            // Must have horizontal and vertical lines
            prop_assert!(output.contains(box_chars::HORIZONTAL), "Missing horizontal line");
            prop_assert!(output.contains(box_chars::VERTICAL), "Missing vertical line");

            // If more than one column, must have T-junctions
            if table.schema.len() > 1 {
                prop_assert!(output.contains(box_chars::T_DOWN), "Missing T-down junction");
                prop_assert!(output.contains(box_chars::T_UP), "Missing T-up junction");
                prop_assert!(output.contains(box_chars::CROSS), "Missing cross junction");
            }

            // Must have header separator T-junctions
            prop_assert!(output.contains(box_chars::T_RIGHT), "Missing T-right junction");
            prop_assert!(output.contains(box_chars::T_LEFT), "Missing T-left junction");
        }

        /// **Feature: dxm-human-format, Property 5: Table Alignment by Type**
        /// *For any* table with numeric cells, the Table_Renderer SHALL right-align
        /// numeric values, and for text cells, SHALL left-align text values.
        /// **Validates: Requirements 4.3, 4.4**
        #[test]
        fn prop_table_alignment_by_type(table in arb_table()) {
            let renderer = TableRenderer::new();

            for row in &table.rows {
                for (i, cell) in row.iter().enumerate() {
                    let col_def = table.schema.get(i);
                    let alignment = renderer.get_cell_alignment(cell, col_def);

                    // Check alignment based on type hint first, then cell type
                    if let Some(col) = col_def {
                        if let Some(hint) = &col.type_hint {
                            match hint {
                                TypeHint::Integer | TypeHint::Float => {
                                    prop_assert_eq!(alignment, Alignment::Right,
                                        "Numeric type hint should be right-aligned");
                                }
                                TypeHint::String | TypeHint::Boolean | TypeHint::Date => {
                                    prop_assert_eq!(alignment, Alignment::Left,
                                        "Text/boolean type hint should be left-aligned");
                                }
                            }
                            continue;
                        }
                    }

                    // No type hint, check cell type
                    match cell {
                        CellValue::Integer(_) | CellValue::Float(_) => {
                            prop_assert_eq!(alignment, Alignment::Right,
                                "Numeric cells should be right-aligned");
                        }
                        CellValue::Text(_) | CellValue::Boolean(_) | CellValue::Null => {
                            prop_assert_eq!(alignment, Alignment::Left,
                                "Text/boolean/null cells should be left-aligned");
                        }
                    }
                }
            }
        }

        /// **Feature: dxm-human-format, Property 4: Table Column Width Calculation**
        /// *For any* table with varying cell content lengths, the Table_Renderer SHALL
        /// calculate column widths such that all cell content fits within its column
        /// without truncation.
        /// **Validates: Requirements 4.2, 2.5**
        #[test]
        fn prop_column_width_calculation(table in arb_table()) {
            let renderer = TableRenderer::new();
            let widths = renderer.calculate_column_widths(&table);

            // Widths should match number of columns
            prop_assert_eq!(widths.len(), table.schema.len(),
                "Width count should match column count");

            // Each width should be at least as wide as the header
            for (i, col) in table.schema.iter().enumerate() {
                let header_width = col.name.chars().count();
                prop_assert!(widths[i] >= header_width,
                    "Column {} width {} should be >= header width {}",
                    i, widths[i], header_width);
            }

            // Each width should be at least as wide as the widest cell
            for row in &table.rows {
                for (i, cell) in row.iter().enumerate() {
                    if i < widths.len() {
                        let cell_str = renderer.cell_to_string(cell);
                        let cell_width = cell_str.chars().count();
                        prop_assert!(widths[i] >= cell_width,
                            "Column {} width {} should be >= cell width {}",
                            i, widths[i], cell_width);
                    }
                }
            }
        }
    }
}
