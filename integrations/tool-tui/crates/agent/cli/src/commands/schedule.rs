//! Schedule commands

use crate::ScheduleCommands;
use colored::Colorize;

pub async fn run(action: ScheduleCommands) -> anyhow::Result<()> {
    match action {
        ScheduleCommands::Add {
            name,
            cron,
            skill,
            context,
        } => {
            println!(
                "{} Adding scheduled task: {}",
                "📅".bright_cyan(),
                name.bright_yellow()
            );
            println!("  Schedule: {}", cron);
            println!("  Skill: {}", skill);
            if let Some(ctx) = context {
                println!("  Context: {}", ctx);
            }

            // In production: add to scheduler

            println!("{} Task scheduled!", "✅".bright_green());
        }

        ScheduleCommands::Remove { name } => {
            println!(
                "{} Removing scheduled task: {}",
                "🗑️".bright_yellow(),
                name.bright_yellow()
            );

            // In production: remove from scheduler

            println!("{} Task removed.", "✅".bright_green());
        }

        ScheduleCommands::List => {
            // Delegate to list command
            crate::commands::list::run(crate::ListCommands::Tasks).await?;
        }
    }

    Ok(())
}
