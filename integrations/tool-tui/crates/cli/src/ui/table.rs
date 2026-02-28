//! Table formatting for CLI output

use crate::ui::theme::icons;
use owo_colors::OwoColorize;

/// A simple table formatter for CLI output
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    column_widths: Vec<usize>,
}

impl Table {
    pub fn new(headers: Vec<&str>) -> Self {
        let headers: Vec<String> = headers.into_iter().map(String::from).collect();
        let column_widths = headers.iter().map(|h| h.len()).collect();

        Self {
            headers,
            rows: Vec::new(),
            column_widths,
        }
    }

    pub fn add_row(&mut self, row: Vec<&str>) {
        let row: Vec<String> = row.into_iter().map(String::from).collect();

        // Update column widths
        for (i, cell) in row.iter().enumerate() {
            if i < self.column_widths.len() {
                self.column_widths[i] = self.column_widths[i].max(cell.len());
            }
        }

        self.rows.push(row);
    }

    pub fn print(&self) {
        // Print header
        eprint!("  ");
        for (i, header) in self.headers.iter().enumerate() {
            let width = self.column_widths.get(i).copied().unwrap_or(0);
            eprint!("{:width$}  ", header.bright_black().bold().to_string(), width = width);
        }
        eprintln!();

        // Print separator
        eprint!("  ");
        for width in &self.column_widths {
            eprint!("{:─<width$}  ", "", width = *width);
        }
        eprintln!();

        // Print rows
        for row in &self.rows {
            eprint!("  ");
            for (i, cell) in row.iter().enumerate() {
                let width = self.column_widths.get(i).copied().unwrap_or(0);
                eprint!("{:width$}  ", cell.white().to_string(), width = width);
            }
            eprintln!();
        }
    }
}

/// Print a key-value list
pub fn print_kv_list(items: &[(&str, &str)]) {
    let max_key_len = items.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

    for (key, value) in items {
        eprintln!(
            "  {} {:width$}  {}",
            icons::VERTICAL.bright_black(),
            format!("{key}:").bright_black(),
            value.white(),
            width = max_key_len + 1
        );
    }
}

/// Print a file tree
pub fn print_file_tree(files: &[(&str, &str)]) {
    eprintln!();
    for (i, (path, info)) in files.iter().enumerate() {
        let prefix = if i == files.len() - 1 {
            icons::CORNER
        } else {
            icons::TEE
        };
        eprintln!(
            "  {}{}{} {} {}",
            prefix.bright_black(),
            icons::HORIZONTAL.bright_black(),
            icons::HORIZONTAL.bright_black(),
            path.white(),
            format!("({info})").bright_black()
        );
    }
    eprintln!();
}
