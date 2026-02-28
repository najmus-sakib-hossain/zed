//! Media processing tools commands

use anyhow::Result;
use std::path::PathBuf;

use crate::cli_unified::args::{
    ArchiveToolCommands, AudioToolCommands, ImageToolCommands, ToolCommands, VideoToolCommands,
};
use crate::cli_unified::output::{print_info, print_success};

pub async fn execute_tool_command(command: ToolCommands) -> Result<()> {
    match command {
        ToolCommands::Image { command } => execute_image_tool(command).await,
        ToolCommands::Video { command } => execute_video_tool(command).await,
        ToolCommands::Audio { command } => execute_audio_tool(command).await,
        ToolCommands::Archive { command } => execute_archive_tool(command).await,
    }
}

async fn execute_image_tool(command: ImageToolCommands) -> Result<()> {
    match command {
        ImageToolCommands::Convert {
            input,
            output,
            quality,
        } => {
            print_info(&format!("🖼️  Converting {} to {}...", input.display(), output.display()));

            #[cfg(feature = "image-core")]
            {
                use crate::tools::image::native::convert_native;
                match convert_native(&input, &output, quality) {
                    Ok(_) => print_success("Image converted successfully!"),
                    Err(e) => anyhow::bail!("Conversion failed: {}", e),
                }
                return Ok(());
            }

            #[cfg(not(feature = "image-core"))]
            {
                let _ = (input, output, quality);
                anyhow::bail!("Image tools not enabled. Rebuild with --features image-core")
            }
        }
        ImageToolCommands::Resize {
            input,
            output,
            width,
            height,
        } => {
            print_info(&format!("🖼️  Resizing {}...", input.display()));

            #[cfg(feature = "image-core")]
            {
                use crate::tools::image::native::resize_native;
                match resize_native(&input, &output, width, height, true) {
                    Ok(_) => print_success("Image resized successfully!"),
                    Err(e) => anyhow::bail!("Resize failed: {}", e),
                }
                return Ok(());
            }

            #[cfg(not(feature = "image-core"))]
            {
                let _ = (input, output, width, height);
                anyhow::bail!("Image tools not enabled. Rebuild with --features image-core")
            }
        }
        ImageToolCommands::Favicon { input, output } => {
            print_info(&format!("🎨 Generating favicons from {}...", input.display()));

            #[cfg(feature = "image-svg")]
            {
                use crate::tools::image::svg::generate_web_icons;
                generate_web_icons(&input, &output)?;
                print_success(&format!("Favicons generated in {}", output.display()));
                return Ok(());
            }

            #[cfg(not(feature = "image-svg"))]
            {
                let _ = input;
                anyhow::bail!("SVG tools not enabled. Rebuild with --features image-svg")
            }
        }
    }
}

async fn execute_video_tool(command: VideoToolCommands) -> Result<()> {
    match command {
        VideoToolCommands::Convert { input, output } => {
            print_info(&format!("🎬 Converting {} to {}...", input.display(), output.display()));
            print_success("Video conversion requires FFmpeg. Feature coming soon!");
            Ok(())
        }
        VideoToolCommands::ExtractAudio { input, output: _ } => {
            print_info(&format!("🎵 Extracting audio from {}...", input.display()));
            print_success("Audio extraction requires FFmpeg. Feature coming soon!");
            Ok(())
        }
        VideoToolCommands::ToGif {
            input,
            output: _,
            fps,
        } => {
            print_info(&format!("🎞️  Converting {} to GIF ({}fps)...", input.display(), fps));
            print_success("GIF conversion requires FFmpeg. Feature coming soon!");
            Ok(())
        }
    }
}

async fn execute_audio_tool(command: AudioToolCommands) -> Result<()> {
    match command {
        AudioToolCommands::Convert { input, output } => {
            print_info(&format!("🎵 Converting {} to {}...", input.display(), output.display()));
            print_success("Audio conversion requires FFmpeg. Feature coming soon!");
            Ok(())
        }
        AudioToolCommands::Trim {
            input,
            output: _,
            start,
            duration,
        } => {
            print_info(&format!(
                "✂️  Trimming {} (start: {}s, duration: {}s)...",
                input.display(),
                start,
                duration
            ));
            print_success("Audio trimming requires FFmpeg. Feature coming soon!");
            Ok(())
        }
    }
}

async fn execute_archive_tool(command: ArchiveToolCommands) -> Result<()> {
    match command {
        ArchiveToolCommands::Zip { files, output } => {
            print_info(&format!("📦 Creating archive {}...", output.display()));

            #[cfg(feature = "archive-core")]
            {
                use crate::tools::ArchiveTools;
                let tools = ArchiveTools::new();

                let file_paths: Vec<&PathBuf> = files.iter().collect();
                tools.create_zip(&file_paths, &output)?;

                print_success(&format!("Archive created: {}", output.display()));
            }

            #[cfg(not(feature = "archive-core"))]
            {
                anyhow::bail!("Archive tools not enabled. Rebuild with --features archive-core");
            }

            Ok(())
        }
        ArchiveToolCommands::Extract { input, output } => {
            print_info(&format!("📂 Extracting {}...", input.display()));

            #[cfg(feature = "archive-core")]
            {
                use crate::tools::ArchiveTools;
                let tools = ArchiveTools::new();

                tools.extract_zip(&input, &output)?;

                print_success(&format!("Extracted to: {}", output.display()));
            }

            #[cfg(not(feature = "archive-core"))]
            {
                anyhow::bail!("Archive tools not enabled. Rebuild with --features archive-core");
            }

            Ok(())
        }
    }
}
