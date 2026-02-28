//! dx sound: Audio playback with visualizer
//!
//! Play audio files with real-time terminal visualizations

use anyhow::Result;
use clap::{Args, Subcommand};
use owo_colors::OwoColorize;

use crate::ui::{animations::visualizer::VisualizerStyle, theme::Theme};

#[derive(Args)]
pub struct SoundArgs {
    #[command(subcommand)]
    pub command: SoundCommands,
}

#[derive(Subcommand)]
pub enum SoundCommands {
    /// Play audio file with visualizer
    Play {
        /// Audio file path
        #[arg(index = 1)]
        file: String,

        /// Visualizer style (bars, spectrum, vu, oscilloscope, radial)
        #[arg(short, long, default_value = "bars")]
        style: String,

        /// Disable visualizer
        #[arg(long)]
        no_viz: bool,
    },

    /// Play audio from URL
    Url {
        /// Audio URL
        #[arg(index = 1)]
        url: String,

        /// Visualizer style
        #[arg(short, long, default_value = "bars")]
        style: String,

        /// Disable visualizer
        #[arg(long)]
        no_viz: bool,
    },

    /// Demo visualizer with generated audio
    Demo {
        /// Visualizer style (bars, spectrum, vu, oscilloscope, radial)
        #[arg(short, long, default_value = "bars")]
        style: String,

        /// Duration in seconds
        #[arg(short, long, default_value = "10")]
        duration: u64,
    },

    /// List available sound effects
    List,
}

pub async fn run(args: SoundArgs, theme: &Theme) -> Result<()> {
    match args.command {
        SoundCommands::Play {
            file,
            style,
            no_viz,
        } => run_play(&file, &style, !no_viz, theme).await,
        SoundCommands::Url { url, style, no_viz } => run_url(&url, &style, !no_viz, theme).await,
        SoundCommands::Demo { style, duration } => run_demo(&style, duration, theme).await,
        SoundCommands::List => run_list(theme).await,
    }
}

async fn run_play(file: &str, style: &str, show_viz: bool, theme: &Theme) -> Result<()> {
    theme.print_section("Audio Player");
    eprintln!();

    let path = std::path::Path::new(file);
    if !path.exists() {
        eprintln!("   {} File not found: {}", "✗".red(), file.red());
        return Ok(());
    }

    eprintln!("   {} {}", "File".bright_black(), file.cyan());
    if show_viz {
        eprintln!(
            "   {} {} ({})",
            "Mode".bright_black(),
            "visualizer".green(),
            style.bright_black()
        );
    } else {
        eprintln!("   {} {}", "Mode".bright_black(), "audio only".bright_black());
    }
    eprintln!();

    // Play with visualizer
    use crate::ui::animations::sounds;
    sounds::play_audio_file_with_visualizer(path, show_viz)?;

    eprintln!();
    Ok(())
}

async fn run_url(url: &str, style: &str, show_viz: bool, theme: &Theme) -> Result<()> {
    theme.print_section("dx sound: Stream from URL");
    eprintln!();

    eprintln!("  {} URL: {}", "│".bright_black(), url.cyan());
    eprintln!(
        "  {} Visualizer: {}",
        "│".bright_black(),
        if show_viz {
            format!("{} ({})", "enabled".green(), style.cyan())
        } else {
            "disabled".bright_black().to_string()
        }
    );
    eprintln!();

    // Download and play with visualizer
    use crate::ui::animations::sounds;
    sounds::play_audio_from_url_with_visualizer(url, show_viz)?;

    Ok(())
}

async fn run_demo(style: &str, duration: u64, theme: &Theme) -> Result<()> {
    theme.print_section("dx sound: Visualizer Demo");
    eprintln!();

    let viz_style = match style.to_lowercase().as_str() {
        "bars" => VisualizerStyle::Bars,
        "spectrum" => VisualizerStyle::Spectrum,
        "vu" | "vumeter" => VisualizerStyle::VuMeter,
        "oscilloscope" | "scope" => VisualizerStyle::Oscilloscope,
        "radial" | "circular" => VisualizerStyle::Radial,
        _ => {
            eprintln!("  {} Unknown style: {}", "✗".red(), style.red());
            eprintln!("  {} Available: bars, spectrum, vu, oscilloscope, radial", "→".cyan());
            return Ok(());
        }
    };

    eprintln!("  {} Style: {}", "│".bright_black(), style.cyan());
    eprintln!("  {} Duration: {}s", "│".bright_black(), duration.to_string().cyan());
    eprintln!();
    eprintln!("  {} Starting demo...", "→".cyan());
    eprintln!();

    // Run demo
    use crate::ui::animations::visualizer;
    visualizer::demo_visualizer(viz_style, duration)?;

    eprintln!();
    eprintln!("  {} Demo complete!", "✓".green());
    eprintln!();

    Ok(())
}

async fn run_list(theme: &Theme) -> Result<()> {
    theme.print_section("Available Sounds");
    eprintln!();

    let sounds = [
        ("notification-click-sound-455421.mp3", "Notification", "12 KB"),
        ("menu-open-sound-effect-432999.mp3", "Menu open", "8 KB"),
        ("soft-treble-win-fade-out-ending-sound-effect-416829.mp3", "Success", "24 KB"),
        ("ui-3-sound-effect-warn-242229.mp3", "Warning", "6 KB"),
        ("digital-unlock-433002.mp3", "Unlock", "15 KB"),
        ("new-notification-444814.mp3", "Notification", "9 KB"),
    ];

    for (file, desc, size) in sounds {
        let exists = std::path::Path::new(file).exists();

        if exists {
            eprintln!(
                "   {} {} • {} • {}",
                "✓".green(),
                desc.white(),
                size.bright_black(),
                file.bright_black()
            );
        } else {
            eprintln!(
                "   {} {} • {} • {}",
                "✗".red(),
                desc.white(),
                size.bright_black(),
                file.bright_black()
            );
        }
    }

    eprintln!();
    eprintln!("   {} Visualizer Styles", "Styles".bright_black());
    eprintln!("   {} bars • spectrum • vu • oscilloscope • radial", "│".bright_black());
    eprintln!();

    eprintln!("   {} Usage", "Usage".bright_black());
    eprintln!("   {} dx sound play <file>", "│".bright_black());
    eprintln!("   {} dx sound demo --style spectrum", "│".bright_black());
    eprintln!();

    Ok(())
}
