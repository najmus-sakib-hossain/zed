use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DragDropItem {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DropZone {
    Source,
    Target,
    Trash,
}

pub struct DragDropManager {
    source_items: Vec<DragDropItem>,
    target_items: Vec<DragDropItem>,
    dragging_item: Option<(usize, DragDropItem, DropZone)>,
    hover_zone: Option<DropZone>,
    source_selected: usize,
    target_selected: usize,
}

impl DragDropManager {
    pub fn new(items: Vec<DragDropItem>) -> Self {
        Self {
            source_items: items,
            target_items: Vec::new(),
            dragging_item: None,
            hover_zone: None,
            source_selected: 0,
            target_selected: 0,
        }
    }

    pub fn start_drag(&mut self, zone: DropZone) {
        match zone {
            DropZone::Source => {
                if self.source_selected < self.source_items.len() {
                    let item = self.source_items[self.source_selected].clone();
                    self.dragging_item = Some((self.source_selected, item, zone));
                }
            }
            DropZone::Target => {
                if self.target_selected < self.target_items.len() {
                    let item = self.target_items[self.target_selected].clone();
                    self.dragging_item = Some((self.target_selected, item, zone));
                }
            }
            _ => {}
        }
    }

    pub fn drop_item(&mut self, target_zone: DropZone) {
        if let Some((index, item, source_zone)) = self.dragging_item.take() {
            match (source_zone, target_zone) {
                (DropZone::Source, DropZone::Target) => {
                    self.target_items.push(item);
                }
                (DropZone::Target, DropZone::Source) => {
                    self.source_items.push(item);
                }
                (DropZone::Source, DropZone::Trash) | (DropZone::Target, DropZone::Trash) => {
                    // Item deleted
                }
                (DropZone::Source, DropZone::Source) => {
                    // No-op, item stays
                }
                (DropZone::Target, DropZone::Target) => {
                    // No-op, item stays
                }
                _ => {}
            }

            // Remove from source if moved
            if source_zone != target_zone && target_zone != DropZone::Trash {
                match source_zone {
                    DropZone::Source => {
                        if index < self.source_items.len() {
                            self.source_items.remove(index);
                        }
                    }
                    DropZone::Target => {
                        if index < self.target_items.len() {
                            self.target_items.remove(index);
                        }
                    }
                    _ => {}
                }
            } else if target_zone == DropZone::Trash {
                // Remove from source when trashed
                match source_zone {
                    DropZone::Source => {
                        if index < self.source_items.len() {
                            self.source_items.remove(index);
                        }
                    }
                    DropZone::Target => {
                        if index < self.target_items.len() {
                            self.target_items.remove(index);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn cancel_drag(&mut self) {
        self.dragging_item = None;
        self.hover_zone = None;
    }

    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> bool {
        match code {
            KeyCode::Char('d') if mods.contains(KeyModifiers::CONTROL) => {
                // Start drag with Ctrl+D
                self.start_drag(DropZone::Source);
                true
            }
            KeyCode::Char('t') if self.dragging_item.is_some() => {
                // Drop to target with 't'
                self.drop_item(DropZone::Target);
                true
            }
            KeyCode::Char('s') if self.dragging_item.is_some() => {
                // Drop to source with 's'
                self.drop_item(DropZone::Source);
                true
            }
            KeyCode::Delete if self.dragging_item.is_some() => {
                // Drop to trash with Delete
                self.drop_item(DropZone::Trash);
                true
            }
            KeyCode::Esc if self.dragging_item.is_some() => {
                // Cancel drag with Esc
                self.cancel_drag();
                true
            }
            KeyCode::Up => {
                if self.dragging_item.is_none() {
                    self.source_selected = self.source_selected.saturating_sub(1);
                }
                true
            }
            KeyCode::Down => {
                if self.dragging_item.is_none() {
                    self.source_selected =
                        (self.source_selected + 1).min(self.source_items.len().saturating_sub(1));
                }
                true
            }
            KeyCode::Tab => {
                // Switch between source and target
                if self.dragging_item.is_none() {
                    std::mem::swap(&mut self.source_selected, &mut self.target_selected);
                }
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Title with instructions
        let title_text = vec![Line::from(vec![
            Span::styled(
                "Drag & Drop Demo",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - "),
            Span::styled("Ctrl+D", Style::default().fg(Color::Green)),
            Span::raw(" to drag, "),
            Span::styled("T", Style::default().fg(Color::Yellow)),
            Span::raw("/"),
            Span::styled("S", Style::default().fg(Color::Yellow)),
            Span::raw(" to drop, "),
            Span::styled("Del", Style::default().fg(Color::Red)),
            Span::raw(" to trash"),
        ])];
        let title = Paragraph::new(title_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(title, main_layout[0]);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Percentage(35),
                Constraint::Percentage(30),
            ])
            .split(main_layout[1]);

        // Render source list with better styling
        self.render_zone(
            f,
            chunks[0],
            DropZone::Source,
            "📁 Source Files",
            &self.source_items,
            self.source_selected,
        );

        // Render target list with better styling
        self.render_zone(
            f,
            chunks[1],
            DropZone::Target,
            "📂 Target Folder",
            &self.target_items,
            self.target_selected,
        );

        // Render trash zone with better styling
        self.render_trash(f, chunks[2]);

        // Render drag indicator with animation
        if let Some((_, item, source)) = &self.dragging_item {
            self.render_drag_indicator(f, item, *source);
        }
    }

    fn render_zone(
        &self,
        f: &mut Frame,
        area: Rect,
        zone: DropZone,
        title: &str,
        items: &[DragDropItem],
        selected: usize,
    ) {
        let is_dragging_from_here = if let Some((_, _, source)) = &self.dragging_item {
            *source == zone
        } else {
            false
        };

        let border_color = if is_dragging_from_here {
            Color::Magenta
        } else {
            Color::Cyan
        };

        let border_style = if is_dragging_from_here {
            Modifier::BOLD
        } else {
            Modifier::empty()
        };

        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = i == selected;
                let is_being_dragged = if let Some((idx, _, src)) = &self.dragging_item {
                    *src == zone && *idx == i
                } else {
                    false
                };

                let style = if is_being_dragged {
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)
                } else if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let prefix = if is_being_dragged {
                    "⋯ "
                } else if is_selected {
                    "▶ "
                } else {
                    "  "
                };

                let icon = if item.content.contains("File") {
                    "📄"
                } else {
                    "📋"
                };

                ListItem::new(format!("{}{} {}", prefix, icon, item.content)).style(style)
            })
            .collect();

        let count_text = format!(" ({} items) ", items.len());
        let full_title = format!("{}{}", title, count_text);

        let list = List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color).add_modifier(border_style))
                .title(full_title),
        );

        f.render_widget(list, area);
    }

    fn render_trash(&self, f: &mut Frame, area: Rect) {
        let is_hover = self.hover_zone == Some(DropZone::Trash);
        let border_color = if is_hover {
            Color::Red
        } else {
            Color::DarkGray
        };

        let trash_text = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "🗑️",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "TRASH",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled("Drop here", Style::default().fg(Color::Gray))),
            Line::from(Span::styled("to delete", Style::default().fg(Color::Gray))),
        ];

        let paragraph = Paragraph::new(trash_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title("🗑️  Delete Zone"),
            )
            .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(paragraph, area);
    }

    fn render_drag_indicator(&self, f: &mut Frame, item: &DragDropItem, source: DropZone) {
        let area = f.area();
        let indicator_area = Rect {
            x: area.width / 2 - 20,
            y: area.height / 2,
            width: 40,
            height: 5,
        };

        let source_name = match source {
            DropZone::Source => "Source",
            DropZone::Target => "Target",
            DropZone::Trash => "Trash",
        };

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "🚀 Dragging: ",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &item.content,
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("From: ", Style::default().fg(Color::Gray)),
                Span::styled(source_name, Style::default().fg(Color::Magenta)),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            )
            .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(paragraph, indicator_area);
    }
}
