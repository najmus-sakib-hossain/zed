//! Shell integration command

use anyhow::Result;

use crate::cli::ShellArgs;
use crate::ui::theme::Theme;

pub async fn run_shell(_args: ShellArgs, theme: &Theme) -> Result<()> {
    theme.info("shell command not yet implemented");
    Ok(())
}
