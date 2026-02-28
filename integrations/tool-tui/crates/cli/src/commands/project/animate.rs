//! Animation commands

use anyhow::Result;

use crate::cli::{AnimateArgs, AnimateCommand, AnimationType};
use crate::ui::theme::Theme;

pub fn run_animate(args: AnimateArgs, theme: &Theme) -> Result<()> {
    match args.command {
        AnimateCommand::Show {
            animation,
            duration,
            message,
        } => run_animation_show(animation, duration, message, theme),
        AnimateCommand::Sound { sound, visualize } => run_animation_sound(&sound, visualize, theme),
        AnimateCommand::Image { path, ascii } => run_animation_image(&path, ascii, theme),
        AnimateCommand::Gif { path, duration } => run_animation_gif(&path, duration, theme),
        AnimateCommand::Video { path, duration } => run_animation_video(&path, duration, theme),
        AnimateCommand::Download {
            url,
            media_type,
            ascii,
            duration,
        } => run_animation_download(&url, media_type, ascii, duration, theme),
    }
}

fn run_animation_show(
    animation: AnimationType,
    duration: Option<u64>,
    _message: Option<String>,
    _theme: &Theme,
) -> Result<()> {
    use std::time::Duration;

    let duration_secs = duration.unwrap_or(5);

    match animation {
        AnimationType::Matrix => {
            let mut matrix = crate::ui::animations::matrix::MatrixRain::new()
                .with_duration(Duration::from_secs(duration_secs));
            matrix.run().map_err(|e| anyhow::anyhow!("Matrix error: {}", e))
        }
        AnimationType::Train => {
            let train = crate::ui::animations::train::TrainAnimation::new()
                .with_duration(Duration::from_secs(duration_secs));
            train.run().map_err(|e| anyhow::anyhow!("Train error: {}", e))
        }
        AnimationType::Confetti => crate::ui::animations::confetti::show_confetti()
            .map_err(|e| anyhow::anyhow!("Confetti error: {}", e)),
        AnimationType::Starfield => {
            let mut starfield = crate::ui::animations::particles::Starfield::new(100)
                .with_duration(Duration::from_secs(duration_secs));
            starfield.run().map_err(|e| anyhow::anyhow!("Starfield error: {}", e))
        }
        AnimationType::Rain => {
            // Rain animation not available in particles module
            Err(anyhow::anyhow!("Rain animation not fully implemented yet"))
        }
        AnimationType::Life | AnimationType::Nyan | AnimationType::Dvd => {
            // These animations don't have with_duration yet
            Err(anyhow::anyhow!("{:?} animation not fully implemented yet", animation))
        }
        AnimationType::All => {
            // Run all working animations in sequence
            let animations = vec![
                AnimationType::Matrix,
                AnimationType::Train,
                AnimationType::Starfield,
                AnimationType::Confetti,
            ];

            for anim in animations {
                run_animation_show(anim, Some(3), None, _theme)?;
            }
            Ok(())
        }
    }
}

fn run_animation_sound(_sound: &str, _visualize: bool, theme: &Theme) -> Result<()> {
    theme.info("Sound playback not yet implemented");
    Ok(())
}

fn run_animation_image(_path: &str, _ascii: bool, theme: &Theme) -> Result<()> {
    theme.info("Image display not yet implemented");
    Ok(())
}

fn run_animation_gif(_path: &str, _duration: u64, theme: &Theme) -> Result<()> {
    theme.info("GIF playback not yet implemented");
    Ok(())
}

fn run_animation_video(_path: &str, _duration: u64, theme: &Theme) -> Result<()> {
    theme.info("Video playback not yet implemented");
    Ok(())
}

fn run_animation_download(
    _url: &str,
    _media_type: Option<crate::cli::MediaType>,
    _ascii: bool,
    _duration: u64,
    theme: &Theme,
) -> Result<()> {
    theme.info("Media download not yet implemented");
    Ok(())
}
