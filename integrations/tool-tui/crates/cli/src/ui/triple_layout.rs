use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

// use super::components::sidebar::Sidebar;  // Deprecated - sidebar not yet implemented

/// Widget that displays content in left, center, and right sections within a single box
pub struct TripleLayout<'a> {
    left_content: Vec<Line<'a>>,
    center_content: Vec<Line<'a>>,
    right_content: Vec<Line<'a>>,
    /// Optional sidebar for the left section
    left_sidebar: Option<()>,
    title: Option<&'a str>,
    border_color: Color,
}

impl<'a> TripleLayout<'a> {
    pub fn new() -> Self {
        Self {
            left_content: Vec::new(),
            center_content: Vec::new(),
            right_content: Vec::new(),
            left_sidebar: None,
            title: None,
            border_color: Color::White,
        }
    }

    pub fn left(mut self, content: Vec<Line<'a>>) -> Self {
        self.left_content = content;
        self
    }

    pub fn center(mut self, content: Vec<Line<'a>>) -> Self {
        self.center_content = content;
        self
    }

    pub fn right(mut self, content: Vec<Line<'a>>) -> Self {
        self.right_content = content;
        self
    }

    /// Set a sidebar for the left section
    pub fn left_sidebar(mut self, sidebar: ()) -> Self {
        self.left_sidebar = Some(sidebar);
        self
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }
}

impl Widget for TripleLayout<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Create outer block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .title(self.title.unwrap_or(""));

        let inner = block.inner(area);
        block.render(area, buf);

        // Split into three columns
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(inner);

        // Render left content (sidebar or paragraph)
        if let Some(_sidebar) = self.left_sidebar {
            // Note: Sidebar rendering would need to be adapted to ratatui's Widget trait
            // For now, we render the left content as before
            // TODO: Implement proper sidebar rendering integration
            Paragraph::new(self.left_content)
                .alignment(Alignment::Left)
                .render(chunks[0], buf);
        } else {
            Paragraph::new(self.left_content)
                .alignment(Alignment::Left)
                .render(chunks[0], buf);
        }

        // Render center content (center-aligned)
        Paragraph::new(self.center_content)
            .alignment(Alignment::Center)
            .render(chunks[1], buf);

        // Render right content (right-aligned)
        Paragraph::new(self.right_content)
            .alignment(Alignment::Right)
            .render(chunks[2], buf);
    }
}

impl Default for TripleLayout<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Demo function showing triple layout usage with sidebar
pub fn demo_triple_layout_with_sidebar(_frame: &mut Frame) {
    // use super::components::sidebar::{Sidebar, SidebarItem, SidebarMode};  // Not yet implemented
    // Sidebar demo temporarily disabled

    // TODO: Re-enable when sidebar component is implemented
    // let sidebar = Sidebar::new()...
    // let widget = TripleLayout::new()...
    // frame.render_widget(widget, area);
}

/// Demo function showing triple layout usage
pub fn demo_triple_layout(frame: &mut Frame) {
    let area = frame.area();

    // Create multiple triple layouts
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Min(5),
        ])
        .split(area);

    // Example 1: Simple text
    let widget1 = TripleLayout::new()
        .title(" Example 1: Basic Text ")
        .left(vec![
            Line::from(Span::styled("Left", Style::default().fg(Color::Cyan))),
            Line::from("Content"),
        ])
        .center(vec![
            Line::from(Span::styled("Center", Style::default().fg(Color::Yellow))),
            Line::from("Content"),
        ])
        .right(vec![
            Line::from(Span::styled("Right", Style::default().fg(Color::Magenta))),
            Line::from("Content"),
        ])
        .border_color(Color::White);

    frame.render_widget(widget1, main_layout[0]);

    // Example 2: Status bar style
    let widget2 = TripleLayout::new()
        .title(" Example 2: Status Bar ")
        .left(vec![
            Line::from(vec![
                Span::styled("⚡ ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    "DX CLI",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled("v0.1.0", Style::default().fg(Color::Gray))),
        ])
        .center(vec![
            Line::from(Span::styled("🚀 Building...", Style::default().fg(Color::Green))),
            Line::from(Span::styled("45%", Style::default().fg(Color::Yellow))),
        ])
        .right(vec![
            Line::from(Span::styled("⏱️  2.3s", Style::default().fg(Color::Blue))),
            Line::from(Span::styled("✓ Ready", Style::default().fg(Color::Green))),
        ])
        .border_color(Color::Green);

    frame.render_widget(widget2, main_layout[1]);

    // Example 3: Dashboard style
    let widget3 = TripleLayout::new()
        .title(" Example 3: Dashboard ")
        .left(vec![
            Line::from(Span::styled(
                "CPU",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from("45%"),
            Line::from(""),
            Line::from(Span::styled(
                "Memory",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from("2.1 GB"),
        ])
        .center(vec![
            Line::from(Span::styled(
                "Network",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from("↓ 1.2 MB/s"),
            Line::from("↑ 0.8 MB/s"),
            Line::from(""),
            Line::from(Span::styled("Active", Style::default().fg(Color::Green))),
        ])
        .right(vec![
            Line::from(Span::styled(
                "Disk",
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            )),
            Line::from("156 GB"),
            Line::from(""),
            Line::from(Span::styled(
                "Uptime",
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            )),
            Line::from("2d 5h"),
        ])
        .border_color(Color::Cyan);

    frame.render_widget(widget3, main_layout[2]);

    // Example 4: Multi-line content
    let widget4 = TripleLayout::new()
        .title(" Example 4: Multi-line Content ")
        .left(vec![
            Line::from(Span::styled(
                "📁 Files",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  src/"),
            Line::from("  tests/"),
            Line::from("  docs/"),
        ])
        .center(vec![
            Line::from(Span::styled(
                "📊 Stats",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Lines: 1,234"),
            Line::from("Files: 45"),
            Line::from("Size: 2.1 MB"),
        ])
        .right(vec![
            Line::from(Span::styled(
                "✓ Status",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Build: OK"),
            Line::from("Tests: 45/45"),
            Line::from("Lint: Clean"),
        ])
        .border_color(Color::Blue);

    frame.render_widget(widget4, main_layout[3]);
}

/// Run the triple layout demo
pub fn run_demo() -> anyhow::Result<()> {
    use crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };
    use ratatui::{Terminal, backend::CrosstermBackend};
    use std::io;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(demo_triple_layout)?;

        if let Event::Key(key) = event::read()?
            && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
        {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
