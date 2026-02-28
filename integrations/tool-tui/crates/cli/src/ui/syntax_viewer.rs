use anyhow::{Context, Result};
use std::path::Path;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, ThemeSet},
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};

/// Syntax highlighter for displaying code files
pub struct SyntaxViewer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxViewer {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Display a file with syntax highlighting
    pub fn show_file(&self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");

        self.show_code(&content, extension, Some(path.display().to_string()))
    }

    /// Display code with syntax highlighting
    pub fn show_code(&self, code: &str, language: &str, title: Option<String>) -> Result<()> {
        use owo_colors::OwoColorize;

        // Find syntax definition
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(language)
            .or_else(|| self.syntax_set.find_syntax_by_name(language))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Use a dark theme
        let theme = &self.theme_set.themes["base16-ocean.dark"];

        // Print header
        println!("\n{}", "═".repeat(80).bright_cyan());
        if let Some(title) = title {
            println!("  {} {}", "📄".bright_yellow(), title.bright_white().bold());
        }
        println!("  {} Language: {}", "│".bright_black(), syntax.name.bright_cyan());
        println!("{}", "═".repeat(80).bright_cyan());
        println!();

        // Highlight and print
        let mut highlighter = HighlightLines::new(syntax, theme);

        for (line_num, line) in LinesWithEndings::from(code).enumerate() {
            let ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &self.syntax_set)
                .context("Failed to highlight line")?;

            let line_number = format!("{:4} │ ", line_num + 1);
            print!("{}", line_number.bright_black());
            print!("{}", as_24_bit_terminal_escaped(&ranges[..], false));
        }

        println!("\n{}", "═".repeat(80).bright_cyan());
        println!();

        Ok(())
    }

    /// List all supported languages
    pub fn list_languages(&self) -> Vec<String> {
        let mut languages: Vec<String> =
            self.syntax_set.syntaxes().iter().map(|s| s.name.clone()).collect();
        languages.sort();
        languages
    }
}

impl Default for SyntaxViewer {
    fn default() -> Self {
        Self::new()
    }
}

/// Show syntax highlighting demo
pub fn demo_syntax_highlighting() -> Result<()> {
    use owo_colors::OwoColorize;

    let viewer = SyntaxViewer::new();

    println!("\n{}", "═".repeat(80).bright_cyan());
    println!("  {}", "DX CLI Syntax Highlighting Demo".bright_white().bold());
    println!("{}\n", "═".repeat(80).bright_cyan());

    // Rust example
    let rust_code = r#"use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("hello", "world");
    
    for (key, value) in &map {
        println!("{}: {}", key, value);
    }
}
"#;
    viewer.show_code(rust_code, "rs", Some("Example: Rust".to_string()))?;

    // JavaScript example
    let js_code = r#"const express = require('express');
const app = express();

app.get('/', (req, res) => {
    res.json({ message: 'Hello World!' });
});

app.listen(3000, () => {
    console.log('Server running on port 3000');
});
"#;
    viewer.show_code(js_code, "js", Some("Example: JavaScript".to_string()))?;

    // TypeScript example
    let ts_code = r#"interface User {
    id: number;
    name: string;
    email: string;
}

class UserService {
    private users: User[] = [];

    addUser(user: User): void {
        this.users.push(user);
    }

    getUser(id: number): User | undefined {
        return this.users.find(u => u.id === id);
    }
}
"#;
    viewer.show_code(ts_code, "ts", Some("Example: TypeScript".to_string()))?;

    // Python example
    let python_code = r#"from typing import List, Dict
import asyncio

class DataProcessor:
    def __init__(self, data: List[Dict]):
        self.data = data
    
    async def process(self) -> List[Dict]:
        results = []
        for item in self.data:
            result = await self._process_item(item)
            results.append(result)
        return results
    
    async def _process_item(self, item: Dict) -> Dict:
        await asyncio.sleep(0.1)
        return {**item, 'processed': True}
"#;
    viewer.show_code(python_code, "py", Some("Example: Python".to_string()))?;

    // JSON example
    let json_code = r#"{
  "name": "dx-cli",
  "version": "0.1.0",
  "dependencies": {
    "syntect": "5.2",
    "similar": "2.6"
  },
  "features": {
    "syntax-highlighting": true,
    "diff-viewer": true
  }
}
"#;
    viewer.show_code(json_code, "json", Some("Example: JSON".to_string()))?;

    println!("{}", "Supported Languages:".bright_yellow().bold());
    println!("{}", "─".repeat(80).bright_black());

    let languages = viewer.list_languages();
    for chunk in languages.chunks(5) {
        println!("  {}", chunk.iter().map(|l| format!("{:15}", l)).collect::<Vec<_>>().join(" "));
    }
    println!();

    Ok(())
}
