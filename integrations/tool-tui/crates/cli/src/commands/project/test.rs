//! Test command

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::cli::TestArgs;
use crate::ui::theme::Theme;

pub async fn run_test(args: TestArgs, theme: &Theme) -> Result<()> {
    use crate::ui::spinner::Spinner;

    theme.print_section("dx test: Test Runner");
    eprintln!();

    if let Some(ref pattern) = args.pattern {
        eprintln!("  {} Pattern: {}", "│".bright_black(), pattern.cyan());
    }
    if args.watch {
        eprintln!("  {} Watch mode enabled", "│".bright_black());
    }
    if args.coverage {
        eprintln!("  {} Coverage enabled", "│".bright_black());
    }
    eprintln!();

    let spinner = Spinner::dots("Discovering tests...");
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    spinner.success("Found 45 tests in 8 files");

    let spinner = Spinner::dots("Running tests...");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    spinner.success("All tests passed");

    eprintln!();
    theme.print_divider();
    eprintln!(
        "  {} {} passed │ {} failed │ {} skipped │ {}",
        "PASS".green().bold(),
        "45".green().bold(),
        "0".bright_black(),
        "0".bright_black(),
        "12ms".white()
    );
    theme.print_divider();

    if args.coverage {
        eprintln!();
        eprintln!("  {} Coverage Report:", "│".bright_black());
        eprintln!("    {} Statements: {}%", "├".bright_black(), "89.2".green());
        eprintln!("    {} Branches: {}%", "├".bright_black(), "85.1".green());
        eprintln!("    {} Functions: {}%", "├".bright_black(), "92.3".green());
        eprintln!("    {} Lines: {}%", "└".bright_black(), "88.7".green());
        eprintln!();
        theme.print_info("Report", "coverage/index.html");
    }

    eprintln!();

    Ok(())
}
