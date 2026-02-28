//! Theme styling implementation

use super::icons;
use super::types::ColorMode;
use crate::theme::Theme as LegacyTheme;
use console::{Style, Term};
use owo_colors::OwoColorize;

/// The DX CLI theme with Vercel-inspired styling
#[allow(dead_code)]
pub struct Theme {
    term: Term,
    pub primary: Style,
    pub secondary: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub dim: Style,
    pub bold: Style,
    pub highlight: Style,
    pub colors_enabled: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}

impl Theme {
    pub fn new() -> Self {
        Self::with_color_mode(ColorMode::Auto)
    }

    pub fn with_color_mode(mode: ColorMode) -> Self {
        let colors_enabled = match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => atty::is(atty::Stream::Stderr),
        };

        Self {
            term: Term::stderr(),
            primary: Style::new().cyan(),
            secondary: Style::new().magenta(),
            success: Style::new().green(),
            warning: Style::new().yellow(),
            error: Style::new().red(),
            dim: Style::new().dim(),
            bold: Style::new().bold(),
            highlight: Style::new().cyan().bold(),
            colors_enabled,
        }
    }

    pub fn print_logo(&self) {
        let version = env!("CARGO_PKG_VERSION");
        eprintln!();
        if self.colors_enabled {
            eprintln!(
                "  {}  {} {}",
                "◆".cyan().bold(),
                "DX".white().bold(),
                format!("v{version}").bright_black()
            );
        } else {
            eprintln!("  ◆  DX v{}", version);
        }
        eprintln!();
    }

    pub fn print_logo_inline(&self) {
        if self.colors_enabled {
            eprint!("{} {}", "◆".cyan().bold(), "DX".white().bold());
        } else {
            eprint!("◆ DX");
        }
    }

    pub fn print_banner(&self, title: &str) {
        let version = env!("CARGO_PKG_VERSION");
        eprintln!();
        if self.colors_enabled {
            eprintln!("  {}  {}", "◆".cyan().bold(), title.white().bold());
            eprintln!("     {}", format!("v{version}").bright_black());
        } else {
            eprintln!("  ◆  {}", title);
            eprintln!("     v{}", version);
        }
        eprintln!();
    }

    pub fn print_header(&self) {
        self.print_logo();
    }

    pub fn success(&self, message: &str) {
        if self.colors_enabled {
            eprintln!("  {} {}", icons::SUCCESS.green().bold(), message.white());
        } else {
            eprintln!("  {} {}", icons::SUCCESS, message);
        }
    }

    pub fn print_success(&self, message: &str) {
        eprintln!();
        self.success(message);
    }

    pub fn error(&self, message: &str) {
        if self.colors_enabled {
            eprintln!("  {} {}", icons::ERROR.red().bold(), message.red());
        } else {
            eprintln!("  {} {}", icons::ERROR, message);
        }
    }

    pub fn print_error(&self, message: &str) {
        eprintln!();
        self.error(message);
    }

    pub fn info(&self, message: &str) {
        if self.colors_enabled {
            eprintln!("  {} {}", icons::ARROW.cyan(), message.white());
        } else {
            eprintln!("  {} {}", icons::ARROW, message);
        }
    }

    pub fn warn(&self, message: &str) {
        if self.colors_enabled {
            eprintln!("  {} {}", icons::WARNING.yellow().bold(), message.yellow());
        } else {
            eprintln!("  {} {}", icons::WARNING, message);
        }
    }

    pub fn print_warning(&self, message: &str) {
        self.warn(message);
    }

    pub fn step(&self, current: usize, total: usize, message: &str) {
        let step_info = format!("[{}/{}]", current, total);
        if self.colors_enabled {
            eprintln!("  {} {} {}", step_info.bright_black(), icons::ARROW.cyan(), message.white());
        } else {
            eprintln!("  {} {} {}", step_info, icons::ARROW, message);
        }
    }

    pub fn print_step(&self, step: usize, total: usize, message: &str) {
        self.step(step, total, message);
    }

    pub fn hint(&self, message: &str) {
        if self.colors_enabled {
            eprintln!("  {} {}", "hint:".bright_black(), message.bright_black());
        } else {
            eprintln!("  hint: {}", message);
        }
    }

    pub fn suggest_command(&self, cmd: &str) {
        eprintln!();
        if self.colors_enabled {
            eprintln!(
                "  {} Run {} to get started",
                icons::ARROW.cyan(),
                format!("`{cmd}`").cyan().bold()
            );
        } else {
            eprintln!("  {} Run `{}` to get started", icons::ARROW, cmd);
        }
    }

    pub fn print_hint(&self, command: &str) {
        self.suggest_command(command);
    }

    pub fn print_section(&self, title: &str) {
        if self.colors_enabled {
            eprintln!("  {} {}", icons::VERTICAL.bright_black(), title.bright_white().bold());
        } else {
            eprintln!("  {} {}", icons::VERTICAL, title);
        }
    }

    pub fn print_info(&self, label: &str, value: &str) {
        if self.colors_enabled {
            eprintln!(
                "  {} {}: {}",
                icons::VERTICAL.bright_black(),
                label.bright_black(),
                value.white()
            );
        } else {
            eprintln!("  {} {}: {}", icons::VERTICAL, label, value);
        }
    }

    pub fn print_link(&self, label: &str, url: &str) {
        if self.colors_enabled {
            eprintln!(
                "  {} {}: {}",
                icons::VERTICAL.bright_black(),
                label.bright_black(),
                url.cyan().underline()
            );
        } else {
            eprintln!("  {} {}: {}", icons::VERTICAL, label, url);
        }
    }

    pub fn print_divider(&self) {
        if self.colors_enabled {
            eprintln!("  {}", icons::HORIZONTAL.repeat(48).bright_black());
        } else {
            eprintln!("  {}", icons::HORIZONTAL.repeat(48));
        }
    }

    #[allow(dead_code)]
    pub fn print_empty(&self) {
        if self.colors_enabled {
            eprintln!("  {}", icons::VERTICAL.bright_black());
        } else {
            eprintln!("  {}", icons::VERTICAL);
        }
    }

    pub fn print_ready(&self, url: &str, time_ms: u64) {
        eprintln!();
        if self.colors_enabled {
            eprintln!(
                "  {} Ready in {}",
                icons::SUCCESS.green().bold(),
                format!("{time_ms}ms").cyan().bold()
            );
            eprintln!();
            eprintln!("  {} Local:   {}", icons::ARROW.cyan(), url.cyan().bold().underline());
        } else {
            eprintln!("  {} Ready in {}ms", icons::SUCCESS, time_ms);
            eprintln!();
            eprintln!("  {} Local:   {}", icons::ARROW, url);
        }
        eprintln!();
    }

    pub fn print_cancelled(&self) {
        eprintln!();
        if self.colors_enabled {
            eprintln!("  {} Cancelled", icons::BULLET_EMPTY.bright_black());
        } else {
            eprintln!("  {} Cancelled", icons::BULLET_EMPTY);
        }
        eprintln!();
    }

    pub fn print_build_stats(&self, duration_ms: u64, bundle_size: &str, files: usize) {
        eprintln!();
        self.print_divider();
        if self.colors_enabled {
            eprintln!(
                "  {} Built in {} {} {} {} {} files",
                icons::SUCCESS.green().bold(),
                format!("{duration_ms}ms").cyan().bold(),
                icons::VERTICAL.bright_black(),
                bundle_size.magenta().bold(),
                icons::VERTICAL.bright_black(),
                files.to_string().white().bold()
            );
        } else {
            eprintln!(
                "  {} Built in {}ms {} {} {} {} files",
                icons::SUCCESS,
                duration_ms,
                icons::VERTICAL,
                bundle_size,
                icons::VERTICAL,
                files
            );
        }
        self.print_divider();
        eprintln!();
    }

    pub fn print_test_results(
        &self,
        passed: usize,
        failed: usize,
        skipped: usize,
        duration_ms: u64,
    ) {
        eprintln!();
        self.print_divider();

        if self.colors_enabled {
            let status = if failed == 0 {
                "PASS".green().bold().to_string()
            } else {
                "FAIL".red().bold().to_string()
            };

            let failed_str = if failed > 0 {
                failed.to_string().red().bold().to_string()
            } else {
                failed.to_string().bright_black().to_string()
            };

            eprintln!(
                "  {} {} {} passed {} {} failed {} {} skipped {} in {}",
                status,
                icons::VERTICAL.bright_black(),
                passed.to_string().green().bold(),
                icons::VERTICAL.bright_black(),
                failed_str,
                icons::VERTICAL.bright_black(),
                skipped.to_string().bright_black(),
                icons::VERTICAL.bright_black(),
                format!("{duration_ms}ms").cyan()
            );
        } else {
            let status = if failed == 0 { "PASS" } else { "FAIL" };
            eprintln!(
                "  {} {} {} passed {} {} failed {} {} skipped {} in {}ms",
                status,
                icons::VERTICAL,
                passed,
                icons::VERTICAL,
                failed,
                icons::VERTICAL,
                skipped,
                icons::VERTICAL,
                duration_ms
            );
        }

        self.print_divider();
        eprintln!();
    }

    #[allow(dead_code)]
    pub fn width(&self) -> usize {
        self.term.size().1 as usize
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        let _ = self.term.clear_screen();
    }

    pub fn format_success(&self, message: &str) -> String {
        if self.colors_enabled {
            format!("  {} {}", icons::SUCCESS.green().bold(), message.white())
        } else {
            format!("  {} {}", icons::SUCCESS, message)
        }
    }

    pub fn format_error(&self, message: &str) -> String {
        if self.colors_enabled {
            format!("  {} {}", icons::ERROR.red().bold(), message.red())
        } else {
            format!("  {} {}", icons::ERROR, message)
        }
    }

    pub fn format_info(&self, message: &str) -> String {
        if self.colors_enabled {
            format!("  {} {}", icons::ARROW.cyan(), message.white())
        } else {
            format!("  {} {}", icons::ARROW, message)
        }
    }

    pub fn format_warn(&self, message: &str) -> String {
        if self.colors_enabled {
            format!("  {} {}", icons::WARNING.yellow().bold(), message.yellow())
        } else {
            format!("  {} {}", icons::WARNING, message)
        }
    }

    pub fn format_step(&self, current: usize, total: usize, message: &str) -> String {
        let step_info = format!("[{}/{}]", current, total);
        if self.colors_enabled {
            format!("  {} {} {}", step_info.bright_black(), icons::ARROW.cyan(), message.white())
        } else {
            format!("  {} {} {}", step_info, icons::ARROW, message)
        }
    }
}

impl LegacyTheme for Theme {}
