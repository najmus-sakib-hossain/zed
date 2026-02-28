//! Rendering logic for chat application

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use std::time::Duration;

use super::app_state::ChatApp;
use super::{
    app_data::Focus,
    app_splash,
    components::{MessageList, SecondaryBar},
    modals,
    modes::ChatMode,
};

impl ChatApp {
    pub fn render(&mut self, frame: &mut ratatui::Frame) {
        use ratatui::widgets::Widget;

        // If animations are active, render them instead of normal UI
        if self.show_train_animation {
            // Show train animation
            self.render_train_animation(frame);
            return;
        }

        if self.show_dx_splash {
            // Show DX splash screen
            app_splash::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                self.splash_font_index,
            );
            return;
        }

        if self.show_matrix_animation {
            // Show matrix animation in chat area only (not full screen)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(10),
                    Constraint::Length(5),
                    Constraint::Length(1),
                ])
                .split(frame.area());

            self.input_area = chunks[1];

            // Render matrix animation in the chat area
            self.render_matrix_animation_in_area(chunks[0], frame);

            // Still render the input bar and secondary bar
            let (add_area, plan_area, model_area, audio_area, local_area, send_area) =
                self.render_combined_input_bar(chunks[1], frame.buffer_mut());

            self.add_button_area = add_area;
            self.plan_button_area = plan_area;
            self.model_button_area = model_area;
            self.audio_button_area = audio_area;
            self.local_button_area = local_area;
            self.send_button_area = send_area;

            let secondary_bar_area = chunks[2];
            SecondaryBar::new(
                &self.theme,
                self.shortcut_index,
                self.changes_count,
                self.tasks_count,
                self.agents_count,
                &self.selected_memory_mode,
                self.audio_mode,
                self.audio_processing,
            )
            .render(secondary_bar_area, frame.buffer_mut());

            self.calculate_button_areas(secondary_bar_area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),
                Constraint::Length(5),
                Constraint::Length(1),
            ])
            .split(frame.area());

        self.input_area = chunks[1];

        if self.messages.is_empty() {
            app_splash::render(chunks[0], frame.buffer_mut(), &self.theme, self.splash_font_index);
        } else {
            MessageList::with_scroll(&self.messages, &self.theme, self.chat_scroll_offset)
                .render(chunks[0], frame.buffer_mut());
        }

        let (add_area, plan_area, model_area, audio_area, local_area, send_area) =
            self.render_combined_input_bar(chunks[1], frame.buffer_mut());

        self.add_button_area = add_area;
        self.plan_button_area = plan_area;
        self.model_button_area = model_area;
        self.audio_button_area = audio_area;
        self.local_button_area = local_area;
        self.send_button_area = send_area;

        let secondary_bar_area = chunks[2];
        SecondaryBar::new(
            &self.theme,
            self.shortcut_index,
            self.changes_count,
            self.tasks_count,
            self.agents_count,
            &self.selected_memory_mode,
            self.audio_mode,
            self.audio_processing,
        )
        .render(secondary_bar_area, frame.buffer_mut());

        self.calculate_button_areas(secondary_bar_area);
        self.render_modals(frame);

        // Render audio recording indicator in top right
        if self.audio_processing {
            self.render_audio_recording_indicator(frame.area(), frame.buffer_mut());
        }

        if let Some(ref shortcut) = self.last_shortcut_pressed
            && self.last_shortcut_time.elapsed() < Duration::from_secs(2)
        {
            self.render_shortcut_debug(frame.area(), frame.buffer_mut(), shortcut);
        }
    }

    fn calculate_button_areas(&mut self, secondary_bar_area: Rect) {
        let padded_secondary = Rect {
            x: secondary_bar_area.x + 1,
            y: secondary_bar_area.y,
            width: secondary_bar_area.width.saturating_sub(2),
            height: secondary_bar_area.height,
        };

        let changes_text_len = if self.changes_count > 0 {
            if self.changes_count >= 1_000_000 {
                format!("Changes:{}M", self.changes_count / 1_000_000).len()
            } else if self.changes_count >= 1_000 {
                format!("Changes:{}k", self.changes_count / 1_000).len()
            } else {
                format!("Changes:{}", self.changes_count).len()
            }
        } else {
            "Changes".len()
        } as u16;

        let tasks_text_len = if self.tasks_count > 0 {
            if self.tasks_count >= 1_000_000 {
                format!("Tasks:{}M", self.tasks_count / 1_000_000).len()
            } else if self.tasks_count >= 1_000 {
                format!("Tasks:{}k", self.tasks_count / 1_000).len()
            } else {
                format!("Tasks:{}", self.tasks_count).len()
            }
        } else {
            "Tasks".len()
        } as u16;

        let agents_text_len = if self.agents_count > 0 {
            if self.agents_count >= 1_000_000 {
                format!("Agents:{}M", self.agents_count / 1_000_000).len()
            } else if self.agents_count >= 1_000 {
                format!("Agents:{}k", self.agents_count / 1_000).len()
            } else {
                format!("Agents:{}", self.agents_count).len()
            }
        } else {
            "Agents".len()
        } as u16;

        let memory_text_len = self.selected_memory_mode.len() as u16;

        let secondary_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(changes_text_len),
                Constraint::Length(2),
                Constraint::Length(tasks_text_len),
                Constraint::Length(2),
                Constraint::Length(agents_text_len),
                Constraint::Min(10),
                Constraint::Length(memory_text_len),
                Constraint::Length(7),
                Constraint::Length(12),
            ])
            .split(padded_secondary);

        self.changes_button_area = secondary_chunks[0];
        self.tasks_button_area = secondary_chunks[2];
        self.agents_button_area = secondary_chunks[4];
        self.memory_button_area = secondary_chunks[6];
        self.tools_button_area = secondary_chunks[7];
    }

    fn render_modals(&mut self, frame: &mut ratatui::Frame) {
        if self.show_add_modal {
            modals::add::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.add_modal_search,
                &self.add_modal_list,
                self.add_modal_focus,
            );
        }

        if self.show_plan_modal {
            modals::plan::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.plan_modal_list,
                self.mode,
            );
        }

        if self.show_model_modal {
            let config = modals::model::ModelConfig {
                auto_mode: self.auto_mode,
                max_mode: self.max_mode,
                use_multiple_models: self.use_multiple_models,
                selected_model: &self.selected_model,
                selected_models: &self.selected_models,
                google_models: &self.google_models,
            };
            modals::model::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.model_modal_search,
                &self.model_modal_list,
                &config,
            );
        }

        if self.show_local_modal {
            modals::local::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.local_modal_list,
                &self.selected_local_mode,
            );
        }

        if self.show_changes_modal {
            modals::changes::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.changes_modal_list,
                &self.git_changes,
                self.changes_count,
            );
        }

        if self.show_tasks_modal {
            modals::drivens::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.tasks_modal_list,
                &self.tasks,
                self.tasks_count,
            );
        }

        if self.show_agents_modal {
            modals::workspaces::render(
                frame.area(),
                frame.buffer_mut(),
                &self.agents,
                &self.agents_modal_list,
                &self.theme,
                self.workspace_create_mode,
                &self.workspace_create_input,
            );
        }

        if self.show_memory_modal {
            modals::checkpoints::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.memory_modal_list,
                &self.selected_memory_mode,
            );
        }

        if self.show_tools_modal {
            modals::tools::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.tools_modal_list,
                &self.tools,
            );
        }

        if self.show_more_modal {
            modals::more::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.more_modal_list,
                &self.more_options,
            );
        }

        if self.show_google_api_modal {
            modals::google_api::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.google_api_input,
                self.cursor_visible,
            );
        }

        if self.show_elevenlabs_api_modal {
            modals::elevenlabs_api::render(
                frame.area(),
                frame.buffer_mut(),
                &self.theme,
                &self.elevenlabs_api_input,
                self.cursor_visible,
            );
        }
    }

    fn render_shortcut_debug(&self, area: Rect, buf: &mut Buffer, shortcut: &str) {
        // Truncate message if too long to prevent buffer overflow
        let max_len = area.width.saturating_sub(10).max(20) as usize;
        let display_text = if shortcut.len() > max_len {
            format!("{}...", &shortcut[..max_len.saturating_sub(3)])
        } else {
            shortcut.to_string()
        };

        let width = (display_text.len() as u16 + 4).min(area.width.saturating_sub(4));
        let debug_area = Rect {
            x: area.width.saturating_sub(width + 2),
            y: 1,
            width,
            height: 3,
        };

        // Ensure debug area is within bounds
        if debug_area.x >= area.width || debug_area.y >= area.height {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.accent))
            .style(Style::default().bg(self.theme.bg));

        let text = Line::from(Span::styled(
            display_text,
            Style::default().fg(self.theme.accent).add_modifier(Modifier::BOLD),
        ));

        Paragraph::new(text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center)
            .render(debug_area, buf);
    }

    fn render_audio_recording_indicator(&self, area: Rect, buf: &mut Buffer) {
        let indicator_text = "[REC] Recording...";
        let width = indicator_text.len() as u16 + 4;
        let indicator_area = Rect {
            x: area.width.saturating_sub(width + 2),
            y: 1,
            width,
            height: 3,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.accent))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .style(Style::default().bg(self.theme.bg));

        // Use shimmer effect for animated recording indicator
        let shimmer_color = self.shimmer.current_color();
        let text = Line::from(vec![
            Span::styled("[REC] ", Style::default().fg(shimmer_color)),
            Span::styled(
                "Recording...",
                Style::default().fg(shimmer_color).add_modifier(Modifier::BOLD),
            ),
        ]);

        Paragraph::new(text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center)
            .render(indicator_area, buf);
    }

    pub fn render_combined_input_bar(
        &self,
        area: Rect,
        buf: &mut Buffer,
    ) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .style(Style::default().bg(self.theme.bg));

        let inner = block.inner(area);
        block.render(area, buf);

        let padded = Rect {
            x: inner.x + 2,
            y: inner.y,
            width: inner.width.saturating_sub(4),
            height: inner.height,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(0),
            ])
            .split(padded);

        self.render_input_text(chunks[0], buf);
        self.render_input_cursor(chunks[0], buf);

        self.render_bottom_bar(chunks[1], buf)
    }

    fn render_input_text(&self, area: Rect, buf: &mut Buffer) {
        let placeholder = "A question or a prompt... (Enter to send, Shift+Enter for new line)";
        let text = if self.input.content.is_empty() {
            Text::from(Line::from(Span::styled(
                placeholder,
                Style::default().fg(self.theme.border),
            )))
        } else {
            Text::from(self.input.content.as_str())
        };

        if self.input.has_selection() {
            self.render_selection(area, buf);
        } else {
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(self.theme.bg).fg(self.theme.fg))
                .render(area, buf);
        }
    }

    fn render_selection(&self, area: Rect, buf: &mut Buffer) {
        let (sel_start, sel_end) = if let (Some(start), Some(end)) =
            (self.input.selection_start, self.input.selection_end)
        {
            if start < end {
                (start, end)
            } else {
                (end, start)
            }
        } else {
            (0, 0)
        };

        let mut x = area.x;
        let mut y = area.y;

        for (i, ch) in self.input.content.chars().enumerate() {
            if x >= area.right() {
                x = area.x;
                y += 1;
                if y >= area.bottom() {
                    break;
                }
            }

            let is_selected = i >= sel_start && i < sel_end;
            let style = if is_selected {
                Style::default().bg(self.theme.fg).fg(self.theme.bg)
            } else {
                Style::default().bg(self.theme.bg).fg(self.theme.fg)
            };

            let cell = &mut buf[(x, y)];
            cell.set_char(ch);
            cell.set_style(style);
            x += 1;
        }
    }

    fn render_input_cursor(&self, area: Rect, buf: &mut Buffer) {
        if self.focus == Focus::Input && self.cursor_visible {
            let cursor_x = area.x + (self.input.cursor_position as u16 % area.width);
            let cursor_y = area.y + (self.input.cursor_position as u16 / area.width);

            if cursor_x < area.right() && cursor_y < area.bottom() {
                let cell = &mut buf[(cursor_x, cursor_y)];
                let existing_char = cell.symbol().chars().next().unwrap_or(' ');
                if existing_char == ' ' || self.input.content.is_empty() {
                    cell.set_char('▎');
                    cell.set_style(Style::default().fg(self.theme.accent));
                } else {
                    cell.set_style(Style::default().bg(self.theme.accent).fg(self.theme.bg));
                }
            }
        }
    }

    fn render_bottom_bar(
        &self,
        area: Rect,
        buf: &mut Buffer,
    ) -> (Rect, Rect, Rect, Rect, Rect, Rect) {
        let mode_text = match self.mode {
            ChatMode::Agent => "Agent",
            ChatMode::Plan => "Plan",
            ChatMode::Ask => "Ask",
        };

        // Calculate dynamic widths
        let add_width = 3; // "Add"
        let mode_width = mode_text.len() as u16;
        let model_width = self.selected_model.len() as u16;
        let audio_width = if self.audio_processing {
            13 // "Processing..."
        } else {
            5 // "Audio"
        };
        let local_width = self.selected_local_mode.len() as u16;
        let send_width = if self.is_loading { 6 } else { 4 }; // "Cancel" or "Send"

        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(add_width),
                Constraint::Length(1), // Minimal gap
                Constraint::Length(mode_width),
                Constraint::Length(1), // Minimal gap
                Constraint::Length(model_width),
                Constraint::Min(5), // Flexible space
                Constraint::Length(audio_width),
                Constraint::Length(1), // Minimal gap
                Constraint::Length(local_width),
                Constraint::Length(1), // Minimal gap
                Constraint::Length(send_width),
            ])
            .split(area);

        Paragraph::new(Span::styled("Add", Style::default().fg(self.theme.fg)))
            .alignment(ratatui::layout::Alignment::Left)
            .render(bottom_chunks[0], buf);

        Paragraph::new(Span::styled(mode_text, Style::default().fg(self.theme.fg)))
            .alignment(ratatui::layout::Alignment::Left)
            .render(bottom_chunks[2], buf);

        Paragraph::new(Span::styled(&self.selected_model, Style::default().fg(self.theme.fg)))
            .alignment(ratatui::layout::Alignment::Left)
            .render(bottom_chunks[4], buf);

        let (audio_text, audio_style) = if self.audio_processing {
            (
                "Processing...",
                Style::default().fg(self.theme.accent).add_modifier(Modifier::BOLD),
            )
        } else if self.audio_mode {
            ("Audio", Style::default().fg(self.theme.accent).add_modifier(Modifier::BOLD))
        } else if !self.input.content.trim().is_empty() {
            // When there's text in input, use muted color
            ("Audio", Style::default().fg(self.theme.border))
        } else {
            ("Audio", Style::default().fg(self.theme.fg))
        };
        Paragraph::new(Span::styled(audio_text, audio_style))
            .alignment(ratatui::layout::Alignment::Left)
            .render(bottom_chunks[6], buf);

        Paragraph::new(Span::styled(&self.selected_local_mode, Style::default().fg(self.theme.fg)))
            .alignment(ratatui::layout::Alignment::Left)
            .render(bottom_chunks[8], buf);

        // Send button - dynamic text based on state
        let (send_text, send_style) = if self.is_loading && self.input.content.trim().is_empty() {
            // When AI is responding and no text, show "Cancel"
            ("Cancel", Style::default().fg(self.theme.accent).add_modifier(Modifier::BOLD))
        } else if !self.input.content.trim().is_empty() {
            // When there's text (even during AI response), show "Send" in white
            ("Send", Style::default().fg(self.theme.fg).add_modifier(Modifier::BOLD))
        } else {
            // When empty and not loading, show dimmed "Send"
            ("Send", Style::default().fg(self.theme.border))
        };

        Paragraph::new(Span::styled(send_text, send_style))
            .alignment(ratatui::layout::Alignment::Left)
            .render(bottom_chunks[10], buf);

        (
            bottom_chunks[0],
            bottom_chunks[2],
            bottom_chunks[4],
            bottom_chunks[6],
            bottom_chunks[8],
            bottom_chunks[10],
        )
    }

    fn render_matrix_animation(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        self.render_matrix_animation_in_area(area, frame);
    }

    fn render_matrix_animation_in_area(&self, area: Rect, frame: &mut ratatui::Frame) {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;

        let bg_color = Color::Rgb(13, 17, 23); // Dark theme background

        // Matrix-style green text with dark theme colors
        let mut lines = vec![];
        let chars = vec![
            '0', '1', 'ﾊ', 'ﾐ', 'ﾋ', 'ｰ', 'ｳ', 'ｼ', 'ﾅ', 'ﾓ', ':', '.', '"', '=', '*', '+', '-',
        ];

        // Create cascading effect
        let elapsed_ms =
            self.animation_start_time.map(|t| t.elapsed().as_millis() as u16).unwrap_or(0);

        for y in 0..area.height {
            let mut spans = vec![];
            for x in 0..(area.width / 2) {
                let idx = ((x + y + elapsed_ms / 100) % chars.len() as u16) as usize;
                // Dark theme green colors
                let brightness = if (x + y + elapsed_ms / 50) % 4 == 0 {
                    Color::Rgb(136, 192, 208) // Bright cyan (accent)
                } else if (x + y) % 3 == 0 {
                    Color::Rgb(88, 166, 255) // Blue accent
                } else if (x + y) % 2 == 0 {
                    Color::Rgb(64, 160, 43) // Green
                } else {
                    Color::Rgb(32, 80, 21) // Dark green
                };
                spans.push(Span::styled(chars[idx].to_string(), Style::default().fg(brightness)));
                spans.push(Span::raw(" "));
            }
            lines.push(Line::from(spans));
        }

        Paragraph::new(lines)
            .style(Style::default().bg(bg_color))
            .render(area, frame.buffer_mut());
    }

    fn render_train_animation(&self, frame: &mut ratatui::Frame) {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;

        let area = frame.area();
        let bg_color = Color::Rgb(13, 17, 23); // Dark theme background
        let elapsed_ms =
            self.animation_start_time.map(|t| t.elapsed().as_millis() as u16).unwrap_or(0);
        let x_pos = (area.width as i32 - (elapsed_ms / 20) as i32).max(-(80i32));

        // Train ASCII art
        let train = vec![
            "      ====        ________                ___________",
            "  _D _|  |_______/        \\__I_I_____===__|_________|",
            "   |(_)---  |   H\\________/ |   |        =|___ ___|",
            "   /     |  |   H  |  |     |   |         ||_| |_||",
            "  |      |  |   H  |__--------------------| [___] |",
            "  | ________|___H__/__|_____/[][]~\\_______|       |",
            "  |/ |   |-----------I_____I [][] []  D   |=======|",
            "__/ =| o |=-~~\\  /~~\\  /~~\\  /~~\\ ____Y___________|",
            " |/-=|___|=O=====O=====O=====O   |_____/~\\___/",
            "  \\_/      \\__/  \\__/  \\__/  \\__/      \\_/",
        ];

        let y_start = (area.height.saturating_sub(train.len() as u16)) / 2;
        let mut lines = vec![];

        for _ in 0..y_start {
            lines.push(Line::from(""));
        }

        // Dark theme train color (orange/yellow accent)
        let train_color = Color::Rgb(255, 184, 108);

        for line in train {
            if x_pos >= 0 {
                let padding = " ".repeat(x_pos as usize);
                lines.push(Line::from(Span::styled(
                    format!("{}{}", padding, line),
                    Style::default().fg(train_color),
                )));
            }
        }

        Paragraph::new(lines)
            .style(Style::default().bg(bg_color))
            .render(area, frame.buffer_mut());
    }
}
