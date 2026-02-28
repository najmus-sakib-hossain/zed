//! Complete prompt system with all 39 components
#![allow(unused_imports)]

pub mod cursor;
pub mod interaction;

pub mod autocomplete;
pub mod calendar;
pub mod code_snippet;
pub mod color_picker;
pub mod color_picker_advanced;
pub mod confirm;
pub mod credit_card;
pub mod date_picker;
pub mod email;
pub mod emoji_picker;
pub mod file_browser;
pub mod input;
pub mod json_editor;
pub mod kanban;
pub mod list;
pub mod markdown_editor;
pub mod matrix_select;
pub mod multiselect;
pub mod number;
pub mod password;
pub mod phone_input;
pub mod progress;
pub mod range_slider;
pub mod rating;
pub mod search_filter;
pub mod select;
pub mod slider;
pub mod spinner;
pub mod table_editor;
pub mod tags;
pub mod text;
pub mod time_picker;
pub mod toggle;
pub mod tree_select;
pub mod url;
pub mod wizard;

use console::Term;
use once_cell::sync::Lazy;
use std::fmt::Display;
use std::io;
use std::sync::RwLock;
use textwrap::wrap;

pub use autocomplete::{Autocomplete, AutocompleteItem, autocomplete};
pub use calendar::{CalendarView, calendar};
pub use code_snippet::{CodeSnippet, CodeSnippetPicker, code_snippet};
pub use color_picker_advanced::{ColorMode, ColorPickerAdvanced, color_picker_advanced};
pub use confirm::Confirm;
pub use credit_card::{CreditCardInput, credit_card};
pub use date_picker::{DatePicker, date_picker};
pub use email::{EmailInput, email};
pub use emoji_picker::{EmojiPicker, emoji_picker};
pub use file_browser::{FileBrowser, file_browser};
pub use input::Input;
pub use interaction::{PromptInteraction, State, Validate};
pub use json_editor::{JsonEditor, json_editor};
pub use kanban::{KanbanBoard, KanbanTask, kanban};
pub use list::{ListEditor, list_editor};
pub use markdown_editor::{MarkdownEditor, markdown_editor};
pub use matrix_select::{MatrixSelect, matrix_select};
pub use multiselect::{MultiSelect, MultiSelectItem};
pub use number::{Number, number};
pub use password::Password;
pub use phone_input::{PhoneInput, phone_input};
pub use progress::ProgressBar;
pub use range_slider::{RangeSlider, range_slider};
pub use rating::{Rating, rating};
pub use search_filter::{SearchFilter, search_filter};
pub use select::{Select, SelectItem};
pub use slider::{Slider, slider};
pub use spinner::Spinner;
pub use table_editor::{TableEditor, table_editor};
pub use tags::{Tags, tags};
pub use text::{Text, text};
pub use time_picker::{TimePicker, time_picker};
pub use toggle::{Toggle, toggle};
pub use tree_select::{TreeNode, TreeSelect, tree_select};
pub use url::{UrlInput, url};
pub use wizard::{Wizard, WizardStep, wizard};

// ─────────────────────────────────────────────────────────────────────────────
// Theme Configuration
// ─────────────────────────────────────────────────────────────────────────────

pub struct DxTheme {
    pub primary: console::Style,
    pub success: console::Style,
    pub warning: console::Style,
    pub error: console::Style,
    pub dim: console::Style,
}

impl Default for DxTheme {
    fn default() -> Self {
        Self {
            primary: console::Style::new().cyan(),
            success: console::Style::new().green(),
            warning: console::Style::new().yellow(),
            error: console::Style::new().red(),
            dim: console::Style::new().dim(),
        }
    }
}

pub static THEME: Lazy<RwLock<DxTheme>> = Lazy::new(|| RwLock::new(DxTheme::default()));

// ─────────────────────────────────────────────────────────────────────────────
// Symbols
// ─────────────────────────────────────────────────────────────────────────────

pub struct Symbols {
    pub step_active: &'static str,
    pub step_cancel: &'static str,
    pub step_error: &'static str,
    pub step_submit: &'static str,
    pub bar_start: &'static str,
    pub bar: &'static str,
    pub bar_end: &'static str,
    pub radio_active: &'static str,
    pub radio_inactive: &'static str,
    pub checkbox_active: &'static str,
    pub checkbox_selected: &'static str,
    pub checkbox_inactive: &'static str,
    pub password_mask: char,
    pub bar_h: &'static str,
    pub corner_top_right: &'static str,
    pub connect_left: &'static str,
    pub corner_bottom_right: &'static str,
    pub box_top_left: &'static str,
    pub box_top_right: &'static str,
    pub box_bottom_left: &'static str,
    pub box_bottom_right: &'static str,
    pub box_horizontal: &'static str,
    pub box_vertical: &'static str,
    pub box_left_t: &'static str,
    pub box_right_t: &'static str,
}

impl Symbols {
    const fn unicode() -> Self {
        Self {
            step_active: "♦",
            step_cancel: "■",
            step_error: "▲",
            step_submit: "♦",
            bar_start: "┌",
            bar: "│",
            bar_end: "└",
            radio_active: "●",
            radio_inactive: "○",
            checkbox_active: "◻",
            checkbox_selected: "◼",
            checkbox_inactive: "◻",
            password_mask: '•',
            bar_h: "─",
            corner_top_right: "╮",
            connect_left: "├",
            corner_bottom_right: "╯",
            box_top_left: "┌",
            box_top_right: "╮",
            box_bottom_left: "├",
            box_bottom_right: "╯",
            box_horizontal: "─",
            box_vertical: "│",
            box_left_t: "├",
            box_right_t: "╯",
        }
    }
}

