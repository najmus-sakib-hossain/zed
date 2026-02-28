//! WASM bindings for dx-markdown
//!
//! This module provides WebAssembly bindings for the dx-markdown compiler,
//! allowing it to be used in VS Code extensions and web applications.

use crate::types::{CompilerMode, TokenizerType};
use crate::{CompilerConfig, DxMarkdown};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct MarkdownCompiler {
    compiler: DxMarkdown,
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct CompileOptions {
    strip_urls: bool,
    strip_images: bool,
    strip_badges: bool,
    tables_to_tsv: bool,
    minify_code: bool,
    collapse_whitespace: bool,
    strip_filler: bool,
    dictionary: bool,
}

#[wasm_bindgen]
impl CompileOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            strip_urls: true,
            strip_images: true,
            strip_badges: true,
            tables_to_tsv: true,
            minify_code: false,
            collapse_whitespace: true,
            strip_filler: true,
            dictionary: false,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn strip_urls(&self) -> bool {
        self.strip_urls
    }

    #[wasm_bindgen(setter)]
    pub fn set_strip_urls(&mut self, value: bool) {
        self.strip_urls = value;
    }

    #[wasm_bindgen(getter)]
    pub fn strip_images(&self) -> bool {
        self.strip_images
    }

    #[wasm_bindgen(setter)]
    pub fn set_strip_images(&mut self, value: bool) {
        self.strip_images = value;
    }

    #[wasm_bindgen(getter)]
    pub fn strip_badges(&self) -> bool {
        self.strip_badges
    }

    #[wasm_bindgen(setter)]
    pub fn set_strip_badges(&mut self, value: bool) {
        self.strip_badges = value;
    }

    #[wasm_bindgen(getter)]
    pub fn tables_to_tsv(&self) -> bool {
        self.tables_to_tsv
    }

    #[wasm_bindgen(setter)]
    pub fn set_tables_to_tsv(&mut self, value: bool) {
        self.tables_to_tsv = value;
    }

    #[wasm_bindgen(getter)]
    pub fn minify_code(&self) -> bool {
        self.minify_code
    }

    #[wasm_bindgen(setter)]
    pub fn set_minify_code(&mut self, value: bool) {
        self.minify_code = value;
    }

    #[wasm_bindgen(getter)]
    pub fn collapse_whitespace(&self) -> bool {
        self.collapse_whitespace
    }

    #[wasm_bindgen(setter)]
    pub fn set_collapse_whitespace(&mut self, value: bool) {
        self.collapse_whitespace = value;
    }

    #[wasm_bindgen(getter)]
    pub fn strip_filler(&self) -> bool {
        self.strip_filler
    }

    #[wasm_bindgen(setter)]
    pub fn set_strip_filler(&mut self, value: bool) {
        self.strip_filler = value;
    }

    #[wasm_bindgen(getter)]
    pub fn dictionary(&self) -> bool {
        self.dictionary
    }

    #[wasm_bindgen(setter)]
    pub fn set_dictionary(&mut self, value: bool) {
        self.dictionary = value;
    }
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
pub struct CompileResult {
    tokens_before: usize,
    tokens_after: usize,
    output: String,
}

#[wasm_bindgen]
impl CompileResult {
    #[wasm_bindgen(getter)]
    pub fn output(&self) -> String {
        self.output.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn tokens_before(&self) -> usize {
        self.tokens_before
    }

    #[wasm_bindgen(getter)]
    pub fn tokens_after(&self) -> usize {
        self.tokens_after
    }

    #[wasm_bindgen]
    pub fn savings_percent(&self) -> f64 {
        if self.tokens_before == 0 {
            return 0.0;
        }
        ((self.tokens_before - self.tokens_after) as f64 / self.tokens_before as f64) * 100.0
    }
}

#[wasm_bindgen]
impl MarkdownCompiler {
    #[wasm_bindgen(constructor)]
    pub fn new(options: CompileOptions) -> Result<MarkdownCompiler, JsValue> {
        #[cfg(feature = "wasm")]
        console_error_panic_hook::set_once();

        let config = CompilerConfig {
            mode: CompilerMode::Full,
            strip_urls: options.strip_urls,
            strip_images: options.strip_images,
            strip_badges: options.strip_badges,
            tables_to_tsv: options.tables_to_tsv,
            minify_code: options.minify_code,
            collapse_whitespace: options.collapse_whitespace,
            strip_filler: options.strip_filler,
            dictionary: options.dictionary,
            tokenizer: TokenizerType::Cl100k,
            llm_headers: false,
        };

        let compiler = DxMarkdown::new(config)
            .map_err(|e| JsValue::from_str(&format!("Failed to create compiler: {}", e)))?;

        Ok(MarkdownCompiler { compiler })
    }

    #[wasm_bindgen]
    pub fn compile(&self, input: &str) -> Result<CompileResult, JsValue> {
        let result = self
            .compiler
            .compile(input)
            .map_err(|e| JsValue::from_str(&format!("Compilation failed: {}", e)))?;

        Ok(CompileResult {
            output: result.output,
            tokens_before: result.tokens_before,
            tokens_after: result.tokens_after,
        })
    }
}

#[wasm_bindgen]
pub fn compile_markdown(input: &str, options: CompileOptions) -> Result<CompileResult, JsValue> {
    let compiler = MarkdownCompiler::new(options)?;
    compiler.compile(input)
}

/// Convert LLM format to Human format
#[wasm_bindgen]
pub fn llm_to_human(input: &str) -> Result<String, JsValue> {
    // Use MarkdownBeautifier which is what the CLI uses
    let beautifier = crate::MarkdownBeautifier::new();
    beautifier
        .beautify(input)
        .map_err(|e| JsValue::from_str(&format!("LLM to Human conversion failed: {}", e)))
}

/// Convert Human format to LLM format (DX-Markdown LLM format)
#[wasm_bindgen]
pub fn human_to_llm(input: &str) -> Result<String, JsValue> {
    // For regular markdown files, we want DX-Markdown LLM format:
    // - Keep # headers (NOT convert to N| syntax)
    // - Remove blank lines
    // - Convert tables to compact format
    // - Strip URLs, images, badges (optional)

    // Use the markdown compiler with LLM optimization settings
    let config = crate::CompilerConfig {
        mode: crate::types::CompilerMode::Full,
        strip_urls: false, // Keep URLs for now
        strip_images: false,
        strip_badges: false,
        tables_to_tsv: true,
        minify_code: false,
        collapse_whitespace: true,
        strip_filler: true,
        dictionary: false,
        tokenizer: crate::types::TokenizerType::Cl100k,
        llm_headers: false, // Keep # headers, don't convert to N|
    };

    let compiler = crate::DxMarkdown::new(config)
        .map_err(|e| JsValue::from_str(&format!("Failed to create compiler: {}", e)))?;

    let result = compiler
        .compile(input)
        .map_err(|e| JsValue::from_str(&format!("Compilation failed: {}", e)))?;

    Ok(result.output)
}

/// Convert LLM format to Machine (binary) format
#[wasm_bindgen]
pub fn llm_to_machine(input: &str) -> Result<Vec<u8>, JsValue> {
    crate::convert::llm_to_machine(input)
        .map_err(|e| JsValue::from_str(&format!("LLM to Machine conversion failed: {}", e)))
}

/// Convert Machine (binary) format to LLM format
#[wasm_bindgen]
pub fn machine_to_llm(input: &[u8]) -> Result<String, JsValue> {
    crate::convert::machine_to_llm(input)
        .map_err(|e| JsValue::from_str(&format!("Machine to LLM conversion failed: {}", e)))
}

/// Convert Human format to Machine (binary) format
#[wasm_bindgen]
pub fn human_to_machine(input: &str) -> Result<Vec<u8>, JsValue> {
    crate::convert::human_to_machine(input)
        .map_err(|e| JsValue::from_str(&format!("Human to Machine conversion failed: {}", e)))
}

/// Convert Machine (binary) format to Human format
#[wasm_bindgen]
pub fn machine_to_human(input: &[u8]) -> Result<String, JsValue> {
    crate::convert::machine_to_human(input)
        .map_err(|e| JsValue::from_str(&format!("Machine to Human conversion failed: {}", e)))
}

/// Apply red list filters to markdown content
#[wasm_bindgen]
pub fn apply_red_list_filters(input: &str, config_json: &str) -> Result<String, JsValue> {
    use crate::red_list_config::RedListConfig;

    let config: RedListConfig = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid config JSON: {}", e)))?;

    // Apply filters to the markdown content
    let filtered = apply_filters_internal(input, &config);

    Ok(filtered)
}

fn apply_filters_internal(content: &str, config: &crate::red_list_config::RedListConfig) -> String {
    let mut result = content.to_string();

    // Apply element filters
    if config.elements.remove_images {
        result = remove_images(&result);
    }

    if config.elements.remove_links {
        result = remove_links(&result);
    }

    if config.elements.remove_horizontal_rules {
        result = remove_horizontal_rules(&result);
    }

    if config.elements.remove_blockquotes {
        result = remove_blockquotes(&result);
    }

    if config.elements.remove_code_blocks {
        result = remove_code_blocks(&result);
    }

    if config.elements.remove_inline_code {
        result = remove_inline_code(&result);
    }

    if config.elements.remove_emphasis {
        result = remove_emphasis(&result);
    }

    if config.elements.remove_html {
        result = remove_html(&result);
    }

    // Apply section filters
    let sections_to_remove = config.sections.get_sections_to_remove();
    if !sections_to_remove.is_empty() {
        result = remove_sections(&result, &sections_to_remove);
    }

    result
}

fn remove_images(content: &str) -> String {
    // Remove ![alt](url) patterns
    let re = regex::Regex::new(r"!\[([^\]]*)\]\([^\)]+\)").unwrap();
    re.replace_all(content, "").to_string()
}

fn remove_links(content: &str) -> String {
    // Replace [text](url) with just text
    let re = regex::Regex::new(r"\[([^\]]+)\]\([^\)]+\)").unwrap();
    re.replace_all(content, "$1").to_string()
}

fn remove_horizontal_rules(content: &str) -> String {
    let re = regex::Regex::new(r"(?m)^[\s]*[-*_]{3,}[\s]*$").unwrap();
    re.replace_all(content, "").to_string()
}

fn remove_blockquotes(content: &str) -> String {
    let re = regex::Regex::new(r"(?m)^>\s*(.*)$").unwrap();
    re.replace_all(content, "$1").to_string()
}

fn remove_code_blocks(content: &str) -> String {
    let re = regex::Regex::new(r"(?s)```[^\n]*\n.*?```").unwrap();
    re.replace_all(content, "").to_string()
}

fn remove_inline_code(content: &str) -> String {
    let re = regex::Regex::new(r"`([^`]+)`").unwrap();
    re.replace_all(content, "$1").to_string()
}

fn remove_emphasis(content: &str) -> String {
    let mut result = content.to_string();
    // Remove **bold**
    let re = regex::Regex::new(r"\*\*([^\*]+)\*\*").unwrap();
    result = re.replace_all(&result, "$1").to_string();
    // Remove *italic*
    let re = regex::Regex::new(r"\*([^\*]+)\*").unwrap();
    result = re.replace_all(&result, "$1").to_string();
    // Remove __bold__
    let re = regex::Regex::new(r"__([^_]+)__").unwrap();
    result = re.replace_all(&result, "$1").to_string();
    // Remove _italic_
    let re = regex::Regex::new(r"_([^_]+)_").unwrap();
    result = re.replace_all(&result, "$1").to_string();
    result
}

fn remove_html(content: &str) -> String {
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(content, "").to_string()
}

fn remove_sections(content: &str, sections: &[String]) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut skip_until_next_section = false;

    for line in lines {
        // Check if this is a heading
        if line.trim_start().starts_with('#') {
            let heading_text = line.trim_start().trim_start_matches('#').trim();

            // Check if this heading should be removed
            if sections.iter().any(|s| heading_text.eq_ignore_ascii_case(s)) {
                skip_until_next_section = true;
                continue;
            } else {
                skip_until_next_section = false;
            }
        }

        if !skip_until_next_section {
            result.push(line);
        }
    }

    result.join("\n")
}

/// Render text using a FIGlet font
#[wasm_bindgen]
pub fn render_figlet(text: &str, font_name: &str) -> Option<String> {
    crate::figlet_manager::render_figlet(text, font_name)
}

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "wasm")]
    console_error_panic_hook::set_once();
}
