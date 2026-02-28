//! Shell completions generation command

use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

/// Generate shell completions for the specified shell
pub fn run(shell: Shell) -> dx_py_core::Result<()> {
    let mut cmd = crate::Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
    Ok(())
}
