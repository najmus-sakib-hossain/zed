//! Tree select prompt for hierarchical selection

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

#[derive(Clone)]
pub struct TreeNode<T: Clone> {
    pub value: T,
    pub label: String,
    pub children: Vec<TreeNode<T>>,
    pub expanded: bool,
}

impl<T: Clone> TreeNode<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            children: Vec::new(),
            expanded: false,
        }
    }

    pub fn child(mut self, child: TreeNode<T>) -> Self {
        self.children.push(child);
        self
    }
}

pub struct TreeSelect<T: Clone> {
    message: String,
    root: Vec<TreeNode<T>>,
    cursor: Vec<usize>,
    state: State,
    last_render_lines: usize,
}

impl<T: Clone> TreeSelect<T> {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            root: Vec::new(),
            cursor: vec![0],
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn node(mut self, node: TreeNode<T>) -> Self {
        self.root.push(node);
        self
    }

    fn get_current_node(&self) -> Option<&TreeNode<T>> {
        let mut current = &self.root;
        for (i, &idx) in self.cursor.iter().enumerate() {
            if idx >= current.len() {
                return None;
            }
            if i == self.cursor.len() - 1 {
                return Some(&current[idx]);
            }
            current = &current[idx].children;
        }
        None
    }

    fn get_current_node_mut(&mut self) -> Option<&mut TreeNode<T>> {
        let mut current = &mut self.root;
        for (i, &idx) in self.cursor.iter().enumerate() {
            if idx >= current.len() {
                return None;
            }
            if i == self.cursor.len() - 1 {
                return Some(&mut current[idx]);
            }
            current = &mut current[idx].children;
        }
        None
    }

    fn render_tree(&self, nodes: &[TreeNode<T>], depth: usize, path: &[usize]) -> Vec<String> {
        let theme = THEME.read().unwrap();
        let mut lines = Vec::new();

        for (i, node) in nodes.iter().enumerate() {
            let mut current_path = path.to_vec();
            current_path.push(i);

            let is_selected = current_path == self.cursor;
            let indent = "  ".repeat(depth);
            let icon = if node.children.is_empty() {
                "  "
            } else if node.expanded {
                "▼ "
            } else {
                "▶ "
            };

            let label = if is_selected {
                theme.primary.apply_to(&node.label).to_string()
            } else {
                node.label.clone()
            };

            let marker = if is_selected { "▸" } else { " " };
            lines.push(format!("{}{}{}{}", marker, indent, icon, label));

            if node.expanded {
                lines.extend(self.render_tree(&node.children, depth + 1, &current_path));
            }
        }

        lines
    }
}

impl<T: Clone> PromptInteraction for TreeSelect<T> {
    type Output = T;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if self.get_current_node().is_some() {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => {
                    self.state = State::Cancel;
                }
                console::Key::ArrowRight | console::Key::Char('l') => {
                    if let Some(node) = self.get_current_node_mut() {
                        if !node.children.is_empty() {
                            if !node.expanded {
                                node.expanded = true;
                            } else {
                                self.cursor.push(0);
                            }
                        }
                    }
                }
                console::Key::ArrowLeft | console::Key::Char('h') => {
                    if let Some(node) = self.get_current_node_mut() {
                        if node.expanded {
                            node.expanded = false;
                        } else if self.cursor.len() > 1 {
                            self.cursor.pop();
                        }
                    }
                }
                console::Key::ArrowUp | console::Key::Char('k') => {
                    if let Some(last) = self.cursor.last_mut() {
                        if *last > 0 {
                            *last -= 1;
                        }
                    }
                }
                console::Key::ArrowDown | console::Key::Char('j') => {
                    if let Some(last) = self.cursor.last_mut() {
                        *last += 1;
                    }
                }
                _ => {}
            },
            Event::Error => {
                self.state = State::Error;
            }
        }
    }

    fn render(&mut self, term: &Term) -> io::Result<()> {
        if self.last_render_lines > 0 {
            for _ in 0..self.last_render_lines {
                term.move_cursor_up(1)?;
                term.clear_line()?;
            }
        }

        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        let mut lines = 0;

        match self.state {
            State::Active => {
                let bar = theme.dim.apply_to(symbols.bar);
                term.write_line(&format!(
                    "{}{}",
                    theme.primary.apply_to(symbols.step_submit),
                    format!("  {}  ", self.message).bold()
                ))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                let tree_lines = self.render_tree(&self.root, 0, &[]);
                for line in tree_lines {
                    term.write_line(&format!("{}  {}", bar, line))?;
                    lines += 1;
                }

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("↑↓: navigate, →: expand, ←: collapse, Enter: select")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let selected = self.get_current_node().map(|n| n.label.clone()).unwrap_or_default();
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(selected)
                ))?;
                lines += 1;
                term.write_line(&format!("{}", theme.dim.apply_to(symbols.bar)))?;
                lines += 1;
            }
            _ => {}
        }

        self.last_render_lines = lines;
        Ok(())
    }

    fn value(&self) -> T {
        self.get_current_node().map(|n| n.value.clone()).expect("No node selected")
    }
}

pub fn tree_select<T: Clone>(message: impl Into<String>) -> TreeSelect<T> {
    TreeSelect::new(message)
}
