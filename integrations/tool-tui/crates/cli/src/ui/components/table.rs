//! Table component

pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub widths: Vec<usize>,
}

impl Table {
    pub fn new(headers: Vec<String>) -> Self {
        let widths = headers.iter().map(|h| h.len()).collect();
        Self {
            headers,
            rows: Vec::new(),
            widths,
        }
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        for (i, cell) in row.iter().enumerate() {
            if i < self.widths.len() {
                self.widths[i] = self.widths[i].max(cell.len());
            }
        }
        self.rows.push(row);
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        // Header
        let header_line = self
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| format!("{:width$}", h, width = self.widths[i]))
            .collect::<Vec<_>>()
            .join(" │ ");
        output.push_str(&format!("│ {} │\n", header_line));

        // Separator
        let sep = self.widths.iter().map(|w| "─".repeat(*w)).collect::<Vec<_>>().join("─┼─");
        output.push_str(&format!("├─{}─┤\n", sep));

        // Rows
        for row in &self.rows {
            let row_line = row
                .iter()
                .enumerate()
                .map(|(i, cell)| format!("{:width$}", cell, width = self.widths[i]))
                .collect::<Vec<_>>()
                .join(" │ ");
            output.push_str(&format!("│ {} │\n", row_line));
        }

        output
    }
}
