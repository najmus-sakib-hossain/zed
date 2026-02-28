//! Clean command

use anyhow::Result;

use crate::cli::CleanArgs;
use crate::ui::theme::Theme;

pub async fn run_clean(_args: CleanArgs, theme: &Theme) -> Result<()> {
    theme.info("clean command not yet implemented");
    Ok(())
}
