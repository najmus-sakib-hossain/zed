//! Event handling for chat application

use crossterm::event::{self, KeyCode, KeyModifiers};
use std::time::Instant;

use super::app_state::ChatApp;
use super::{
    app_data::Focus,
    app_handlers, app_helpers,
    input::InputAction,
    modal_list::ModalListAction,
    modals::{self, add::AddModalFocus},
    text_input::TextInputAction,
};

impl ChatApp {
    pub fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::MouseEventKind;

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if self.chat_scroll_offset > 0 {
                    self.chat_scroll_offset = self.chat_scroll_offset.saturating_sub(3);
                }
            }
            MouseEventKind::ScrollDown => {
                // Calculate max scroll based on total content height
                let total_height =
                    self.messages.iter().map(|msg| msg.content.lines().count() + 4).sum::<usize>();

                // Only scroll if there's more content below
                let max_scroll = total_height.saturating_sub(20); // Assume ~20 lines visible
                if self.chat_scroll_offset < max_scroll {
                    self.chat_scroll_offset =
                        self.chat_scroll_offset.saturating_add(3).min(max_scroll);
                }
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                let x = mouse.column;
                let y = mouse.row;

                if self.is_in_rect(x, y, self.input_area) {
                    self.focus = Focus::Input;
                    self.cursor_visible = true;
                    self.last_cursor_blink = Instant::now();
                    app_helpers::play_sound("click");
                    return;
                }

                if self.is_in_rect(x, y, self.add_button_area)
                    || self.is_in_rect(x, y, self.plan_button_area)
                    || self.is_in_rect(x, y, self.model_button_area)
                {
                    app_helpers::play_sound("click");
                } else if self.is_in_rect(x, y, self.audio_button_area) {
                    self.audio_mode = !self.audio_mode;
                    app_helpers::play_sound("click");

                    if self.audio_mode {
                        self.last_shortcut_pressed = Some("🎤 Recording...".to_string());
                        self.last_shortcut_time = Instant::now();
                        self.start_audio_recording();
                    } else {
                        self.last_shortcut_pressed = Some("⏳ Transcribing...".to_string());
                        self.last_shortcut_time = Instant::now();
                        self.stop_audio_recording();
                    }
                } else if self.is_in_rect(x, y, self.local_button_area) {
                    self.show_local_modal = true;
                    self.local_modal_list.reset();
                    app_helpers::play_sound("click");
                } else if self.is_in_rect(x, y, self.send_button_area) {
                    if !self.input.content.trim().is_empty() {
                        let msg = self.input.content.clone();
                        self.input.content.clear();
                        self.input.cursor_position = 0;
                        self.input.clear_selection();
                        self.prompt_history.push(msg.clone());
                        self.history_index = None;
                        self.send_message(msg);
                        app_helpers::play_sound("send");
                    }
                } else if self.is_in_rect(x, y, self.changes_button_area) {
                    let (git_changes, changes_count) = app_helpers::fetch_git_changes();
                    self.git_changes = git_changes;
                    self.changes_count = changes_count;
                    self.changes_modal_list.set_items_count(self.git_changes.len());
                    self.changes_modal_list.reset();
                    self.show_changes_modal = true;
                    app_helpers::play_sound("click");
                } else if self.is_in_rect(x, y, self.tasks_button_area) {
                    let (tasks, tasks_count) = app_helpers::fetch_tasks();
                    self.tasks = tasks;
                    self.tasks_count = tasks_count;
                    self.tasks_modal_list.set_items_count(self.tasks.len());
                    self.tasks_modal_list.reset();
                    self.show_tasks_modal = true;
                    app_helpers::play_sound("click");
                } else if self.is_in_rect(x, y, self.agents_button_area) {
                    let (agents, agents_count) = app_helpers::fetch_agents();
                    self.agents = agents;
                    self.agents_count = agents_count;
                    self.agents_modal_list.set_items_count(self.agents.len());
                    self.agents_modal_list.reset();
                    self.show_agents_modal = true;
                    app_helpers::play_sound("click");
                } else if self.is_in_rect(x, y, self.memory_button_area) {
                    self.memory_modal_list.set_items_count(4); // 4 checkpoints
                    self.memory_modal_list.reset();
                    self.show_memory_modal = true;
                    app_helpers::play_sound("click");
                } else if self.is_in_rect(x, y, self.tools_button_area) {
                    self.show_tools_modal = true;
                    self.tools_modal_list.set_items_count(self.tools.len());
                    self.tools_modal_list.reset();
                    app_helpers::play_sound("click");
                }
            }
            _ => {}
        }
    }

    pub fn handle_key(&mut self, key: event::KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return;
            }
            (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                self.show_bottom_menu = !self.show_bottom_menu;
                self.last_shortcut_pressed = Some("Ctrl+M: Toggle Menu".to_string());
                self.last_shortcut_time = Instant::now();
                return;
            }
            _ => {}
        }

        self.handle_key_with_focus(key);
    }

    fn handle_key_with_focus(&mut self, key: event::KeyEvent) {
        // Handle Enter key for audio toggle when input is empty (but not when modals are open)
        if key.code == KeyCode::Enter
            && self.focus == Focus::Input
            && self.input.content.trim().is_empty()
            && !self.show_add_modal
            && !self.show_plan_modal
            && !self.show_model_modal
            && !self.show_local_modal
            && !self.show_changes_modal
            && !self.show_tasks_modal
            && !self.show_agents_modal
            && !self.show_memory_modal
            && !self.show_tools_modal
            && !self.show_more_modal
            && !self.show_google_api_modal
        {
            use crossterm::event::KeyEventKind;
            if key.kind == KeyEventKind::Press {
                // Toggle audio mode
                self.audio_mode = !self.audio_mode;
                app_helpers::play_sound("click");

                if self.audio_mode {
                    // Start audio recording
                    self.last_shortcut_pressed = Some("🎤 Recording...".to_string());
                    self.start_audio_recording();
                } else {
                    // Stop recording and transcribe
                    self.last_shortcut_pressed = Some("⏳ Transcribing...".to_string());
                    self.stop_audio_recording();
                }

                self.last_shortcut_time = Instant::now();
                return;
            }
        }

        match (key.code, key.modifiers, self.focus) {
            (KeyCode::Tab, _, _) if self.show_add_modal => {
                self.add_modal_focus = match self.add_modal_focus {
                    AddModalFocus::Search => AddModalFocus::Options,
                    AddModalFocus::Options => AddModalFocus::Search,
                };
                app_helpers::play_sound("click");
            }
            (KeyCode::Tab, _, _) => {
                // Tab cycles through chat modes instead of focus
                self.mode = self.mode.next();
                self.last_shortcut_pressed = Some(format!(
                    "Tab: Mode {}",
                    match self.mode {
                        super::modes::ChatMode::Agent => "Agent",
                        super::modes::ChatMode::Plan => "Plan",
                        super::modes::ChatMode::Ask => "Ask",
                    }
                ));
                self.last_shortcut_time = Instant::now();
                app_helpers::play_sound("click");
            }
            (KeyCode::Left, _, Focus::ModeSelector) => {
                self.mode = self.mode.prev();
            }
            (KeyCode::Right, _, Focus::ModeSelector) => {
                self.mode = self.mode.next();
            }
            (_, _, _) if self.show_add_modal && self.add_modal_focus == AddModalFocus::Search => {
                self.handle_add_modal_search(key);
            }
            (_, _, _) if self.show_add_modal && self.add_modal_focus == AddModalFocus::Options => {
                self.handle_add_modal_options(key);
            }
            (_, _, _) if self.show_plan_modal => {
                self.handle_plan_modal(key);
            }
            (_, _, _) if self.show_model_modal => {
                self.handle_model_modal(key);
            }
            (_, _, _) if self.show_local_modal => {
                self.handle_local_modal(key);
            }
            (_, _, _) if self.show_changes_modal => {
                self.handle_changes_modal(key);
            }
            (_, _, _) if self.show_tasks_modal => {
                self.handle_tasks_modal(key);
            }
            (_, _, _) if self.show_agents_modal => {
                self.handle_agents_modal(key);
            }
            (_, _, _) if self.show_memory_modal => {
                self.handle_memory_modal(key);
            }
            (_, _, _) if self.show_tools_modal => {
                self.handle_tools_modal(key);
            }
            (_, _, _) if self.show_more_modal => {
                self.handle_more_modal(key);
            }
            (_, _, _) if self.show_google_api_modal => {
                self.handle_google_api_modal(key);
            }
            (_, _, _) if self.show_elevenlabs_api_modal => {
                self.handle_elevenlabs_api_modal(key);
            }
            (KeyCode::Char('1'), KeyModifiers::NONE, _) => {
                self.handle_shortcut_1();
            }
            (KeyCode::Char('2'), KeyModifiers::NONE, _) => {
                self.handle_shortcut_2();
            }
            (KeyCode::Char('3'), KeyModifiers::NONE, _) => {
                self.handle_shortcut_3();
            }
            (KeyCode::Char('4'), KeyModifiers::NONE, _) => {
                self.handle_shortcut_4();
            }
            (KeyCode::Char('5'), KeyModifiers::NONE, _) => {
                self.handle_shortcut_5();
            }
            (KeyCode::Char('6'), KeyModifiers::NONE, _) => {
                self.handle_shortcut_6();
            }
            (KeyCode::Char('7'), KeyModifiers::NONE, _) => {
                self.handle_shortcut_7();
            }
            (KeyCode::Char('8'), KeyModifiers::NONE, _) => {
                self.handle_shortcut_8();
            }
            (KeyCode::Char('9'), KeyModifiers::NONE, _) => {
                self.handle_shortcut_9();
            }
            (KeyCode::Char('0'), KeyModifiers::NONE, _) => {
                self.handle_shortcut_0();
            }
            (_, _, Focus::Input) => {
                self.handle_input_key(key);
            }
            (KeyCode::Char(c), KeyModifiers::NONE, Focus::ModeSelector) if !c.is_ascii_digit() => {
                // Switch to input mode when typing non-digit characters
                self.focus = Focus::Input;
                self.cursor_visible = true;
                self.last_cursor_blink = Instant::now();
                // Handle the character in input mode
                self.handle_input_key(key);
            }
            _ => {}
        }
    }

    fn handle_add_modal_search(&mut self, key: event::KeyEvent) {
        // Handle ESC to close modal
        if key.code == KeyCode::Esc {
            self.show_add_modal = false;
            self.add_modal_search.clear();
            app_helpers::play_sound("click");
            return;
        }

        let action = self.add_modal_search.handle_key(key.code, key.modifiers);
        match action {
            TextInputAction::NumberKey(c) => {
                // If the number key matches the modal trigger (1), close the modal
                if c == '1' {
                    self.show_add_modal = false;
                    self.add_modal_search.clear();
                    app_helpers::play_sound("click");
                    return;
                } else {
                    // Otherwise, insert the character and continue
                    self.add_modal_search.insert_char(c);
                }
            }
            TextInputAction::RequestPaste => {
                if let Ok(clipboard_content) = cli_clipboard::get_contents() {
                    self.add_modal_search.insert_text(&clipboard_content);
                }
            }
            TextInputAction::Copy(text) => {
                let _ = cli_clipboard::set_contents(text);
            }
            TextInputAction::Cut(text) => {
                let _ = cli_clipboard::set_contents(text);
            }
            _ => {}
        }
        if key.code == KeyCode::Down {
            self.add_modal_focus = AddModalFocus::Options;
        }
    }

    fn handle_add_modal_options(&mut self, key: event::KeyEvent) {
        let action = self.add_modal_list.handle_key(key.code, key.modifiers, 10);
        match action {
            ModalListAction::ItemSelected(idx) => {
                let message = app_handlers::handle_add_modal_selection(idx);
                self.last_shortcut_pressed = Some(message);
                self.last_shortcut_time = Instant::now();
                self.show_add_modal = false;
                app_helpers::play_sound("send");
            }
            ModalListAction::Close => {
                self.show_add_modal = false;
                self.add_modal_search.clear();
                app_helpers::play_sound("click");
            }
            ModalListAction::SelectionChanged => {
                app_helpers::play_sound("click");
            }
            _ => {}
        }
    }

    fn handle_plan_modal(&mut self, key: event::KeyEvent) {
        let action = self.plan_modal_list.handle_key(key.code, key.modifiers, 4);
        match action {
            ModalListAction::ItemSelected(idx) => {
                let (new_mode, message) = app_handlers::handle_plan_modal_selection(idx);
                if let Some(mode) = new_mode {
                    self.mode = mode;
                }
                self.last_shortcut_pressed = Some(message);
                self.last_shortcut_time = Instant::now();
                self.show_plan_modal = false;
                app_helpers::play_sound("send");
            }
            ModalListAction::Close => {
                self.show_plan_modal = false;
                app_helpers::play_sound("click");
            }
            ModalListAction::SelectionChanged => {
                app_helpers::play_sound("click");
            }
            _ => {}
        }
    }

    fn handle_model_modal(&mut self, key: event::KeyEvent) {
        let action = self.model_modal_search.handle_key(key.code, key.modifiers);
        match action {
            TextInputAction::NumberKey(c) => {
                // If the number key matches the modal trigger (2), close the modal
                if c == '2' {
                    self.show_model_modal = false;
                    self.model_modal_search.clear();
                    app_helpers::play_sound("click");
                    return;
                } else {
                    // Otherwise, insert the character and continue
                    self.model_modal_search.insert_char(c);
                    let models =
                        modals::model::get_filtered_models(&self.model_modal_search.content);
                    self.model_modal_list
                        .set_items_count(1 + 3 + self.google_models.len() + models.len() + 1); // Sign in + 3 config + models + Configure at bottom
                    self.model_modal_list.reset();
                }
            }
            TextInputAction::RequestPaste => {
                if let Ok(clipboard_content) = cli_clipboard::get_contents() {
                    self.model_modal_search.insert_text(&clipboard_content);
                    let models =
                        modals::model::get_filtered_models(&self.model_modal_search.content);
                    self.model_modal_list
                        .set_items_count(1 + 3 + self.google_models.len() + models.len() + 1); // Sign in + 3 config + models + Configure at bottom
                    self.model_modal_list.reset();
                }
            }
            TextInputAction::Copy(text) => {
                let _ = cli_clipboard::set_contents(text);
            }
            TextInputAction::Cut(text) => {
                let _ = cli_clipboard::set_contents(text);
            }
            TextInputAction::Changed => {
                let models = modals::model::get_filtered_models(&self.model_modal_search.content);
                self.model_modal_list
                    .set_items_count(1 + 3 + self.google_models.len() + models.len() + 1); // Sign in + 3 config + models + Configure at bottom
                self.model_modal_list.reset();
            }
            _ => {}
        }

        let list_action = self.model_modal_list.handle_key(key.code, key.modifiers, 15);
        match list_action {
            ModalListAction::ItemSelected(idx) => {
                let models = modals::model::get_filtered_models(&self.model_modal_search.content);

                // Check if "Configure Google API Key" was selected (index 0)
                if idx == 0 {
                    self.show_model_modal = false;

                    // Pre-populate with existing API key if available
                    if let Some(llm) = &self.llm {
                        if let Some(api_key) = llm.get_google_api_key() {
                            self.google_api_input.content = api_key;
                            self.google_api_input.cursor_position =
                                self.google_api_input.content.len();
                        }
                    }

                    self.show_google_api_modal = true;
                    app_helpers::play_sound("click");
                    return;
                }

                // Check if "Sign in with Google" was selected (index 1)
                if idx == 1 {
                    // Trigger OAuth flow for Antigravity models
                    self.show_model_modal = false;

                    self.last_shortcut_pressed =
                        Some("Opening browser for Google sign-in...".to_string());
                    self.last_shortcut_time = Instant::now();
                    app_helpers::play_sound("click");

                    // Start OAuth flow in background
                    let tx = self.llm_tx.clone();
                    let llm = self.llm.clone();
                    tokio::spawn(async move {
                        use crate::ui::chat::google_oauth::GoogleOAuthConfig;

                        // Send status update
                        let _ = tx.send("__GOOGLE_ERROR__:Loading OAuth config...".to_string());

                        match GoogleOAuthConfig::load_from_file() {
                            Ok(oauth_config) => {
                                let _ =
                                    tx.send("__GOOGLE_ERROR__:Starting OAuth flow...".to_string());

                                match oauth_config.get_access_token().await {
                                    Ok(access_token) => {
                                        let _ = tx.send(
                                            "__GOOGLE_ERROR__:OAuth successful! Saving token..."
                                                .to_string(),
                                        );

                                        // Save OAuth token to config
                                        if let Some(llm) = llm {
                                            if let Err(e) = llm
                                                .set_antigravity_oauth_token(access_token.clone())
                                                .await
                                            {
                                                let _ = tx.send(format!(
                                                    "__GOOGLE_ERROR__:Failed to save token: {}",
                                                    e
                                                ));
                                                return;
                                            }
                                        }

                                        let _ = tx.send(
                                            "__GOOGLE_ERROR__:Fetching Antigravity models..."
                                                .to_string(),
                                        );

                                        // Fetch Antigravity models
                                        match crate::ui::chat::google_oauth::fetch_antigravity_models(&access_token).await {
                                            Ok(models) => {
                                                let _ = tx.send(format!(
                                                    "__GOOGLE_ERROR__:Found {} models",
                                                    models.len()
                                                ));

                                                // Convert to GoogleModel format with [Antigravity] tag
                                                let google_models: Vec<crate::ui::chat::app_state::GoogleModel> = models
                                                    .into_iter()
                                                    .map(|name| crate::ui::chat::app_state::GoogleModel {
                                                        display_name: format!("{} [Antigravity]", name),
                                                        api_name: name,
                                                    })
                                                    .collect();

                                                let models_json = serde_json::to_string(&google_models).unwrap_or_default();
                                                let _ = tx.send(format!("__GOOGLE_MODELS_WITH_MODAL__:{}", models_json));
                                            }
                                            Err(e) => {
                                                let _ = tx.send(format!("__GOOGLE_ERROR__:Failed to fetch Antigravity models: {}", e));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx
                                            .send(format!("__GOOGLE_ERROR__:OAuth failed: {}", e));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(format!(
                                    "__GOOGLE_ERROR__:Failed to load OAuth config: {}",
                                    e
                                ));
                            }
                        }
                    });

                    return;
                }

                // Check if a Google model was selected (indices 5 to 5+google_models.len()-1)
                let google_model_start_idx = 5;
                let google_model_end_idx = 5 + self.google_models.len();

                if idx >= google_model_start_idx && idx < google_model_end_idx {
                    let google_model_idx = idx - google_model_start_idx;
                    if let Some(google_model) = self.google_models.get(google_model_idx) {
                        // Store the display name for UI
                        self.selected_model = google_model.display_name.clone();

                        // Check if this is an Antigravity model (has [Antigravity] tag)
                        let is_antigravity = google_model.display_name.contains("[Antigravity]");

                        if let Some(llm) = &self.llm {
                            let llm_clone = llm.clone();
                            let api_name = google_model.api_name.clone();

                            if is_antigravity {
                                // Switch to Antigravity backend
                                tokio::spawn(async move {
                                    let _ = llm_clone.set_backend("antigravity".to_string()).await;
                                    let _ = llm_clone.set_antigravity_model(api_name).await;
                                });
                            } else {
                                // Switch to Google backend
                                tokio::spawn(async move {
                                    let _ = llm_clone.set_backend("google".to_string()).await;
                                    let _ = llm_clone.set_google_model(api_name).await;
                                });
                            }
                        }

                        self.last_shortcut_pressed = Some(format!(
                            "Selected {} model: {}",
                            if is_antigravity {
                                "Antigravity"
                            } else {
                                "Google"
                            },
                            google_model.display_name
                        ));
                        self.last_shortcut_time = Instant::now();
                        self.show_model_modal = false;
                        app_helpers::play_sound("send");
                        return;
                    }
                }

                let result = app_handlers::handle_model_modal_selection(
                    idx,
                    &models,
                    self.auto_mode,
                    self.max_mode,
                    self.use_multiple_models,
                    &self.selected_models,
                );

                if let Some(model) = result.selected_model {
                    self.selected_model = model;
                }
                if let Some(models) = result.selected_models {
                    self.selected_models = models;
                }
                if let Some(auto) = result.auto_mode {
                    self.auto_mode = auto;
                }
                if let Some(max) = result.max_mode {
                    self.max_mode = max;
                }
                if let Some(multiple) = result.use_multiple_models {
                    self.use_multiple_models = multiple;
                }

                self.last_shortcut_pressed = Some(result.message);
                self.last_shortcut_time = Instant::now();

                if result.should_close {
                    self.show_model_modal = false;
                }
                app_helpers::play_sound("send");
            }
            ModalListAction::Close => {
                self.show_model_modal = false;
                self.model_modal_search.clear();
                app_helpers::play_sound("click");
            }
            ModalListAction::SelectionChanged => {
                app_helpers::play_sound("click");
            }
            _ => {}
        }
    }

    fn handle_local_modal(&mut self, key: event::KeyEvent) {
        // Handle number key 3 to close modal
        if key.code == KeyCode::Char('3') && key.modifiers == KeyModifiers::NONE {
            self.show_local_modal = false;
            app_helpers::play_sound("click");
            return;
        }

        let action = self.local_modal_list.handle_key(key.code, key.modifiers, 3);
        match action {
            ModalListAction::ItemSelected(idx) => {
                let (mode, message) = app_handlers::handle_local_modal_selection(idx);
                self.selected_local_mode = mode;
                self.last_shortcut_pressed = Some(message);
                self.last_shortcut_time = Instant::now();
                self.show_local_modal = false;
                app_helpers::play_sound("send");
            }
            ModalListAction::Close => {
                self.show_local_modal = false;
                app_helpers::play_sound("click");
            }
            ModalListAction::SelectionChanged => {
                app_helpers::play_sound("click");
            }
            _ => {}
        }
    }

    fn handle_changes_modal(&mut self, key: event::KeyEvent) {
        // Handle number key 4 to close modal
        if key.code == KeyCode::Char('4') && key.modifiers == KeyModifiers::NONE {
            self.show_changes_modal = false;
            app_helpers::play_sound("click");
            return;
        }

        let action = self.changes_modal_list.handle_key(key.code, key.modifiers, 10);
        match action {
            ModalListAction::Close => {
                self.show_changes_modal = false;
                app_helpers::play_sound("click");
            }
            ModalListAction::SelectionChanged => {
                app_helpers::play_sound("click");
            }
            _ => {}
        }
    }

    fn handle_tasks_modal(&mut self, key: event::KeyEvent) {
        // Handle number key 5 to close modal
        if key.code == KeyCode::Char('5') && key.modifiers == KeyModifiers::NONE {
            self.show_tasks_modal = false;
            app_helpers::play_sound("click");
            return;
        }

        let action = self.tasks_modal_list.handle_key(key.code, key.modifiers, 10);
        match action {
            ModalListAction::Close => {
                self.show_tasks_modal = false;
                app_helpers::play_sound("click");
            }
            ModalListAction::SelectionChanged => {
                app_helpers::play_sound("click");
            }
            _ => {}
        }
    }

    fn handle_agents_modal(&mut self, key: event::KeyEvent) {
        // Handle number key 6 to close modal
        if key.code == KeyCode::Char('6') && key.modifiers == KeyModifiers::NONE {
            self.show_agents_modal = false;
            app_helpers::play_sound("click");
            return;
        }

        // Handle 'n' key to create new workspace
        if key.code == KeyCode::Char('n') && key.modifiers == KeyModifiers::NONE {
            self.workspace_create_mode = true;
            self.workspace_create_input.clear();
            app_helpers::play_sound("click");
            return;
        }

        // If in create mode, handle text input
        if self.workspace_create_mode {
            if key.code == KeyCode::Esc {
                self.workspace_create_mode = false;
                self.workspace_create_input.clear();
                app_helpers::play_sound("click");
                return;
            }

            if key.code == KeyCode::Enter {
                let workspace_name = self.workspace_create_input.content.clone();
                if !workspace_name.is_empty() {
                    // Trigger workspace transition animation
                    self.switching_workspace = true;
                    self.animation_start_time = Some(Instant::now());
                    self.current_workspace = Some(workspace_name.clone());

                    self.last_shortcut_pressed =
                        Some(format!("Creating workspace: {}", workspace_name));
                    self.last_shortcut_time = Instant::now();

                    // Clear messages to show fresh workspace
                    self.messages.clear();

                    self.workspace_create_mode = false;
                    self.workspace_create_input.clear();
                    self.show_agents_modal = false;
                    app_helpers::play_sound("send");
                }
                return;
            }

            let action = self.workspace_create_input.handle_key(key.code, key.modifiers);
            match action {
                TextInputAction::NumberKey(c) => {
                    self.workspace_create_input.insert_char(c);
                }
                TextInputAction::Changed => {}
                _ => {}
            }
            return;
        }

        let action = self.agents_modal_list.handle_key(key.code, key.modifiers, 10);
        match action {
            ModalListAction::ItemSelected(idx) => {
                // Switch to selected workspace with animation
                if idx < self.agents.len() {
                    let workspace_name = self.agents[idx].name.clone();
                    self.switching_workspace = true;
                    self.animation_start_time = Some(Instant::now());
                    self.current_workspace = Some(workspace_name.clone());

                    self.last_shortcut_pressed = Some(format!("Switching to: {}", workspace_name));
                    self.last_shortcut_time = Instant::now();

                    self.show_agents_modal = false;
                    app_helpers::play_sound("send");
                }
            }
            ModalListAction::Close => {
                self.show_agents_modal = false;
                app_helpers::play_sound("click");
            }
            ModalListAction::SelectionChanged => {
                app_helpers::play_sound("click");
            }
            _ => {}
        }
    }

    fn handle_memory_modal(&mut self, key: event::KeyEvent) {
        // Handle number key 7 to close modal
        if key.code == KeyCode::Char('7') && key.modifiers == KeyModifiers::NONE {
            self.show_memory_modal = false;
            app_helpers::play_sound("click");
            return;
        }

        let action = self.memory_modal_list.handle_key(key.code, key.modifiers, 3);
        match action {
            ModalListAction::ItemSelected(idx) => {
                let (mode, message) = app_handlers::handle_memory_modal_selection(idx);
                self.selected_memory_mode = mode;
                self.last_shortcut_pressed = Some(message);
                self.last_shortcut_time = Instant::now();
                self.show_memory_modal = false;
                app_helpers::play_sound("send");
            }
            ModalListAction::Close => {
                self.show_memory_modal = false;
                app_helpers::play_sound("click");
            }
            ModalListAction::SelectionChanged => {
                app_helpers::play_sound("click");
            }
            _ => {}
        }
    }

    fn handle_tools_modal(&mut self, key: event::KeyEvent) {
        use crossterm::event::KeyCode;

        // Handle number key 8 to close modal
        if key.code == KeyCode::Char('8') && key.modifiers == KeyModifiers::NONE {
            self.show_tools_modal = false;
            app_helpers::play_sound("click");
            return;
        }

        if key.code == KeyCode::Char(' ') {
            // Toggle tool at current selection
            let idx = self.tools_modal_list.selected;
            if idx < self.tools.len() {
                let tool_name = self.tools[idx].name.clone();

                // If it's ElevenLabs, open the API key modal instead of toggling
                if tool_name == "elevenlabs" {
                    self.show_elevenlabs_api_modal = true;
                    self.elevenlabs_api_input.content.clear();
                    self.elevenlabs_api_input.cursor_position = 0;
                    app_helpers::play_sound("click");
                } else {
                    self.tools[idx].enabled = !self.tools[idx].enabled;
                    app_helpers::play_sound("click");
                }
            }
            return;
        }

        let action = self.tools_modal_list.handle_key(key.code, key.modifiers, 10);
        match action {
            ModalListAction::ItemSelected(idx) => {
                // Check if ElevenLabs was selected
                if idx < self.tools.len() && self.tools[idx].name == "elevenlabs" {
                    self.show_elevenlabs_api_modal = true;
                    self.elevenlabs_api_input.content.clear();
                    self.elevenlabs_api_input.cursor_position = 0;
                    app_helpers::play_sound("click");
                } else {
                    // Enter key saves and closes
                    let enabled_count = self.tools.iter().filter(|t| t.enabled).count();
                    self.last_shortcut_pressed = Some(format!("Tools: {} enabled", enabled_count));
                    self.last_shortcut_time = Instant::now();
                    self.show_tools_modal = false;
                    app_helpers::play_sound("send");
                }
            }
            ModalListAction::Close => {
                self.show_tools_modal = false;
                app_helpers::play_sound("click");
            }
            ModalListAction::SelectionChanged => {
                app_helpers::play_sound("click");
            }
            _ => {}
        }
    }

    fn handle_more_modal(&mut self, key: event::KeyEvent) {
        // Handle number key 9 to close modal
        if key.code == KeyCode::Char('9') && key.modifiers == KeyModifiers::NONE {
            self.show_more_modal = false;
            app_helpers::play_sound("click");
            return;
        }

        let action = self.more_modal_list.handle_key(key.code, key.modifiers, 6);
        match action {
            ModalListAction::ItemSelected(idx) => {
                if idx < self.more_options.len() {
                    let option = &self.more_options[idx];
                    self.last_shortcut_pressed = Some(format!("More: {}", option.name));
                    self.last_shortcut_time = Instant::now();
                    // Handle the selected option
                    match option.name.as_str() {
                        "Settings" => {
                            // TODO: Open settings
                        }
                        "History" => {
                            // TODO: Open history
                        }
                        "Export" => {
                            // TODO: Export conversation
                        }
                        "Clear" => {
                            // TODO: Clear conversation
                        }
                        "Help" => {
                            // TODO: Show help
                        }
                        "About" => {
                            // TODO: Show about
                        }
                        _ => {}
                    }
                }
                self.show_more_modal = false;
                app_helpers::play_sound("send");
            }
            ModalListAction::Close => {
                self.show_more_modal = false;
                app_helpers::play_sound("click");
            }
            ModalListAction::SelectionChanged => {
                app_helpers::play_sound("click");
            }
            _ => {}
        }
    }

    fn handle_shortcut_1(&mut self) {
        // 1: Add - Toggle add modal
        if self.show_add_modal {
            self.show_add_modal = false;
            self.add_modal_search.clear();
            app_helpers::play_sound("click");
        } else {
            self.show_add_modal = true;
            self.add_modal_list.reset();
            self.add_modal_search.clear();
            self.add_modal_focus = AddModalFocus::Search;
            app_helpers::play_sound("click");
        }
        self.last_shortcut_pressed = Some("1: Add".to_string());
        self.last_shortcut_time = Instant::now();
    }

    fn handle_shortcut_2(&mut self) {
        // 2: LLM Models - Toggle model modal
        if self.show_model_modal {
            self.show_model_modal = false;
            app_helpers::play_sound("click");
        } else {
            self.show_model_modal = true;
            let models = modals::model::get_filtered_models(&self.model_modal_search.content);
            self.model_modal_list
                .set_items_count(1 + 1 + 3 + self.google_models.len() + models.len()); // Configure API Key + Sign in + 3 config + Google models + regular models
            self.model_modal_list.reset();
            self.model_modal_search.clear();
            app_helpers::play_sound("click");
        }
        self.last_shortcut_pressed = Some("2: LLM Models".to_string());
        self.last_shortcut_time = Instant::now();
    }

    fn handle_shortcut_3(&mut self) {
        // 3: Local - Toggle local modal
        if self.show_local_modal {
            self.show_local_modal = false;
            app_helpers::play_sound("click");
        } else {
            self.show_local_modal = true;
            self.local_modal_list.reset();
            app_helpers::play_sound("click");
        }
        self.last_shortcut_pressed = Some("3: Local".to_string());
        self.last_shortcut_time = Instant::now();
    }

    fn handle_shortcut_4(&mut self) {
        // 4: Changes - Toggle changes modal
        if self.show_changes_modal {
            self.show_changes_modal = false;
            app_helpers::play_sound("click");
        } else {
            let (git_changes, changes_count) = app_helpers::fetch_git_changes();
            self.git_changes = git_changes;
            self.changes_count = changes_count;
            self.changes_modal_list.set_items_count(self.git_changes.len());
            self.changes_modal_list.reset();
            self.show_changes_modal = true;
            app_helpers::play_sound("click");
        }
        self.last_shortcut_pressed = Some("4: Changes".to_string());
        self.last_shortcut_time = Instant::now();
    }

    fn handle_shortcut_5(&mut self) {
        // 5: Drivens - Toggle tasks modal
        if self.show_tasks_modal {
            self.show_tasks_modal = false;
            app_helpers::play_sound("click");
        } else {
            let (tasks, tasks_count) = app_helpers::fetch_tasks();
            self.tasks = tasks;
            self.tasks_count = tasks_count;
            self.tasks_modal_list.set_items_count(self.tasks.len());
            self.tasks_modal_list.reset();
            self.show_tasks_modal = true;
            app_helpers::play_sound("click");
        }
        self.last_shortcut_pressed = Some("5: Drivens".to_string());
        self.last_shortcut_time = Instant::now();
    }

    fn handle_shortcut_6(&mut self) {
        // 6: Workspaces - Toggle agents modal
        if self.show_agents_modal {
            self.show_agents_modal = false;
            app_helpers::play_sound("click");
        } else {
            let (agents, agents_count) = app_helpers::fetch_agents();
            self.agents = agents;
            self.agents_count = agents_count;
            self.agents_modal_list.set_items_count(self.agents.len());
            self.agents_modal_list.reset();
            self.show_agents_modal = true;
            app_helpers::play_sound("click");
        }
        self.last_shortcut_pressed = Some("6: Workspaces".to_string());
        self.last_shortcut_time = Instant::now();
    }

    fn handle_shortcut_7(&mut self) {
        // 7: Checkpoints - Toggle memory modal
        if self.show_memory_modal {
            self.show_memory_modal = false;
            app_helpers::play_sound("click");
        } else {
            self.memory_modal_list.set_items_count(4); // 4 checkpoints
            self.memory_modal_list.reset();
            self.show_memory_modal = true;
            app_helpers::play_sound("click");
        }
        self.last_shortcut_pressed = Some("7: Checkpoints".to_string());
        self.last_shortcut_time = Instant::now();
    }

    fn handle_shortcut_8(&mut self) {
        // 8: Tools - Toggle tools modal
        if self.show_tools_modal {
            self.show_tools_modal = false;
            app_helpers::play_sound("click");
        } else {
            self.show_tools_modal = true;
            self.tools_modal_list.set_items_count(self.tools.len());
            self.tools_modal_list.reset();
            app_helpers::play_sound("click");
        }
        self.last_shortcut_pressed = Some("8: Tools".to_string());
        self.last_shortcut_time = Instant::now();
    }

    fn handle_shortcut_9(&mut self) {
        // 9: More - Toggle more modal
        if self.show_more_modal {
            self.show_more_modal = false;
            app_helpers::play_sound("click");
        } else {
            self.show_more_modal = true;
            self.more_modal_list.set_items_count(self.more_options.len());
            self.more_modal_list.reset();
            app_helpers::play_sound("click");
        }
        self.last_shortcut_pressed = Some("9: More".to_string());
        self.last_shortcut_time = Instant::now();
    }

    fn handle_shortcut_0(&mut self) {
        // 0: Audio - Toggle audio mode
        self.audio_mode = !self.audio_mode;
        app_helpers::play_sound("click");

        if self.audio_mode {
            // Start audio recording
            self.last_shortcut_pressed = Some("0: 🎤 Recording...".to_string());
            self.start_audio_recording();
        } else {
            // Stop recording and transcribe
            self.last_shortcut_pressed = Some("0: ⏳ Transcribing...".to_string());
            self.stop_audio_recording();
        }

        self.last_shortcut_time = Instant::now();
    }

    fn handle_input_key(&mut self, key: event::KeyEvent) {
        let action = self.input.handle_key(key);
        match action {
            InputAction::Submit(msg) => {
                self.prompt_history.push(msg.clone());
                self.history_index = None;
                self.send_message(msg);
                app_helpers::play_sound("send");
            }
            InputAction::Exit => {
                self.should_quit = true;
            }
            InputAction::PreviousHistory => {
                if !self.prompt_history.is_empty() {
                    let new_index = match self.history_index {
                        None => Some(self.prompt_history.len() - 1),
                        Some(idx) if idx > 0 => Some(idx - 1),
                        Some(idx) => Some(idx),
                    };
                    if let Some(idx) = new_index {
                        self.input.content = self.prompt_history[idx].clone();
                        self.input.cursor_position = self.input.content.len();
                        self.input.clear_selection();
                        self.history_index = new_index;
                    }
                }
            }
            InputAction::NextHistory => {
                if let Some(idx) = self.history_index {
                    if idx + 1 < self.prompt_history.len() {
                        let new_idx = idx + 1;
                        self.input.content = self.prompt_history[new_idx].clone();
                        self.input.cursor_position = self.input.content.len();
                        self.input.clear_selection();
                        self.history_index = Some(new_idx);
                    } else {
                        self.input.content.clear();
                        self.input.cursor_position = 0;
                        self.input.clear_selection();
                        self.history_index = None;
                    }
                }
            }
            InputAction::None => {}
        }
    }

    fn handle_google_api_modal(&mut self, key: event::KeyEvent) {
        if key.code == KeyCode::Esc {
            self.show_google_api_modal = false;
            self.google_api_input.clear();
            app_helpers::play_sound("click");
            return;
        }

        if key.code == KeyCode::Enter {
            let api_key = self.google_api_input.content.clone();
            if !api_key.is_empty() {
                // Save API key and fetch available models
                if let Some(llm) = &self.llm {
                    let llm_clone = llm.clone();
                    let tx = self.llm_tx.clone();
                    tokio::spawn(async move {
                        // Save API key
                        if llm_clone.set_google_api_key(api_key.clone()).await.is_err() {
                            return;
                        }

                        // Fetch available Google models
                        match llm_clone.fetch_google_models().await {
                            Ok(models) => {
                                // Send models as a special message
                                let models_json =
                                    serde_json::to_string(&models).unwrap_or_default();
                                let _ = tx
                                    .send(format!("__GOOGLE_MODELS_WITH_MODAL__:{}", models_json));
                            }
                            Err(e) => {
                                let _ = tx.send(format!("__GOOGLE_ERROR__:{}", e));
                            }
                        }
                    });
                }
                self.show_google_api_modal = false;
                self.google_api_input.clear();
                app_helpers::play_sound("send");
            }
            return;
        }

        let action = self.google_api_input.handle_key(key.code, key.modifiers);
        match action {
            TextInputAction::NumberKey(c) => {
                // For Google API modal, just insert the number (no specific trigger to close)
                self.google_api_input.insert_char(c);
            }
            TextInputAction::RequestPaste => {
                if let Ok(clipboard_content) = cli_clipboard::get_contents() {
                    self.google_api_input.insert_text(&clipboard_content);
                }
            }
            TextInputAction::Copy(text) => {
                let _ = cli_clipboard::set_contents(text);
            }
            TextInputAction::Cut(text) => {
                let _ = cli_clipboard::set_contents(text);
            }
            _ => {}
        }
    }

    fn handle_elevenlabs_api_modal(&mut self, key: event::KeyEvent) {
        if key.code == KeyCode::Esc {
            self.show_elevenlabs_api_modal = false;
            self.elevenlabs_api_input.clear();
            app_helpers::play_sound("click");
            return;
        }

        if key.code == KeyCode::Enter {
            let api_key = self.elevenlabs_api_input.content.clone();
            if !api_key.is_empty() {
                // Save API key to config
                if let Some(llm) = &self.llm {
                    let llm_clone = llm.clone();
                    let api_key_clone = api_key.clone();
                    tokio::spawn(async move {
                        // Save ElevenLabs API key
                        if let Err(e) = llm_clone.set_elevenlabs_api_key(api_key_clone).await {
                            eprintln!("Failed to save ElevenLabs API key: {}", e);
                        }
                    });
                }

                // Enable the ElevenLabs tool
                if let Some(tool) = self.tools.iter_mut().find(|t| t.name == "elevenlabs") {
                    tool.enabled = true;
                }

                self.last_shortcut_pressed = Some("ElevenLabs API key saved".to_string());
                self.last_shortcut_time = Instant::now();
                self.show_elevenlabs_api_modal = false;
                self.elevenlabs_api_input.clear();
                app_helpers::play_sound("send");
            }
            return;
        }

        let action = self.elevenlabs_api_input.handle_key(key.code, key.modifiers);
        match action {
            TextInputAction::NumberKey(c) => {
                self.elevenlabs_api_input.insert_char(c);
            }
            TextInputAction::Changed => {}
            _ => {}
        }
    }
}
