//! Animation and media command arguments

use clap::{Args, Subcommand, ValueEnum};

/// Arguments for the animate command
#[derive(Args)]
pub struct AnimateArgs {
    #[command(subcommand)]
    pub command: AnimateCommand,
}

#[derive(Subcommand)]
pub enum AnimateCommand {
    /// Show an animation
    Show {
        /// Animation type to show
        #[arg(value_enum)]
        animation: AnimationType,

        /// Duration in seconds (default varies by animation)
        #[arg(short, long)]
        duration: Option<u64>,

        /// Message to display with animation
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Play a sound effect
    Sound {
        /// Sound type or path/URL to audio file
        sound: String,

        /// Visualize audio waveform
        #[arg(long)]
        visualize: bool,
    },

    /// Display an image in the terminal
    Image {
        /// Path to image file
        path: String,

        /// Convert to ASCII art instead of direct display
        #[arg(long)]
        ascii: bool,
    },

    /// Play a GIF animation
    Gif {
        /// Path or URL to GIF file
        path: String,

        /// Duration to play (loops if shorter than GIF)
        #[arg(short, long, default_value = "10")]
        duration: u64,
    },

    /// Play a video file
    Video {
        /// Path or URL to video file (GIF recommended for terminal)
        path: String,

        /// Duration to play
        #[arg(short, long, default_value = "10")]
        duration: u64,
    },

    /// Download and display media from URL
    Download {
        /// URL to download from
        url: String,

        /// Media type (auto-detected if not specified)
        #[arg(short, long, value_enum)]
        media_type: Option<MediaType>,

        /// Convert images to ASCII art
        #[arg(long)]
        ascii: bool,

        /// Duration for GIF playback
        #[arg(short, long, default_value = "10")]
        duration: u64,
    },
}

#[derive(ValueEnum, Clone)]
pub enum SoundType {
    /// Simple beep
    Beep,
    /// Train whistle (choo choo!)
    Train,
    /// Success sound
    Success,
    /// Error sound
    Error,
}

#[derive(ValueEnum, Clone)]
pub enum MediaType {
    /// Image file
    Image,
    /// GIF animation
    Gif,
    /// Video file
    Video,
    /// Sound effect
    Sound,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum AnimationType {
    /// Matrix digital rain (the ultimate thinking loader)
    Matrix,
    /// Steam locomotive train (fun error/celebration)
    Train,
    /// Confetti explosion (victory celebration)
    Confetti,
    /// Starfield (ambient/idle mode)
    Starfield,
    /// Rain effect (particle system demo)
    Rain,
    /// Conway's Game of Life (screensaver)
    Life,
    /// Nyan Cat flying across screen
    Nyan,
    /// Bouncing DVD logo
    Dvd,
    /// Show all animations in sequence
    All,
}

/// Arguments for the token command
#[derive(Args)]
pub struct TokenArgs {
    #[command(flatten)]
    pub args: crate::commands::tokens::TokensArgs,
}
