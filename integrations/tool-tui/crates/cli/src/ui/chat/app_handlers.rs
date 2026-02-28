//! Event handlers for chat app

use super::modes::ChatMode;

pub fn handle_add_modal_selection(idx: usize) -> String {
    match idx {
        0 => "Opening file picker...".to_string(),
        1 => "Adding instructions...".to_string(),
        2 => "Taking screenshot...".to_string(),
        3 => "Opening source control...".to_string(),
        4 => "Loading problems...".to_string(),
        5 => "Loading symbols...".to_string(),
        6 => "Opening tools...".to_string(),
        8 => "Adding README.md...".to_string(),
        9 => "Opening recent files...".to_string(),
        _ => String::new(),
    }
}

pub fn handle_plan_modal_selection(idx: usize) -> (Option<ChatMode>, String) {
    match idx {
        0 => (Some(ChatMode::Agent), "Switched to Agent mode".to_string()),
        1 => (Some(ChatMode::Plan), "Switched to Plan mode".to_string()),
        2 => (None, "Debug mode coming soon...".to_string()),
        3 => (Some(ChatMode::Ask), "Switched to Ask mode".to_string()),
        _ => (None, String::new()),
    }
}

pub fn handle_local_modal_selection(idx: usize) -> (String, String) {
    match idx {
        0 => ("Local".to_string(), "Switched to Local execution".to_string()),
        1 => ("Remote".to_string(), "Switched to Remote execution".to_string()),
        2 => ("Timely".to_string(), "Switched to Timely execution".to_string()),
        _ => (String::new(), String::new()),
    }
}

pub struct ModelSelectionResult {
    pub should_close: bool,
    pub selected_model: Option<String>,
    pub selected_models: Option<Vec<String>>,
    pub auto_mode: Option<bool>,
    pub max_mode: Option<bool>,
    pub use_multiple_models: Option<bool>,
    pub message: String,
}

pub fn handle_model_modal_selection(
    idx: usize,
    models: &[(&str, &str)],
    current_auto_mode: bool,
    current_max_mode: bool,
    current_use_multiple: bool,
    current_selected_models: &[String],
) -> ModelSelectionResult {
    // idx 0 = Configure Google API Key (handled in app_events.rs)
    // idx 1 = Sign in with Google (handled in app_events.rs)
    // idx 2-4 = Auto, MAX Mode, Use Multiple Models
    // idx 5+ = Google models then regular models (handled in app_events.rs)

    if idx >= 2 && idx < 5 {
        match idx {
            2 => {
                let new_auto = !current_auto_mode;
                ModelSelectionResult {
                    should_close: true,
                    selected_model: if new_auto {
                        Some("Auto".to_string())
                    } else {
                        None
                    },
                    selected_models: if new_auto { Some(Vec::new()) } else { None },
                    auto_mode: Some(new_auto),
                    max_mode: if new_auto { Some(false) } else { None },
                    use_multiple_models: if new_auto { Some(false) } else { None },
                    message: "Auto mode toggled".to_string(),
                }
            }
            3 => {
                let new_max = !current_max_mode;
                ModelSelectionResult {
                    should_close: false,
                    selected_model: None,
                    selected_models: None,
                    auto_mode: if new_max { Some(false) } else { None },
                    max_mode: Some(new_max),
                    use_multiple_models: None,
                    message: "MAX Mode toggled".to_string(),
                }
            }
            4 => {
                let new_multiple = !current_use_multiple;
                let mut result = ModelSelectionResult {
                    should_close: false,
                    selected_model: None,
                    selected_models: None,
                    auto_mode: if new_multiple { Some(false) } else { None },
                    max_mode: None,
                    use_multiple_models: Some(new_multiple),
                    message: "Use Multiple Models toggled".to_string(),
                };

                if !new_multiple && !current_selected_models.is_empty() {
                    result.selected_model = Some(current_selected_models[0].clone());
                    result.selected_models = Some(Vec::new());
                }

                result
            }
            _ => ModelSelectionResult {
                should_close: false,
                selected_model: None,
                selected_models: None,
                auto_mode: None,
                max_mode: None,
                use_multiple_models: None,
                message: String::new(),
            },
        }
    } else {
        let model_index = idx - 5;
        if let Some((name, _)) = models.get(model_index) {
            let selected = name.to_string();

            if current_use_multiple {
                let mut new_models = current_selected_models.to_vec();
                if let Some(pos) = new_models.iter().position(|m| m == &selected) {
                    new_models.remove(pos);
                } else {
                    new_models.push(selected.clone());
                }

                let display_model = if new_models.is_empty() {
                    "No models selected".to_string()
                } else if new_models.len() == 1 {
                    new_models[0].clone()
                } else {
                    format!("{} models", new_models.len())
                };

                ModelSelectionResult {
                    should_close: false,
                    selected_model: Some(display_model),
                    selected_models: Some(new_models),
                    auto_mode: None,
                    max_mode: None,
                    use_multiple_models: None,
                    message: format!("Toggled: {}", selected),
                }
            } else {
                ModelSelectionResult {
                    should_close: true,
                    selected_model: Some(selected.clone()),
                    selected_models: Some(Vec::new()),
                    auto_mode: Some(false),
                    max_mode: None,
                    use_multiple_models: None,
                    message: format!("Selected model: {}", selected),
                }
            }
        } else {
            ModelSelectionResult {
                should_close: false,
                selected_model: None,
                selected_models: None,
                auto_mode: None,
                max_mode: None,
                use_multiple_models: None,
                message: String::new(),
            }
        }
    }
}

pub fn handle_memory_modal_selection(idx: usize) -> (String, String) {
    match idx {
        0 => ("Permanent".to_string(), "Memory: Permanent - All context retained".to_string()),
        1 => ("Moderate".to_string(), "Memory: Moderate - Session context".to_string()),
        2 => ("Checkpoints".to_string(), "Memory: Checkpoints - Minimal context".to_string()),
        _ => ("Checkpoints".to_string(), "Memory: Checkpoints".to_string()),
    }
}
