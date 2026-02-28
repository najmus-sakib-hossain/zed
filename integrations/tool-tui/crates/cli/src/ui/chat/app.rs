//! Main chat application coordination

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io,
    time::{Duration, Instant},
};

// Re-export ChatApp from app_state
pub use super::app_state::ChatApp;

use super::{app_data::Focus, app_helpers};

impl ChatApp {
    pub fn run(&mut self) -> Result<()> {
        let (git_changes, changes_count) = app_helpers::fetch_git_changes();
        self.git_changes = git_changes;
        self.changes_count = changes_count;

        let (tasks, tasks_count) = app_helpers::fetch_tasks();
        self.tasks = tasks;
        self.tasks_count = tasks_count;

        let (agents, agents_count) = app_helpers::fetch_agents();
        self.agents = agents;
        self.agents_count = agents_count;

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            crossterm::cursor::Hide,
            crossterm::event::EnableFocusChange,
            crossterm::event::EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_loop(&mut terminal);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            crossterm::cursor::Show,
            crossterm::event::DisableFocusChange,
            crossterm::event::DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if self.should_quit {
                break;
            }

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == crossterm::event::KeyEventKind::Press {
                            self.terminal_focused = true;
                            self.last_interaction = Instant::now();
                            self.cursor_visible = true;
                            self.last_cursor_blink = Instant::now();
                            self.handle_key(key);
                        }
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse);
                    }
                    Event::FocusGained => {
                        self.terminal_focused = true;
                        self.cursor_visible = true;
                        self.last_cursor_blink = Instant::now();
                    }
                    Event::FocusLost => {
                        self.terminal_focused = false;
                        self.cursor_visible = false;
                    }
                    _ => {}
                }
            }

            // Check for LLM responses
            self.check_llm_response();

            // Check for audio transcriptions
            self.check_audio_transcription();

            self.update_animations();
        }
        Ok(())
    }

    fn update_animations(&mut self) {
        // Check for audio transcription
        self.check_audio_transcription();

        // Check animation states
        self.check_matrix_animation();
        self.check_workspace_animation();

        if self.is_loading {
            self.typing_indicator.update();
        }

        if self.last_shortcut_update.elapsed() >= Duration::from_secs(3) {
            self.shortcut_index = (self.shortcut_index + 1) % 6;
            self.last_shortcut_update = Instant::now();
        }

        if self.messages.is_empty() && self.last_font_change.elapsed() >= Duration::from_secs(3) {
            self.splash_font_index = (self.splash_font_index + 1) % 382;
            self.last_font_change = Instant::now();
        }

        if self.focus == Focus::Input && self.terminal_focused {
            if self.last_cursor_blink.elapsed() >= Duration::from_millis(500) {
                self.cursor_visible = !self.cursor_visible;
                self.last_cursor_blink = Instant::now();
            }
        } else {
            self.cursor_visible = false;
        }

        if self.last_render.elapsed() >= Duration::from_millis(50) {
            self.last_render = Instant::now();
        }
    }
}
