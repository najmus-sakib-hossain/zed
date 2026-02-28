use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

pub const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone)]
pub enum InteractionEvent {
    Click(Position),
    DoubleClick(Position),
    DragStart(Position),
    DragMove(Position),
    DragEnd(Position),
    KeyPress(KeyCode, KeyModifiers),
    Scroll(i8),
}

#[derive(Debug, Clone)]
pub struct DragState {
    pub start: Position,
    pub current: Position,
    pub is_dragging: bool,
}

pub struct TerminalInteractionHandler {
    last_click: Option<(Position, Instant)>,
    drag_state: Option<DragState>,
    selected_items: Vec<usize>,
    hover_index: Option<usize>,
}

impl TerminalInteractionHandler {
    pub fn new() -> Self {
        Self {
            last_click: None,
            drag_state: None,
            selected_items: Vec::new(),
            hover_index: None,
        }
    }

    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<InteractionEvent> {
        let pos = Position {
            x: event.column,
            y: event.row,
        };

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check for double-click
                if let Some((last_pos, last_time)) = self.last_click
                    && last_time.elapsed() < DOUBLE_CLICK_THRESHOLD
                    && last_pos.x == pos.x
                    && last_pos.y == pos.y
                {
                    self.last_click = None;
                    return Some(InteractionEvent::DoubleClick(pos));
                }

                self.last_click = Some((pos, Instant::now()));
                self.drag_state = Some(DragState {
                    start: pos,
                    current: pos,
                    is_dragging: false,
                });
                Some(InteractionEvent::Click(pos))
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(ref mut drag) = self.drag_state {
                    drag.current = pos;
                    if !drag.is_dragging {
                        drag.is_dragging = true;
                        Some(InteractionEvent::DragStart(drag.start))
                    } else {
                        Some(InteractionEvent::DragMove(pos))
                    }
                } else {
                    None
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(drag) = self.drag_state.take() {
                    if drag.is_dragging {
                        Some(InteractionEvent::DragEnd(pos))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            MouseEventKind::ScrollDown => Some(InteractionEvent::Scroll(1)),
            MouseEventKind::ScrollUp => Some(InteractionEvent::Scroll(-1)),
            _ => None,
        }
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> Option<InteractionEvent> {
        Some(InteractionEvent::KeyPress(event.code, event.modifiers))
    }

    pub fn toggle_selection(&mut self, index: usize) {
        if let Some(pos) = self.selected_items.iter().position(|&i| i == index) {
            self.selected_items.remove(pos);
        } else {
            self.selected_items.push(index);
        }
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_items.contains(&index)
    }

    pub fn clear_selection(&mut self) {
        self.selected_items.clear();
    }

    pub fn get_drag_state(&self) -> Option<&DragState> {
        self.drag_state.as_ref()
    }
}

pub struct InteractiveList {
    items: Vec<String>,
    handler: TerminalInteractionHandler,
    scroll_offset: usize,
}

impl InteractiveList {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            handler: TerminalInteractionHandler::new(),
            scroll_offset: 0,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_loop(&mut terminal);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        terminal.show_cursor()?;

        result
    }

    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.code == KeyCode::Char('q')
                            || (key.code == KeyCode::Char('c')
                                && key.modifiers.contains(KeyModifiers::CONTROL))
                        {
                            break;
                        }

                        if let Some(interaction) = self.handler.handle_key_event(key) {
                            self.handle_interaction(interaction);
                        }
                    }
                    Event::Mouse(mouse) => {
                        if let Some(interaction) = self.handler.handle_mouse_event(mouse) {
                            self.handle_interaction(interaction);
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn handle_interaction(&mut self, event: InteractionEvent) {
        match event {
            InteractionEvent::Click(pos) => {
                if let Some(index) = self.position_to_index(pos) {
                    self.handler.toggle_selection(index);
                }
            }
            InteractionEvent::DoubleClick(pos) => {
                if let Some(index) = self.position_to_index(pos) {
                    // Double-click action (e.g., open/execute)
                    self.handler.clear_selection();
                    self.handler.toggle_selection(index);
                }
            }
            InteractionEvent::Scroll(delta) => {
                if delta > 0 {
                    self.scroll_offset =
                        (self.scroll_offset + 1).min(self.items.len().saturating_sub(1));
                } else {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                }
            }
            InteractionEvent::KeyPress(code, mods) => {
                match code {
                    KeyCode::Up => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        self.scroll_offset =
                            (self.scroll_offset + 1).min(self.items.len().saturating_sub(1));
                    }
                    KeyCode::Char('a') if mods.contains(KeyModifiers::CONTROL) => {
                        // Select all
                        self.handler.selected_items = (0..self.items.len()).collect();
                    }
                    KeyCode::Esc => {
                        self.handler.clear_selection();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn position_to_index(&self, pos: Position) -> Option<usize> {
        // Adjust for borders and scroll
        if pos.y >= 2 {
            let relative_y = (pos.y - 2) as usize;
            let index = self.scroll_offset + relative_y;
            if index < self.items.len() {
                return Some(index);
            }
        }
        None
    }

    fn render(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(f.area());

        // Render list
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .map(|(i, item)| {
                let style = if self.handler.is_selected(i) {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let prefix = if self.handler.is_selected(i) {
                    "✓ "
                } else {
                    "  "
                };

                ListItem::new(format!("{}{}", prefix, item)).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Interactive List (Click/Double-click/Drag/Keys)"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC));

        f.render_widget(list, chunks[0]);

        // Render status bar
        let status = self.render_status();
        f.render_widget(status, chunks[1]);

        // Render drag indicator
        if let Some(drag) = self.handler.get_drag_state()
            && drag.is_dragging
        {
            self.render_drag_indicator(f, drag);
        }
    }

    fn render_status(&self) -> Paragraph<'_> {
        let selected_count = self.handler.selected_items.len();
        let status_text = vec![Line::from(vec![
            Span::styled("Selected: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{}", selected_count),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | "),
            Span::styled("Ctrl+A", Style::default().fg(Color::Green)),
            Span::raw(": Select All | "),
            Span::styled("Esc", Style::default().fg(Color::Green)),
            Span::raw(": Clear | "),
            Span::styled("Q", Style::default().fg(Color::Red)),
            Span::raw(": Quit"),
        ])];

        Paragraph::new(status_text).block(Block::default().borders(Borders::ALL).title("Controls"))
    }

    fn render_drag_indicator(&self, f: &mut Frame, drag: &DragState) {
        let width = (drag.current.x as i16 - drag.start.x as i16).unsigned_abs();
        let height = (drag.current.y as i16 - drag.start.y as i16).unsigned_abs();

        let x = drag.start.x.min(drag.current.x);
        let y = drag.start.y.min(drag.current.y);

        let area = Rect {
            x,
            y,
            width,
            height,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        f.render_widget(block, area);
    }
}
