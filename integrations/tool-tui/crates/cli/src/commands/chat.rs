use anyhow::Result;
use clap::Args;

use crate::ui::chat::ChatApp;

#[derive(Debug, Args)]
pub struct ChatCommand {
    /// Start in a specific mode
    #[arg(short, long, value_enum)]
    mode: Option<ChatModeArg>,

    /// Theme variant
    #[arg(short, long, value_enum)]
    theme: Option<ThemeArg>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ChatModeArg {
    Agent,
    Plan,
    Ask,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ThemeArg {
    Dark,
    Light,
}

impl ChatCommand {
    pub fn execute(&self) -> Result<()> {
        let mut app = ChatApp::new();

        // Initialize LLM
        app.initialize_llm();

        if let Some(mode) = self.mode {
            app.mode = match mode {
                ChatModeArg::Agent => crate::ui::chat::modes::ChatMode::Agent,
                ChatModeArg::Plan => crate::ui::chat::modes::ChatMode::Plan,
                ChatModeArg::Ask => crate::ui::chat::modes::ChatMode::Ask,
            };
        }

        if let Some(theme) = self.theme {
            app.theme = crate::ui::chat::theme::ChatTheme::new(match theme {
                ThemeArg::Dark => crate::ui::chat::theme::ThemeVariant::Dark,
                ThemeArg::Light => crate::ui::chat::theme::ThemeVariant::Light,
            });
            app.shimmer =
                crate::ui::chat::effects::ShimmerEffect::new(app.theme.shimmer_colors.clone());
        }

        app.run()
    }
}