pub static SYMBOLS: Lazy<Symbols> = Lazy::new(Symbols::unicode);

// ─────────────────────────────────────────────────────────────────────────────
// Public API Functions
// ─────────────────────────────────────────────────────────────────────────────

fn term_write(line: impl Display) -> io::Result<()> {
    Term::stderr().write_str(line.to_string().as_str())
}

pub fn intro(title: impl Display) -> io::Result<()> {
    let theme = THEME.read().unwrap();
    let symbols = &*SYMBOLS;
    term_write(format!(
        "{}{}{}",
        theme.dim.apply_to(symbols.bar_start),
        theme.dim.apply_to("─"),
        format!(" {}", title)
    ))?;
    term_write("\n")?;
    term_write(format!("{}\n", theme.dim.apply_to(symbols.bar)))
}

pub fn outro(message: impl Display) -> io::Result<()> {
    let theme = THEME.read().unwrap();
    let symbols = &*SYMBOLS;
    term_write(format!(
        "{}{} {}\n",
        theme.dim.apply_to(symbols.bar),
        theme.success.apply_to(symbols.step_submit),
        message,
    ))
}

fn render_box_section(title: &str, lines: &[&str], min_content_width: usize) -> io::Result<()> {
    let theme = THEME.read().unwrap();
    let symbols = &*SYMBOLS;
    let bar = theme.dim.apply_to(symbols.bar);

    let terminal_columns = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(120)
        .max(40);

    // Content row format is: "│" + content + "│"
    let max_box_content_width = terminal_columns.saturating_sub(2).max(20);

    let requested_content_width = lines
        .iter()
        .map(|line| line.chars().count().saturating_add(2))
        .max()
        .unwrap_or(0)
        .max(min_content_width)
        .min(max_box_content_width);

    let wrapped_lines = lines
        .iter()
        .flat_map(|line| {
            let wraps = wrap(line, requested_content_width.saturating_sub(2).max(1));
            if wraps.is_empty() {
                vec![String::new()]
            } else {
                wraps.into_iter().map(|item| item.into_owned()).collect()
            }
        })
        .collect::<Vec<_>>();

    let max_content_len = wrapped_lines
        .iter()
        .map(|line| format!("  {}", line).chars().count())
        .max()
        .unwrap_or(0)
        .max(min_content_width)
        .min(max_box_content_width);

    let title_with_spaces = format!(" {}  ", title);
    let title_len = title_with_spaces.chars().count();
    let remaining = max_content_len.saturating_sub(title_len);

    term_write(format!(
        "{}{}{}{}",
        theme.dim.apply_to(symbols.bar),
        title_with_spaces,
        theme.dim.apply_to(symbols.box_horizontal.repeat(remaining)),
        theme.dim.apply_to(symbols.box_top_right)
    ))?;
    term_write("\n")?;

    term_write(format!("{}{}{}\n", bar, " ".repeat(max_content_len), bar))?;

    for line in &wrapped_lines {
        let content = format!("  {}", line);
        let content_len = content.chars().count();
        let spaces_needed = max_content_len.saturating_sub(content_len);
        term_write(format!("{}{}{}{}\n", bar, content, " ".repeat(spaces_needed), bar))?;
    }

    term_write(format!("{}{}{}\n", bar, " ".repeat(max_content_len), bar))?;

    term_write(format!(
        "{}{}{}\n",
        theme.dim.apply_to(symbols.box_bottom_left),
        theme.dim.apply_to(symbols.box_horizontal.repeat(max_content_len)),
        theme.dim.apply_to(symbols.box_bottom_right)
    ))?;

    term_write(format!("{}\n", bar))
}

pub fn section_with_width<F>(title: &str, content_width: usize, build: F) -> io::Result<()>
where
    F: FnOnce(&mut Vec<String>),
{
    let mut lines: Vec<String> = Vec::new();
    build(&mut lines);
    let line_refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    render_box_section(title, &line_refs, content_width)
}

pub fn box_section(title: &str, lines: &[&str]) -> io::Result<()> {
    render_box_section(title, lines, 83)
}

pub fn confirm(prompt: impl Into<String>) -> Confirm {
    Confirm::new(prompt.into())
}

pub fn select<T: Clone>(prompt: impl Into<String>) -> Select<T> {
    Select::new(prompt.into())
}

pub fn multiselect<T: Clone>(prompt: impl Into<String>) -> MultiSelect<T> {
    MultiSelect::new(prompt.into())
}

pub mod log {
    use super::*;
    use owo_colors::OwoColorize;

    pub fn info(text: impl Display) -> io::Result<()> {
        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        eprintln!("{} {}", "●".blue(), text);
        eprintln!("{}", theme.dim.apply_to(symbols.bar));
        Ok(())
    }

    pub fn success(text: impl Display) -> io::Result<()> {
        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        eprintln!("{} {}", "✓".green().bold(), text);
        eprintln!("{}", theme.dim.apply_to(symbols.bar));
        Ok(())
    }

    pub fn warning(text: impl Display) -> io::Result<()> {
        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        eprintln!("{} {}", "⚠".yellow().bold(), text);
        eprintln!("{}", theme.dim.apply_to(symbols.bar));
        Ok(())
    }

    pub fn step(text: impl Display) -> io::Result<()> {
        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        eprintln!("{} {} {}", theme.dim.apply_to(symbols.bar), "◇".green(), text);
        Ok(())
    }
}
