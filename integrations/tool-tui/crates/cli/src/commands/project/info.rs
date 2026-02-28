//! Info command

use anyhow::Result;

use crate::cli::InfoArgs;
use crate::ui::theme::Theme;

pub async fn run_info(_args: InfoArgs, theme: &Theme) -> Result<()> {
    theme.info("info command not yet implemented");
    Ok(())
}
