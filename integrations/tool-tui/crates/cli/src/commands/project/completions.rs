//! Shell completions command

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};

use crate::cli::{Cli, CompletionShell, CompletionsArgs};

pub fn run_completions(args: CompletionsArgs) -> Result<()> {
    let shell = match args.shell {
        CompletionShell::Bash => Shell::Bash,
        CompletionShell::Zsh => Shell::Zsh,
        CompletionShell::Fish => Shell::Fish,
        CompletionShell::PowerShell => Shell::PowerShell,
        CompletionShell::Elvish => Shell::Elvish,
    };

    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "dx", &mut std::io::stdout());

    Ok(())
}
