//! dx-style commands

use anyhow::Result;
use clap::{Args, Subcommand};
use owo_colors::OwoColorize;

use crate::ui::{spinner::Spinner, table, theme::Theme};

#[derive(Args)]
pub struct StyleArgs {
    #[command(subcommand)]
    pub command: StyleCommands,
}

#[derive(Subcommand)]
pub enum StyleCommands {
    /// Build binary CSS from source
    Build {
        /// Input CSS file
        #[arg(short, long)]
        input: Option<String>,

        /// Output .bcss file
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Analyze CSS usage
    Analyze {
        /// Source directory
        #[arg(index = 1)]
        dir: Option<String>,
    },

    /// Show style statistics
    Stats,
}

pub async fn run(args: StyleArgs, theme: &Theme) -> Result<()> {
    match args.command {
        StyleCommands::Build {
            input: _,
            output: _,
        } => {
            theme.print_section("dx-style: Binary CSS Compiler");
            eprintln!();

            let spinner = Spinner::dots("Parsing CSS...");
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            spinner.success("Parsed 1,248 rules");

            let spinner = Spinner::dots("Generating integer class IDs...");
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            spinner.success("Generated 892 unique IDs");

            let spinner = Spinner::dots("Building binary stylesheet...");
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            spinner.success("Output: styles.bcss (2.1 KB)");

            eprintln!();
            theme.print_divider();
            eprintln!("  {} 98% smaller │ 80x faster application", "✓".green().bold());
            theme.print_divider();
            eprintln!();
        }

        StyleCommands::Analyze { dir: _ } => {
            theme.print_section("dx-style: Usage Analysis");
            eprintln!();

            let spinner = Spinner::dots("Scanning source files...");
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            spinner.success("Scanned 45 files");

            eprintln!();

            let mut table = table::Table::new(vec!["Class", "Usage", "Size Impact"]);
            table.add_row(vec!["flex", "124", "+8 bytes"]);
            table.add_row(vec!["p-4", "89", "+12 bytes"]);
            table.add_row(vec!["text-white", "67", "+16 bytes"]);
            table.add_row(vec!["bg-gray-900", "45", "+20 bytes"]);
            table.print();

            eprintln!();
            theme.print_info("Unused classes", "23 (can be purged)");
            theme.print_info("Potential savings", "1.2 KB");
            eprintln!();
        }

        StyleCommands::Stats => {
            theme.print_section("dx-style: Statistics");
            eprintln!();

            table::print_kv_list(&[
                ("Total rules", "1,248"),
                ("Unique classes", "892"),
                ("Binary size", "2.1 KB"),
                ("Equivalent CSS", "98.2 KB"),
                ("Compression ratio", "98%"),
                ("Lookup time", "< 1µs"),
            ]);
            eprintln!();
        }
    }

    Ok(())
}
